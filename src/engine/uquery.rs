use std::collections::BTreeMap;

use super::facts::Facts;
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
    Transition {
        iface: Term, position: Term, action: Term, target_pos: Term, args: Slot,
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

fn match_goal(goal: &Goal, facts: &Facts, subst: &Subst) -> Vec<Subst> {
    match goal {
        Goal::Iface { iface, params } => facts
            .ifaces
            .iter()
            .filter_map(|f| {
                let s = unify_term(iface, f.iface, subst)?;
                unify_slot(params, Value::Params(f.params.clone()), &s)
            })
            .collect(),
        Goal::IfaceInternal { internal, external } => facts
            .iface_internals
            .iter()
            .filter_map(|f| {
                let s = unify_term(internal, f.internal, subst)?;
                unify_term(external, f.external, &s)
            })
            .collect(),
        Goal::SchemaRecord { schema, fields } => facts
            .schema_records
            .iter()
            .filter_map(|f| {
                let s = unify_term(schema, f.schema, subst)?;
                unify_slot(fields, Value::Params(f.fields.clone()), &s)
            })
            .collect(),
        Goal::SchemaSum { schema, variants } => facts
            .schema_sums
            .iter()
            .filter_map(|f| {
                let s = unify_term(schema, f.schema, subst)?;
                unify_slot(variants, Value::Variants(f.variants.clone()), &s)
            })
            .collect(),
        Goal::Position { iface, position, params, guard } => facts
            .positions
            .iter()
            .filter_map(|f| {
                let s = unify_term(iface, f.iface, subst)?;
                let s = unify_term(position, f.position, &s)?;
                let s = unify_slot(params, Value::Params(f.params.clone()), &s)?;
                unify_slot(guard, Value::Guard(f.guard.clone()), &s)
            })
            .collect(),
        Goal::Direction { iface, position, action, params, guard } => facts
            .directions
            .iter()
            .filter_map(|f| {
                let s = unify_term(iface, f.iface, subst)?;
                let s = unify_term(position, f.position, &s)?;
                let s = unify_term(action, f.action, &s)?;
                let s = unify_slot(params, Value::Params(f.params.clone()), &s)?;
                unify_slot(guard, Value::Guard(f.guard.clone()), &s)
            })
            .collect(),
        Goal::Transition { iface, position, action, target_pos, args } => facts
            .transitions
            .iter()
            .filter_map(|f| {
                let s = unify_term(iface, f.iface, subst)?;
                let s = unify_term(position, f.position, &s)?;
                let s = unify_term(action, f.action, &s)?;
                let s = unify_term(target_pos, f.target_pos, &s)?;
                unify_slot(args, Value::Args(f.args.clone()), &s)
            })
            .collect(),
        Goal::Defer { defer, source, target } => facts
            .defers
            .iter()
            .filter_map(|f| {
                let s = unify_term(defer, f.defer, subst)?;
                let s = unify_term(source, f.source, &s)?;
                unify_term(target, f.target, &s)
            })
            .collect(),
        Goal::DeferEntry {
            defer, entry_idx, source_pos, src_pattern, src_guard,
            target_pos, target_args,
        } => facts
            .defer_entries
            .iter()
            .filter_map(|f| {
                let s = unify_term(defer, f.defer, subst)?;
                let s = unify_index_slot(entry_idx, f.entry_idx, &s)?;
                let s = unify_term(source_pos, f.source_pos, &s)?;
                let s = unify_slot(src_pattern, Value::Pattern(f.src_pattern.clone()), &s)?;
                let s = unify_slot(src_guard, Value::Guard(f.src_guard.clone()), &s)?;
                let s = unify_term(target_pos, f.target_pos, &s)?;
                unify_slot(target_args, Value::Args(f.target_args.clone()), &s)
            })
            .collect(),
        Goal::DeferDir { defer, entry_idx, target_dir, source_dir } => facts
            .defer_dirs
            .iter()
            .filter_map(|f| {
                let s = unify_term(defer, f.defer, subst)?;
                let s = unify_index_slot(entry_idx, f.entry_idx, &s)?;
                let s = unify_dir_ref_pat(target_dir, &f.target_dir, &s)?;
                unify_dir_ref_pat(source_dir, &f.source_dir, &s)
            })
            .collect(),
    }
}


// ============================================================================
// Solver
// ============================================================================

fn solve(goals: &[Goal], facts: &Facts, subst: Subst) -> Vec<Subst> {
    let Some((first, rest)) = goals.split_first() else {
        return vec![subst];
    };
    let mut out = Vec::new();
    for s in match_goal(first, facts, &subst) {
        out.extend(solve(rest, facts, s));
    }
    out
}

pub fn run_query(facts: &Facts, query: &Query) -> Vec<Answer> {
    let mut answers = Vec::new();
    for body in &query.bodies {
        for s in solve(body, facts, Subst::default()) {
            answers.push(Answer { subst: s });
        }
    }
    answers
}


// ============================================================================
// Tests: hand-written reductions of existing queries
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn load(path: &str) -> Engine {
        let src = std::fs::read_to_string(path).expect("read example");
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
        facts: &Facts,
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
        run_query(facts, &q)
            .iter()
            .map(|a| (answer_sym(a, i_var), answer_sym(a, p_var)))
            .collect()
    }

    fn locate_action_native(eng: &Engine, action: &str) -> BTreeSet<(Sym, Sym)> {
        eng.locate_action(action)
            .locations
            .iter()
            .map(|l| (l.interface, l.position))
            .collect()
    }

    #[test]
    fn locate_action_matches_native_counter() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        for action in ["Increment", "Decrement", "Press"] {
            let q = locate_action_via_query(&eng, &facts, action);
            let n = locate_action_native(&eng, action);
            assert_eq!(q, n, "mismatch for action={action}");
        }
    }

    fn next_position_via_query(
        eng: &Engine,
        facts: &Facts,
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
        run_query(facts, &q)
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
        let facts = eng.facts();
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(&eng, &facts, "Counter", "Count", "Increment");
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].0, count);
        // args is [n + 1]
        assert_eq!(answers[0].1.len(), 1);
    }

    #[test]
    fn next_position_via_internal_realization() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(
            &eng, &facts, "Counter::Internal", "Count", "Increment",
        );
        assert_eq!(answers.len(), 1, "answers={answers:?}");
        assert_eq!(answers[0].0, count);
    }

    fn explain_position_via_query(
        eng: &Engine,
        facts: &Facts,
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
        let actions: BTreeSet<Sym> = run_query(facts, &actions_q)
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
        let forward: BTreeSet<Sym> = run_query(facts, &fwd_q)
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
        let backward: BTreeSet<Sym> = run_query(facts, &bwd_q)
            .iter()
            .map(|a| answer_sym(a, bd))
            .collect();

        (actions, forward, backward)
    }

    fn explain_position_native(
        eng: &Engine,
        iface: &str,
        pos: &str,
    ) -> (BTreeSet<Sym>, BTreeSet<Sym>, BTreeSet<Sym>) {
        let exp = eng.explain_position(iface, pos).unwrap();
        let actions: BTreeSet<Sym> = exp.actions.iter().copied().collect();
        let forward: BTreeSet<Sym> = exp.forward.iter().map(|f| f.defer).collect();
        let backward: BTreeSet<Sym> = exp.backward.iter().map(|b| b.defer).collect();
        (actions, forward, backward)
    }

    #[test]
    fn explain_position_counter_internal_count() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        let q = explain_position_via_query(&eng, &facts, "Counter::Internal", "Count");
        let n = explain_position_native(&eng, "Counter::Internal", "Count");
        assert_eq!(q, n);
    }

    #[test]
    fn explain_position_counter_count() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        let q = explain_position_via_query(&eng, &facts, "Counter", "Count");
        let n = explain_position_native(&eng, "Counter", "Count");
        assert_eq!(q, n);
    }

    #[test]
    fn explain_position_button_button() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        let q = explain_position_via_query(&eng, &facts, "Button", "Button");
        let n = explain_position_native(&eng, "Button", "Button");
        assert_eq!(q, n);
    }

    #[test]
    fn explain_position_grid_cell() {
        let eng = load("examples/grid.poly");
        let facts = eng.facts();
        let q = explain_position_via_query(&eng, &facts, "Grid", "Cell");
        let n = explain_position_native(&eng, "Grid", "Cell");
        assert_eq!(q, n);
    }

    #[test]
    fn next_position_via_setto10() {
        let eng = load("examples/counter.poly");
        let facts = eng.facts();
        let count = eng.interner.find("Count").unwrap();
        let answers = next_position_via_query(
            &eng, &facts, "Counter::Internal", "Count", "Press",
        );
        assert_eq!(answers.len(), 1, "answers={answers:?}");
        assert_eq!(answers[0].0, count);
        // args is [10]
        assert!(matches!(answers[0].1[0], Expr::LitInt(10)));
    }

    #[test]
    fn locate_action_matches_native_grid() {
        let eng = load("examples/grid.poly");
        let facts = eng.facts();
        for action in ["Left", "Right", "Up", "Down"] {
            let q = locate_action_via_query(&eng, &facts, action);
            let n = locate_action_native(&eng, action);
            assert_eq!(q, n, "mismatch for action={action}");
        }
    }
}
