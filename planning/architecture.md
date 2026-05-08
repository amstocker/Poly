# Poly architecture (current state)

The runtime today is a constraint-aware logic engine over a polynomial-functor
schema. A `.poly` source declares schemas, interfaces, and defers; the engine
projects those into a fact base; queries run as unification against the fact
base, with guards and other expressions carried as residual constraints and
reduced by a hand-rolled simplifier.

This doc pins what's load-bearing right now. Open work is in `roadmap.md`.
Historical session notes are in `log/`.

## Source language

Three top-level declarations:

- **`schema`** — passive algebraic data. Either a record (`title: String,
  priority: Priority`) or a sum (`High, Medium, Low`). Constructed in
  expressions via `Schema(...)`; field access via `.field`. No recursion.
- **`interface`** — a polynomial $p = \Sigma_i y^{p[i]}$. A list of positions;
  each position has params, an optional guard, and a list of directions; each
  direction has params, an optional guard, and an optional transition target.
- **`defer`** — a lens between two interfaces. Forward map on positions,
  backward map on directions. Per-entry the source position carries a
  pattern (`Count[n]`, `Count[_]`); the target position carries arg
  expressions (`Count[n+1]`).

Expressions cover ints, bools, strings, vars, field access, `Schema(...)`
construction, the usual arithmetic/comparison/boolean operators, `not`,
`and`, `or`. Guards (`if (...)`) and abstract direction refs (`Count[n]
=> Count[n+1]`) are the syntactic surfaces that produce residuals.

## State-machine sugar

An interface containing any direction with an `Action -> NextPos` transition
is rewritten by the parser into three declarations:

1. **`Foo`** — the external interface. Same positions; each direction's
   transition is stripped (only the action name and guard remain).
2. **`Foo::Internal`** — the universal-state-machine carrier $S \cdot y^S$
   over the same position set, with directions intentionally left empty.
   Directions of `Foo::Internal` are addressed only via abstract refs.
3. **`Foo::Run : Foo::Internal -> Foo`** — the realization defer. One entry
   per position (identity on positions); direction mappings realize each
   declared transition as an abstract direction ref of the form
   `(Count[n] => Count[n+1])`.

The `::Internal` suffix on a defer's source is the validation carrier: only
defers whose source ends in `::Internal` may use abstract direction refs in
their bodies. (Promoting this to an explicit `state` block is on the
roadmap.)

There is no surface form for an interface that *isn't* desugared this way,
so `Direction` in the engine never carries a transition field — the parser
keeps transitions as a parse-internal type, and they're consumed during
sugar rewriting.

## Fact base

The loaded program is queryable as a set of flat relations exposed by
`*_relation()` iterator methods on `Engine` (see `engine/src/relations.rs`).
There is no separate projection step — the iterators flat-map over the
nested AST on demand. Relations:

- `schema_record(S, fields)`, `schema_sum(S, variants)`
- `iface(I, params)`
- `iface_internal(I_internal, I_external)` — derived from the `::Internal`
  suffix; lets queries find the realization carrier structurally
- `position(I, P, params, guard?)`
- `direction(I, P, A, params, guard?)`
- `defer(D, I_source, I_target)`
- `defer_entry(D, idx, P_source, src_pattern, src_guard?, P_target, tgt_args)`
- `defer_dir(D, entry_idx, target_dir, source_dir)` where each `dir` is
  either `Named(Sym)` or `Abstract { src_pos, src_pattern, tgt_pos, tgt_args }`

Guards and arg expressions live inside the AST nodes themselves; they
aren't factored into separate fact rows. Abstract direction refs are
first-class in `defer_dir` (no expansion into a transition table — the
parameter space may be infinite).

`poly facts <file>` prints the relations in Datalog notation for inspection.

## Query layer

`uquery::Query` is a vector of disjunctive bodies, each a sequence of
`Goal`s. Goals match against the fact relations above; logic variables
(`VarId`) bind values during unification. Three kinds of binding slots:

- `Term` — symbol-valued slot (`Term::Sym(s)`, `Term::Var(v)`, `Term::Anon`).
- `Slot` — structured-value slot (params, args, patterns, guards) bound
  whole rather than pattern-matched on internals.
- `IndexSlot` — `defer_entry`'s entry index, plus a literal-int matcher.

`Goal::Where(Expr)` lets a caller push an arbitrary expression onto the
answer's residual on top of the auto-accumulated guards.

An `Answer` is a `Subst` (var → value) plus a `residual: Vec<Expr<Sym>>`.
Empty residual means the answer is unconditionally true; otherwise the
residual is a conjunction of constraints under which the answer holds.

`run_query(eng, facts, query, env)` solves each disjunct against the fact
base, accumulates residuals, runs the simplifier on the conjoined residual,
and returns the surviving answers:

- residual reduces to `true` → cleared on the answer.
- residual reduces to `false` → answer dropped.
- otherwise → kept as a single conjunct on the residual.

## Simplifier

`simplify::reduce(eng, expr, env)` is the residual reasoner. It runs an
iterated pipeline (max 8 passes, fixpoint by structural equality):

1. **`apply_identities`** — bottom-up rewrite. Boolean absorption /
   annihilation, arithmetic identities (`n + 0 → n`, `n * 0 → 0`,
   `n - n → 0`), double-negation elimination, syntactic equalities
   (`e ∧ e → e`, `e = e → true`), per-op constant folding.
2. **`flatten_and`** — top-level `And`-tree → flat conjunct list, with
   short-circuit on any `false` conjunct.
3. **`extract_equalities`** — `var = expr` → substitution map (when `var`
   is a bare `Var(s)` not appearing in `expr`). Substituted everywhere,
   re-folded, re-rewritten.
4. **Linear normalization** — every comparison atom is reduced to a
   `c0 + Σ ci · vi ⋈ 0` form. Single-variable atoms with `|coef|=1` get
   pulled out as `SimpleAtom { var, op, rhs }`; the rest stay symbolic.
5. **Per-variable `Interval`** — `lo`, `hi` (each with inclusivity), plus
   a set of explicit `≠` values. Merged across atoms; empty interval
   short-circuits to `false`.
6. **Singleton promotion** — closed `[k,k]` interval → `var = k`, which
   feeds the next iteration's substitution.
7. **Reassemble + dedupe** — emit equalities, then narrowed atoms, then
   everything else. Empty list → `true`.

`eval::const_fold` is the leaf operator the pipeline calls: walk Expr,
substitute env values, fold any subexpression that becomes fully concrete.

## CLI

```
poly show <file>                                  # pretty-print decls
poly facts <file>                                 # Datalog-style projection
poly query <file> --explain <iface> <pos>         # actions + forward + backward links
poly query <file> --locate <action>               # iface.position list with residuals
```

The CLI is a thin renderer over the named ops on `Engine` (see below);
each flag dispatches to one method.

## Embedding API

`Engine` is the public surface for service consumers — it's the loaded
program *and* the query handle:

- `Engine::load(src) -> Result<Self, EngineError>` — parse + lower +
  validate, in one call.
- `Engine::explain_position(iface, position) -> Result<ExplainResult, ApiError>`
  — actions + forward/backward defer links.
- `Engine::locate_action(action) -> Vec<ActionLocation>` — every `(iface,
  position)` where the action is enabled, with residual.
- `Engine::resolve(sym) -> &str` — Sym → name lookup.

Result types (`ExplainResult`, `DeferLink`, `ActionLocation`, `ApiError`)
live in `api` and are typed Rust structs with `Sym` and `Expr<Sym>`
payloads. JSON serialization is intentionally not added yet; it will be
a thin layer once the future service needs it.

Queries against the program also walk `Engine` directly via
`*_relation()` iterator methods (one per logical relation in the fact
base — see `relations.rs`). These are `pub` so consumers can iterate
the loaded program, but most service callers stay at the named-op level.

## Layout

The repo is a Cargo workspace with two crates:

- `engine/` — `poly-engine`, the library. The active surface for all
  language/runtime work.
- `cli/` — `poly`, the binary. Renders results; no engine logic.

Inside `engine/src/`, public modules are `api` and `types`; everything
else is `pub(crate)`. Top-level re-exports: `Engine`, `EngineError`,
`Sym`, `Interner`.

- `api.rs` — named ops on `Engine` (`explain_position`, `locate_action`)
  + result types; the embedding surface.
- `types.rs` — `Schema`, `Interface`, `Position`, `Direction`, `Defer`,
  `DeferEntry`, `Pattern`, `DirRef`, `DirMapping`, `Expr`, `Param`,
  `Type`, `Decl`. No transition field on `Direction`.
- `interner.rs` — `Sym` + `Interner`.
- `parse.rs` — chumsky parser; comment pre-pass; sugar rewrite (uses
  parse-internal `RawDirection`/`RawTransition` so transitions never
  leak past parsing).
- `lower.rs` — `Decl<String>` → `Decl<Sym>`.
- `validate.rs` — defer validation (positions exist, arities match,
  abstract refs only on `::Internal` source).
- `fmt.rs` — Display impls; round-trip with source; `fmt_facts` Datalog
  rendering.
- `eval.rs` — `Value`, `Bindings`, `const_fold`, `conjoin`.
- `relations.rs` — `*_relation()` iterators on `Engine`; the queryable
  view of the loaded program (replaces the old `Facts` projection).
- `uquery.rs` — query AST + unifier + solver + integration with the
  simplifier. Tests reduce Q1/Q2/Q3 queries against this.
- `simplify.rs` — residual reasoner.
- `engine.rs` — `Engine` struct, `EngineError`, `Engine::load`.

## Working hypothesis

Poly is a *typed live knowledge base for agentic systems*. The polynomial
schema declares the space of allowable systems and evolutions; agents and
humans both read/write the same artifact; the runtime mechanically
enforces the boundary.

Three layers worth keeping distinct: schema (the static type),
configuration (the live realized state), trajectory (the append-only
history). Today only the schema layer exists. Configuration and trajectory
are roadmap items, not blockers for the constraint-engine work that's
currently the focus.
