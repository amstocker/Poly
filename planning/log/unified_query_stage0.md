# Stage 0 Findings: Hand-Translating Existing Queries

Date: 2026-04-29
Status: paper exercise complete; relation schema sharpened
Reads alongside: `unified_query.md`

## Method

For each of `locate_action`, `next_position`, and `explain_position`, hand-write
the query against the relation schema in `unified_query.md` §1 and check it
against actual cases from `counter.poly` and `grid.poly`. Each gap surfaced is
something the relation schema or the query language has to handle before
implementation begins.

## Verdict

All three queries translate. None require new relations beyond what §1 lists,
but the exercise surfaced **six gaps** that need design pinning before Stage 1.
Listed by severity.

---

## Gap 1: `eval` is a meta-predicate, not a relation

**Where it bit:** `next_position` translation has `eval(pos_guard, bindings) =
true` as a clause. That's not first-order — it's executing an expression.

**Resolution:** treat expression-evaluation as a *built-in* the query engine
recognizes, not a normal relation. The unifier accumulates expressions into a
residual constraint; the simplifier resolves them at the end.

**Consequence for §3 of the planning doc:** the "no simplifier" first cut isn't
quite tenable — even concrete-input queries need at least constant folding to
collapse residuals to `true`/`false`. Revise: v1 includes a *trivial*
simplifier that handles fully-bound expressions (basically just running the
existing `eval`); leaves symbolic expressions as residuals.

---

## Gap 2: pattern scope crosses the defer-entry / dir-ref boundary

**Where it bit:** `next_position(Counter::Internal, Count, Press, {n=3})` walks
into the `SetTo10` defer. The entry source pattern is `Count[_]`. The matching
direction is `abstract(Count, [_], Count, [10])`. Both `_`s are *anonymous and
distinct*, but in the realization defer `Counter::Run`, the entry pattern is
`Count[n]` and the abstract direction is `Count[n] => Count[n+1]` — same `n`.
The unifier needs to recognize when binders in the abstract pattern are bound
*at the entry level* vs. fresh.

**Resolution:** scope rule, written into §1 of the planning doc:
*Names introduced in `defer_entry`'s `src_pattern` are visible in that entry's
`defer_dir` abstract patterns.* `_` is always fresh (per `ns_and_internal.md`).
A named binder in an abstract pattern shadows nothing at the entry level —
either it matches an entry-level name (bound) or it's a fresh local binder.

**Consequence:** the unifier carries a per-entry symbol environment, not just
a global substitution. Mechanically: when unifying a `defer_dir`, prepopulate
its environment with the bindings established by its containing entry's
`src_pattern`.

---

## Gap 3: `bindings` is environmental, not a query variable

**Where it bit:** `next_position`'s `bindings` argument doesn't fit naturally as
a logic variable in the query. It's the caller's evaluation context.

**Resolution:** the query engine takes a substitution σ as input alongside the
query. σ applies to position params during unification. Bindings don't appear
in query syntax; they're an evaluation context.

**API shape (tentative):**
```rust
fn run_query(query: &Query, env: &Bindings) -> Vec<Answer>
```

For Q3 (`locate_action`) the env is empty. For Q2 (`next_position`) the env is
the user-supplied bindings.

---

## Gap 4: result shape — flat tuples vs. nested structure

**Where it bit:** the current `explain_position` output groups `defer_dir`
mappings under their containing `defer_entry`. A flat query result (one row per
direction mapping) loses that grouping, and the natural rendering wants it
back.

**Resolution options:**
1. **Group at render time.** Query returns flat tuples; the CLI renderer
   reassembles. Loses the abstraction — every consumer reimplements grouping.
2. **Query supports `group by`.** Adds aggregation primitives. Flagged as
   probably-out-of-scope in the planning doc; this exercise suggests pulling
   it forward.
3. **Result is nested by construction.** Query syntax allows declaring nested
   shapes ("for each entry, collect its direction mappings"). Closer to GraphQL
   than Datalog.

**Tentative call:** option (1) for Stage 1–2 (just get it working), option (2)
once we have a use case. Don't go to (3) — too far from the relational core.

The planning doc's "aggregation" open question gets promoted: not v2, not
"out of scope," but **acknowledged as part of v1's renderer**, even if the
query engine itself stays aggregation-free.

---

## Gap 5: `explain_position` is two queries, not one

**Where it bit:** `explain_position(I, P)` returns forward + backward links.
These are two separate queries against the fact base. They share no joins.

**Resolution:** the unified query language must support **disjunction (or query
union)**. `explain_position` becomes a query that's the union of two clauses,
plus an action-list query. No deep design issue — just confirms disjunction is
v1, not deferrable.

---

## Gap 6: `locate_action` doesn't traverse defer chains

**Where it bit:** `locate_action("Press")` finds Button.Button.Press but not
Counter::Internal.Count (where Press is reachable via SetTo10's defer_dir).
That's the current behavior, but the planning doc's "recursive defer chains"
open question is exactly this case.

**Resolution:** Stage 0 confirms it's an open question, not a Stage 1 concern.
v1 of `locate_action` is the direct query. A separate "reaches via defers"
query — which is recursive, hits fixpoint semantics, and is the natural push
toward a real Datalog runtime — stays in the open-questions list.

---

## Net effect on §1 of the planning doc

The relation schema is essentially correct. Two amendments:

1. **Scope rule pinned:** entry source-pattern binders are visible in the
   entry's abstract direction refs. The unifier respects this scope.
2. **Built-in `eval`:** expressions in relations (guards, args) are unified
   structurally; concrete-input expressions are folded by the simplifier;
   symbolic expressions are residuals.

No new relations needed.

## Net effect on Stage 1

Plan stands, with these refinements:

- Trivial simplifier (constant folding only) ships with Stage 1 — needed to
  resolve concrete-input residuals to true/false. Bespoke simplifier (Stage 4)
  remains the upgrade path for symbolic residuals.
- Disjunction is in v1.
- Result-grouping is the renderer's responsibility, not the query engine's.

## What didn't shake out

The planning doc's design held up better than I expected. No relations needed
to be added or restructured. The hand-roll-vs-logru tradeoff didn't sharpen —
both are still credible for Stage 2; the entry-scope rule (Gap 2) might
nudge us toward hand-rolled (logru's term language is harder to extend with
custom scope semantics), but it's not decisive.

## Next step

Stage 1: implement `Engine::facts() -> Facts` and a `poly facts <file>` CLI for
inspecting the projection. Acceptance: byte-for-byte stable output across
schema-equivalent edits to the source `.poly`.

---

## Addendum (2026-04-29) — Stage 2 findings

Stages 1 and 2 landed. Implementation surfaced one finding that updates the
relation schema in §1 of the planning doc; everything else held.

### Gap 7: `transition` is reserved-but-empty under current sugar

**Where it bit:** while hand-writing `next_position` as a query (Q2), the
direct-transition disjunct never produced any answers. Tracing back: state-
machine sugar in `parse.rs` *always* rewrites positions with transitions —
it strips the transition off the external interface's directions and
materializes them as abstract refs in the realization defer. Net effect: no
`Direction<Sym>` in the engine ever carries a `Transition`. The `transition`
relation in the fact base is permanently empty.

**Resolution:** keep the relation in the schema (cheap, zero cost when empty),
but document that current practice routes everything through `defer_dir`.
Updated `unified_query.md` §1 with a design note. Q2 collapses to two
disjuncts (realization + defer-source-abstract).

If/when we add a way to declare an interface that *isn't* desugared (e.g. a
hand-written non-sugar form), `transition` becomes live again. Not a v1
concern.

### What Stage 2 confirmed

- The relation schema in §1 is sufficient. No relations added or restructured.
- Disjunction in v1 (Gap 5) is correct — Q2 needs it, Q1's forward+backward
  also need it.
- Slot-based binding (binding whole structured values into vars rather than
  pattern-matching on internals) was the right simplification for v1. Every
  reduction works without it.
- The per-entry scope rule (Gap 2) didn't bite Stage 2 directly because v1
  doesn't yet evaluate the bound expressions. It will bite Stage 3/4 when
  bindings flow through abstract patterns.

### Code state at end of Stage 2

- `src/engine/facts.rs` — typed relation tuples, `Engine::facts()`,
  `Engine::fmt_facts()`, `poly facts <file>` CLI.
- `src/engine/uquery.rs` — query AST (Term, Slot, IndexSlot, DirRefPat, Goal,
  Query), `Value`/`Subst`/`Answer`, hand-rolled unifier, backtracking solver,
  9 tests reducing Q1/Q2/Q3 against the fact base.

### Open from Stage 2

- ~~No `Goal::Where(Expr)` yet — residual constraints aren't expressible.
  Needed for the parameterized-answer shape ("`Decrement` enabled when
  `n > 0`") that the vision memory pins as the immediate frontier.~~ Landed.
- ~~No simplifier. Even constant folding for fully-bound constraint
  expressions is missing — needed to collapse concrete-input residuals to
  true/false.~~ Landed (trivial simplifier — constant folding only).
- No surface syntax. Queries are constructed as Rust values.

---

## Addendum (2026-04-29) — Residuals + trivial simplifier landed

The first two Open-from-Stage-2 items closed in one increment. Vision-memory
frontier ("`Decrement` enabled when `n > 0`") now works.

### What shipped

- **`Answer.residual: Vec<Expr<Sym>>`** — first-class on every answer.
  Empty residual = unconditionally true. The Vec is conjuncted at the end
  of the query and replaced with a single simplified expression (or
  cleared, or the answer is dropped).
- **Auto-accumulation from `Position` and `Direction` goals** — when a
  matched fact has a non-empty guard, that guard's expression is pushed
  onto the answer's residual. The query author doesn't have to write a
  separate `Where` to "ask for" the guard.
- **`Goal::Where(Expr<Sym>)`** — explicit constraint layering. Same
  residual channel as auto-accumulated guards.
- **`eval::const_fold(eng, expr, env)`** *(was `eval::simplify`)* —
  partial evaluator. Walks Expr, substitutes any `Var(s)` whose Sym is
  in `env`, constant-folds every subexpression that becomes fully bound.
  Returns `Expr<Sym>` (literal if reduced, otherwise partially-folded
  tree). Round-trips Records through `Construct/Field` so schema-typed
  env entries simplify correctly. Renamed in Stage 4 because
  `simplify::reduce` now wraps it.
- **`run_query(eng, facts, query, env)`** — `env: &Bindings` is now
  threaded through. After solving, per-answer simplification: residual
  reduces to `LitBool(true)` → cleared; reduces to `LitBool(false)` →
  answer dropped; otherwise → kept as a single conjunct on the residual.

### What the simplifier *doesn't* do (still Stage 4)

- Range narrowing (`n > 0 ∧ n < 10` stays as written).
- Contradiction detection that needs more than constant folding
  (`n > 5 ∧ n < 3` is symbolic).
- Equivalence rewriting (`n + 0 → n`).

These are bespoke-simplifier territory; the trivial pass is enough for
"concrete-input collapses, parameterized stays symbolic," which was the
acceptance bar.

*(Update: all three landed in Stage 4 — see addendum below.)*

### Confirmed by implementation

- **Gap 1 resolution holds.** Auto-accumulation of guards into residuals,
  with end-of-query simplification, was the right shape. No new goals
  needed beyond `Where`.
- **Gap 2 (per-entry scope) still hasn't bitten.** None of the new tests
  exercise abstract direction refs that bind names visible to a `Where`.
  Will surface when the Stage 3 surface syntax lets users write queries
  that traverse `defer_dir` patterns symbolically.
- **Gap 3 resolution holds.** `env: &Bindings` is the input shape; it
  applies to expression simplification, not to logic-variable binding.

### Open from Stage 2-residuals

- **Surface syntax (Stage 3 of `unified_query.md`).** Last
  Open-from-Stage-2 item. The hand-built Rust query values are workable
  for tests but not for the agent-tool surface in the vision memory.
- ~~**Bespoke simplifier (Stage 4).** Triggered by a use case where the
  symbolic residual is too noisy to render.~~ Landed (see Stage 4
  addendum below).
- **Per-entry scope rule (Gap 2).** Will need pinning when surface syntax
  lets users name binders that flow through `defer_dir` abstract refs.

---

## Addendum (2026-04-30) — Stage 4: residual reasoning landed

The remaining "bespoke simplifier" work shipped: range narrowing,
contradiction detection, algebraic identities, and equality
substitution. The user pulled in equality substitution alongside the
other three rather than deferring it.

### What shipped

New module `src/engine/simplify.rs`. Public entry
`simplify::reduce(eng, expr, env)` that runs an iterated pipeline (max
8 iterations, fixpoint detected by structural equality):

1. **`apply_identities`** — bottom-up rewrite. Boolean absorption/
   annihilation (`e ∧ true → e`, `e ∧ false → false`, …), arithmetic
   identities (`n + 0 → n`, `n * 1 → n`, `n * 0 → 0`, `n - n → 0`),
   double-negation elimination, syntactic-equality reductions
   (`e ∧ e → e`, `e = e → true`), full per-op constant folding.
2. **`flatten_and`** — top-level And-tree → flat conjunct list.
   Short-circuits on any `LitBool(false)` conjunct.
3. **`extract_equalities`** — `var = expr` becomes a substitution
   entry whenever `var` is a bare `Var(s)` not appearing in `expr`.
   Substituted everywhere (and the equality fact itself is kept in the
   final output, since it's part of the answer the caller needs).
4. **Linear normalization (`Linear` + `to_linear`)** — every
   comparison atom is normalized to `c0 + Σ ci * vi ⋈ 0`. If the
   resulting linear has exactly one variable with coefficient ±1, we
   extract a `SimpleAtom { var, op, rhs }` (flipping the inequality
   when coefficient is -1). Examples: `n + 1 > 5` → `n > 4`,
   `5 - n < 3` → `n > 2`. Multiplication is permitted only when at
   least one side is a pure constant — `n * m` is detected as
   nonlinear and left in `others`.
5. **Per-variable `Interval`** — `lo`, `hi` (each `Option<(i64, bool)>`
   carrying its inclusivity), plus a `BTreeSet<i64>` of explicit `≠`
   values. Merge rules: same value, exclusive (strict) wins; different
   values, max wins on lower bounds, min wins on upper bounds. Empty
   intervals (`lo > hi`, or singleton excluded by `ne`) trigger a
   `LitBool(false)` short-circuit.
6. **Singleton promotion** — closed `[k,k]` interval becomes
   `var = k`, which feeds into the next iteration's substitution.
7. **Reassemble + dedupe** — emit equalities, then narrowed atoms,
   then everything else. Dedupe by Debug repr (cheap; residuals are
   tiny). Empty list → `LitBool(true)`.

### Renames

- `eval::simplify` → `eval::const_fold`. The new name is more
  descriptive — it's the leaf operator the bigger pipeline calls — and
  avoids confusion with the new `simplify` module.

### Acceptance bar (all met)

- `n > 0 ∧ n > 5` → `n > 5`
- `n >= 0 ∧ n > 0` → `n > 0`
- `n > 5 ∧ n < 3` → `false` (answer dropped)
- `n + 0 + 0 > 5` → `n > 5`
- `n + 1 > 5` → `n > 4` (linearization)
- `(n > 0) ∧ (n > 0)` → `n > 0`
- `n >= 5 ∧ n <= 5` → `n = 5` (singleton promotion)
- `n = 5 ∧ n + m > 10` → `n = 5 ∧ m > 5` (equality substitution)

### What still doesn't simplify

- Multi-variable linear arithmetic without an equality to substitute.
  `n + m > 10` stays as written (no narrowing on either variable
  alone).
- Comparisons with coefficients other than ±1. `2*n > 5` is left in
  `others` rather than promoted to `n >= 3`. Adding this would need
  signed integer division with rounding, plausibly a Stage 5 piece if
  examples ever produce such atoms.
- Disjunction narrowing. `(n > 5) ∨ (n > 10)` is left as written.
  Disjunctions are rare in current residuals; defer until they aren't.
- Field-access folding beyond what `const_fold` already does (which is
  Field-on-Construct).

### Test count

The simplifier module has 10 unit tests covering each pipeline stage.
`uquery.rs` has 14 tests (9 reductions + 5 residual/simplifier). All
24 pass. One existing test (`where_clause_adds_user_constraint`) had
its no-env assertion updated — what was previously a 2-conjunct
And-tree (`n > 0 ∧ n > 5`) is now a single `n > 5` after narrowing.

### Open from Stage 4

- Multi-variable narrowing (deferred — no use case yet).
- Coefficient-≠-±1 narrowing (deferred — no use case yet).
- Disjunction narrowing (deferred — no use case yet).

### Next step

Stage 3 (surface syntax) is now the only remaining Open-from-Stage-2
item. Worth picking up next once the user wants the agent-tool surface
or just a friendlier CLI than constructing query values in Rust.
