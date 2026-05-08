# 2026-05-08 — Goal::Step (one-hop transitions, Stage B)

## What landed

`Goal::Step` — one-hop state-machine transition as a query goal.

```rust
Goal::Step {
    iface: Term,
    from_position: Term,
    from_args: Vec<Expr<Sym>>,
    action: Term,
    to_position: Term,
    to_args: Slot,
}
```

Mechanics in `match_goal`:

1. Resolve `iface`, `from_position`, `action` to concrete `Sym`s
   (else fail).
2. Locate the realization defer for `iface` via
   `iface_internal_relation` — the unique defer whose source ends in
   `::Internal` and whose target is `iface`.
3. Find the entry whose `source_pos == from_position`.
4. Find the direction mapping whose `target_dir == Named(action)`
   and whose `source_dir` is `Abstract { src_pattern, tgt_pos,
   tgt_args, .. }`.
5. Build `src_sub` from `src_pattern.zip(from_args)` (Pattern::Bind →
   sub entry; Pattern::Wildcard → skip).
6. Substitute `src_sub` through `tgt_args` to compute the destination
   args.
7. Look up the destination position's declaration in `iface`; build
   `dst_sub` from its formal params + computed args; substitute
   through its guard; push to residual.
8. Unify `to_position` with `tgt_pos`; unify `to_args` (Slot) with
   `Value::Args(computed)`.

The residual machinery does the rest: in-bounds destinations reduce
their guard to true and clear; out-of-bounds (e.g. `Cell[(0, 5)]`
fails `1 ≤ c.x`) drops the answer entirely; symbolic args carry the
guard parameterized by the user's chosen names.

Three tests:
- `step_right_in_grid_yields_neighbour` — `Right` at `Cell[(5,5)]`
  in `Grid[10,10]` → `Cell[(6,5)]`, residual cleared.
- `step_left_at_left_edge_drops_via_guard` — `Left` at `Cell[(1,5)]`
  → destination `Cell[(0,5)]` fails the position guard, dropped.
- `step_increment_counter` — `Increment` at `Count[3]` → `Count[4]`.

## Decisions made

### Realization defer is the substrate

Stage B is grounded in the state-machine sugar: every iface with
transitions desugars to `Foo::Run : Foo::Internal -> Foo` with
abstract direction refs encoding the action semantics. Stepping is
"pull the abstract direction's `tgt_args`, evaluate them under the
current params." We don't need a separate transition table; the
defer entries already encode it.

### `to_args` binds via `Slot`, not via `Vec<Expr>`

The output (computed) args is naturally a `Value::Args(...)`
binding on the answer's substitution. Slot::Var(v) reads it; Slot::Anon
discards it.

The asymmetry with `from_args: Vec<Expr<Sym>>` (input) is
deliberate — input args must be expression literals because the
caller is *supplying* them, not querying.

### Destination guard substitution

When the destination position has a parameterized guard like
`1 ≤ c.x ≤ Width`, we substitute the *computed args* into it before
pushing to the residual. The simplifier then folds field accesses
(`Coordinate(6, 5).x → 6`) and reduces the conjunction. This is the
same pattern as Stage A's `Goal::Position::args` handling.

## Open

### Stage C: chaining + path tracking

The current `Goal::Step` is one-hop and fundamentally non-chainable
within a single query. `to_args` is read out as `Value::Args`, but
the next `Goal::Step` expects `from_args: Vec<Expr<Sym>>` — there's
no goal-level way to promote the slot binding back into expression
form.

Two ways forward (decision deferred):

1. **Term-level args.** Introduce a `Term::ArgsRef(VarId)` or similar
   that resolves through the answer's substitution. Then
   `Goal::Step`'s `from_args` becomes `Vec<ArgPat>` where ArgPat is
   either `Expr` or `ResolveSlot`. Allows chaining within one query.
   Mid-sized refactor.

2. **Path-finding as a Rust function.** Don't try to express it in
   the query language; expose `Engine::find_paths(start, end, env)`
   that uses `collect_action_steps` internally with BFS/DFS. The
   query language stays simple; Rust callers do graph search.
   Consistent with the project's "let real call sites force the
   shape" principle.

Decide when a real call site asks for it.

### Backward Step

`Goal::Step` only handles forward transitions today. A backward
Step ("what action takes me from B to A?") would need to enumerate
direction mappings whose `tgt_args` substitute to the desired
destination — non-trivial because of the symbolic substitution.
Defer until needed.

### Multi-position state machines

Today's `collect_action_steps` assumes the destination position is
`source_dir.tgt_pos`, which the realization defer's entry maps
identity-wise back to the external iface. If a state machine has
transitions between *different* position names (e.g.
`Pressed -> Released`), we'd need to handle the cross-position case.
Counter-style and grid-style examples are self-loop on a single
position name, so this case isn't exercised yet.

## Why this matters

Stage B closes the loop on the original "find paths from Cell[a]
to Cell[b]" question — at least at the one-hop level. Each `Step`
now answers "what does action X do at parameterized state Y?" with
a fully evaluated destination + guard check. The arithmetic and
field-folding falls out of the existing simplifier; we didn't need
new evaluation machinery, just a new way to compose what we have.

Multi-hop is the obvious next thing, but the chaining design call
(Term-level args vs free-function path search) wants more evidence.
