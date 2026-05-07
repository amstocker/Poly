// Embedding-friendly API surface.
//
// `Poly` owns the loaded `Engine` and projected `Facts`, and exposes named
// operations on top of them. Each method maps to a `uquery::Query` internally
// — callers don't see logic-variable plumbing.
//
// This is the surface a Rust service consumer depends on. Lower-level
// modules (`uquery`, `eval`, `simplify`, …) are `pub(crate)` and not
// reachable from outside the crate.

use std::collections::BTreeSet;

use crate::eval::Bindings;
use crate::facts::Facts;
use crate::types::Expr;
use crate::uquery::{
    run_query, Answer, Goal, IndexSlot, Query, Slot, Term, Value, VarGen, VarId,
};
use crate::{Engine, EngineError, Sym};


// =============================================================================
// Top-level handle
// =============================================================================

/// A loaded Poly program with a projected fact base, ready to query.
pub struct Poly {
    engine: Engine,
    facts: Facts,
}

impl Poly {
    /// Parse, lower, validate, and project `src` into a queryable handle.
    pub fn from_source(src: &str) -> Result<Self, EngineError> {
        let engine = Engine::load(src)?;
        let facts = engine.facts();
        Ok(Self { engine, facts })
    }

    pub fn engine(&self) -> &Engine { &self.engine }
    pub fn facts(&self) -> &Facts { &self.facts }
    pub fn resolve(&self, sym: Sym) -> &str { self.engine.resolve(sym) }
}


// =============================================================================
// API errors
// =============================================================================

#[derive(Debug)]
pub enum ApiError {
    UnknownInterface(String),
    UnknownPosition { iface: String, position: String },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::UnknownInterface(name) => write!(f, "unknown interface: {name}"),
            ApiError::UnknownPosition { iface, position } =>
                write!(f, "unknown position: {iface}.{position}"),
        }
    }
}


// =============================================================================
// Result types
// =============================================================================

/// What is determined elsewhere when an interface is at a given position:
/// available actions, plus forward and backward defer links.
#[derive(Debug, Clone)]
pub struct ExplainResult {
    pub iface: Sym,
    pub position: Sym,
    pub actions: Vec<Sym>,
    pub forward: Vec<DeferLink>,
    pub backward: Vec<DeferLink>,
}

/// One defer entry connecting a (source iface, source pos) to a
/// (target iface, target pos), with any residual constraint.
#[derive(Debug, Clone)]
pub struct DeferLink {
    pub defer: Sym,
    pub source_iface: Sym,
    pub target_iface: Sym,
    pub source_pos: Sym,
    pub target_pos: Sym,
    /// Residual conjuncts, simplified by the constraint reasoner. Empty
    /// means "unconditionally"; otherwise the link holds only when the
    /// conjunction is satisfied.
    pub residual: Vec<Expr<Sym>>,
}

/// One (interface, position) where an action is enabled, with any residual
/// constraint that must hold for the action to fire.
#[derive(Debug, Clone)]
pub struct ActionLocation {
    pub iface: Sym,
    pub position: Sym,
    pub residual: Vec<Expr<Sym>>,
}


// =============================================================================
// Operations
// =============================================================================

impl Poly {
    /// `--explain <iface> <position>`: actions available at that position
    /// plus every defer link that touches it.
    pub fn explain_position(&self, iface: &str, position: &str)
        -> Result<ExplainResult, ApiError>
    {
        let i_sym = self.engine.interner.find(iface)
            .filter(|s| self.engine.interfaces.contains_key(s))
            .ok_or_else(|| ApiError::UnknownInterface(iface.to_string()))?;
        let p_sym = self.engine.interner.find(position)
            .ok_or_else(|| ApiError::UnknownPosition {
                iface: iface.to_string(),
                position: position.to_string(),
            })?;

        let env = Bindings::default();

        // Available actions.
        let mut g = VarGen::new();
        let action_v = g.fresh();
        let actions_q = Query::single(vec![Goal::Direction {
            iface: Term::Sym(i_sym),
            position: Term::Sym(p_sym),
            action: Term::Var(action_v),
            params: Slot::Anon,
            guard: Slot::Anon,
        }]);
        let action_answers = run_query(&self.engine, &self.facts, &actions_q, &env);
        let mut seen: BTreeSet<Sym> = BTreeSet::new();
        let mut actions: Vec<Sym> = Vec::new();
        for a in &action_answers {
            if let Some(Value::Sym(s)) = a.subst.get(&action_v) {
                if seen.insert(*s) {
                    actions.push(*s);
                }
            }
        }

        // Forward defers (this iface as source).
        let mut g = VarGen::new();
        let fd = g.fresh();
        let f_tgt = g.fresh();
        let f_entry = g.fresh();
        let f_tgt_pos = g.fresh();
        let fwd_q = Query::single(vec![
            Goal::Defer {
                defer: Term::Var(fd),
                source: Term::Sym(i_sym),
                target: Term::Var(f_tgt),
            },
            Goal::DeferEntry {
                defer: Term::Var(fd),
                entry_idx: IndexSlot::Var(f_entry),
                source_pos: Term::Sym(p_sym),
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Var(f_tgt_pos),
                target_args: Slot::Anon,
            },
        ]);
        let forward: Vec<DeferLink> = run_query(&self.engine, &self.facts, &fwd_q, &env)
            .into_iter()
            .map(|ans| DeferLink {
                defer: sym_of(&ans, fd),
                source_iface: i_sym,
                target_iface: sym_of(&ans, f_tgt),
                source_pos: p_sym,
                target_pos: sym_of(&ans, f_tgt_pos),
                residual: ans.residual,
            })
            .collect();

        // Backward defers (this iface as target).
        let mut g = VarGen::new();
        let bd = g.fresh();
        let b_src = g.fresh();
        let b_entry = g.fresh();
        let b_src_pos = g.fresh();
        let bwd_q = Query::single(vec![
            Goal::Defer {
                defer: Term::Var(bd),
                source: Term::Var(b_src),
                target: Term::Sym(i_sym),
            },
            Goal::DeferEntry {
                defer: Term::Var(bd),
                entry_idx: IndexSlot::Var(b_entry),
                source_pos: Term::Var(b_src_pos),
                src_pattern: Slot::Anon,
                src_guard: Slot::Anon,
                target_pos: Term::Sym(p_sym),
                target_args: Slot::Anon,
            },
        ]);
        let backward: Vec<DeferLink> = run_query(&self.engine, &self.facts, &bwd_q, &env)
            .into_iter()
            .map(|ans| DeferLink {
                defer: sym_of(&ans, bd),
                source_iface: sym_of(&ans, b_src),
                target_iface: i_sym,
                source_pos: sym_of(&ans, b_src_pos),
                target_pos: p_sym,
                residual: ans.residual,
            })
            .collect();

        Ok(ExplainResult {
            iface: i_sym,
            position: p_sym,
            actions,
            forward,
            backward,
        })
    }

    /// `--locate <action>`: every (iface, position) where the action is
    /// enabled, with any residual constraint. Empty Vec means "no matches"
    /// (including the case where the action name was never seen).
    pub fn locate_action(&self, action: &str) -> Vec<ActionLocation> {
        let Some(a_sym) = self.engine.interner.find(action) else {
            return Vec::new();
        };
        let mut g = VarGen::new();
        let i_v = g.fresh();
        let p_v = g.fresh();
        let q = Query::single(vec![
            Goal::Direction {
                iface: Term::Var(i_v),
                position: Term::Var(p_v),
                action: Term::Sym(a_sym),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
            Goal::Position {
                iface: Term::Var(i_v),
                position: Term::Var(p_v),
                params: Slot::Anon,
                guard: Slot::Anon,
            },
        ]);
        run_query(&self.engine, &self.facts, &q, &Bindings::default())
            .into_iter()
            .map(|ans| ActionLocation {
                iface: sym_of(&ans, i_v),
                position: sym_of(&ans, p_v),
                residual: ans.residual,
            })
            .collect()
    }
}


// =============================================================================
// Internal helpers
// =============================================================================

fn sym_of(ans: &Answer, v: VarId) -> Sym {
    match ans.subst.get(&v) {
        Some(Value::Sym(s)) => *s,
        _ => panic!("expected Sym binding for variable"),
    }
}
