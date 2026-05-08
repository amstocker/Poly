# 2026-05-08 — Parameterized positions in queries (Stage A)

## What landed

`Goal::Position` gains an `args: Vec<Expr<Sym>>` field. Empty = no
arg constraint (current behavior). Non-empty = arity-checked against
the position's formal params; the args are substituted directly into
the position's guard, and the substituted guard is what lands on the
residual.

```rust
Goal::Position {
    iface: Term,
    position: Term,
    args: Vec<Expr<Sym>>,    // NEW
    params: Slot,
    guard: Slot,
}
```

The substitution is done in `match_goal` using `simplify::substitute`
(promoted from private to `pub(crate)`), not by emitting `formal = arg`
equalities. Equalities would echo back through the simplifier's
reassemble pass, polluting the answer's residual with what is really
just internal plumbing. Substituting upfront keeps the residual
consumer-facing.

Three tests against `grid.poly`:

- `cell_in_bounds_yields_one_answer_with_empty_residual` —
  `Cell[Coordinate(5, 5)]` in `Grid[10, 10]`: guard reduces to true,
  residual clears, one answer.
- `cell_out_of_bounds_drops_the_answer` —
  `Cell[Coordinate(11, 5)]` in `Grid[10, 10]`: guard reduces to
  false, answer dropped.
- `cell_symbolic_carries_guard_in_residual` —
  `Cell[c]` (using the schema's formal name `c`), no env: guard
  passes through unchanged, residual carries `1 ≤ c.x ≤ Width ∧
  1 ≤ c.y ≤ Height`.

Iface-level params (`Width`, `Height`) are supplied via the existing
`Bindings` env, not goal args — that mechanism already worked.

## Decisions made

### Substitute args, don't emit equalities

The first attempt pushed `formal = arg` onto the residual and let the
simplifier do equality substitution. That worked for guard reduction
(`c.x` got replaced by the arg's `.x`) but left the equality itself
in the output residual, even when the guard fully cleared. The
caller doesn't want `c = Coordinate(5, 5)` echoed back — they
*supplied* it, they know.

Switching to upfront substitution in `match_goal` keeps the residual
output as "constraints the caller still needs to satisfy," which is
the right meaning. The simplifier still does its work (folding,
narrowing, identity collapse) on the substituted guard.

### No logic-variable args

`args: Vec<Expr<Sym>>` admits literals, constructors, arithmetic, and
named symbolic vars (`Expr::Var(sym)`). It does *not* admit query
logic vars (`VarId`). Reason: parameterized positions aren't
enumerable. There's no `Cell[Coordinate(0,0)]`, `Cell[Coordinate(0,1)]`,
… list to iterate; the caller has to supply the value or leave it
symbolic. A `Term::Var` arg would have nothing to bind to.

### No `args` on `Goal::Iface` / `Goal::Direction`

Iface params (`Grid[Width, Height]`) are reachable via the `Bindings`
env passed to `Engine::query`. The user supplies `env: { Width: 10,
Height: 10 }` and it flows through const_fold automatically. Adding
goal-level args here would be redundant.

Direction params would behave identically to position args, but no
current example exercises parameterized directions. The pattern
trivially extends when needed.

## Open

- **Stage B: transitions.** `Goal::Reach` walks defer entries by Sym
  comparison only — it ignores parameter args on entries. To follow
  state-machine-like steps (e.g. `Count[c] -> Count[c + 1]` along
  `Increment`), we need a goal that:
  1. Looks up the realization defer for the iface.
  2. Finds the matching named direction.
  3. Substitutes the source args through the abstract direction's
     `tgt_args` to compute the destination args.
  4. Checks the destination's guard; yields if satisfied.
  Likely shape: `Goal::Step { iface, src_args, action, dst_args }`.
  The motivating use case is path-finding in `grid.poly`.

- **Path tracking.** Once Step works, transitive closure with action
  history yields the "path from A to B" answer the user originally
  asked about.

## Why this matters

Stage A is the foundational change: queries can now refer to specific
parameterized states. Without it, every other Stage (transitions,
reachability, path-finding) was blocked on "how does the query say
`Cell[(0, 0)]`?" That's settled. The simplifier's existing machinery
(field-on-construct folding, env substitution, interval narrowing)
handles the heavy lifting once args reach it.
