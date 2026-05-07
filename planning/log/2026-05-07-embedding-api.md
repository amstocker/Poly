# 2026-05-07 — Embedding API + workspace split

## What landed

- **Workspace reorg.** Repo is now a Cargo workspace with two crates:
  `engine/` (lib `poly-engine`) and `cli/` (bin `poly`). The CLI depends
  on the library; nothing else does yet.
- **Public surface tightened.** Inside `poly-engine`, only `api` and
  `types` are `pub mod`. Everything else (`uquery`, `eval`, `simplify`,
  `parse`, `lower`, `validate`, `facts`, `fmt`, `interner`, `engine`)
  is `pub(crate)`. Top-level re-exports: `Engine`, `EngineError`, `Sym`,
  `Interner`, `Facts`, plus everything in `types`.
- **`Poly` handle.** New `api::Poly` owns `Engine + Facts` and exposes
  named ops:
  - `Poly::from_source(src) -> Result<Self, EngineError>`
  - `Poly::explain_position(iface, position) -> Result<ExplainResult, ApiError>`
  - `Poly::locate_action(action) -> Vec<ActionLocation>`
  - `Poly::engine() / facts() / resolve()` — escape hatches for renderers.
- **CLI migrated.** `cli/src/main.rs` is a thin renderer over `Poly`;
  no inline `uquery::Query` construction. Output is byte-identical to
  pre-migration.

## Decisions made

### Embedding API first; JSON deferred

The future "service" managing state + agents will be in Rust and import
`poly-engine` as a crate. Given that, a JSON wire format would just be
overhead — the consumer can call typed Rust functions directly. JSON is a
later thin layer once a non-Rust consumer or human-facing query tool
appears.

### Named ops, not generic query passthrough

We considered exposing `query_facts(goals: Vec<Goal>) -> Vec<Answer>` as
a single generic op. Rejected: it would push the internal `uquery` AST
into the public surface, freezing it. Named ops (`explain_position`,
`locate_action`, …) keep the public surface aligned with what services
actually want to do, and let the internal Query shape evolve freely.

### Tagged objects for parameterized values

When the API grows ops that take parameterized positions (e.g.
`enabled_actions(iface, Count[3])`), the wire form will be a tagged
object: `PositionRef { name: Sym, args: Vec<Value> }`. Decided over
positional arrays (`["Count", 3]`) and string forms (`"Count[3]"`):
self-documenting, minimal token overhead, no in-band parsing.

The first ops landed (`explain_position`, `locate_action`) take bare
`&str` names because their use cases don't require parameterized
positions yet. Will revisit when the first parameterized op is added.

### Result types own their residuals

Every result struct (`ExplainResult`, `DeferLink`, `ActionLocation`)
carries a `Vec<Expr<Sym>>` residual. The simplifier has already reduced
it; consumers either ignore the residual (treat the answer as
unconditional), pretty-print it, or feed it back to a constraint solver
above. The engine doesn't decide what "this answer holds when …" means
— it just hands the constraint over.

## Open from this turn

- **Parameterized inputs.** Decide `PositionRef`'s arg type. Probably
  `Vec<Value>` where `Value = Int(i64) | Str(String) | Bool(bool)`,
  matching the schema's type system. Or reuse `eval::Value` if it's
  already shaped this way.
- **Sym vs &str on inputs.** Today the named ops take `&str` and intern
  internally. For a hot-path service this might be wasteful — consider a
  variant that accepts pre-interned `Sym` for callers holding handles.
- **Visibility of `Engine` fields.** `Engine` still has public fields
  (`interner`, `schemas`, `interfaces`, `defers`). The CLI uses these to
  render `show`. For embedding we may want accessors instead, with the
  fields private. Defer until a concrete need appears.

## Why this matters

This turn shifts the project's centre of gravity. Up to this point the
"product" was the engine binary plus its CLI; now the product is
explicitly a Rust library that someone else will embed, with the binary
demoted to "renderer + manual driver." That framing makes the rest of
the roadmap (named ops, JSON wrapper, eventually a textual query
surface) fall out as concentric layers around the same `Poly` core.
