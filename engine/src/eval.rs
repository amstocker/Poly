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

/// Concrete values supplied to `Engine::query` via `Bindings`. Distinct from
/// `query::Value`, which is the substitution-side value bound to a logic
/// variable inside an `Answer`. Two enums, two purposes — the previous shared
/// name was a footgun in scopes that imported both via `use super::*`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EnvValue {
    Int(i64),
    Bool(bool),
    Str(String),
    // Schema-typed env values: round-tripped through Construct/Field by
    // const_fold so that `coord.x` simplifies when `coord` is bound to a
    // Coordinate record. Constructed only by callers building a Bindings
    // env with record values.
    Record { schema: Sym, fields: BTreeMap<Sym, EnvValue> },
}

pub type Bindings = BTreeMap<Sym, EnvValue>;


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

fn eval_unop(op: UnOp, v: EnvValue) -> Option<EnvValue> {
    match (op, v) {
        (UnOp::Neg, EnvValue::Int(n)) => Some(EnvValue::Int(-n)),
        (UnOp::Not, EnvValue::Bool(p)) => Some(EnvValue::Bool(!p)),
        _ => None,
    }
}

fn eval_binop(op: BinOp, l: EnvValue, r: EnvValue) -> Option<EnvValue> {
    use BinOp::*;
    use EnvValue::*;
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

fn expr_as_value(e: &Expr<Sym>) -> Option<EnvValue> {
    match e {
        Expr::LitInt(n) => Some(EnvValue::Int(*n)),
        Expr::LitBool(p) => Some(EnvValue::Bool(*p)),
        Expr::LitStr(s) => Some(EnvValue::Str(s.clone())),
        _ => None,
    }
}

fn value_to_expr(eng: &Engine, v: &EnvValue) -> Option<Expr<Sym>> {
    match v {
        EnvValue::Int(n) => Some(Expr::LitInt(*n)),
        EnvValue::Bool(p) => Some(Expr::LitBool(*p)),
        EnvValue::Str(s) => Some(Expr::LitStr(s.clone())),
        EnvValue::Record { schema, fields } => {
            let s = eng.schemas.get(schema)?;
            let SchemaBody::Record(params) = &s.body else {
                debug_assert!(
                    false,
                    "EnvValue::Record schema `{}` is not a record schema",
                    eng.resolve(*schema),
                );
                return None;
            };
            let args: Option<Vec<Expr<Sym>>> = params
                .iter()
                .map(|p| {
                    let fv = fields.get(&p.name);
                    debug_assert!(
                        fv.is_some(),
                        "EnvValue::Record for `{}` is missing field `{}`",
                        eng.resolve(*schema),
                        eng.resolve(p.name),
                    );
                    fv.and_then(|v| value_to_expr(eng, v))
                })
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
