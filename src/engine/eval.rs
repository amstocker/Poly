use std::collections::BTreeMap;
use super::{BinOp, Engine, Expr, SchemaBody, Sym, UnOp};


// ============================================================================
// Partial evaluation (the "trivial simplifier" — constant folding only)
// ============================================================================
//
// `const_fold` walks an expression and reduces every subexpression that
// becomes fully bound by the env, leaving the rest symbolic. The result is
// always an `Expr<Sym>`: a fully-reduced expression collapses to a literal,
// an unbindable expression is returned unchanged, and a partially-bound
// expression is returned with the reduced parts folded in place.
//
// This is the leaf operator used by `simplify::reduce`. By itself it handles
// (a) substituting env values into the expression and (b) folding any
// subexpression whose operands have all become literals — but it does no
// algebraic reasoning beyond constant folding (no `n + 0 → n`, no interval
// narrowing, no contradiction detection — those live in `simplify`).


// ============================================================================
// Values and bindings
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Record { schema: Sym, fields: BTreeMap<Sym, Value> },
}

pub type Bindings = BTreeMap<Sym, Value>;

#[derive(Clone, Debug)]
pub enum EvalError {
    Unbound(Sym),
    UnknownSchema(Sym),
    UnknownField { schema: Sym, field: Sym },
    NotARecord,
    SumNotSupported(Sym),
    ConstructArity { schema: Sym, expected: usize, got: usize },
    TypeMismatch { op: &'static str },
    DivByZero,
}


// ============================================================================
// Evaluator
// ============================================================================

pub fn eval(eng: &Engine, e: &Expr<Sym>, b: &Bindings) -> Result<Value, EvalError> {
    match e {
        Expr::LitInt(n) => Ok(Value::Int(*n)),
        Expr::LitStr(s) => Ok(Value::Str(s.clone())),
        Expr::LitBool(v) => Ok(Value::Bool(*v)),
        Expr::Var(s) => b.get(s).cloned().ok_or(EvalError::Unbound(*s)),
        Expr::Field(base, name) => {
            let v = eval(eng, base, b)?;
            match v {
                Value::Record { schema, fields } => fields
                    .get(name)
                    .cloned()
                    .ok_or(EvalError::UnknownField { schema, field: *name }),
                _ => Err(EvalError::NotARecord),
            }
        }
        Expr::Construct(name, args) => {
            let schema = eng.schemas.get(name).ok_or(EvalError::UnknownSchema(*name))?;
            match &schema.body {
                SchemaBody::Record(params) => {
                    if params.len() != args.len() {
                        return Err(EvalError::ConstructArity {
                            schema: *name,
                            expected: params.len(),
                            got: args.len(),
                        });
                    }
                    let mut fields: BTreeMap<Sym, Value> = BTreeMap::new();
                    for (p, a) in params.iter().zip(args.iter()) {
                        fields.insert(p.name, eval(eng, a, b)?);
                    }
                    Ok(Value::Record { schema: *name, fields })
                }
                SchemaBody::Sum(_) => Err(EvalError::SumNotSupported(*name)),
            }
        }
        Expr::UnOp(op, inner) => {
            let v = eval(eng, inner, b)?;
            match (op, v) {
                (UnOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
                (UnOp::Not, Value::Bool(p)) => Ok(Value::Bool(!p)),
                _ => Err(EvalError::TypeMismatch { op: "unary" }),
            }
        }
        Expr::BinOp(op, l, r) => {
            let lv = eval(eng, l, b)?;
            let rv = eval(eng, r, b)?;
            eval_binop(*op, lv, rv)
        }
    }
}

fn eval_binop(op: BinOp, l: Value, r: Value) -> Result<Value, EvalError> {
    use BinOp::*;
    use Value::*;
    match (op, l, r) {
        (Add, Int(a), Int(b)) => Ok(Int(a + b)),
        (Sub, Int(a), Int(b)) => Ok(Int(a - b)),
        (Mul, Int(a), Int(b)) => Ok(Int(a * b)),
        (Div, Int(_), Int(0)) | (Mod, Int(_), Int(0)) => Err(EvalError::DivByZero),
        (Div, Int(a), Int(b)) => Ok(Int(a / b)),
        (Mod, Int(a), Int(b)) => Ok(Int(a % b)),
        (Eq, a, b) => Ok(Bool(a == b)),
        (Neq, a, b) => Ok(Bool(a != b)),
        (Lt, Int(a), Int(b)) => Ok(Bool(a < b)),
        (Le, Int(a), Int(b)) => Ok(Bool(a <= b)),
        (Gt, Int(a), Int(b)) => Ok(Bool(a > b)),
        (Ge, Int(a), Int(b)) => Ok(Bool(a >= b)),
        (And, Bool(a), Bool(b)) => Ok(Bool(a && b)),
        (Or, Bool(a), Bool(b)) => Ok(Bool(a || b)),
        _ => Err(EvalError::TypeMismatch { op: "binary" }),
    }
}

pub fn eval_bool(eng: &Engine, e: &Expr<Sym>, b: &Bindings) -> Result<bool, EvalError> {
    match eval(eng, e, b)? {
        Value::Bool(p) => Ok(p),
        _ => Err(EvalError::TypeMismatch { op: "guard" }),
    }
}

pub fn const_fold(eng: &Engine, e: &Expr<Sym>, b: &Bindings) -> Expr<Sym> {
    match e {
        Expr::LitInt(_) | Expr::LitStr(_) | Expr::LitBool(_) => e.clone(),
        Expr::Var(s) => match b.get(s) {
            Some(v) => value_to_expr(eng, v).unwrap_or_else(|| e.clone()),
            None => e.clone(),
        },
        Expr::UnOp(op, inner) => {
            let inner_s = const_fold(eng, inner, b);
            if let Some(v) = expr_as_value(&inner_s) {
                if let Ok(folded) = eval_unop(*op, v) {
                    if let Some(ex) = value_to_expr(eng, &folded) {
                        return ex;
                    }
                }
            }
            Expr::UnOp(*op, Box::new(inner_s))
        }
        Expr::BinOp(op, l, r) => {
            let ls = const_fold(eng, l, b);
            let rs = const_fold(eng, r, b);
            if let (Some(lv), Some(rv)) = (expr_as_value(&ls), expr_as_value(&rs)) {
                if let Ok(folded) = eval_binop(*op, lv, rv) {
                    if let Some(ex) = value_to_expr(eng, &folded) {
                        return ex;
                    }
                }
            }
            Expr::BinOp(*op, Box::new(ls), Box::new(rs))
        }
        Expr::Field(base, name) => {
            let bs = const_fold(eng, base, b);
            if let Expr::Construct(schema, args) = &bs {
                if let Some(s) = eng.schemas.get(schema) {
                    if let SchemaBody::Record(params) = &s.body {
                        if let Some(idx) = params.iter().position(|p| p.name == *name) {
                            return args[idx].clone();
                        }
                    }
                }
            }
            Expr::Field(Box::new(bs), *name)
        }
        Expr::Construct(name, args) => {
            let args_s: Vec<_> = args.iter().map(|a| const_fold(eng, a, b)).collect();
            Expr::Construct(*name, args_s)
        }
    }
}

fn eval_unop(op: UnOp, v: Value) -> Result<Value, EvalError> {
    match (op, v) {
        (UnOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
        (UnOp::Not, Value::Bool(p)) => Ok(Value::Bool(!p)),
        _ => Err(EvalError::TypeMismatch { op: "unary" }),
    }
}

fn expr_as_value(e: &Expr<Sym>) -> Option<Value> {
    match e {
        Expr::LitInt(n) => Some(Value::Int(*n)),
        Expr::LitBool(p) => Some(Value::Bool(*p)),
        Expr::LitStr(s) => Some(Value::Str(s.clone())),
        _ => None,
    }
}

fn value_to_expr(eng: &Engine, v: &Value) -> Option<Expr<Sym>> {
    match v {
        Value::Int(n) => Some(Expr::LitInt(*n)),
        Value::Bool(p) => Some(Expr::LitBool(*p)),
        Value::Str(s) => Some(Expr::LitStr(s.clone())),
        Value::Record { schema, fields } => {
            let s = eng.schemas.get(schema)?;
            let SchemaBody::Record(params) = &s.body else { return None };
            let args: Option<Vec<Expr<Sym>>> = params
                .iter()
                .map(|p| fields.get(&p.name).and_then(|fv| value_to_expr(eng, fv)))
                .collect();
            Some(Expr::Construct(*schema, args?))
        }
    }
}

pub fn conjoin(parts: &[Expr<Sym>]) -> Option<Expr<Sym>> {
    let mut iter = parts.iter().cloned();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, e| {
        Expr::BinOp(BinOp::And, Box::new(acc), Box::new(e))
    }))
}


// ============================================================================
// Display
// ============================================================================

impl Engine {
    pub fn fmt_value(&self, v: &Value) -> String {
        match v {
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => format!("\"{s}\""),
            Value::Record { schema, fields } => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", self.resolve(*k), self.fmt_value(v)))
                    .collect();
                format!("{}({})", self.resolve(*schema), parts.join(", "))
            }
        }
    }

    pub fn fmt_bindings(&self, b: &Bindings) -> String {
        if b.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = b
            .iter()
            .map(|(k, v)| format!("{}={}", self.resolve(*k), self.fmt_value(v)))
            .collect();
        format!("[{}]", parts.join(", "))
    }

    pub fn fmt_eval_error(&self, e: &EvalError) -> String {
        match e {
            EvalError::Unbound(s) => format!("unbound variable: {}", self.resolve(*s)),
            EvalError::UnknownSchema(s) => format!("unknown schema: {}", self.resolve(*s)),
            EvalError::UnknownField { schema, field } => format!(
                "schema {} has no field {}",
                self.resolve(*schema),
                self.resolve(*field),
            ),
            EvalError::NotARecord => "field access on non-record value".to_string(),
            EvalError::SumNotSupported(s) => {
                format!("sum constructors not yet supported: {}", self.resolve(*s))
            }
            EvalError::ConstructArity { schema, expected, got } => format!(
                "constructor {} has {} arg(s), expected {}",
                self.resolve(*schema),
                got,
                expected,
            ),
            EvalError::TypeMismatch { op } => format!("type mismatch in {op} expression"),
            EvalError::DivByZero => "division by zero".to_string(),
        }
    }
}
