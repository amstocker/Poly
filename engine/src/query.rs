// The query AST below (Term, Slot, IndexSlot, DirRefPat, Goal, Query) is
// part of the library API. Variants reachable only from tests or from the
// not-yet-landed surface-syntax parser would otherwise show as dead from
// the binary build.
#![allow(dead_code)]

use std::collections::BTreeMap;

use super::eval::{conjoin, Bindings};
use super::simplify::reduce;
use super::*;


// ============================================================================
// Logic variables
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub u32);


// ============================================================================
// Query AST
// ============================================================================

#[derive(Clone, Debug)]
pub enum Term {
    Var(VarId),
    Sym(Sym),
    Anon,
}

#[derive(Clone, Debug)]
pub enum Slot {
    Var(VarId),
    Anon,
}

#[derive(Clone, Debug)]
pub enum IndexSlot {
    Var(VarId),
    Anon,
    Lit(usize),
}

#[derive(Clone, Debug)]
pub enum DirRefPat {
    Var(VarId),
    Anon,
    Named(Term),
    Abstract {
        src_pos: Term,
        src_pattern: Slot,
        tgt_pos: Term,
        tgt_args: Slot,
    },
}

#[derive(Clone, Debug)]
pub enum Goal {
    Iface { iface: Term, params: Slot },
    IfaceInternal { internal: Term, external: Term },
    SchemaRecord { schema: Term, fields: Slot },
    SchemaSum { schema: Term, variants: Slot },
    Position { iface: Term, position: Term, params: Slot, guard: Slot },
    Direction {
        iface: Term, position: Term, action: Term, params: Slot, guard: Slot,
    },
    Defer { defer: Term, source: Term, target: Term },
    DeferEntry {
        defer: Term, entry_idx: IndexSlot, source_pos: Term,
        src_pattern: Slot, src_guard: Slot,
        target_pos: Term, target_args: Slot,
    },
    DeferDir {
        defer: Term, entry_idx: IndexSlot,
        target_dir: DirRefPat, source_dir: DirRefPat,
    },
    /// A user-written constraint. The expression is added to the answer's
    /// residual; goals never short-circuit on residuals during search — the
    /// simplifier resolves them once at the end of the query.
    Where(Expr<Sym>),
}

#[derive(Clone, Debug, Default)]
pub struct Query {
    pub bodies: Vec<Vec<Goal>>,
}

impl Query {
    pub fn single(goals: Vec<Goal>) -> Self {
        Self { bodies: vec![goals] }
    }
    pub fn or(bodies: Vec<Vec<Goal>>) -> Self {
        Self { bodies }
    }
}


// ============================================================================
// Values, substitution, answers
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Sym(Sym),
    Index(usize),
    Params(Vec<Param<Sym>>),
    Variants(Vec<Variant<Sym>>),
    Guard(Option<Expr<Sym>>),
    Args(Vec<Expr<Sym>>),
    Pattern(Vec<Pattern<Sym>>),
    DirRef(DirRef<Sym>),
}

pub type Subst = BTreeMap<VarId, Value>;

#[derive(Clone, Debug)]
pub struct Answer {
    pub subst: Subst,
    /// Conjuncts left over after matching: position guards, direction guards,
    /// and `Goal::Where` expressions. Resolved against the caller's env by the
    /// simplifier before `Engine::query` returns. An empty residual means the
    /// answer is unconditionally true.
    pub residual: Vec<Expr<Sym>>,
}

impl Answer {
    pub fn empty() -> Self {
        Self { subst: Subst::default(), residual: Vec::new() }
    }
    pub fn with_subst(&self, subst: Subst) -> Self {
        Self { subst, residual: self.residual.clone() }
    }
    pub fn push_residual(&self, e: Expr<Sym>) -> Self {
        let mut next = self.clone();
        next.residual.push(e);
        next
    }
}


// ============================================================================
// Variable counter (for query construction)
// ============================================================================

#[derive(Clone, Debug, Default)]
pub struct VarGen {
    next: u32,
}

impl VarGen {
    pub fn new() -> Self { Self::default() }
    pub fn fresh(&mut self) -> VarId {
        let v = VarId(self.next);
        self.next += 1;
        v
    }
}


// ============================================================================
// Unification helpers
// ============================================================================

fn bind(subst: &Subst, v: VarId, val: Value) -> Option<Subst> {
    if let Some(existing) = subst.get(&v) {
        if existing == &val {
            return Some(subst.clone());
        }
        return None;
    }
    let mut next = subst.clone();
    next.insert(v, val);
    Some(next)
}

fn unify_term(t: &Term, sym: Sym, subst: &Subst) -> Option<Subst> {
    match t {
        Term::Sym(s) => if *s == sym { Some(subst.clone()) } else { None },
        Term::Var(v) => bind(subst, *v, Value::Sym(sym)),
        Term::Anon => Some(subst.clone()),
    }
}

fn unify_slot(slot: &Slot, val: Value, subst: &Subst) -> Option<Subst> {
    match slot {
        Slot::Anon => Some(subst.clone()),
        Slot::Var(v) => bind(subst, *v, val),
    }
}

fn unify_index_slot(slot: &IndexSlot, idx: usize, subst: &Subst) -> Option<Subst> {
    match slot {
        IndexSlot::Anon => Some(subst.clone()),
        IndexSlot::Lit(n) => if *n == idx { Some(subst.clone()) } else { None },
        IndexSlot::Var(v) => bind(subst, *v, Value::Index(idx)),
    }
}

fn unify_dir_ref_pat(
    pat: &DirRefPat,
    dr: &DirRef<Sym>,
    subst: &Subst,
) -> Option<Subst> {
    match (pat, dr) {
        (DirRefPat::Anon, _) => Some(subst.clone()),
        (DirRefPat::Var(v), _) => bind(subst, *v, Value::DirRef(dr.clone())),
        (DirRefPat::Named(t), DirRef::Named(s)) => unify_term(t, *s, subst),
        (DirRefPat::Named(_), DirRef::Abstract { .. }) => None,
        (
            DirRefPat::Abstract { src_pos, src_pattern, tgt_pos, tgt_args },
            DirRef::Abstract {
                src_pos: sp, src_pattern: spat, tgt_pos: tp, tgt_args: targs,
            },
        ) => {
            let s = unify_term(src_pos, *sp, subst)?;
            let s = unify_slot(src_pattern, Value::Pattern(spat.clone()), &s)?;
            let s = unify_term(tgt_pos, *tp, &s)?;
            unify_slot(tgt_args, Value::Args(targs.clone()), &s)
        }
        (DirRefPat::Abstract { .. }, DirRef::Named(_)) => None,
    }
}


// ============================================================================
// Per-goal matching against a fact relation
// ============================================================================

fn match_goal(goal: &Goal, eng: &Engine, ans: &Answer) -> Vec<Answer> {
    match goal {
        Goal::Iface { iface, params } => eng
            .iface_relation()
            .filter_map(|i| {
                let s = unify_term(iface, i.name, &ans.subst)?;
                let s = unify_slot(params, Value::Params(i.params.clone()), &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::IfaceInternal { internal, external } => eng
            .iface_internal_relation()
            .filter_map(|(int_sym, ext_sym)| {
                let s = unify_term(internal, int_sym, &ans.subst)?;
                let s = unify_term(external, ext_sym, &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::SchemaRecord { schema, fields } => eng
            .schema_record_relation()
            .filter_map(|(name, flds)| {
                let s = unify_term(schema, name, &ans.subst)?;
                let s = unify_slot(fields, Value::Params(flds.to_vec()), &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::SchemaSum { schema, variants } => eng
            .schema_sum_relation()
            .filter_map(|(name, vars)| {
                let s = unify_term(schema, name, &ans.subst)?;
                let s = unify_slot(variants, Value::Variants(vars.to_vec()), &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::Position { iface, position, params, guard } => eng
            .position_relation()
            .filter_map(|(i_sym, p)| {
                let s = unify_term(iface, i_sym, &ans.subst)?;
                let s = unify_term(position, p.name, &s)?;
                let s = unify_slot(params, Value::Params(p.params.clone()), &s)?;
                let s = unify_slot(guard, Value::Guard(p.guard.clone()), &s)?;
                let mut next = ans.with_subst(s);
                if let Some(g) = &p.guard {
                    next.residual.push(g.clone());
                }
                Some(next)
            })
            .collect(),
        Goal::Direction { iface, position, action, params, guard } => eng
            .direction_relation()
            .filter_map(|(i_sym, p_sym, d)| {
                let s = unify_term(iface, i_sym, &ans.subst)?;
                let s = unify_term(position, p_sym, &s)?;
                let s = unify_term(action, d.name, &s)?;
                let s = unify_slot(params, Value::Params(d.params.clone()), &s)?;
                let s = unify_slot(guard, Value::Guard(d.guard.clone()), &s)?;
                let mut next = ans.with_subst(s);
                if let Some(g) = &d.guard {
                    next.residual.push(g.clone());
                }
                Some(next)
            })
            .collect(),
        Goal::Defer { defer, source, target } => eng
            .defer_relation()
            .filter_map(|d| {
                let s = unify_term(defer, d.name, &ans.subst)?;
                let s = unify_term(source, d.source, &s)?;
                let s = unify_term(target, d.target, &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::DeferEntry {
            defer, entry_idx, source_pos, src_pattern, src_guard,
            target_pos, target_args,
        } => eng
            .defer_entry_relation()
            .filter_map(|(d_sym, idx, e)| {
                let s = unify_term(defer, d_sym, &ans.subst)?;
                let s = unify_index_slot(entry_idx, idx, &s)?;
                let s = unify_term(source_pos, e.source_pos, &s)?;
                let s = unify_slot(src_pattern, Value::Pattern(e.source_pattern.clone()), &s)?;
                let s = unify_slot(src_guard, Value::Guard(e.source_guard.clone()), &s)?;
                let s = unify_term(target_pos, e.target_pos, &s)?;
                let s = unify_slot(target_args, Value::Args(e.target_args.clone()), &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::DeferDir { defer, entry_idx, target_dir, source_dir } => eng
            .defer_dir_relation()
            .filter_map(|(d_sym, idx, m)| {
                let s = unify_term(defer, d_sym, &ans.subst)?;
                let s = unify_index_slot(entry_idx, idx, &s)?;
                let s = unify_dir_ref_pat(target_dir, &m.target_dir, &s)?;
                let s = unify_dir_ref_pat(source_dir, &m.source_dir, &s)?;
                Some(ans.with_subst(s))
            })
            .collect(),
        Goal::Where(expr) => vec![ans.push_residual(expr.clone())],
    }
}


// ============================================================================
// Solver
// ============================================================================

fn solve(goals: &[Goal], eng: &Engine, ans: Answer) -> Vec<Answer> {
    let Some((first, rest)) = goals.split_first() else {
        return vec![ans];
    };
    let mut out = Vec::new();
    for next in match_goal(first, eng, &ans) {
        out.extend(solve(rest, eng, next));
    }
    out
}

impl Engine {
    /// Run `query` against the loaded program. Each disjunctive body is
    /// solved by unification against the relations exposed in `relations.rs`;
    /// per-position guards and `Goal::Where` expressions accumulate as
    /// residuals on the resulting `Answer`s; the simplifier then reduces
    /// each residual against `env`. An empty `env` (`Bindings::default()`)
    /// is the common case — pass concrete variable bindings to specialize.
    ///
    /// Answers whose residuals reduce to `false` are dropped. Answers
    /// whose residuals reduce to `true` are returned with `residual:
    /// vec![]`. Anything else is kept as a single residual conjunct.
    pub fn query(&self, query: &Query, env: &Bindings) -> Vec<Answer> {
        let mut out = Vec::new();
        for body in &query.bodies {
            for ans in solve(body, self, Answer::empty()) {
                if let Some(simplified) = simplify_answer(self, &ans, env) {
                    out.push(simplified);
                }
            }
        }
        out
    }
}

/// Conjoin and simplify an answer's residual against `env`. Returns `None`
/// when the residual reduces to `false` (the answer is dropped). When the
/// residual reduces to `true`, the residual is cleared. Otherwise the
/// simplified expression is kept as a single conjunct on the residual.
fn simplify_answer(eng: &Engine, ans: &Answer, env: &Bindings) -> Option<Answer> {
    let Some(joined) = conjoin(&ans.residual) else {
        return Some(ans.clone());
    };
    let reduced = reduce(eng, &joined, env);
    match reduced {
        Expr::LitBool(true) => Some(Answer { subst: ans.subst.clone(), residual: Vec::new() }),
        Expr::LitBool(false) => None,
        other => Some(Answer { subst: ans.subst.clone(), residual: vec![other] }),
    }
}


// ============================================================================
// Tests: hand-written reductions of existing queries
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn load(path: &str) -> Engine {
        let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(path);
        let src = std::fs::read_to_string(&full).expect("read example");
        Engine::load(&src).expect("load engine")
    }

    fn answer_sym(a: &Answer, v: VarId) -> Sym {
        match a.subst.get(&v) {
            Some(Value::Sym(s)) => *s,
            other => panic!("expected Sym for {v:?}, got {other:?}"),
        }
    }

    fn locate_action_via_query(
        eng: &Engine,
        action: &str,
    ) -> BTreeSet<(Sym, Sym)> {
        let mut g = VarGen::new();
        let i_var = g.fresh();
        let p_var = g.fresh();
        let action_sym = eng.interner.find(action).expect("action interned");
        let q = Query::single(vec![
            Goal::Direction {
                iface: Term::Var(i_var),
                position: Term::Var(p_var),
                action: Term::Sym(action_sym),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
            Goal::Position {
                iface: Term::Var(i_var),
                position: Term::Var(p_var),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
        ]);
        eng.query(&q, &Bindings::default())
            .iter()
            .map(|a| (answer_sym(a, i_var), answer_sym(a, p_var)))
            .collect()
    }

    #[test]
    fn locate_action_counter() {
        let eng = load("examples/counter.poly");
        let count = eng.interner.find("Count").unwrap();
        let counter = eng.interner.find("Counter").unwrap();
        let internal = eng.interner.find("Counter::Internal").unwrap();
        let button = eng.interner.find("Button").unwrap();

        // Increment lives on the external Counter only.
        let inc = locate_action_via_query(&eng, "Increment");
        assert_eq!(inc, [(counter, count)].into_iter().collect());

        // Decrement: same.
        let dec = locate_action_via_query(&eng, "Decrement");
        assert_eq!(dec, [(counter, count)].into_iter().collect());

        // Press lives on Button.Button. Counter::Internal has no directions of
        // its own (universal-state-machine carrier), so Press doesn't surface
        // there even though SetTo10 wires it through abstractly.
        let press = locate_action_via_query(&eng, "Press");
        assert_eq!(press, [(button, button)].into_iter().collect());

        // The internal carrier has empty directions.
        let on_internal = locate_action_via_query(&eng, "Increment")
            .iter()
            .filter(|(i, _)| *i == internal)
            .count();
        assert_eq!(on_internal, 0);
    }

    fn next_position_via_query(
        eng: &Engine,
        iface: &str,
        pos: &str,
        action: &str,
    ) -> Vec<(Sym, Vec<Expr<Sym>>)> {
        let i_sym = eng.interner.find(iface).expect("iface");
        let p_sym = eng.interner.find(pos).expect("pos");
        let a_sym = eng.interner.find(action).expect("action");

        let mut g = VarGen::new();
        let tgt_pos = g.fresh();
        let tgt_args = g.fresh();

        // Realization disjunct: I is target of a defer whose source is I::Internal.
        let r_int = g.fresh();
        let r_defer = g.fresh();
        let r_entry = g.fresh();
        let realization = vec![
            Goal::IfaceInternal {
                internal: Term::Var(r_int),
                external: Term::Sym(i_sym),
            },
            Goal::Defer {
                defer: Term::Var(r_defer),
                source: Term::Var(r_int),
                target: Term::Sym(i_sym),
            },
            Goal::DeferEntry {
                defer: Term::Var(r_defer),
                entry_idx: IndexSlot::Var(r_entry),
                source_pos: Term::Sym(p_sym),
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Sym(p_sym),
                target_args: Slot::Anon,
            },
            Goal::DeferDir {
                defer: Term::Var(r_defer),
                entry_idx: IndexSlot::Var(r_entry),
                target_dir: DirRefPat::Named(Term::Sym(a_sym)),
                source_dir: DirRefPat::Abstract {
                    src_pos: Term::Sym(p_sym),
                    src_pattern: Slot::Anon,
                    tgt_pos: Term::Var(tgt_pos),
                    tgt_args: Slot::Var(tgt_args),
                },
            },
        ];

        // Defer-source-abstract disjunct: I is source of a defer with abstract refs.
        let s_defer = g.fresh();
        let s_entry = g.fresh();
        let defer_source_abs = vec![
            Goal::Defer {
                defer: Term::Var(s_defer),
                source: Term::Sym(i_sym),
                target: Term::Anon,
            },
            Goal::DeferEntry {
                defer: Term::Var(s_defer),
                entry_idx: IndexSlot::Var(s_entry),
                source_pos: Term::Sym(p_sym),
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Anon,
                target_args: Slot::Anon,
            },
            Goal::DeferDir {
                defer: Term::Var(s_defer),
                entry_idx: IndexSlot::Var(s_entry),
                target_dir: DirRefPat::Named(Term::Sym(a_sym)),
                source_dir: DirRefPat::Abstract {
                    src_pos: Term::Sym(p_sym),
                    src_pattern: Slot::Anon,
                    tgt_pos: Term::Var(tgt_pos),
                    tgt_args: Slot::Var(tgt_args),
                },
            },
        ];

        let q = Query::or(vec![realization, defer_source_abs]);
        eng.query(&q, &Bindings::default())
            .into_iter()
            .map(|a| {
                let tp = answer_sym(&a, tgt_pos);
                let args = match a.subst.get(&tgt_args) {
                    Some(Value::Args(args)) => args.clone(),
                    _ => Vec::new(),
                };
                (tp, args)
            })
            .collect()
    }

    #[test]
    fn next_position_direct_transition() {
        let eng = load("examples/counter.poly");
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(&eng, "Counter", "Count", "Increment");
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].0, count);
        // args is [n + 1]
        assert_eq!(answers[0].1.len(), 1);
    }

    #[test]
    fn next_position_via_internal_realization() {
        let eng = load("examples/counter.poly");
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(
            &eng, "Counter::Internal", "Count", "Increment",
        );
        assert_eq!(answers.len(), 1, "answers={answers:?}");
        assert_eq!(answers[0].0, count);
    }

    fn explain_position_via_query(
        eng: &Engine,
        iface: &str,
        pos: &str,
    ) -> (BTreeSet<Sym>, BTreeSet<Sym>, BTreeSet<Sym>) {
        let i_sym = eng.interner.find(iface).unwrap();
        let p_sym = eng.interner.find(pos).unwrap();

        let mut g = VarGen::new();
        let action_v = g.fresh();
        let actions_q = Query::single(vec![Goal::Direction {
            iface: Term::Sym(i_sym),
            position: Term::Sym(p_sym),
            action: Term::Var(action_v),
            params: Slot::Anon,
            guard: Slot::Anon,
        }]);
        let actions: BTreeSet<Sym> = eng.query(&actions_q, &Bindings::default())
            .iter()
            .map(|a| answer_sym(a, action_v))
            .collect();

        let fd = g.fresh();
        let fwd_q = Query::single(vec![
            Goal::Defer {
                defer: Term::Var(fd),
                source: Term::Sym(i_sym),
                target: Term::Anon,
            },
            Goal::DeferEntry {
                defer: Term::Var(fd),
                entry_idx: IndexSlot::Anon,
                source_pos: Term::Sym(p_sym),
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Anon,
                target_args: Slot::Anon,
            },
        ]);
        let forward: BTreeSet<Sym> = eng.query(&fwd_q, &Bindings::default())
            .iter()
            .map(|a| answer_sym(a, fd))
            .collect();

        let bd = g.fresh();
        let bwd_q = Query::single(vec![
            Goal::Defer {
                defer: Term::Var(bd),
                source: Term::Anon,
                target: Term::Sym(i_sym),
            },
            Goal::DeferEntry {
                defer: Term::Var(bd),
                entry_idx: IndexSlot::Anon,
                source_pos: Term::Anon,
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Sym(p_sym),
                target_args: Slot::Anon,
            },
        ]);
        let backward: BTreeSet<Sym> = eng.query(&bwd_q, &Bindings::default())
            .iter()
            .map(|a| answer_sym(a, bd))
            .collect();

        (actions, forward, backward)
    }

    #[test]
    fn explain_position_counter_internal_count() {
        // Counter::Internal at Count: no own directions; one forward defer
        // (Counter::Run -> Counter); one outbound defer (SetTo10 -> Button).
        // No backward defers.
        let eng = load("examples/counter.poly");
        let (actions, forward, backward) =
            explain_position_via_query(&eng, "Counter::Internal", "Count");
        let run = eng.interner.find("Counter::Run").unwrap();
        let setto10 = eng.interner.find("SetTo10").unwrap();
        assert!(actions.is_empty());
        assert_eq!(forward, [run, setto10].into_iter().collect());
        assert!(backward.is_empty());
    }

    #[test]
    fn explain_position_counter_count() {
        // Counter at Count: Increment + Decrement directions; no forward
        // defers; one backward defer (Counter::Run from Counter::Internal).
        let eng = load("examples/counter.poly");
        let (actions, forward, backward) =
            explain_position_via_query(&eng, "Counter", "Count");
        let inc = eng.interner.find("Increment").unwrap();
        let dec = eng.interner.find("Decrement").unwrap();
        let run = eng.interner.find("Counter::Run").unwrap();
        assert_eq!(actions, [inc, dec].into_iter().collect());
        assert!(forward.is_empty());
        assert_eq!(backward, [run].into_iter().collect());
    }

    #[test]
    fn explain_position_button_button() {
        // Button at Button: one direction (Press); no forward defers; one
        // backward defer (SetTo10 from Counter::Internal).
        let eng = load("examples/counter.poly");
        let (actions, forward, backward) =
            explain_position_via_query(&eng, "Button", "Button");
        let press = eng.interner.find("Press").unwrap();
        let setto10 = eng.interner.find("SetTo10").unwrap();
        assert_eq!(actions, [press].into_iter().collect());
        assert!(forward.is_empty());
        assert_eq!(backward, [setto10].into_iter().collect());
    }

    #[test]
    fn explain_position_grid_cell() {
        // Grid at Cell: four directions (Left, Right, Up, Down); no forward
        // defer; one backward defer (Grid::Run from Grid::Internal).
        let eng = load("examples/grid.poly");
        let (actions, forward, backward) =
            explain_position_via_query(&eng, "Grid", "Cell");
        let l = eng.interner.find("Left").unwrap();
        let r = eng.interner.find("Right").unwrap();
        let u = eng.interner.find("Up").unwrap();
        let d = eng.interner.find("Down").unwrap();
        let run = eng.interner.find("Grid::Run").unwrap();
        assert_eq!(actions, [l, r, u, d].into_iter().collect());
        assert!(forward.is_empty());
        assert_eq!(backward, [run].into_iter().collect());
    }

    #[test]
    fn next_position_via_setto10() {
        let eng = load("examples/counter.poly");
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(
            &eng, "Counter::Internal", "Count", "Press",
        );
        assert_eq!(answers.len(), 1, "answers={answers:?}");
        assert_eq!(answers[0].0, count);
        // args is [10]
        assert!(matches!(answers[0].1[0], Expr::LitInt(10)));
    }

    #[test]
    fn locate_action_grid() {
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();
        for action in ["Left", "Right", "Up", "Down"] {
            let q = locate_action_via_query(&eng, action);
            assert_eq!(
                q,
                [(grid, cell)].into_iter().collect(),
                "mismatch for action={action}"
            );
        }
    }

    // -------------------------------------------------------------------
    // Residuals + simplifier
    //
    // Counter has a position guard `n >= 0` on Count and a direction guard
    // `n > 0` on Decrement. These exercise the three simplifier outcomes:
    // symbolic residual (env empty), drop (residual reduces to false), and
    // satisfied (residual reduces to true and is cleared).
    // -------------------------------------------------------------------

    fn decrement_query(eng: &Engine) -> Query {
        let counter = eng.interner.find("Counter").unwrap();
        let count = eng.interner.find("Count").unwrap();
        let dec = eng.interner.find("Decrement").unwrap();
        Query::single(vec![Goal::Direction {
            iface: Term::Sym(counter),
            position: Term::Sym(count),
            action: Term::Sym(dec),
            params: Slot::Anon,
            guard: Slot::Anon,
        }])
    }

    #[test]
    fn decrement_residual_is_symbolic_with_empty_env() {
        let eng = load("examples/counter.poly");
        let q = decrement_query(&eng);
        let answers = eng.query(&q, &Bindings::default());
        assert_eq!(answers.len(), 1);
        // Residual is `n > 0` — left symbolic because env is empty.
        let n = eng.interner.find("n").unwrap();
        assert_eq!(answers[0].residual.len(), 1);
        match &answers[0].residual[0] {
            Expr::BinOp(BinOp::Gt, l, r) => {
                assert!(matches!(**l, Expr::Var(s) if s == n));
                assert!(matches!(**r, Expr::LitInt(0)));
            }
            other => panic!("expected `n > 0`, got {other:?}"),
        }
    }

    #[test]
    fn decrement_residual_collapses_to_true_when_satisfied() {
        let eng = load("examples/counter.poly");
        let q = decrement_query(&eng);
        let n = eng.interner.find("n").unwrap();
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(3));
        let answers = eng.query(&q, &env);
        assert_eq!(answers.len(), 1);
        assert!(answers[0].residual.is_empty(), "residual should be cleared");
    }

    #[test]
    fn decrement_answer_dropped_when_residual_false() {
        let eng = load("examples/counter.poly");
        let q = decrement_query(&eng);
        let n = eng.interner.find("n").unwrap();
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(0));
        let answers = eng.query(&q, &env);
        assert!(answers.is_empty(), "residual `0 > 0` is false; answer should be dropped");
    }

    #[test]
    fn position_guard_also_lands_in_residual() {
        // Querying Position alone (no direction) picks up the position guard.
        let eng = load("examples/counter.poly");
        let counter = eng.interner.find("Counter").unwrap();
        let count = eng.interner.find("Count").unwrap();
        let q = Query::single(vec![Goal::Position {
            iface: Term::Sym(counter),
            position: Term::Sym(count),
            params: Slot::Anon,
            guard: Slot::Anon,
        }]);
        let answers = eng.query(&q, &Bindings::default());
        assert_eq!(answers.len(), 1);
        // Residual is `n >= 0`.
        match &answers[0].residual[..] {
            [Expr::BinOp(BinOp::Ge, _, _)] => {}
            other => panic!("expected single `n >= 0` residual, got {other:?}"),
        }
    }

    #[test]
    fn where_clause_adds_user_constraint() {
        // Goal::Where lets the caller layer an extra constraint on top of any
        // guards picked up automatically. Here we layer `n > 5` on top of
        // Decrement's `n > 0` and resolve both with a concrete env.
        let eng = load("examples/counter.poly");
        let counter = eng.interner.find("Counter").unwrap();
        let count = eng.interner.find("Count").unwrap();
        let dec = eng.interner.find("Decrement").unwrap();
        let n = eng.interner.find("n").unwrap();

        let q = Query::single(vec![
            Goal::Direction {
                iface: Term::Sym(counter),
                position: Term::Sym(count),
                action: Term::Sym(dec),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
            Goal::Where(Expr::BinOp(
                BinOp::Gt,
                Box::new(Expr::Var(n)),
                Box::new(Expr::LitInt(5)),
            )),
        ]);

        // n=10: both `n > 0` and `n > 5` true → answer kept, residual cleared.
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(10));
        let answers = eng.query(&q, &env);
        assert_eq!(answers.len(), 1);
        assert!(answers[0].residual.is_empty());

        // n=3: `n > 0` true but `n > 5` false → answer dropped.
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(3));
        let answers = eng.query(&q, &env);
        assert!(answers.is_empty());

        // No env: the simplifier narrows `n > 0 ∧ n > 5` to the tighter
        // bound `n > 5` (Stage 4 interval narrowing).
        let answers = eng.query(&q, &Bindings::default());
        assert_eq!(answers.len(), 1);
        match &answers[0].residual[..] {
            [Expr::BinOp(BinOp::Gt, l, r)] => {
                assert!(matches!(**l, Expr::Var(s) if s == n));
                assert!(matches!(**r, Expr::LitInt(5)));
            }
            other => panic!("expected `n > 5`, got {other:?}"),
        }
    }
}
