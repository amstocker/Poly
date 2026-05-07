# Unified Pattern-Matching Query

Date: 2026-04-29
Status: design sketch — pre-implementation

## Motivation

The engine currently exposes three queries:

- `explain_position(I, P)` — what does `(I, P)` determine elsewhere via defers?
- `next_position(I, P, A, bindings)` — what's the next position?
- `locate_action(A)` — where is `A` available?

Each is a special case of one shape: a *partial specification* over interfaces,
positions, parameters, and actions, with the engine returning the set of
*completions* (with residual constraints) consistent with the schema and the
defer wiring.

The vision memory pins this:

> The endgame is a single query type: caller supplies a partial specification of
> states and/or actions across various interfaces; engine answers either "valid
> / not valid" or fills in valid completions.

This doc pins the relation model, the query language, and the execution
strategy. It does *not* land code — implementation is staged.

## (1) The fact base

The engine projects schemas, interfaces, and defers into a fixed set of typed
relations. A query runs against that projection.

```
schema_record(S, [field_name : type, ...])
schema_sum(S, [variant_name([field_name : type, ...]), ...])

iface(I, [param : type, ...])
iface_internal(I_internal, I_external)        # I_internal == I_external::Internal

position(I, P, [param : type, ...], guard?)
direction(I, P, A, [param : type, ...], guard?, transition?)
transition(I, P, A, P', [arg_expr, ...])

defer(D, I_source, I_target)
defer_entry(D, I_source, P_source, src_pattern, src_guard?, P_target, [tgt_arg, ...])
defer_dir(D, entry_idx, target_dir : DirRef, source_dir : DirRef)
```

A few design notes about this projection:

- **`transition/5` is reserved but currently empty.** State-machine sugar
  always rewrites direct transitions into the realization defer
  (`I::Internal -> I`'s `defer_dir` abstract refs), so no `Direction<Sym>` in
  the engine carries a `transition` field after parsing. The relation is kept
  in the schema for future use (non-sugar interfaces, hand-written transition
  tables) but every practical query today goes through `defer_dir`. Discovered
  in Stage 2.


- **Guards / transitions / args travel with the relation, not as separate
  facts.** A position `Cell[c] if (1 <= c.x <= W)` is *one* fact whose
  constraint is part of its identity. Splitting position into "bare position +
  guard" facts loses the guard's binding scope (`c` is bound by the position's
  param list). Cleaner: relations carry expressions, and the query engine
  evaluates / unifies against them.
- **Abstract direction refs (`Count[n] => Count[n+1]`) are first-class** in
  `defer_dir`. The unifier walks them like patterns — it does not "expand" them
  into a transition table (impossible in general — the parameter space is
  infinite for `Count[n: Int]`).
- **`iface_internal/2` makes the universal-state-machine layer queryable.** A
  query can ask "what is the realization defer of `I`?" by joining
  `iface_internal(J, I)` with `defer(_, J, I)`.

## (2) Query language

Logic-variable / pattern syntax over the relations above. Sketch:

```
?- position(I, P, args), direction(I, P, "Increment", _, guard, _)
   where eval(guard, args) = true
```

Concrete query shape (tentative — wide design latitude here):

```
query enabled_for(?I, ?P, ?args) {
    direction(?I, ?P, "Increment", _, ?g, _)
    where ?g
}
```

Three styles of binding worth supporting:

- **Bound variable** — `?I = Counter` — fixes the value before search.
- **Free variable** — `?P` — to be filled in by the engine.
- **Pattern** — `args = [n]`, `n > 5` — partial structural binding plus
  constraints.

The query result is a list of *answer substitutions*. Each substitution maps
free variables to either concrete values or *(parameterized binding +
constraint)*. Following the vision memory:

> `locate_action` should return positions plus the constraint expression under
> which the action is enabled.

So an answer for "where is `Decrement` enabled" against Counter would be:

```
{ I = Counter, P = Count, args = [n], n > 0 }
```

— not enumerated (that's infinite), and not yet evaluated to a concrete
position set. The constraint is part of the answer.

### Reduction of existing queries

- **explain_position(I, P)** —
  ```
  ?- defer(?D, ?I_src, ?I_tgt), defer_entry(?D, ?, ?P_src, ?, ?, ?P_tgt, ?),
     (?I_src = I and ?P_src = P) or (?I_tgt = I and ?P_tgt = P)
  ```
  with `defer_dir` joined per entry to recover the per-action correspondence.

- **next_position(I, P, A, bindings)** — two disjuncts (the `transition`-based
  case is currently unreachable; see the design note above):
  - realization: `iface_internal(?I_int, I), defer(?D, ?I_int, I),
    defer_entry(?D, _, P, _, _, P, _),
    defer_dir(?D, _, named(A), abstract(P, _, ?P', ?args'))`
  - defer-source: `defer(?D, I, _), defer_entry(?D, _, P, _, _, _, _),
    defer_dir(?D, _, named(A), abstract(P, _, ?P', ?args'))`

- **locate_action(A)** —
  ```
  ?- direction(?I, ?P, A, _, ?g, _)
  → (?I, ?P, ?P_params, ?g)
  ```

These translations are the acceptance test for the relation schema: each
existing query must reduce cleanly. If any reduction needs a relation we
haven't modeled, that's a gap to fix before implementing.

## (3) Execution: unification + residual constraints

Two halves:

**Unification** is structural and runs first. It unifies query patterns against
fact patterns (position params, transition args, abstract direction refs).
Standard Prolog-style first-order unification, extended to handle `Wildcard`
and to carry expressions (not just terms) through.

**Constraint handling** is residual. Guards and arithmetic expressions
encountered during unification are accumulated rather than forced. The result
of a query is a substitution + a residual constraint expression.

For concrete inputs (e.g. `next_position` with all args bound), the residual
should evaluate to `true` (success) or `false` (rejection). The existing eval
already handles this case.

For parameterized queries (e.g. "where can I Decrement"), the residual stays
symbolic. **The simplifier is the open piece.** Three escalating options:

1. **No simplifier.** Return the raw constraint expression. The user
   (or a downstream tool) does the reasoning. Cheapest first cut. Probably
   sufficient for `locate_action`-shaped questions in the near term.
2. **Bespoke simplifier** for the language's expression fragment (linear
   arithmetic over Int + equality on records + boolean combinations). Hand-roll
   constant folding, range narrowing, contradiction detection. Vision memory
   leans this way.
3. **External solver** (logru already on the bench, or a small SMT). Defer
   until the bespoke simplifier proves insufficient.

Start with (1). Move to (2) when a use case forces it.

## (4) `logru` vs hand-roll

`logru` is in `Cargo.toml` as a Prolog-style engine. Worth weighing against a
hand-rolled unifier:

- **logru pro:** mature unification, indexing, backtracking. Frees us from
  rebuilding standard Prolog plumbing.
- **logru con:** native term language is first-order; encoding expressions
  (with field access, constructors, guards) into pure Horn-clause shape is
  awkward. Constraints want to be *carried*, not *resolved*; logru resolves.
  Marshalling between Poly's `Expr` and logru's term language is a real cost.
- **Hand-roll pro:** the unifier sees `Expr` directly. Guards stay as
  expressions until the simplifier handles them. Closer to the polynomial
  semantics we want to track.
- **Hand-roll con:** more code; reinventing well-trodden ground.

**Tentative call:** start hand-rolled, scoped narrowly to what the existing
queries need. Revisit logru once the relation schema is stable and the
performance/feature gap is concrete. The Datalog framing in the vision memory
is aspirational — we don't need a full Datalog runtime to land the first
version.

## Implementation stages

**Stage 0: relation schema, on paper.** *(landed — see `unified_query_stage0.md`)*
- Pin the exact set of relations and their types.
- Hand-translate `explain_position`, `next_position`, `locate_action` into
  queries against them.
- Look for relations that are missing or awkward; iterate.
- Output: a short doc (this section, sharpened) + handwritten query examples.

**Stage 1: fact projection + introspection.** *(landed — `Engine::facts()`, `poly facts <file>`)*
- Add `Engine::facts() -> Facts` that walks schemas/interfaces/defers and
  produces the relation tuples.
- Add `poly facts <file>` CLI for round-trip inspection.
- No query engine yet; just the data layer.

**Stage 2: unifier + constraint accumulation.** *(landed — `src/engine/uquery.rs`)*
- Hand-roll first-order unification over `Pattern<T>` and `Expr<T>`.
- Accumulate residual constraints rather than evaluating them eagerly.
- Test by writing the explicit queries' joins by hand against the fact base
  and verifying the answers match the existing implementations.
- **Stage 2.5 (added):** `Goal::Where(Expr<Sym>)`, auto-accumulation of
  position/direction guards into `Answer.residual`, trivial constant-folding
  simplifier (`eval::simplify`), `run_query` takes a `&Bindings` env. This
  was the immediate-frontier work the vision memory called for.

**Stage 3: query syntax + parser.** *(next)*
- Pick the surface syntax (the sketch in §2 is provisional). Likely a separate
  parser module — query syntax doesn't have to match `.poly` syntax.
- `poly query <file> <query>` CLI.
- `poly explain` / `poly locate` become thin wrappers (or get deprecated) once
  the unified query reproduces their output.

**Stage 4: bespoke simplifier.** *(landed — `src/engine/simplify.rs`)*
- Constant folding *(landed in Stage 2.5)*, monotonic comparison narrowing,
  contradiction detection, algebraic identities, and equality substitution.
- Integrated into `run_query`: residual reduces to `true` → cleared on the
  answer; reduces to `false` → answer dropped; otherwise → kept as a single
  conjunct on the answer.
- Trigger ended up being "user asked for it before the residuals got noisy."
  The acceptance bar (`n > 0 ∧ n > 5 → n > 5`, contradictions to false,
  singleton promotion to equality, equality substitution) is met. See
  `unified_query_stage0.md`'s Stage 4 addendum for what's still open
  (multi-variable narrowing, non-±1 coefficients, disjunction narrowing —
  none with a current use case).

**Stage 5 (open): solver integration.**
- If parameterized queries start producing residuals the bespoke simplifier
  can't reduce. logru if Datalog/Horn-shaped, SMT if arithmetic-shaped.

## Open questions

- **Aggregate queries.** "How many positions enable `Increment`?" — needs
  counting / cardinality, not just substitutions. Probably out of scope for v1
  but worth holding in mind.
- **Negation.** "What positions *don't* enable `Increment`?" Stratified
  negation is the standard Datalog answer; whether we need it depends on the
  questions agents actually ask.
- **Recursive defer chains.** A query like "where does Press *eventually* take
  Counter (through any chain of defers)?" is recursive. Datalog handles this
  cleanly via fixpoint; hand-rolled needs explicit traversal. Probably the
  thing that eventually pushes us toward a real Datalog runtime.
- **Trajectory layer.** When the trajectory layer lands, it adds more
  relations (`step(t, I, P, A, P')` etc.) the same query language should be
  able to talk about. Worth keeping the relation schema open to extension.
- **Query-as-tool surface for agents.** The vision memory flags an agent-tool
  surface (list interfaces, query enabled actions, attempt to set state). The
  unified query is the substrate for those tools — but the tool API itself is
  separate. Don't conflate them.

## Out of scope for this work

- Parameterized direction names (still deferred from `ns_and_internal.md`).
- The trajectory layer.
- Permission/privilege scoping (vision memory: lives above the engine).
- Limits/colimits-flavored composition.
- Performance / indexing concerns. Get correctness first; the example sizes
  don't yet justify worrying about index choice.
