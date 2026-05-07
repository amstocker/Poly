# Poly roadmap

Open work, with status. Closed items live in the session logs under `log/`.

## Active

### Query surface syntax

Today: queries are constructed as Rust values (`uquery::Query`), reachable
from tests and from `main.rs`'s `--explain`/`--locate` flags. The agent-tool
surface and the friendlier CLI both want a parseable query language.

Sketch in `log/unified_query.md` §2: logic-variable syntax over the
relation schema, three styles of binding (bound / free / pattern). Result
is a list of `Answer { subst, residual }` — the residual is part of the
answer, not a precondition.

Open shape questions:

- Does the query parser share the expression parser with `.poly`? Probably
  yes (same `Expr<T>` AST, same operators).
- How are disjuncts written? `or {} {}` blocks, or `;` separators, or one
  query per disjunct with an explicit union?
- Result rendering: flat tuples vs. grouped (Gap 4 in
  `log/unified_query_stage0.md`). Default to flat; let consumers group.

Acceptance: the existing `--explain`/`--locate` Rust-built queries can be
written as text and produce identical answers.

### `state` blocks

`log/state_blocks.md` proposes promoting the universal-state-machine
carrier (currently `<X>::Internal` interfaces) to a first-class `state`
declaration. Removes the `::Internal` string-suffix as the validation
carrier; replaces it with structural lookup. Cleans up the layering rule.

The case for doing it now: with the concrete engine gone, validation and
the fact base are the only places the suffix convention is load-bearing.
Both are small enough that the migration is tractable. Wait if the query
surface syntax is more pressing.

### Recursive defer chains

"Where does Press *eventually* take Counter (through any chain of defers)?"
is recursive. Today's solver has no fixpoint; it walks one defer at a
time. The natural answer is Datalog-with-fixpoint; this is the case that
might force adopting an external Datalog runtime.

Probably not active until an example demands it.

## Constraint engine — known limitations

The simplifier handles single-variable linear arithmetic with `|coef|=1`
plus equality substitution plus singleton promotion. Gaps:

- **Multi-variable narrowing.** `n + m > 10` stays as written; no
  per-variable bound is derived without an equality.
- **`|coef|≠1` atoms.** `2*n > 5` is left unreduced; would need rounded
  signed division.
- **Disjunction narrowing.** `(n > 5) ∨ (n > 10)` is left as written.
- **Field-access folding** beyond `Field-on-Construct` (already handled).

Each blocks on a real example producing such residuals. None do today.

## Schema work (deferred)

- **Sum constructors** in expressions: `High` / `Low` / `Medium` aren't
  expressible as expressions yet; only record `Schema(...)` is.
- **Generics** in schemas: `Result[T] { Ok[value: T], Err[message: String] }`.
  Touches the type system; not pressing.

## Vision-layer items (not active work)

- **Configuration layer.** Live current-state of all interfaces, mutable.
- **Trajectory layer.** Append-only log of accepted state changes.
  Replayable. Adds `step(t, I, P, A, P')`-shaped relations the same query
  language should be able to talk about.
- **Constraint enforcement at runtime.** Reject schema-violating proposed
  changes. This is where "bounded latitude" becomes operational.
- **Agent-tool surface.** List interfaces, query enabled actions,
  attempt-to-set state. Built on top of the unified query, not in place of
  it.
- **Permission / privilege scoping.** Lives above the engine.
- **Limits / colimits-flavored composition.** Theoretical; no use case yet.

## Open questions worth holding

- **`schema` constructors for sums.** If sum schemas grow constructors,
  `Priority::High` vs. `High` syntax decision pending.
- **Parameterized direction names.** Action-with-params (e.g.
  `Set[value: Int]`). No example currently exercises this.
- **Multi-entry defers with overlapping source positions.** `Count[n] if
  (n <= 10) -> Off, Count[n] if (n >= 11) -> On`. Tractable extension to
  the parser + lowering when needed.
