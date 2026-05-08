# 2026-05-08 — Lazy query iterator + transitive defer walking

## What landed

Two solver changes that go together.

### Lazy query iterator

`Engine::query` returned `Vec<Answer>`; now returns `Answers<'a>` —
a struct holding the search state, with `impl Iterator<Item = Answer>`.

Internals:
- `match_goal` returns `Box<dyn Iterator<Item = Answer> + 'a>` per arm.
  Each arm captures `ans` by move and yields matching answers lazily.
- `Answers<'a>` holds an explicit DFS stack of `Frame { rest_goals,
  matches }`. `next()` drives the search by pulling from the top frame,
  pushing for the next goal, popping on exhaustion, advancing to the
  next disjunct when the stack drains.
- The simplifier runs as a per-answer `filter_map` step inside `next()`
  — false-residual answers are dropped silently without being yielded.

The boxed-dyn-iterator is the cost of recursive iterator types in Rust;
it's per-frame allocation, but search depth = goal count so it's small.
The CLI and tests `.collect()` at use sites where they need a `Vec`;
nothing forced the laziness yet, but it's now available.

### `Goal::Reach { walk, from, to }`

A new variant for transitive defer walking. Given a concrete starting
`(from_iface, from_position)`, BFS over defer edges in the requested
direction (`Walk::Forward` or `Walk::Backward`), yielding every
reachable `(iface, position)` pair (including the start, 0-hop). A
visited set terminates cycles.

Forward edge:
`(defer.source, entry.source_pos) → (defer.target, entry.target_pos)`.
Backward is the inverse.

The motivating use case (chain.poly) is "given InterfaceA at StateA,
what actions are possible at InterfaceC?" Composes naturally:

```rust
Reach { Forward, from=(A, StateA), to=(C, ?p) }
Direction { iface=C, position=?p, action=?a }
```

Three new tests in `query.rs`:
- `reach_forward_chain_two_hops` — verifies the BFS visits all three pairs
- `reach_forward_then_direction_finds_action_via_chain` — the user's
  motivating use case end-to-end
- `reach_backward_chain_two_hops` — same chain walked in reverse

## Why these go together

`Goal::Reach` opens the door to infinite answer spaces: parameterized
states with no upper bound (Counter::Run keeps incrementing) become
reachable in unbounded chains. The lazy iterator is what makes those
queries tractable — consumers can `.take(n)` or filter without forcing
the whole answer space.

For finite cases (chain.poly, all the existing tests), eager and lazy
produce the same answers; iteration order is preserved.

## Open

- **Parameterized positions.** `Goal::Reach` today walks edges by
  comparing `entry.source_pos` to a target `Sym`; it ignores the
  parameter `pattern`/`args` on entries. So `Count[n] -> Count[n+1]`
  in counter.poly steps as if it were `Count -> Count` (loops on
  itself). Fine for chain.poly's parameter-free positions; needs a
  proper handling when a real call site queries against a
  parameterized chain. Same design question as the broader
  "parameterized inputs" item in the roadmap.
- **Visited-set granularity.** Visited is keyed on `(iface, pos_sym)`
  — fine for parameter-free, but two distinct parameterized
  configurations of the same position name would collapse to one
  visited entry. Revisit when parameters are handled.
- **Direction goals on the chain itself.** `Reach` only yields
  positions; it doesn't tell you which actions you traversed. If a
  use case wants the path of actions, that becomes a different goal
  shape (or a multi-step query with `DeferDir` as one of the goals).

## Why this matters

The `--explain`/`--locate` CLI flags can answer "what's local to this
interface" — they don't follow chains. With `Goal::Reach`, queries can
finally express the questions a service or agent actually wants to ask
across multi-defer compositions. Combined with the lazy iterator, the
solver is now a real (if simple) Datalog-flavored query engine instead
of a single-step pattern matcher.
