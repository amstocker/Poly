use std::collections::BTreeMap;
use super::{BinOp, Engine, Expr, Sym, UnOp};


// ============================================================================
// Values and bindings
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
}

pub type Bindings = BTreeMap<Sym, Value>;

#[derive(Clone, Debug)]
pub enum EvalError {
    Unbound(Sym),
    TypeMismatch { op: &'static str },
    DivByZero,
    NotSupported(&'static str),
}


// ============================================================================
// Evaluator
// ============================================================================

pub fn eval(e: &Expr<Sym>, b: &Bindings) -> Result<Value, EvalError> {
    match e {
        Expr::LitInt(n) => Ok(Value::Int(*n)),
        Expr::LitStr(s) => Ok(Value::Str(s.clone())),
        Expr::LitBool(v) => Ok(Value::Bool(*v)),
        Expr::Var(s) => b.get(s).cloned().ok_or(EvalError::Unbound(*s)),
        Expr::Field(_, _) => Err(EvalError::NotSupported("field access")),
        Expr::Construct(_, _) => Err(EvalError::NotSupported("constructor")),
        Expr::UnOp(op, inner) => {
            let v = eval(inner, b)?;
            match (op, v) {
                (UnOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
                (UnOp::Not, Value::Bool(p)) => Ok(Value::Bool(!p)),
                _ => Err(EvalError::TypeMismatch { op: "unary" }),
            }
        }
        Expr::BinOp(op, l, r) => {
            let lv = eval(l, b)?;
            let rv = eval(r, b)?;
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

pub fn eval_bool(e: &Expr<Sym>, b: &Bindings) -> Result<bool, EvalError> {
    match eval(e, b)? {
        Value::Bool(p) => Ok(p),
        _ => Err(EvalError::TypeMismatch { op: "guard" }),
    }
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
            EvalError::TypeMismatch { op } => format!("type mismatch in {op} expression"),
            EvalError::DivByZero => "division by zero".to_string(),
            EvalError::NotSupported(what) => format!("not supported in evaluator: {what}"),
        }
    }
}
