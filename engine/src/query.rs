// The query AST below (Term, Slot, IndexSlot, DirRefPat, Goal, Query) is
// part of the library API. Variants reachable only from tests or from the
// not-yet-landed surface-syntax parser would otherwise show as dead from
// the binary build.
#![allow(dead_code)]

use std::collections::BTreeMap;

use super::eval::{conjoin, Bindings};
use super::simplify::{reduce, substitute};
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

/// Direction of a transitive defer walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    /// Follow defer entries `(defer.source, entry.source_pos) ->
    /// (defer.target, entry.target_pos)`.
    Forward,
    /// Follow defer entries in reverse: `(defer.target, entry.target_pos)
    /// -> (defer.source, entry.source_pos)`.
    Backward,
}

#[derive(Clone, Debug)]
pub enum Goal {
    Iface { iface: Term, params: Slot },
    IfaceInternal { internal: Term, external: Term },
    SchemaRecord { schema: Term, fields: Slot },
    SchemaSum { schema: Term, variants: Slot },
    Position {
        iface: Term,
        position: Term,
        /// Concrete args supplied to the position's formal parameters.
        /// Empty = no arg constraint (matches any instance). When
        /// non-empty, must match the arity of the position's formal
        /// params; for each pair, an equality `formal = arg` is pushed
        /// onto the answer's residual, so the simplifier substitutes
        /// the arg through the position's guard. Args may be concrete
        /// (`Coordinate(0, 0)`), arithmetic, or symbolic via
        /// `Expr::Var(sym)`. Logic-variable args (`Term::Var`) are not
        /// supported because the engine doesn't enumerate parameterized
        /// instances.
        args: Vec<Expr<Sym>>,
        params: Slot,
        guard: Slot,
    },
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
    /// Transitive walk over defer edges. Starting from `(from_iface,
    /// from_position)` (which must resolve to concrete `Sym`s in the
    /// current answer's substitution), yields every `(iface, position)`
    /// reachable by following defer edges in the given `walk` direction
    /// — including the starting pair itself (0-hop). A visited set
    /// terminates cycles.
    Reach {
        walk: Walk,
        from_iface: Term,
        from_position: Term,
        to_iface: Term,
        to_position: Term,
    },
    /// One-hop transition along a state-machine action. Looks up the
    /// realization defer for `iface` (the `Foo::Run` defer), finds the
    /// entry whose `source_pos` matches `from_position`, finds the
    /// direction mapping whose `target_dir` is `Named(action)`, and
    /// pulls the abstract `source_dir`'s `tgt_args`. The query's
    /// `from_args` are substituted through the abstract direction's
    /// `src_pattern` to evaluate the destination args; the destination
    /// position's guard (also substituted) lands on the residual.
    ///
    /// Yields the destination via `to_position` (unify with
    /// `source_dir.tgt_pos`) and binds the computed args through
    /// `to_args` as `Value::Args`. `iface`, `from_position`, and
    /// `action` must all resolve to concrete `Sym`s in the current
    /// substitution; otherwise the goal yields nothing.
    Step {
        iface: Term,
        from_position: Term,
        from_args: Vec<Expr<Sym>>,
        action: Term,
        to_position: Term,
        to_args: Slot,
    },
    /// Find action paths from `(iface, from_position, from_args)` to
    /// reachable destinations. BFS over `Step` edges with a visited set
    /// keyed on `(pos, folded_args)`; each reachable state is yielded
    /// exactly once, with the shortest action sequence discovered. The
    /// start state itself is yielded with an empty path (depth 0).
    ///
    /// `from_args` must fold to a fully concrete value under the env
    /// — the visited set requires evaluable args. Symbolic starts
    /// yield no answers.
    ///
    /// `to_position` (a `Term`) and `to_args` (a `Vec<Expr<Sym>>`,
    /// empty = no constraint) filter the yielded set: only reached
    /// states matching them surface. `path` binds `Value::Path(Vec<Sym>)`
    /// — the action names traversed, in order. `max_depth` optionally
    /// caps BFS depth.
    Path {
        iface: Term,
        from_position: Term,
        from_args: Vec<Expr<Sym>>,
        to_position: Term,
        to_args: Vec<Expr<Sym>>,
        path: Slot,
        max_depth: Option<usize>,
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
    /// A sequence of action names — the path produced by `Goal::Path`'s
    /// BFS over state-machine transitions.
    Path(Vec<Sym>),
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

fn match_goal<'a>(
    goal: &'a Goal,
    eng: &'a Engine,
    env: &'a Bindings,
    ans: Answer,
) -> Box<dyn Iterator<Item = Answer> + 'a> {
    match goal {
        Goal::Iface { iface, params } => Box::new(
            eng.iface_relation().filter_map(move |i| {
                let s = unify_term(iface, i.name, &ans.subst)?;
                let s = unify_slot(params, Value::Params(i.params.clone()), &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::IfaceInternal { internal, external } => Box::new(
            eng.iface_internal_relation().filter_map(move |(int_sym, ext_sym)| {
                let s = unify_term(internal, int_sym, &ans.subst)?;
                let s = unify_term(external, ext_sym, &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::SchemaRecord { schema, fields } => Box::new(
            eng.schema_record_relation().filter_map(move |(name, flds)| {
                let s = unify_term(schema, name, &ans.subst)?;
                let s = unify_slot(fields, Value::Params(flds.to_vec()), &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::SchemaSum { schema, variants } => Box::new(
            eng.schema_sum_relation().filter_map(move |(name, vars)| {
                let s = unify_term(schema, name, &ans.subst)?;
                let s = unify_slot(variants, Value::Variants(vars.to_vec()), &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::Position { iface, position, args, params, guard } => Box::new(
            eng.position_relation().filter_map(move |(i_sym, p)| {
                let s = unify_term(iface, i_sym, &ans.subst)?;
                let s = unify_term(position, p.name, &s)?;
                // Arity check: query supplied args must match formal-param
                // count if non-empty. Empty `args` means no arg constraint.
                if !args.is_empty() && args.len() != p.params.len() {
                    return None;
                }
                let s = unify_slot(params, Value::Params(p.params.clone()), &s)?;
                let s = unify_slot(guard, Value::Guard(p.guard.clone()), &s)?;
                let mut next = ans.with_subst(s);
                // Substitute the query's args directly into the position
                // guard before pushing it onto the residual. We don't emit
                // equalities (`formal = arg`) because the simplifier would
                // surface them in the output residual; the args are
                // internal plumbing, not constraints the caller wants
                // back. Empty `args` skips substitution.
                if let Some(g) = &p.guard {
                    let g_subst = if args.is_empty() {
                        g.clone()
                    } else {
                        let mut sub: BTreeMap<Sym, Expr<Sym>> = BTreeMap::new();
                        for (formal, arg) in p.params.iter().zip(args.iter()) {
                            sub.insert(formal.name, arg.clone());
                        }
                        substitute(g, &sub)
                    };
                    next.residual.push(g_subst);
                }
                Some(next)
            }),
        ),
        Goal::Direction { iface, position, action, params, guard } => Box::new(
            eng.direction_relation().filter_map(move |(i_sym, p_sym, d)| {
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
            }),
        ),
        Goal::Defer { defer, source, target } => Box::new(
            eng.defer_relation().filter_map(move |d| {
                let s = unify_term(defer, d.name, &ans.subst)?;
                let s = unify_term(source, d.source, &s)?;
                let s = unify_term(target, d.target, &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::DeferEntry {
            defer, entry_idx, source_pos, src_pattern, src_guard,
            target_pos, target_args,
        } => Box::new(
            eng.defer_entry_relation().filter_map(move |(d_sym, idx, e)| {
                let s = unify_term(defer, d_sym, &ans.subst)?;
                let s = unify_index_slot(entry_idx, idx, &s)?;
                let s = unify_term(source_pos, e.source_pos, &s)?;
                let s = unify_slot(src_pattern, Value::Pattern(e.source_pattern.clone()), &s)?;
                let s = unify_slot(src_guard, Value::Guard(e.source_guard.clone()), &s)?;
                let s = unify_term(target_pos, e.target_pos, &s)?;
                let s = unify_slot(target_args, Value::Args(e.target_args.clone()), &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::DeferDir { defer, entry_idx, target_dir, source_dir } => Box::new(
            eng.defer_dir_relation().filter_map(move |(d_sym, idx, m)| {
                let s = unify_term(defer, d_sym, &ans.subst)?;
                let s = unify_index_slot(entry_idx, idx, &s)?;
                let s = unify_dir_ref_pat(target_dir, &m.target_dir, &s)?;
                let s = unify_dir_ref_pat(source_dir, &m.source_dir, &s)?;
                Some(ans.with_subst(s))
            }),
        ),
        Goal::Reach { walk, from_iface, from_position, to_iface, to_position } => {
            // Resolve the starting (iface, position) to concrete Syms via
            // the current substitution. If either is unbound, the goal
            // fails (no answers); the caller should ground the start.
            let Some(start_iface) = resolve_term(from_iface, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let Some(start_pos) = resolve_term(from_position, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let reachable = collect_reachable(eng, start_iface, start_pos, *walk);
            Box::new(reachable.into_iter().filter_map(move |(i, p)| {
                let s = unify_term(to_iface, i, &ans.subst)?;
                let s = unify_term(to_position, p, &s)?;
                Some(ans.with_subst(s))
            }))
        }
        Goal::Step {
            iface,
            from_position,
            from_args,
            action,
            to_position,
            to_args,
        } => {
            let Some(iface_sym) = resolve_term(iface, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let Some(from_pos_sym) = resolve_term(from_position, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let Some(action_sym) = resolve_term(action, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let steps = collect_action_steps(
                eng,
                iface_sym,
                from_pos_sym,
                from_args,
                action_sym,
            );
            Box::new(steps.into_iter().filter_map(move |step| {
                let s = unify_term(to_position, step.tgt_pos, &ans.subst)?;
                let s = unify_slot(to_args, Value::Args(step.tgt_args.clone()), &s)?;
                let mut next = ans.with_subst(s);
                if let Some(g) = step.guard {
                    next.residual.push(g);
                }
                Some(next)
            }))
        }
        Goal::Path {
            iface,
            from_position,
            from_args,
            to_position,
            to_args,
            path,
            max_depth,
        } => {
            let Some(iface_sym) = resolve_term(iface, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let Some(from_pos_sym) = resolve_term(from_position, &ans.subst) else {
                return Box::new(std::iter::empty());
            };
            let reachable = collect_action_paths(
                eng,
                env,
                iface_sym,
                from_pos_sym,
                from_args,
                *max_depth,
            );
            Box::new(reachable.into_iter().filter_map(move |reached| {
                let s = unify_term(to_position, reached.pos, &ans.subst)?;
                // If the user supplied to_args, require structural
                // equality with the (folded) reached args.
                if !to_args.is_empty() {
                    if to_args.len() != reached.args.len() {
                        return None;
                    }
                    for (want, got) in to_args.iter().zip(reached.args.iter()) {
                        let want_folded =
                            super::eval::const_fold(eng, want, env);
                        if &want_folded != got {
                            return None;
                        }
                    }
                }
                let s = unify_slot(path, Value::Path(reached.path.clone()), &s)?;
                Some(ans.with_subst(s))
            }))
        }
        Goal::Where(expr) => Box::new(std::iter::once(ans.push_residual(expr.clone()))),
    }
}


// ============================================================================
// Reachability via defer chains (Goal::Reach)
// ============================================================================

/// Resolve a `Term` to a concrete `Sym` if possible.
fn resolve_term(t: &Term, subst: &Subst) -> Option<Sym> {
    match t {
        Term::Sym(s) => Some(*s),
        Term::Var(v) => match subst.get(v) {
            Some(Value::Sym(s)) => Some(*s),
            _ => None,
        },
        Term::Anon => None,
    }
}

/// BFS from `(start_iface, start_pos)` over defer edges in the given
/// direction, yielding each reachable pair (including the start) in
/// discovery order with cycles handled by a visited set.
fn collect_reachable(
    eng: &Engine,
    start_iface: Sym,
    start_pos: Sym,
    walk: Walk,
) -> Vec<(Sym, Sym)> {
    use std::collections::{BTreeSet, VecDeque};
    let mut visited: BTreeSet<(Sym, Sym)> = BTreeSet::new();
    let mut queue: VecDeque<(Sym, Sym)> = VecDeque::new();
    let mut out: Vec<(Sym, Sym)> = Vec::new();

    queue.push_back((start_iface, start_pos));
    visited.insert((start_iface, start_pos));
    while let Some((i, p)) = queue.pop_front() {
        out.push((i, p));
        for next in step(eng, i, p, walk) {
            if visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    out
}

/// One-hop neighbours of `(iface, pos)` via defer entries in the given
/// direction.
fn step(eng: &Engine, iface: Sym, pos: Sym, walk: Walk) -> Vec<(Sym, Sym)> {
    let mut out = Vec::new();
    for d in eng.defer_relation() {
        let touches_iface = match walk {
            Walk::Forward => d.source == iface,
            Walk::Backward => d.target == iface,
        };
        if !touches_iface {
            continue;
        }
        for entry in &d.entries {
            let (match_pos, next_iface, next_pos) = match walk {
                Walk::Forward => (entry.source_pos, d.target, entry.target_pos),
                Walk::Backward => (entry.target_pos, d.source, entry.source_pos),
            };
            if match_pos == pos {
                out.push((next_iface, next_pos));
            }
        }
    }
    out
}


// ============================================================================
// Action steps via the realization defer (Goal::Step)
// ============================================================================

/// Result of one action step: destination position name, computed args,
/// and the destination's substituted guard (if any).
struct ActionStep {
    tgt_pos: Sym,
    tgt_args: Vec<Expr<Sym>>,
    guard: Option<Expr<Sym>>,
}

/// Find all action steps from `(iface, from_pos, from_args)` via
/// `action`. Each step substitutes `from_args` through the matching
/// abstract direction's `tgt_args` to compute the destination args,
/// and substitutes through the destination position's guard.
fn collect_action_steps(
    eng: &Engine,
    iface: Sym,
    from_pos: Sym,
    from_args: &[Expr<Sym>],
    action: Sym,
) -> Vec<ActionStep> {
    let mut out = Vec::new();

    // The realization defer for `iface` is the one paired with its
    // `::Internal` carrier — find it via iface_internal_relation.
    let Some(internal_sym) = eng
        .iface_internal_relation()
        .find(|(_, ext)| *ext == iface)
        .map(|(int, _)| int)
    else {
        return out;
    };
    let Some(realization) = eng
        .defer_relation()
        .find(|d| d.source == internal_sym && d.target == iface)
    else {
        return out;
    };

    for entry in &realization.entries {
        if entry.source_pos != from_pos {
            continue;
        }
        for mapping in &entry.directions {
            // We only care about realizations: target_dir is a Named
            // direction (the external action), source_dir is the
            // Abstract internal direction whose tgt_args we evaluate.
            let DirRef::Named(named) = &mapping.target_dir else {
                continue;
            };
            if *named != action {
                continue;
            }
            let DirRef::Abstract { src_pattern, tgt_pos, tgt_args, .. } =
                &mapping.source_dir
            else {
                continue;
            };

            // Build a substitution from src_pattern bindings to from_args.
            // Wildcards skipped; arity must match.
            let Some(src_sub) = bind_pattern(src_pattern, from_args) else {
                continue;
            };
            let computed_to_args: Vec<Expr<Sym>> =
                tgt_args.iter().map(|e| substitute(e, &src_sub)).collect();

            // Look up the destination position in the external iface to
            // pull its formal params + guard. (The external position
            // `tgt_pos` is the same name as the internal one — the
            // realization defer maps source positions to target positions
            // with the same name and arity.)
            let dst_guard =
                eng.interfaces.get(&iface).and_then(|i| i.position(tgt_pos)).and_then(|p| {
                    p.guard.as_ref().map(|g| {
                        let dst_sub: BTreeMap<Sym, Expr<Sym>> = p
                            .params
                            .iter()
                            .zip(computed_to_args.iter())
                            .map(|(formal, arg)| (formal.name, arg.clone()))
                            .collect();
                        substitute(g, &dst_sub)
                    })
                });

            out.push(ActionStep {
                tgt_pos: *tgt_pos,
                tgt_args: computed_to_args,
                guard: dst_guard,
            });
        }
    }

    out
}

/// Build a substitution from a pattern + concrete args. `Pattern::Bind(name)`
/// inserts `name -> arg`; `Pattern::Wildcard` is skipped. Returns `None`
/// on arity mismatch.
fn bind_pattern(
    pattern: &[Pattern<Sym>],
    args: &[Expr<Sym>],
) -> Option<BTreeMap<Sym, Expr<Sym>>> {
    if pattern.len() != args.len() {
        return None;
    }
    let mut sub = BTreeMap::new();
    for (pat, arg) in pattern.iter().zip(args.iter()) {
        match pat {
            Pattern::Bind(name) => {
                sub.insert(*name, arg.clone());
            }
            Pattern::Wildcard => {}
        }
    }
    Some(sub)
}


// ============================================================================
// Path-finding via BFS over Step edges (Goal::Path)
// ============================================================================

/// One reached state in the BFS: position name, evaluated (folded)
/// args, and the action sequence that got us there.
struct ReachedPath {
    pos: Sym,
    args: Vec<Expr<Sym>>,
    path: Vec<Sym>,
}

/// BFS from `(iface, from_pos, from_args)` along state-machine action
/// transitions. The visited set is keyed on `(pos, folded_args)` so
/// each reachable state is yielded once with the shortest discovered
/// path (BFS order). Folding uses `env`, so iface-level params
/// (e.g. `Width`) substitute through arithmetic. A destination whose
/// guard folds to `false` under `env` is skipped — anything else (true
/// or symbolic) is enqueued. The starting state is yielded with an
/// empty path (depth 0).
fn collect_action_paths(
    eng: &Engine,
    env: &Bindings,
    iface: Sym,
    from_pos: Sym,
    from_args: &[Expr<Sym>],
    max_depth: Option<usize>,
) -> Vec<ReachedPath> {
    use std::collections::VecDeque;
    use super::eval::const_fold;

    let folded_start: Vec<Expr<Sym>> =
        from_args.iter().map(|e| const_fold(eng, e, env)).collect();
    let mut visited: Vec<(Sym, Vec<Expr<Sym>>)> = Vec::new();
    let mut queue: VecDeque<ReachedPath> = VecDeque::new();
    let mut out: Vec<ReachedPath> = Vec::new();

    visited.push((from_pos, folded_start.clone()));
    queue.push_back(ReachedPath {
        pos: from_pos,
        args: folded_start,
        path: Vec::new(),
    });

    while let Some(node) = queue.pop_front() {
        // Yield this state. (Always include the start at depth 0.)
        out.push(ReachedPath {
            pos: node.pos,
            args: node.args.clone(),
            path: node.path.clone(),
        });
        // Stop expanding at max_depth.
        if let Some(max) = max_depth {
            if node.path.len() >= max {
                continue;
            }
        }
        // Try every named action available at this position.
        let Some(iface_decl) = eng.interfaces.get(&iface) else { continue };
        let Some(pos_decl) = iface_decl.position(&node.pos) else { continue };
        for dir in &pos_decl.directions {
            for step in collect_action_steps(eng, iface, node.pos, &node.args, dir.name) {
                // Drop transitions whose destination guard folds to literal false.
                if let Some(g) = &step.guard {
                    let folded = const_fold(eng, g, env);
                    if matches!(folded, Expr::LitBool(false)) {
                        continue;
                    }
                }
                let folded_args: Vec<Expr<Sym>> = step
                    .tgt_args
                    .iter()
                    .map(|e| const_fold(eng, e, env))
                    .collect();
                let key = (step.tgt_pos, folded_args.clone());
                if visited.iter().any(|v| v == &key) {
                    continue;
                }
                visited.push(key);
                let mut new_path = node.path.clone();
                new_path.push(dir.name);
                queue.push_back(ReachedPath {
                    pos: step.tgt_pos,
                    args: folded_args,
                    path: new_path,
                });
            }
        }
    }
    out
}


// ============================================================================
// Solver — explicit-stack, iteration-based
// ============================================================================

/// One frame on the search stack: an in-flight goal whose `matches`
/// iterator is producing answers, along with the goals to attempt
/// after this one succeeds.
struct Frame<'a> {
    rest_goals: &'a [Goal],
    matches: Box<dyn Iterator<Item = Answer> + 'a>,
}

/// Lazy stream of answers for a `Query`. Created by `Engine::query`.
///
/// The solver is depth-first with explicit backtracking: pull the next
/// match from the top frame, push a child frame for the next goal, pop
/// when a frame is exhausted. The simplifier runs per-answer as a
/// `filter_map`-style step in `next()` — answers whose residual reduces
/// to `false` are silently dropped.
///
/// Disjunctive bodies (`Query::or`) are processed sequentially: when
/// the search stack drains for one body, the next body's first goal is
/// pushed.
pub struct Answers<'a> {
    eng: &'a Engine,
    env: &'a Bindings,
    bodies: &'a [Vec<Goal>],
    body_idx: usize,
    stack: Vec<Frame<'a>>,
}

impl<'a> Iterator for Answers<'a> {
    type Item = Answer;

    fn next(&mut self) -> Option<Answer> {
        loop {
            // No active frames: advance to the next disjunct, or finish.
            if self.stack.is_empty() {
                if self.body_idx >= self.bodies.len() {
                    return None;
                }
                let body = &self.bodies[self.body_idx];
                self.body_idx += 1;
                if let Some((first, rest)) = body.split_first() {
                    let matches = match_goal(first, self.eng, self.env, Answer::empty());
                    self.stack.push(Frame { rest_goals: rest, matches });
                } else {
                    // Vacuously-true body: yield the empty answer (modulo
                    // the simplifier, which just sees an empty residual).
                    if let Some(simplified) =
                        simplify_answer(self.eng, &Answer::empty(), self.env)
                    {
                        return Some(simplified);
                    }
                }
                continue;
            }

            // Try to advance the top frame.
            let last = self.stack.len() - 1;
            let next_match = self.stack[last].matches.next();
            let rest_goals = self.stack[last].rest_goals;

            match next_match {
                None => {
                    // Frame exhausted; backtrack.
                    self.stack.pop();
                }
                Some(ans) => match rest_goals.split_first() {
                    Some((next_goal, new_rest)) => {
                        // More goals: push a frame for the next one.
                        let matches = match_goal(next_goal, self.eng, self.env, ans);
                        self.stack.push(Frame { rest_goals: new_rest, matches });
                    }
                    None => {
                        // All goals matched: simplify and yield (or drop
                        // if the residual reduces to `false`).
                        if let Some(simplified) =
                            simplify_answer(self.eng, &ans, self.env)
                        {
                            return Some(simplified);
                        }
                    }
                },
            }
        }
    }
}

impl Engine {
    /// Run `query` against the loaded program. Returns a lazy iterator
    /// of answers; the solver runs incrementally as the consumer pulls
    /// items, and answer spaces with infinite valid completions can be
    /// truncated with `.take(n)` or filtered.
    ///
    /// Each disjunctive body is solved by unification against the
    /// relations exposed in `relations.rs`; per-position guards and
    /// `Goal::Where` expressions accumulate as residuals on the
    /// resulting `Answer`s; the simplifier then reduces each residual
    /// against `env`. An empty `env` (`Bindings::default()`) is the
    /// common case — pass concrete variable bindings to specialize.
    ///
    /// Answers whose residuals reduce to `false` are dropped. Answers
    /// whose residuals reduce to `true` are yielded with `residual:
    /// vec![]`. Anything else is kept as a single residual conjunct.
    pub fn query<'a>(&'a self, query: &'a Query, env: &'a Bindings) -> Answers<'a> {
        Answers {
            eng: self,
            env,
            bodies: &query.bodies,
            body_idx: 0,
            stack: Vec::new(),
        }
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
                args: Vec::new(),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
        ]);
        eng.query(&q, &Bindings::default())
            .map(|a| (answer_sym(&a, i_var), answer_sym(&a, p_var)))
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
            .map(|a| answer_sym(&a, action_v))
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
            .map(|a| answer_sym(&a, fd))
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
            .map(|a| answer_sym(&a, bd))
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
        let answers: Vec<_> = eng.query(&q, &Bindings::default()).collect();
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
        let answers: Vec<_> = eng.query(&q, &env).collect();
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
        let answers: Vec<_> = eng.query(&q, &env).collect();
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
            args: Vec::new(),
            params: Slot::Anon,
            guard: Slot::Anon,
        }]);
        let answers: Vec<_> = eng.query(&q, &Bindings::default()).collect();
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
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert_eq!(answers.len(), 1);
        assert!(answers[0].residual.is_empty());

        // n=3: `n > 0` true but `n > 5` false → answer dropped.
        let mut env = Bindings::default();
        env.insert(n, super::super::eval::Value::Int(3));
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert!(answers.is_empty());

        // No env: the simplifier narrows `n > 0 ∧ n > 5` to the tighter
        // bound `n > 5` (Stage 4 interval narrowing).
        let answers: Vec<_> = eng.query(&q, &Bindings::default()).collect();
        assert_eq!(answers.len(), 1);
        match &answers[0].residual[..] {
            [Expr::BinOp(BinOp::Gt, l, r)] => {
                assert!(matches!(**l, Expr::Var(s) if s == n));
                assert!(matches!(**r, Expr::LitInt(5)));
            }
            other => panic!("expected `n > 5`, got {other:?}"),
        }
    }


    // ========================================================================
    // Goal::Reach — transitive defer walks
    // ========================================================================

    #[test]
    fn reach_forward_chain_two_hops() {
        // chain.poly: A.StateA --Defer1--> B.StateB --Defer2--> C.StateC.
        // Walking forward from (A, StateA) should reach all three pairs.
        let eng = load("examples/chain.poly");
        let a = eng.interner.find("InterfaceA").unwrap();
        let b = eng.interner.find("InterfaceB").unwrap();
        let c = eng.interner.find("InterfaceC").unwrap();
        let state_a = eng.interner.find("StateA").unwrap();
        let state_b = eng.interner.find("StateB").unwrap();
        let state_c = eng.interner.find("StateC").unwrap();

        let mut g = VarGen::new();
        let to_iface = g.fresh();
        let to_pos = g.fresh();
        let q = Query::single(vec![Goal::Reach {
            walk: Walk::Forward,
            from_iface: Term::Sym(a),
            from_position: Term::Sym(state_a),
            to_iface: Term::Var(to_iface),
            to_position: Term::Var(to_pos),
        }]);
        let pairs: BTreeSet<(Sym, Sym)> = eng
            .query(&q, &Bindings::default())
            .map(|ans| (answer_sym(&ans, to_iface), answer_sym(&ans, to_pos)))
            .collect();
        let expected: BTreeSet<(Sym, Sym)> =
            [(a, state_a), (b, state_b), (c, state_c)].into_iter().collect();
        assert_eq!(pairs, expected);
    }

    #[test]
    fn reach_forward_then_direction_finds_action_via_chain() {
        // The motivating use case: "given InterfaceA at StateA, what
        // actions are possible at InterfaceC?" Answer: ActionC.
        let eng = load("examples/chain.poly");
        let a = eng.interner.find("InterfaceA").unwrap();
        let c = eng.interner.find("InterfaceC").unwrap();
        let state_a = eng.interner.find("StateA").unwrap();
        let action_c = eng.interner.find("ActionC").unwrap();

        let mut g = VarGen::new();
        let pos_at_c = g.fresh();
        let action = g.fresh();
        let q = Query::single(vec![
            Goal::Reach {
                walk: Walk::Forward,
                from_iface: Term::Sym(a),
                from_position: Term::Sym(state_a),
                to_iface: Term::Sym(c),
                to_position: Term::Var(pos_at_c),
            },
            Goal::Direction {
                iface: Term::Sym(c),
                position: Term::Var(pos_at_c),
                action: Term::Var(action),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
        ]);
        let actions: BTreeSet<Sym> = eng
            .query(&q, &Bindings::default())
            .map(|ans| answer_sym(&ans, action))
            .collect();
        let expected: BTreeSet<Sym> = [action_c].into_iter().collect();
        assert_eq!(actions, expected);
    }

    #[test]
    fn reach_backward_chain_two_hops() {
        // Walking backward from (C, StateC) should reach C, B, A.
        let eng = load("examples/chain.poly");
        let a = eng.interner.find("InterfaceA").unwrap();
        let b = eng.interner.find("InterfaceB").unwrap();
        let c = eng.interner.find("InterfaceC").unwrap();
        let state_a = eng.interner.find("StateA").unwrap();
        let state_b = eng.interner.find("StateB").unwrap();
        let state_c = eng.interner.find("StateC").unwrap();

        let mut g = VarGen::new();
        let to_iface = g.fresh();
        let to_pos = g.fresh();
        let q = Query::single(vec![Goal::Reach {
            walk: Walk::Backward,
            from_iface: Term::Sym(c),
            from_position: Term::Sym(state_c),
            to_iface: Term::Var(to_iface),
            to_position: Term::Var(to_pos),
        }]);
        let pairs: BTreeSet<(Sym, Sym)> = eng
            .query(&q, &Bindings::default())
            .map(|ans| (answer_sym(&ans, to_iface), answer_sym(&ans, to_pos)))
            .collect();
        let expected: BTreeSet<(Sym, Sym)> =
            [(a, state_a), (b, state_b), (c, state_c)].into_iter().collect();
        assert_eq!(pairs, expected);
    }


    // ========================================================================
    // Goal::Position with concrete args (grid.poly)
    //
    // Stage A of parameterized queries: query supplies args for the
    // position's formal parameters; equalities are pushed onto the
    // residual; the simplifier substitutes through the position guard.
    // ========================================================================

    /// Build a `Bindings` env with `Width = w, Height = h` for grid.poly.
    fn grid_env(eng: &Engine, w: i64, h: i64) -> Bindings {
        let mut env = Bindings::default();
        let width = eng.interner.find("Width").unwrap();
        let height = eng.interner.find("Height").unwrap();
        env.insert(width, super::super::eval::Value::Int(w));
        env.insert(height, super::super::eval::Value::Int(h));
        env
    }

    /// Build `Coordinate(x, y)` as an `Expr<Sym>`.
    fn coord_expr(eng: &Engine, x: i64, y: i64) -> Expr<Sym> {
        let coord = eng.interner.find("Coordinate").unwrap();
        Expr::Construct(
            coord,
            vec![Expr::LitInt(x), Expr::LitInt(y)],
        )
    }

    fn cell_query(eng: &Engine, args: Vec<Expr<Sym>>) -> Query {
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();
        Query::single(vec![Goal::Position {
            iface: Term::Sym(grid),
            position: Term::Sym(cell),
            args,
            params: Slot::Anon,
            guard: Slot::Anon,
        }])
    }

    #[test]
    fn cell_in_bounds_yields_one_answer_with_empty_residual() {
        // Cell[Coordinate(5, 5)] in Grid[10, 10]: guard `1 <= 5 <= 10 ∧ 1 <= 5 <= 10`
        // reduces to true under the simplifier, so the residual clears.
        let eng = load("examples/grid.poly");
        let q = cell_query(&eng, vec![coord_expr(&eng, 5, 5)]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert_eq!(answers.len(), 1);
        assert!(
            answers[0].residual.is_empty(),
            "residual should clear, got {:?}",
            answers[0].residual,
        );
    }

    #[test]
    fn cell_out_of_bounds_drops_the_answer() {
        // Cell[Coordinate(11, 5)] in Grid[10, 10]: guard `... ∧ 11 <= 10 ∧ ...`
        // reduces to false; the answer is dropped.
        let eng = load("examples/grid.poly");
        let q = cell_query(&eng, vec![coord_expr(&eng, 11, 5)]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert!(
            answers.is_empty(),
            "out-of-bounds Cell should be dropped, got {answers:?}",
        );
    }

    // ========================================================================
    // Goal::Step — one-hop transitions
    // ========================================================================

    #[test]
    fn step_right_in_grid_yields_neighbour() {
        // Right at Cell[Coordinate(5, 5)] in Grid[10, 10] → Cell[Coordinate(6, 5)].
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();
        let right = eng.interner.find("Right").unwrap();

        let mut g = VarGen::new();
        let dst_pos = g.fresh();
        let dst_args = g.fresh();
        let q = Query::single(vec![Goal::Step {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 5, 5)],
            action: Term::Sym(right),
            to_position: Term::Var(dst_pos),
            to_args: Slot::Var(dst_args),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert_eq!(answers.len(), 1);
        let ans = &answers[0];
        assert_eq!(answer_sym(ans, dst_pos), cell);
        match ans.subst.get(&dst_args) {
            Some(Value::Args(args)) => {
                assert_eq!(args.len(), 1);
                // The substituted+folded coordinate should be Coordinate(6, 5).
                let coord = eng.interner.find("Coordinate").unwrap();
                let folded = super::super::eval::const_fold(&eng, &args[0], &env);
                match &folded {
                    Expr::Construct(name, parts) if *name == coord => {
                        assert!(matches!(parts[0], Expr::LitInt(6)));
                        assert!(matches!(parts[1], Expr::LitInt(5)));
                    }
                    other => panic!("expected Coordinate(6, 5), got {other:?}"),
                }
            }
            other => panic!("expected Value::Args, got {other:?}"),
        }
        // In-bounds destination — guard reduces to true → empty residual.
        assert!(ans.residual.is_empty(), "residual {:?}", ans.residual);
    }

    #[test]
    fn step_left_at_left_edge_drops_via_guard() {
        // Left at Cell[Coordinate(1, 5)] in Grid[10, 10]:
        // computed destination Cell[Coordinate(0, 5)] fails the position
        // guard `1 <= c.x`. Answer dropped.
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();
        let left = eng.interner.find("Left").unwrap();

        let mut g = VarGen::new();
        let dst_pos = g.fresh();
        let dst_args = g.fresh();
        let q = Query::single(vec![Goal::Step {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 1, 5)],
            action: Term::Sym(left),
            to_position: Term::Var(dst_pos),
            to_args: Slot::Var(dst_args),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert!(answers.is_empty(), "out-of-bounds destination should drop, got {answers:?}");
    }

    #[test]
    fn step_increment_counter() {
        // Increment at Count[3] → Count[4]. Counter::Run desugars
        // Increment to abstract direction Count[n] => Count[n + 1].
        let eng = load("examples/counter.poly");
        let counter = eng.interner.find("Counter").unwrap();
        let count = eng.interner.find("Count").unwrap();
        let increment = eng.interner.find("Increment").unwrap();

        let mut g = VarGen::new();
        let dst_pos = g.fresh();
        let dst_args = g.fresh();
        let q = Query::single(vec![Goal::Step {
            iface: Term::Sym(counter),
            from_position: Term::Sym(count),
            from_args: vec![Expr::LitInt(3)],
            action: Term::Sym(increment),
            to_position: Term::Var(dst_pos),
            to_args: Slot::Var(dst_args),
        }]);
        let answers: Vec<_> = eng.query(&q, &Bindings::default()).collect();
        assert_eq!(answers.len(), 1);
        let ans = &answers[0];
        assert_eq!(answer_sym(ans, dst_pos), count);
        match ans.subst.get(&dst_args) {
            Some(Value::Args(args)) => {
                assert_eq!(args.len(), 1);
                let folded = super::super::eval::const_fold(
                    &eng,
                    &args[0],
                    &Bindings::default(),
                );
                assert!(matches!(folded, Expr::LitInt(4)));
            }
            other => panic!("expected Value::Args([4]), got {other:?}"),
        }
    }

    // ========================================================================
    // Goal::Path — BFS over Step edges with action-path tracking
    // ========================================================================

    /// Helper: read a Vec<Sym> action sequence out of the answer's `path`
    /// slot, resolving Syms to their string names.
    fn answer_path<'a>(eng: &'a Engine, ans: &'a Answer, v: VarId) -> Vec<&'a str> {
        match ans.subst.get(&v) {
            Some(Value::Path(syms)) => syms.iter().map(|s| eng.resolve(*s)).collect(),
            other => panic!("expected Value::Path, got {other:?}"),
        }
    }

    #[test]
    fn path_to_self_is_empty() {
        // From Cell[(3, 3)] to Cell[(3, 3)]: zero hops.
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();

        let mut g = VarGen::new();
        let path_v = g.fresh();
        let q = Query::single(vec![Goal::Path {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 3, 3)],
            to_position: Term::Sym(cell),
            to_args: vec![coord_expr(&eng, 3, 3)],
            path: Slot::Var(path_v),
            max_depth: Some(0),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert_eq!(answers.len(), 1);
        assert!(answer_path(&eng, &answers[0], path_v).is_empty());
    }

    #[test]
    fn path_to_neighbour_is_one_action() {
        // From Cell[(3, 3)] to Cell[(4, 3)]: one Right step.
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();

        let mut g = VarGen::new();
        let path_v = g.fresh();
        let q = Query::single(vec![Goal::Path {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 3, 3)],
            to_position: Term::Sym(cell),
            to_args: vec![coord_expr(&eng, 4, 3)],
            path: Slot::Var(path_v),
            max_depth: Some(5),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert_eq!(answers.len(), 1);
        assert_eq!(answer_path(&eng, &answers[0], path_v), vec!["Right"]);
    }

    #[test]
    fn path_finds_shortest_route() {
        // From Cell[(1, 1)] to Cell[(3, 2)] in Grid[10, 10]: shortest path
        // is 3 hops (some interleaving of two Rs and one D, depending on
        // BFS expansion order).
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();

        let mut g = VarGen::new();
        let path_v = g.fresh();
        let q = Query::single(vec![Goal::Path {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 1, 1)],
            to_position: Term::Sym(cell),
            to_args: vec![coord_expr(&eng, 3, 2)],
            path: Slot::Var(path_v),
            max_depth: Some(10),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        // BFS yields the destination exactly once with the shortest path.
        assert_eq!(answers.len(), 1);
        let path = answer_path(&eng, &answers[0], path_v);
        assert_eq!(path.len(), 3);
        let r_count = path.iter().filter(|a| **a == "Right").count();
        let d_count = path.iter().filter(|a| **a == "Down").count();
        assert_eq!((r_count, d_count), (2, 1));
    }

    #[test]
    fn path_max_depth_caps_search() {
        // From Cell[(1, 1)] to Cell[(4, 4)] needs 6 hops; max_depth=3
        // can't reach it.
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();

        let mut g = VarGen::new();
        let path_v = g.fresh();
        let q = Query::single(vec![Goal::Path {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 1, 1)],
            to_position: Term::Sym(cell),
            to_args: vec![coord_expr(&eng, 4, 4)],
            path: Slot::Var(path_v),
            max_depth: Some(3),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        assert!(answers.is_empty(), "depth 3 cannot reach (4,4) from (1,1)");
    }

    #[test]
    fn path_open_destination_yields_reachable_set() {
        // No to_args constraint, max_depth=2 from Cell[(5, 5)]:
        // expect to see (5,5) at depth 0, plus 4 neighbours at depth 1,
        // plus more at depth 2. The set should be larger than 5.
        let eng = load("examples/grid.poly");
        let grid = eng.interner.find("Grid").unwrap();
        let cell = eng.interner.find("Cell").unwrap();

        let mut g = VarGen::new();
        let path_v = g.fresh();
        let q = Query::single(vec![Goal::Path {
            iface: Term::Sym(grid),
            from_position: Term::Sym(cell),
            from_args: vec![coord_expr(&eng, 5, 5)],
            to_position: Term::Sym(cell),
            to_args: Vec::new(),
            path: Slot::Var(path_v),
            max_depth: Some(2),
        }]);
        let env = grid_env(&eng, 10, 10);
        let answers: Vec<_> = eng.query(&q, &env).collect();
        // Depth 0: 1 state (start). Depth 1: 4 neighbours. Depth 2: at
        // most 4*4 = 16 but with dedup ~12. Total >= 13.
        assert!(answers.len() >= 13, "got {} reached states", answers.len());
        // Start is included with empty path.
        let start_match = answers.iter().any(|a| {
            matches!(a.subst.get(&path_v), Some(Value::Path(p)) if p.is_empty())
        });
        assert!(start_match);
    }

    #[test]
    fn cell_symbolic_carries_guard_in_residual() {
        // Cell[c] (the same `c` the schema uses), no env: equality `c = c`
        // collapses to true; the position guard remains as the residual,
        // unchanged.
        let eng = load("examples/grid.poly");
        let c_sym = eng.interner.find("c").unwrap();
        let q = cell_query(&eng, vec![Expr::Var(c_sym)]);
        let answers: Vec<_> = eng.query(&q, &Bindings::default()).collect();
        assert_eq!(answers.len(), 1);
        assert_eq!(
            answers[0].residual.len(),
            1,
            "expected one conjoined residual, got {:?}",
            answers[0].residual,
        );
        // The residual should mention Width and Height symbolically.
        let rendered = eng.fmt_expr(&answers[0].residual[0], 0);
        assert!(rendered.contains("Width"), "expected Width in {rendered}");
        assert!(rendered.contains("Height"), "expected Height in {rendered}");
    }
}
