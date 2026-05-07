use std::collections::BTreeMap;
use super::{BinOp, Engine, Expr, SchemaBody, Sym, UnOp};


// ============================================================================
// Values and bindings
//
// `Bindings` is the env passed into `simplify::reduce` (and through
// `uquery::run_query`): a partial map from `Sym` (parameter name) to
// concrete `Value`. The simplifier substitutes Var(s) for the
// corresponding Value when present, leaves it symbolic otherwise.
// `conjoin` collapses a Vec<Expr> of residual conjuncts into one Expr.
// `const_fold` is the leaf operator for `simplify::reduce`: walk an
// Expr, substitute env values, and fold any subexpression that becomes
// fully concrete (literal). Algebraic identities, interval narrowing,
// and contradiction detection live in `simplify`, not here.
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    // Schema-typed env values: round-tripped through Construct/Field by
    // const_fold so that `coord.x` simplifies when `coord` is bound to a
    // Coordinate record. Constructed only by callers building a Bindings
    // env with record values.
    Record { schema: Sym, fields: BTreeMap<Sym, Value> },
}

pub type Bindings = BTreeMap<Sym, Value>;


// ============================================================================
// Constant folding
// ============================================================================

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
                if let Some(folded) = eval_unop(*op, v) {
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
                if let Some(folded) = eval_binop(*op, lv, rv) {
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

fn eval_unop(op: UnOp, v: Value) -> Option<Value> {
    match (op, v) {
        (UnOp::Neg, Value::Int(n)) => Some(Value::Int(-n)),
        (UnOp::Not, Value::Bool(p)) => Some(Value::Bool(!p)),
        _ => None,
    }
}

fn eval_binop(op: BinOp, l: Value, r: Value) -> Option<Value> {
    use BinOp::*;
    use Value::*;
    Some(match (op, l, r) {
        (Add, Int(a), Int(b)) => Int(a + b),
        (Sub, Int(a), Int(b)) => Int(a - b),
        (Mul, Int(a), Int(b)) => Int(a * b),
        (Div, Int(_), Int(0)) | (Mod, Int(_), Int(0)) => return None,
        (Div, Int(a), Int(b)) => Int(a / b),
        (Mod, Int(a), Int(b)) => Int(a % b),
        (Eq, a, b) => Bool(a == b),
        (Neq, a, b) => Bool(a != b),
        (Lt, Int(a), Int(b)) => Bool(a < b),
        (Le, Int(a), Int(b)) => Bool(a <= b),
        (Gt, Int(a), Int(b)) => Bool(a > b),
        (Ge, Int(a), Int(b)) => Bool(a >= b),
        (And, Bool(a), Bool(b)) => Bool(a && b),
        (Or, Bool(a), Bool(b)) => Bool(a || b),
        _ => return None,
    })
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
