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

`query::Query` is a vector of disjunctive bodies, each a sequence of
`Goal`s. Goals match against the relations above; logic variables
(`VarId`) bind values during unification. Three kinds of binding slots:

- `Term` — symbol-valued slot (`Term::Sym(s)`, `Term::Var(v)`, `Term::Anon`).
- `Slot` — structured-value slot (params, args, patterns, guards) bound
  whole rather than pattern-matched on internals.
- `IndexSlot` — `defer_entry`'s entry index, plus a literal-int matcher.

`Goal::Where(Expr)` lets a caller push an arbitrary expression onto the
answer's residual on top of the auto-accumulated guards.

`Goal::Position { iface, position, args, params, guard }` accepts an
optional `args: Vec<Expr<Sym>>` — concrete args supplied to the
position's formal parameters. When non-empty, the args are substituted
into the position's guard *before* it lands on the residual (rather
than emitting `formal = arg` equalities, which would echo back into
the output). Concrete args reduce the guard to true/false through the
simplifier; symbolic args (`Expr::Var(some_sym)`) leave the guard
parameterized by that name. Logic-variable args (`Term::Var`) aren't
supported because the engine doesn't enumerate parameterized
instances. Iface-level params (e.g. `Width` in `Grid[Width, Height]`)
are bound via the `Bindings` env passed to `Engine::query`, not via
goal args.

`Goal::Path { iface, from_position, from_args, to_position, to_args,
path, max_depth }` is BFS over state-machine action edges. Each
reachable state is yielded once with the shortest discovered path
(visited set keyed on `(pos, folded_args)`); the start is yielded
with an empty path. Folds destination args + guards under `env`;
drops candidates whose guard reduces to literal false. Symbolic
guards are kept (not dropped). `to_position`/`to_args` filter the
yielded set; `path` binds the action sequence as `Value::Path(Vec<Sym>)`.
`from_args` should fold to fully concrete values for the visited
canonicalization to work; otherwise `max_depth` becomes the
termination guarantee.

`Goal::Step { iface, from_position, from_args, action, to_position,
to_args }` is one transition along a state-machine action. Looks up
the realization defer (`Foo::Run`), finds the entry whose
`source_pos == from_position`, finds the direction mapping with
`target_dir == Named(action)`, and pulls the abstract `source_dir`'s
`tgt_args`. The query's `from_args` are substituted through the
`src_pattern` to evaluate the destination args; the destination
position's guard (also substituted) lands on the residual; `to_args`
is bound as `Value::Args(computed)`. Concrete in-bounds steps reduce
the guard to true; out-of-bounds destinations are dropped via guard
contradiction. Composes with `Goal::Position`/`Goal::Where` for
constraint-driven reasoning.

`Goal::Reach { walk, from_iface, from_position, to_iface, to_position }`
walks defer edges transitively from a concrete starting `(iface, pos)`.
Forward follows `(defer.source, entry.source_pos) → (defer.target,
entry.target_pos)`; backward follows the inverse. BFS with a visited
set; yields each reachable pair (including the start) for the
unifier to filter against `to_iface`/`to_position`. The motivating use
case is "given InterfaceA at StateA, what's possible at InterfaceC?"
across a chain of defers — composes with `Goal::Direction` on the
result.

An `Answer` is a `Subst` (var → value) plus a `residual: Vec<Expr<Sym>>`.
Empty residual means the answer is unconditionally true; otherwise the
residual is a conjunction of constraints under which the answer holds.

`Engine::query(query, env)` returns an `Answers<'a>` iterator. The
solver is depth-first with explicit backtracking via a `Vec<Frame>`
stack: pull from the top frame's per-goal match iterator, push for the
next goal, pop when exhausted, advance to the next disjunct when the
stack drains. The simplifier runs as a per-answer `filter_map` step
inside `next()`:

- residual reduces to `true` → cleared on the answer.
- residual reduces to `false` → answer dropped (silently skipped).
- otherwise → kept as a single conjunct on the residual.

Lazy iteration matters for query shapes with infinite valid completions
(e.g. parameterized states with no upper bound); consumers can `.take(n)`
or filter without forcing the whole answer space.

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

The CLI builds `Query` values inline and calls `Engine::query`; it
formats the resulting answers itself.

## Embedding API

The public surface is intentionally minimal — one type, one method.

- `Engine::load(src) -> Result<Self, EngineError>` — parse + lower +
  validate, in one call.
- `Engine::query(query, env) -> Vec<Answer>` — the single way to ask
  the engine anything. Caller composes a `Query` from `Goal`s; engine
  unifies against the relations and returns answers with residuals.
- `Engine::resolve(sym) -> &str` — `Sym` → name lookup.
- `Engine::*_relation()` — iterator views of the loaded program; useful
  for renderers and direct AST walks.

Whatever shape the caller wants for results — typed `ExplainResult`-style
structs, JSON for a wire protocol, etc. — they build it on top of
`Vec<Answer>` themselves. The engine does not bake a "named operation"
vocabulary into its surface; consumers compose queries and project
answers as suits them.

`Bindings` (re-exported at the crate root) is the env passed to `query`:
a `BTreeMap<Sym, Value>` of pre-bound logic variables. The common case
is `Bindings::default()`; supply concrete bindings to specialize.

## Layout

The repo is a Cargo workspace with two crates:

- `engine/` — `poly-engine`, the library. The active surface for all
  language/runtime work.
- `cli/` — `poly`, the binary. Renders results; no engine logic.

Inside `engine/src/`, public modules are `query` and `types`; everything
else is `pub(crate)`. Top-level re-exports: `Engine`, `EngineError`,
`Sym`, `Interner`, `Bindings`.

- `query.rs` — the `Query` AST (`Goal`, `Term`, `Slot`, `IndexSlot`,
  `DirRefPat`), unifier, solver, and `Engine::query` impl. The query
  layer.
- `types.rs` — `Schema`, `Interface`, `Position`, `Direction`, `Defer`,
  `DeferEntry`, `Pattern`, `DirRef`, `DirMapping`, `Expr`, `Param`,
  `Type`, `Decl`. No transition field on `Direction`.
- `interner.rs` — `Sym` + `Interner`.
- `parsing/` — source → validated AST, the three sequential steps
  `Engine::load` runs:
    - `parsing/parse.rs` — chumsky parser; comment pre-pass; sugar
      rewrite (uses parse-internal `RawDirection`/`RawTransition` so
      transitions never leak past parsing).
    - `parsing/lower.rs` — `Decl<String>` → `Decl<Sym>`.
    - `parsing/validate.rs` — defer validation (positions exist,
      arities match, abstract refs only on `::Internal` source).
- `fmt.rs` — Display impls; round-trip with source; `fmt_facts` Datalog
  rendering.
- `eval.rs` — `Value`, `Bindings`, `const_fold`, `conjoin`. The
  primitive expression-folding layer that `simplify` calls.
- `simplify.rs` — residual reasoner. Iterated pipeline of algebraic
  identities, equality substitution, and interval narrowing on top of
  `eval::const_fold`.
- `relations.rs` — `*_relation()` iterators on `Engine`; the queryable
  view of the loaded program.
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
