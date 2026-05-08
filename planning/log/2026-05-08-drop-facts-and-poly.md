# 2026-05-08 — Drop Facts; collapse Poly into Engine

## What landed

Two consolidations the user spotted as redundancy:

### Drop `Facts`

`Facts` was a denormalized projection of the AST: nine `*Fact` tuple
types and a `Facts` container, populated by `Engine::facts()`, consumed
by `match_goal`. There were no joins or indexes that earned the parallel
shape — `match_goal` arms were uniformly `iter().filter_map(unify)`.

Replaced with `*_relation()` iterator helpers on `Engine`
(`engine/src/relations.rs`). Each is a flat-map over the nested AST.
`match_goal` calls these instead of walking `facts.*` vectors.
`fmt_facts` moved to `fmt.rs` and renders directly from the same
iterators.

Deleted `facts.rs` (~349 lines) plus the `Facts` struct, `*Fact` tuple
types, `Engine::facts()`, the `Facts` field on `Poly`, and the
`pub use facts::Facts` re-export.

### Collapse `Poly` into `Engine`

After Facts removal `Poly` was a no-op wrapper around `Engine`:
`from_source` called `Engine::load`, `engine()` returned `&self.engine`,
`resolve` delegated. The named ops (`explain_position`, `locate_action`)
moved directly onto `Engine` as `impl Engine` blocks in `api.rs`.
`Poly` deleted.

CLI updated: `Poly::from_source` → `Engine::load`, `&poly` → `&eng`
throughout. Output byte-identical.

## Why

User asked: "why does the engine store facts internally, but then the
Poly api object also has facts separately?" Right call — the engine
already held everything; `Facts` was a denormalized cache for queries
that didn't actually need a denormalized form. Same logic for `Poly`:
the wrapper added a stable façade, but at this scale the indirection
was overhead with no payoff (no version-skew between API and Engine
internals; same crate; one consumer).

The decision rule for "stable façade vs direct access": only worth a
wrapper when the wrapper *hides* internal evolution. Since `Engine` is
the source of truth and changing its public methods is exactly as
disruptive as changing the embedding surface, there was nothing to
hide.

## Open from this turn

- The third consolidation the user proposed (drop `Raw*` parallel
  hierarchy in `parse.rs`) is **left in place**. After looking, the
  `transition` field on `RawDirection` carries information that's
  *consumed* by sugar rewriting (split into `Foo::Run` defer entries),
  not a renamed-twin field. The parallel structs are doing real work as
  a type-level "transitions never escape parsing" guarantee, even if it
  looks redundant on first read.

- Result types in `api.rs` (`ExplainResult`, `DeferLink`,
  `ActionLocation`) carry `Vec<Expr<Sym>>` for residuals. Always 0 or 1
  elements after the simplifier; could tighten to `Option<Expr<Sym>>`.
  Not pressing.

## Why this matters

Removes ~500 lines of plumbing that existed because of a "relational
projection" framing that hadn't earned its keep. `Engine` is now the
single source of truth: the loaded program, the queryable handle, *and*
the embedding surface. The old `Facts` framing was carried over from
when an external Datalog backend was being evaluated as a possibility;
with the bespoke unification solver in `uquery`, that framing was
vestigial.
