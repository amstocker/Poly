// poly-engine: typed live knowledge base for agentic systems.
//
// Public surface — single source of truth, single way in:
//   - `Engine` — the loaded program. Holds the AST and exposes
//     `*_relation()` iterators over its flat-relation view, plus a
//     single `query()` method that runs a `Query` against the program.
//   - `query` module — the `Query` AST, `Goal` variants, `Answer`, and
//     supporting types. Consumers compose `Query` values and pass them
//     to `Engine::query`.
//   - `types` — the polynomial-functor schema AST (`Interface`,
//     `Defer`, `Schema`, `Expr`, `Pattern`, `DirRef`, …).
//   - top-level re-exports: `Engine`, `EngineError`, `Sym`, `Interner`,
//     `Bindings`, plus everything in `types` for convenience.
//
// Everything else (parser, lowering, validation, the residual
// simplifier) is `pub(crate)` — implementation detail.

pub mod query;
pub mod types;

pub(crate) mod engine;
pub(crate) mod eval;
pub(crate) mod fmt;
pub(crate) mod interner;
pub(crate) mod lower;
pub(crate) mod parse;
pub(crate) mod relations;
pub(crate) mod simplify;
pub(crate) mod validate;

pub use engine::{Engine, EngineError};
pub use eval::Bindings;
pub use interner::{Interner, Sym};
pub use types::*;
