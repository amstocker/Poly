// =============================================================================
// Residual reasoning: the bespoke simplifier on top of `eval::const_fold`.
//
// Pipeline:
//   1. apply_identities       — boolean & arithmetic algebraic laws + folding
//   2. flatten_and            — top-level And-tree → flat conjunct list
//   3. extract_equalities     — `var = expr` → substitution map
//      → substitute through the rest, re-fold, re-apply identities
//   4. extract_intervals      — linearize each comparison atom; fold single-
//      variable |coef|=1 atoms into per-variable Intervals
//   5. detect contradictions  — empty interval ⇒ false
//   6. promote singletons     — closed [k,k] interval ⇒ `var = k`
//   7. dedupe + reassemble    — emit one canonical conjunction
//
// Iterated to a fixpoint (bounded). Each pass is monotone in residual size,
// so it converges in 2–3 iterations on every example we currently produce.
// =============================================================================

use std::collections::{BTreeMap, BTreeSet};

use super::eval::{const_fold, Bindings};
use super::*;


// =============================================================================
// Top-level entry
// =============================================================================

pub fn reduce(eng: &Engine, expr: &Expr<Sym>, env: &Bindings) -> Expr<Sym> {
    let mut current = const_fold(eng, expr, env);
    for _ in 0..8 {
        let next = pass(eng, &current, env);
        if next == current {
            return current;
        }
        current = next;
    }
    current
}

fn pass(eng: &Engine, expr: &Expr<Sym>, env: &Bindings) -> Expr<Sym> {
    let expr = apply_identities(expr);

    let raw = flatten_and(&expr);
    if raw.iter().any(is_false) {
        return Expr::LitBool(false);
    }
    let conjuncts: Vec<Expr<Sym>> = raw.into_iter().filter(|c| !is_true(c)).collect();

    // Equality substitution.
    let (subst, rest) = extract_equalities(&conjuncts);
    let rest: Vec<Expr<Sym>> = rest
        .iter()
        .map(|e| substitute(e, &subst))
        .map(|e| const_fold(eng, &e, env))
        .map(|e| apply_identities(&e))
        .collect();

    // Re-flatten in case substitution exposed nested Ands or trues/falses.
    let mut flat: Vec<Expr<Sym>> = Vec::new();
    for c in rest {
        let pieces = flatten_and(&c);
        if pieces.iter().any(is_false) {
            return Expr::LitBool(false);
        }
        flat.extend(pieces.into_iter().filter(|p| !is_true(p)));
    }

    // Interval extraction.
    let mut intervals: BTreeMap<Sym, Interval> = BTreeMap::new();
    let mut others: Vec<Expr<Sym>> = Vec::new();
    for c in flat {
        if let Some((var, ivl)) = atom_to_interval(&c) {
            intervals.entry(var).or_default().merge(&ivl);
        } else {
            others.push(c);
        }
    }
    for ivl in intervals.values() {
        if ivl.is_empty() {
            return Expr::LitBool(false);
        }
    }
    // Promote singletons.
    let mut singletons: Vec<(Sym, i64)> = Vec::new();
    intervals.retain(|var, ivl| match ivl.singleton() {
        Some(k) => {
            singletons.push((*var, k));
            false
        }
        None => true,
    });

    // Reassemble.
    let mut atoms: Vec<Expr<Sym>> = Vec::new();
    for (v, e) in &subst {
        atoms.push(eq_atom(*v, e.clone()));
    }
    for (v, k) in &singletons {
        atoms.push(eq_atom(*v, Expr::LitInt(*k)));
    }
    for (v, ivl) in &intervals {
        atoms.extend(ivl.to_atoms(*v));
    }
    atoms.extend(others);

    dedupe(&mut atoms);
    if atoms.is_empty() {
        return Expr::LitBool(true);
    }
    conjoin_n(atoms)
}


// =============================================================================
// Identities (bottom-up rewrite)
// =============================================================================

fn apply_identities(e: &Expr<Sym>) -> Expr<Sym> {
    use BinOp::*;
    use UnOp::*;
    match e {
        Expr::LitInt(_) | Expr::LitStr(_) | Expr::LitBool(_) | Expr::Var(_) => e.clone(),
        Expr::UnOp(op, inner) => {
            let inner = apply_identities(inner);
            match (op, &inner) {
                (Neg, Expr::LitInt(n)) => Expr::LitInt(-n),
                (Neg, Expr::UnOp(Neg, x)) => (**x).clone(),
                (Not, Expr::LitBool(b)) => Expr::LitBool(!b),
                (Not, Expr::UnOp(Not, x)) => (**x).clone(),
                _ => Expr::UnOp(*op, Box::new(inner)),
            }
        }
        Expr::BinOp(op, l, r) => {
            let l = apply_identities(l);
            let r = apply_identities(r);

            // Both literals → fold.
            if let (Some(a), Some(b)) = (lit_int(&l), lit_int(&r)) {
                if let Some(folded) = fold_int_binop(*op, a, b) {
                    return folded;
                }
            }
            if let (Some(a), Some(b)) = (lit_bool(&l), lit_bool(&r)) {
                if let Some(folded) = fold_bool_binop(*op, a, b) {
                    return folded;
                }
            }

            // Boolean absorption / annihilation.
            match (op, lit_bool(&l), lit_bool(&r)) {
                (And, Some(true), _) => return r,
                (And, _, Some(true)) => return l,
                (And, Some(false), _) | (And, _, Some(false)) => return Expr::LitBool(false),
                (Or, Some(false), _) => return r,
                (Or, _, Some(false)) => return l,
                (Or, Some(true), _) | (Or, _, Some(true)) => return Expr::LitBool(true),
                _ => {}
            }

            // Arithmetic identities.
            match (op, lit_int(&l), lit_int(&r)) {
                (Add, Some(0), _) => return r,
                (Add, _, Some(0)) => return l,
                (Sub, _, Some(0)) => return l,
                (Mul, Some(1), _) => return r,
                (Mul, _, Some(1)) => return l,
                (Mul, Some(0), _) | (Mul, _, Some(0)) => return Expr::LitInt(0),
                _ => {}
            }

            // Syntactic-equality reductions.
            if matches!(op, Sub) && l == r {
                return Expr::LitInt(0);
            }
            if matches!(op, And | Or) && l == r {
                return l;
            }
            if matches!(op, Eq) && l == r {
                return Expr::LitBool(true);
            }
            if matches!(op, Neq) && l == r {
                return Expr::LitBool(false);
            }

            Expr::BinOp(*op, Box::new(l), Box::new(r))
        }
        Expr::Field(base, name) => {
            let base = apply_identities(base);
            Expr::Field(Box::new(base), *name)
        }
        Expr::Construct(name, args) => {
            let args: Vec<_> = args.iter().map(apply_identities).collect();
            Expr::Construct(*name, args)
        }
    }
}

fn fold_int_binop(op: BinOp, a: i64, b: i64) -> Option<Expr<Sym>> {
    use BinOp::*;
    Some(match op {
        Add => Expr::LitInt(a + b),
        Sub => Expr::LitInt(a - b),
        Mul => Expr::LitInt(a * b),
        Div if b != 0 => Expr::LitInt(a / b),
        Mod if b != 0 => Expr::LitInt(a % b),
        Eq => Expr::LitBool(a == b),
        Neq => Expr::LitBool(a != b),
        Lt => Expr::LitBool(a < b),
        Le => Expr::LitBool(a <= b),
        Gt => Expr::LitBool(a > b),
        Ge => Expr::LitBool(a >= b),
        _ => return None,
    })
}

fn fold_bool_binop(op: BinOp, a: bool, b: bool) -> Option<Expr<Sym>> {
    use BinOp::*;
    Some(match op {
        And => Expr::LitBool(a && b),
        Or => Expr::LitBool(a || b),
        Eq => Expr::LitBool(a == b),
        Neq => Expr::LitBool(a != b),
        _ => return None,
    })
}

fn lit_int(e: &Expr<Sym>) -> Option<i64> { if let Expr::LitInt(n) = e { Some(*n) } else { None } }
fn lit_bool(e: &Expr<Sym>) -> Option<bool> { if let Expr::LitBool(b) = e { Some(*b) } else { None } }
fn is_true(e: &Expr<Sym>) -> bool { matches!(e, Expr::LitBool(true)) }
fn is_false(e: &Expr<Sym>) -> bool { matches!(e, Expr::LitBool(false)) }


// =============================================================================
// Conjunction tools
// =============================================================================

fn flatten_and(e: &Expr<Sym>) -> Vec<Expr<Sym>> {
    let mut out = Vec::new();
    fn go(e: &Expr<Sym>, out: &mut Vec<Expr<Sym>>) {
        if let Expr::BinOp(BinOp::And, l, r) = e {
            go(l, out);
            go(r, out);
        } else {
            out.push(e.clone());
        }
    }
    go(e, &mut out);
    out
}

fn conjoin_n(atoms: Vec<Expr<Sym>>) -> Expr<Sym> {
    let mut iter = atoms.into_iter();
    let first = iter.next().expect("conjoin_n called on empty list");
    iter.fold(first, |acc, e| Expr::BinOp(BinOp::And, Box::new(acc), Box::new(e)))
}

fn dedupe(atoms: &mut Vec<Expr<Sym>>) {
    // Use Debug repr as the dedup key — Expr doesn't implement Hash/Ord and
    // the residuals we produce are tiny, so the cost is negligible.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::with_capacity(atoms.len());
    for a in atoms.drain(..) {
        let key = format!("{a:?}");
        if seen.insert(key) {
            out.push(a);
        }
    }
    *atoms = out;
}

fn eq_atom(v: Sym, rhs: Expr<Sym>) -> Expr<Sym> {
    Expr::BinOp(BinOp::Eq, Box::new(Expr::Var(v)), Box::new(rhs))
}


// =============================================================================
// Equality substitution
// =============================================================================

fn extract_equalities(
    conjuncts: &[Expr<Sym>],
) -> (BTreeMap<Sym, Expr<Sym>>, Vec<Expr<Sym>>) {
    let mut subst: BTreeMap<Sym, Expr<Sym>> = BTreeMap::new();
    let mut rest: Vec<Expr<Sym>> = Vec::new();
    for c in conjuncts {
        if let Expr::BinOp(BinOp::Eq, l, r) = c {
            if let Expr::Var(v) = **l {
                if !contains_var(r, v) && !subst.contains_key(&v) {
                    subst.insert(v, (**r).clone());
                    continue;
                }
            }
            if let Expr::Var(v) = **r {
                if !contains_var(l, v) && !subst.contains_key(&v) {
                    subst.insert(v, (**l).clone());
                    continue;
                }
            }
        }
        rest.push(c.clone());
    }
    (subst, rest)
}

fn contains_var(e: &Expr<Sym>, v: Sym) -> bool {
    match e {
        Expr::Var(s) => *s == v,
        Expr::LitInt(_) | Expr::LitStr(_) | Expr::LitBool(_) => false,
        Expr::UnOp(_, x) => contains_var(x, v),
        Expr::BinOp(_, l, r) => contains_var(l, v) || contains_var(r, v),
        Expr::Field(b, _) => contains_var(b, v),
        Expr::Construct(_, args) => args.iter().any(|a| contains_var(a, v)),
    }
}

fn substitute(e: &Expr<Sym>, subst: &BTreeMap<Sym, Expr<Sym>>) -> Expr<Sym> {
    match e {
        Expr::Var(s) => subst.get(s).cloned().unwrap_or_else(|| e.clone()),
        Expr::LitInt(_) | Expr::LitStr(_) | Expr::LitBool(_) => e.clone(),
        Expr::UnOp(op, x) => Expr::UnOp(*op, Box::new(substitute(x, subst))),
        Expr::BinOp(op, l, r) => Expr::BinOp(
            *op,
            Box::new(substitute(l, subst)),
            Box::new(substitute(r, subst)),
        ),
        Expr::Field(b, n) => Expr::Field(Box::new(substitute(b, subst)), *n),
        Expr::Construct(name, args) => Expr::Construct(
            *name,
            args.iter().map(|a| substitute(a, subst)).collect(),
        ),
    }
}


// =============================================================================
// Linear forms & atom normalization
// =============================================================================

#[derive(Clone, Debug, Default)]
struct Linear {
    constant: i64,
    terms: BTreeMap<Sym, i64>,
}

impl Linear {
    fn lit(n: i64) -> Self { Self { constant: n, terms: BTreeMap::new() } }
    fn var(s: Sym) -> Self {
        let mut t = BTreeMap::new();
        t.insert(s, 1);
        Self { constant: 0, terms: t }
    }
    fn add(mut self, other: Self) -> Self {
        self.constant += other.constant;
        for (k, v) in other.terms {
            *self.terms.entry(k).or_insert(0) += v;
        }
        self.terms.retain(|_, v| *v != 0);
        self
    }
    fn neg(mut self) -> Self {
        self.constant = -self.constant;
        for v in self.terms.values_mut() { *v = -*v; }
        self
    }
    fn sub(self, other: Self) -> Self { self.add(other.neg()) }
    fn scale(mut self, k: i64) -> Self {
        if k == 0 { return Self::lit(0); }
        self.constant *= k;
        for v in self.terms.values_mut() { *v *= k; }
        self
    }
}

fn to_linear(e: &Expr<Sym>) -> Option<Linear> {
    use BinOp::*;
    match e {
        Expr::LitInt(n) => Some(Linear::lit(*n)),
        Expr::Var(s) => Some(Linear::var(*s)),
        Expr::UnOp(UnOp::Neg, inner) => Some(to_linear(inner)?.neg()),
        Expr::BinOp(Add, l, r) => Some(to_linear(l)?.add(to_linear(r)?)),
        Expr::BinOp(Sub, l, r) => Some(to_linear(l)?.sub(to_linear(r)?)),
        Expr::BinOp(Mul, l, r) => {
            let ll = to_linear(l)?;
            let lr = to_linear(r)?;
            // Linear only if at least one side is a pure constant.
            if ll.terms.is_empty() {
                Some(lr.scale(ll.constant))
            } else if lr.terms.is_empty() {
                Some(ll.scale(lr.constant))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct SimpleAtom { var: Sym, op: BinOp, rhs: i64 }

fn atom_to_simple(e: &Expr<Sym>) -> Option<SimpleAtom> {
    use BinOp::*;
    let (op, l, r) = match e {
        Expr::BinOp(op, l, r) if matches!(op, Lt | Le | Gt | Ge | Eq | Neq) => (*op, l, r),
        _ => return None,
    };
    let combined = to_linear(l)?.sub(to_linear(r)?);
    if combined.terms.len() != 1 {
        return None;
    }
    let (var, coef) = combined.terms.into_iter().next().unwrap();
    let constant = combined.constant;
    if coef.abs() != 1 {
        return None;
    }
    let (rhs, op) = if coef == 1 {
        (-constant, op)
    } else {
        // -var + constant op 0  ↔  var op_flipped constant.
        (constant, flip_inequality(op))
    };
    Some(SimpleAtom { var, op, rhs })
}

fn flip_inequality(op: BinOp) -> BinOp {
    match op {
        BinOp::Lt => BinOp::Gt,
        BinOp::Le => BinOp::Ge,
        BinOp::Gt => BinOp::Lt,
        BinOp::Ge => BinOp::Le,
        BinOp::Eq | BinOp::Neq => op,
        _ => op,
    }
}


// =============================================================================
// Per-variable intervals
// =============================================================================

#[derive(Clone, Debug, Default)]
struct Interval {
    /// Tightest lower bound: (value, inclusive). Inclusive=true means `≥ v`.
    lo: Option<(i64, bool)>,
    hi: Option<(i64, bool)>,
    ne: BTreeSet<i64>,
}

impl Interval {
    fn from_atom(atom: &SimpleAtom) -> Self {
        let mut i = Self::default();
        match atom.op {
            BinOp::Lt => i.hi = Some((atom.rhs, false)),
            BinOp::Le => i.hi = Some((atom.rhs, true)),
            BinOp::Gt => i.lo = Some((atom.rhs, false)),
            BinOp::Ge => i.lo = Some((atom.rhs, true)),
            BinOp::Eq => {
                i.lo = Some((atom.rhs, true));
                i.hi = Some((atom.rhs, true));
            }
            BinOp::Neq => { i.ne.insert(atom.rhs); }
            _ => unreachable!("non-comparison op in SimpleAtom"),
        }
        i
    }
    fn merge(&mut self, other: &Interval) {
        if let Some(b) = other.lo {
            self.lo = Some(match self.lo {
                None => b,
                Some(a) => tighter_lo(a, b),
            });
        }
        if let Some(b) = other.hi {
            self.hi = Some(match self.hi {
                None => b,
                Some(a) => tighter_hi(a, b),
            });
        }
        self.ne.extend(other.ne.iter().copied());
    }
    fn is_empty(&self) -> bool {
        if let (Some((lo, lo_inc)), Some((hi, hi_inc))) = (self.lo, self.hi) {
            if lo > hi { return true; }
            if lo == hi && !(lo_inc && hi_inc) { return true; }
        }
        if let Some(k) = self.singleton_pre_ne() {
            if self.ne.contains(&k) { return true; }
        }
        false
    }
    fn singleton_pre_ne(&self) -> Option<i64> {
        if let (Some((lo, true)), Some((hi, true))) = (self.lo, self.hi) {
            if lo == hi { return Some(lo); }
        }
        None
    }
    fn singleton(&self) -> Option<i64> {
        let k = self.singleton_pre_ne()?;
        if self.ne.contains(&k) { return None; }
        Some(k)
    }
    fn to_atoms(&self, var: Sym) -> Vec<Expr<Sym>> {
        let mut out = Vec::new();
        let mk = |op, k| Expr::BinOp(op, Box::new(Expr::Var(var)), Box::new(Expr::LitInt(k)));
        if let Some((lo, inc)) = self.lo {
            out.push(mk(if inc { BinOp::Ge } else { BinOp::Gt }, lo));
        }
        if let Some((hi, inc)) = self.hi {
            out.push(mk(if inc { BinOp::Le } else { BinOp::Lt }, hi));
        }
        for k in &self.ne {
            // Drop ne values already excluded by the bounds.
            if let Some((lo, inc)) = self.lo {
                if *k < lo || (*k == lo && !inc) { continue; }
            }
            if let Some((hi, inc)) = self.hi {
                if *k > hi || (*k == hi && !inc) { continue; }
            }
            out.push(mk(BinOp::Neq, *k));
        }
        out
    }
}

// Lower bound: stricter (higher value) wins; same value, exclusive (>) beats inclusive (≥).
fn tighter_lo(a: (i64, bool), b: (i64, bool)) -> (i64, bool) {
    if a.0 > b.0 { a }
    else if b.0 > a.0 { b }
    else { (a.0, a.1 && b.1) }
}

// Upper bound: stricter (lower value) wins; same value, exclusive (<) beats inclusive (≤).
fn tighter_hi(a: (i64, bool), b: (i64, bool)) -> (i64, bool) {
    if a.0 < b.0 { a }
    else if b.0 < a.0 { b }
    else { (a.0, a.1 && b.1) }
}

fn atom_to_interval(e: &Expr<Sym>) -> Option<(Sym, Interval)> {
    let s = atom_to_simple(e)?;
    Some((s.var, Interval::from_atom(&s)))
}


// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn load() -> Engine {
        // Counter has interesting guards (`n >= 0`, `n > 0`) but we only need
        // the engine for its interner and schemas; Bindings are empty.
        let src = std::fs::read_to_string("examples/counter.poly").expect("read counter");
        Engine::load(&src).expect("load counter")
    }

    fn n_sym(eng: &Engine) -> Sym { eng.interner.find("n").unwrap() }

    fn gt(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::Gt, Box::new(l), Box::new(r))
    }
    fn ge(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::Ge, Box::new(l), Box::new(r))
    }
    fn lt(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::Lt, Box::new(l), Box::new(r))
    }
    fn eq(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::Eq, Box::new(l), Box::new(r))
    }
    fn and(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::And, Box::new(l), Box::new(r))
    }
    fn add(l: Expr<Sym>, r: Expr<Sym>) -> Expr<Sym> {
        Expr::BinOp(BinOp::Add, Box::new(l), Box::new(r))
    }
    fn var(s: Sym) -> Expr<Sym> { Expr::Var(s) }
    fn lit(n: i64) -> Expr<Sym> { Expr::LitInt(n) }

    #[test]
    fn narrow_lower_bounds() {
        // n > 0 ∧ n > 5 → n > 5
        let eng = load();
        let n = n_sym(&eng);
        let r = reduce(&eng, &and(gt(var(n), lit(0)), gt(var(n), lit(5))), &Bindings::default());
        assert_eq!(r, gt(var(n), lit(5)));
    }

    #[test]
    fn narrow_mixed_inclusivity() {
        // n >= 0 ∧ n > 0 → n > 0
        let eng = load();
        let n = n_sym(&eng);
        let r = reduce(&eng, &and(ge(var(n), lit(0)), gt(var(n), lit(0))), &Bindings::default());
        assert_eq!(r, gt(var(n), lit(0)));
    }

    #[test]
    fn contradiction_drops_to_false() {
        // n > 5 ∧ n < 3 → false
        let eng = load();
        let n = n_sym(&eng);
        let r = reduce(&eng, &and(gt(var(n), lit(5)), lt(var(n), lit(3))), &Bindings::default());
        assert_eq!(r, Expr::LitBool(false));
    }

    #[test]
    fn arithmetic_identities_collapse() {
        // n + 0 + 0 > 5 → n > 5
        let eng = load();
        let n = n_sym(&eng);
        let inp = gt(add(add(var(n), lit(0)), lit(0)), lit(5));
        let r = reduce(&eng, &inp, &Bindings::default());
        assert_eq!(r, gt(var(n), lit(5)));
    }

    #[test]
    fn linear_atom_normalization() {
        // n + 1 > 5 → n > 4
        let eng = load();
        let n = n_sym(&eng);
        let r = reduce(&eng, &gt(add(var(n), lit(1)), lit(5)), &Bindings::default());
        assert_eq!(r, gt(var(n), lit(4)));
    }

    #[test]
    fn equality_substitution() {
        // n = 5 ∧ n + m > 10  → n = 5 ∧ m > 5
        let mut eng = load();
        let n = n_sym(&eng);
        let m = eng.interner.intern("m");
        let inp = and(eq(var(n), lit(5)), gt(add(var(n), var(m)), lit(10)));
        let r = reduce(&eng, &inp, &Bindings::default());
        // After substitution: n = 5 ∧ 5 + m > 10 → n = 5 ∧ m > 5.
        // Reassembly emits the equality first, then the narrowed atom.
        assert_eq!(r, and(eq(var(n), lit(5)), gt(var(m), lit(5))));
    }

    #[test]
    fn dedup_identical_conjuncts() {
        // (n > 0) ∧ (n > 0) → n > 0
        let eng = load();
        let n = n_sym(&eng);
        let r = reduce(&eng, &and(gt(var(n), lit(0)), gt(var(n), lit(0))), &Bindings::default());
        assert_eq!(r, gt(var(n), lit(0)));
    }

    #[test]
    fn singleton_interval_promotes_to_equality() {
        // n >= 5 ∧ n <= 5 → n = 5
        let eng = load();
        let n = n_sym(&eng);
        let inp = and(ge(var(n), lit(5)), Expr::BinOp(BinOp::Le, Box::new(var(n)), Box::new(lit(5))));
        let r = reduce(&eng, &inp, &Bindings::default());
        assert_eq!(r, eq(var(n), lit(5)));
    }

    #[test]
    fn unrelated_constraints_pass_through() {
        // n > 0 ∧ m > 0 → n > 0 ∧ m > 0  (independent variables, both kept)
        let mut eng = load();
        let n = n_sym(&eng);
        let m = eng.interner.intern("m");
        let r = reduce(&eng, &and(gt(var(n), lit(0)), gt(var(m), lit(0))), &Bindings::default());
        // Order is by variable Sym (BTreeMap iteration); we just check the conjunct set.
        let expected_a = and(gt(var(n), lit(0)), gt(var(m), lit(0)));
        let expected_b = and(gt(var(m), lit(0)), gt(var(n), lit(0)));
        assert!(r == expected_a || r == expected_b, "got {r:?}");
    }

    #[test]
    fn env_substitution_then_simplify() {
        // n > 0 with env n=3 → true
        let eng = load();
        let n = n_sym(&eng);
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(3));
        let r = reduce(&eng, &gt(var(n), lit(0)), &env);
        assert_eq!(r, Expr::LitBool(true));
    }
}
