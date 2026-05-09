# 2026-05-08 — Goal::Path: BFS path-finding (Stage C)

## What landed

`Goal::Path` — transitive reachability over state-machine action edges,
yielding action sequences.

```rust
Goal::Path {
    iface: Term,
    from_position: Term,
    from_args: Vec<Expr<Sym>>,
    to_position: Term,
    to_args: Vec<Expr<Sym>>,    // empty = no constraint
    path: Slot,                   // binds Value::Path(Vec<Sym>)
    max_depth: Option<usize>,
}
```

Plus `Value::Path(Vec<Sym>)` — a sequence of action names.

`match_goal` runs BFS via the new `collect_action_paths` helper. Each
node in the BFS is `(pos, evaluated_args, action_history)`. Expansion:
for every named direction at the current position, call
`collect_action_steps` (Stage B's helper) to compute the destination,
fold the destination args and guard under `env`, drop candidates whose
guard folds to literal `false`, key the visited set on `(pos,
folded_args)`. If new, enqueue. The starting state is yielded with
an empty path (depth 0).

`env` is now threaded through `match_goal` so guard folding during BFS
can substitute iface-level params (`Width`, `Height` for grid.poly).
This is the single non-trivial structural change to the solver from
this turn.

## Five tests against grid.poly:

- `path_to_self_is_empty` — `(3,3)` → `(3,3)` with `max_depth=0`: one
  answer, empty path.
- `path_to_neighbour_is_one_action` — `(3,3)` → `(4,3)`: one answer,
  path `[Right]`.
- `path_finds_shortest_route` — `(1,1)` → `(3,2)`: one answer, path
  length 3 with exactly 2 Rights and 1 Down (any interleaving).
- `path_max_depth_caps_search` — `(1,1)` → `(4,4)` with
  `max_depth=3`: zero answers (needs 6 hops).
- `path_open_destination_yields_reachable_set` — open destination
  with `max_depth=2`: ≥13 reached states (start + 4 neighbours +
  expansion at depth 2).

## Decisions made

### BFS with visited dedup (canonical paths)

We discussed two semantics for "infinitely many paths":
- **Visited set on `(pos, args)`** — finite, yields each reachable
  end-state once with shortest path. (Picked.)
- **Combinatorial enumeration** — every action sequence up to
  `max_depth`, no dedup. Distinct sequences that revisit cells all
  surface. Combinatorial fanout.

Default to the first. The combinatorial case can be a separate
`Goal::AllPaths` if a use case ever forces it.

### env threading

Originally `match_goal` didn't see `env`; it accumulated guards onto
the residual and `simplify_answer` reduced them at the end. For BFS
that's wrong — without env-aware guard folding *during* expansion,
out-of-bounds states like `Cell[(0, 5)]` would expand to
`Cell[(-1, 5)]` and the BFS would never terminate over the unbounded
arithmetic.

Threading `env: &Bindings` through `match_goal` was the cleanest
fix. Existing arms ignore the parameter; only `Goal::Path` uses it
(for now).

### Visited key: position name + folded args

`(Sym, Vec<Expr<Sym>>)` after folding via `const_fold(eng, e, env)`.
Field-on-Construct folding canonicalizes `Coordinate(c.x + 1, c.y)`
with `c = Coordinate(5, 5)` to `Coordinate(6, 5)`, so two paths
arriving at the same cell collapse correctly.

If args don't fully fold (because env is incomplete or symbolic),
the visited set won't canonicalize and the BFS may not dedup. The
documented requirement: caller supplies enough env that args fold
to concrete values. `max_depth` is the safety net otherwise.

### Linear visited

Visited is `Vec<(Sym, Vec<Expr<Sym>>)>` with linear search instead
of a hash/tree set. `Expr<Sym>` doesn't derive `Hash` or `Ord`, and
adding either touches the public AST. For grids and counters the
visited set stays small (bounded by `width × height` for a grid),
so linear is fine. Promote to a hash-set if a benchmark forces it.

### Symbolic guards: keep, don't drop

Drops only on `LitBool(false)` after fold. `LitBool(true)` proceeds
clean. Anything else (symbolic) also proceeds — we don't surface
the residual on the path's answer (the `path` slot is the primary
output). If a path's intermediate state had a symbolic guard, the
caller is on the hook to verify it themselves; for fully-concrete
queries (grid.poly with env), this never fires.

## Open

- **User-composable chaining.** `Goal::Step` chained N times by
  hand still doesn't work — `to_args` is a `Slot`, `from_args` is
  `Vec<Expr<Sym>>`, no bridge. Defer until a real call site asks.
- **All-paths enumeration.** Combinatorial; needs an explicit goal.
  Defer.
- **Backward `Goal::Path`.** Symbolic inversion of `tgt_args` is
  hard; defer.
- **Visited canonicalization with symbolic args.** Not currently
  supported. Hash on `Expr<Sym>` would let us key without folding,
  but two structurally-different equivalent expressions would be
  treated as distinct. Real reasoning needs the simplifier.

## Why this matters

Grid path-finding works:

```rust
let q = Query::single(vec![Goal::Path {
    iface: Term::Sym(grid),
    from_position: Term::Sym(cell),
    from_args: vec![coord_expr(&eng, 1, 1)],
    to_position: Term::Sym(cell),
    to_args: vec![coord_expr(&eng, 3, 2)],
    path: Slot::Var(path_v),
    max_depth: Some(10),
}]);
let env = grid_env(&eng, 10, 10);
let answers: Vec<_> = eng.query(&q, &env).collect();
// answers[0] has Value::Path([Right, Right, Down]) (or similar order)
```

Stages A + B + C together close the loop on the original "what
paths exist between these parameterized states?" question. The
arithmetic, field-folding, and substitution all fall out of pieces
that already existed (eval.rs's const_fold, simplify.rs's
substitute, the iterator-based solver from earlier this session) —
the only new piece is BFS coordination + the path-tracking machinery.

This is also where the layering pays off: `Goal::Path` calls
`collect_action_steps` (Stage B's helper) which calls `substitute`
(simplify.rs); each layer is small and the composition gives us
robust parameterized path queries.
