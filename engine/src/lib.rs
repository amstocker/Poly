// poly-engine: typed live knowledge base for agentic systems.
//
// Public surface:
//   - `api` — embedding-friendly named operations (the primary entry point
//     for service consumers)
//   - `types` — the polynomial-functor schema AST (`Interface`, `Defer`,
//     `Schema`, `Expr`, `Pattern`, `DirRef`, …)
//   - top-level re-exports: `Engine`, `EngineError`, `Sym`, `Interner`,
//     `Facts`, plus everything in `types` for convenience.
//
// Everything else (parser, lowering, validation, the residual simplifier,
// the unification-based query solver) is `pub(crate)` — implementation detail.

pub mod api;
pub mod types;

pub(crate) mod engine;
pub(crate) mod eval;
pub(crate) mod fmt;
pub(crate) mod interner;
pub(crate) mod lower;
pub(crate) mod parse;
pub(crate) mod relations;
pub(crate) mod simplify;
pub(crate) mod uquery;
pub(crate) mod validate;

pub use engine::{Engine, EngineError};
pub use interner::{Interner, Sym};
pub use types::*;
