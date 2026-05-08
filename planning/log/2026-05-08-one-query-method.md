# 2026-05-08 — Collapse api.rs into Engine; one query method

## What landed

The user asked: "I don't think we need both uquery.rs AND api.rs (also
why is uquery called that? just call it query). I really just want the
single source of truth (engine) and a single way to interface with it.
That is to say I just want a single 'query' method on the Engine
interface."

Three changes together:

1. **`uquery.rs` → `query.rs`**, made `pub`. The historical `u`-prefix
   was when the codebase had a parallel "concrete" query layer; that's
   long gone, so the prefix was just noise.

2. **`api.rs` deleted.** The `explain_position` / `locate_action`
   methods, the `ExplainResult` / `DeferLink` / `ActionLocation` /
   `ApiError` result types — all gone. The named-op surface had
   accumulated assuming a future Rust service would call typed
   methods; in fact, the right move is to expose the `Query` AST and
   let consumers compose what they need.

3. **`Engine::query(query, env) -> Vec<Answer>`** is the single query
   method. The free `run_query` function is gone; it's now an
   `impl Engine` block in `query.rs`.

The CLI is now the only consumer, and it builds `Query` values inline
in `run_explain` and `run_locate`. Output is byte-identical.

## Decision rule for "named ops vs raw query"

The named-op layer (`api.rs`) was justified earlier by: "this is the
stable surface a Rust service consumer depends on; internals can
change." That argument was thin. With one consumer (the CLI) and no
real service yet, the named ops were API surface to maintain without
a corresponding consumer. Worse, they invited speculative additions
(`enabled_actions`, `next_position`, `validate_position`) that solved
problems no real call site had asked for.

The new rule: **the engine has one capability — answer queries —
expressed as one method.** Consumers that want named ops construct
them at their own call site, where the use case is concrete and the
typed result shape can be tailored. If a pattern recurs across
consumers (e.g., several services all need `enabled_actions`), it can
be lifted into a helper crate later.

This aligns with the long-running vision in `project_poly_vision.md`
("one unified query, eventually"): we're not building toward it via
named-op accumulation; we're building toward it by keeping the surface
tiny and letting `Query`'s expressiveness grow.

## What stayed

- `Bindings` is re-exported at the crate root for callers constructing
  the `env` argument.
- The `Query` AST (`Goal`, `Term`, `Slot`, `IndexSlot`, `DirRefPat`,
  `Answer`, `Subst`, `Value`, `VarId`, `VarGen`) is all `pub` in
  `query` — consumers compose against this.
- The `*_relation()` iterators on `Engine` stay (they're how `query`
  matches goals against the loaded program; they're `pub` for renderers
  and walkers that don't need the full query machinery).

## Open

- **Parameterized terms.** Today `Term::Sym(s)` matches a bare symbol;
  there's no shape for "match a parameterized position with these
  param vars bound." If/when a query needs to bind into a `Count[n]`
  position with `n` flowing back, we'll grow `Term` or add a new
  `Goal` variant. Defer until a use case forces it.
- **Convenience constructors.** If certain query shapes recur in CLI
  or future consumers, add `Query::*` helpers (e.g.
  `Query::all_directions_at(iface, pos)`). Don't preemptively build
  them.

## Why this matters

Going from ~270 lines of api.rs + Poly wrapper down to a single
`Engine::query` method is the third consolidation in two days
(after dropping `Facts` and collapsing `Poly`). The codebase now has
exactly one source of truth (`Engine`), exactly one way to ask it
anything (`query`), and exactly one place where consumer-specific
shapes live (the CLI itself). Each removal also pulled out a layer of
indirection that was hiding rather than aiding the design.
