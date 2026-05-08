# Poly roadmap

Open work, with status. Closed items live in the session logs under `log/`.

## Active

### One Engine, one query method

The public surface is intentionally minimal: `Engine::load` to load a
program, `Engine::query(query, env) -> Answers<'_>` (lazy iterator) to
ask anything about it. Caller composes a `Query` from `Goal`s; engine
returns an iterator of answers with simplified residuals. Anything
more domain-specific (typed result structs, JSON wire format, …) is
built *on top* by the consumer, not in the engine.

`Goal::Reach { walk, from, to }` walks defer edges transitively (BFS
with visited set, both directions). The motivating "given A.StateA,
what's possible at C through any chain of defers?" works today.

The CLI is the proving ground: `--explain` and `--locate` construct
`Query` values inline and project the answers into their display
formats. Future consumers (the planned service) do the same.

Open with this shape:

- **Parameterized inputs.** Today queries take `Term::Sym(s)` for
  bound names. To bind into parameterized positions (e.g. `Count[5]`)
  with the param flowing into the answer's residual, we'll want a
  pattern-shaped `Term` variant or a dedicated `Goal::PositionRef`.
  `Goal::Reach` also currently ignores parameter args on entries —
  resolving this is the same design call. Decide when a real call
  site forces it.
- **Convenience constructors.** `Query::single`/`Query::or` is the
  whole API surface today. If common query shapes recur in CLI/tests,
  add small builder helpers (`Query::all_directions_at(iface, pos)`)
  rather than adding more `Goal` variants.

### Query surface syntax (text or JSON)

The Rust API above is the priority. A textual or JSON query surface is
useful for human exploration and (later) non-Rust consumers; design it
once the named-op set is stable enough that the wire format is mostly a
serialization of those ops.

`log/unified_query.md` §2 has the original sketch (logic-variable syntax
over relation schema). Open shape questions still apply when this lands:

- Does the query parser share the expression parser with `.poly`?
  Probably yes (same `Expr<T>` AST, same operators).
- How are disjuncts written? `or {} {}` blocks, or `;` separators, or
  one query per disjunct with an explicit union?
- Result rendering: flat tuples vs. grouped (Gap 4 in
  `log/unified_query_stage0.md`). Default to flat; let consumers group.

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
