# `state` blocks: promoting the universal state machine to a primitive

Date: 2026-04-29
Status: design sketch — pre-implementation
Reads alongside: `ns_and_internal.md`, `unified_query.md`, `unified_query_stage0.md`

## Motivation

Two related observations point at the same gap:

1. **Stage 2 finding (Gap 7).** The `transition/5` fact relation is permanently
   empty under current sugar — every `interface` with declared transitions is
   rewritten into an external `interface` (transitions stripped) plus a
   sugar-derived `<X>::Internal` plus a realization defer. There's no first-
   class way to ask "give me a state space whose dynamics are universal" — you
   only get one as a hidden by-product of the sugar.

2. **Layering rule's "::Internal" carrier.** `ns_and_internal.md` made abstract
   direction refs (`Count[n] => Count[n+1]`) legal *iff* the defer's source
   interface name ends in `::Internal`. That works, but it's a string-suffix
   convention standing in for a *structural* property: the source is the
   universal state machine `S · y^S` over its position set. The convention is
   load-bearing for validation but invisible at the grammar level.

A `state` block names the structural property explicitly:

```
state Counter::Internal
    Count[n: Int] if (n >= 0)
```

This declares the positions S and asserts directions are *implicitly*
$S \cdot y^S$ — every conceivable abstract transition between positions,
constrained by the ambient guards. No directions are written; none are
expected.

## Categorical framing

- `interface` is the general polynomial $p = \Sigma_i y^{p[i]}$. Each position
  carries an explicit direction set.
- `state` is the special polynomial $S \cdot y^S$ over positions $S$. Every
  position has the same direction set: "transition to any other position."

Both desugar to the same `Interface<T>` runtime structure (positions list +
per-position params + per-position guards). The difference is *intent* and
*validation*:
- `interface` directions are enumerable; queries can ask "what actions are
  available."
- `state` directions are infinite-as-written; queries that touch them must go
  through abstract direction refs in defers.

## Sugar redefinition

Today's `interface Foo { Action -> Dest }` sugar produces three declarations.
With `state` available, the same sugar should produce:

```
state    Foo::Internal
    [positions, params, guards copied]

interface Foo
    [positions, params, guards copied; only direction *names* listed]

defer    Foo::Run : Foo::Internal -> Foo
    [one entry per position; directions realize each declared transition as
     an abstract direction ref]
```

The desugared output is more verbose but more legible — each piece is its own
top-level declaration with no naming-convention dependence. Round-tripping
sugar through the parser becomes "elaborate to three blocks" instead of
"elaborate to three blocks and remember the suffix mapping."

## Layering rule (cleaned)

Abstract direction refs `(SrcPat => TgtExpr)` are valid in a defer body iff
the defer's source declaration is a `state`. The error message becomes:

> abstract direction ref is not valid here; the defer's source must be a
> `state` block, but `Foo` is an `interface`

The string-suffix check (`name ends with "::Internal"`) goes away. Validation
walks the declaration kind, not the name.

## AST changes

```rust
pub enum Decl<T> {
    Interface(Interface<T>),
    Defer(Defer<T>),
    Schema(Schema<T>),
    State(StateBlock<T>),                  // new
}

pub struct StateBlock<T> {
    pub name: T,
    pub params: Vec<Param<T>>,             // outer params (Width, Height, ...)
    pub positions: Vec<Position<T>>,       // each with empty directions
}
```

The `Position<T>` already supports empty `directions: []`, so no change there.

`Engine` gains a `pub states: BTreeMap<Sym, StateBlock<Sym>>` field. The
existing `interfaces` map drops the sugar-derived `<X>::Internal` entries —
those now live in `states` instead.

## Interaction with the fact base

The unified-query relation schema (`unified_query.md` §1) needs three
amendments:

1. **New relation `state_block/2`:**
   ```
   state_block(S, [param: type, ...])
   ```
   Mirrors `iface/2` but identifies state declarations. A position lookup uses
   `state_position(S, P, params, guard)` — same shape as `position/4` but on
   the `state` declarations rather than `interface` declarations.

2. **Or, a unified `position/5` with a `kind` field:**
   ```
   position(I_or_S, P, params, guard, kind)   -- kind ∈ {iface, state}
   ```
   Single relation, queries discriminate by `kind` when needed. Probably
   cleaner — keeps "position" as one concept across both declaration kinds.

3. **Drop `iface_internal/2`.** Replaced by:
   - `state_block(?S, ?)` — does this name a state block?
   - `defer(?D, ?S, ?I), state_block(?S, _), iface(?I, _)` — the realization-
     defer pattern, expressed structurally rather than by name suffix.

   `iface_internal/2` was a stand-in for the structural relationship; with
   `state` first-class, the structural query is direct.

The Q2 reduction in `unified_query.md` §2 simplifies: the realization disjunct
becomes `defer(?D, ?S, I), state_block(?S, _), defer_entry(...)`. Cleaner than
the current `iface_internal(?I_int, I), defer(?D, ?I_int, I), ...` chain.

The `transition/5` relation stays empty under current sugar — that finding
isn't *resolved* by `state` blocks per se, but the resolution becomes more
plausible: "if you want direct transitions on an `interface`, write them; the
`state` block is for the universal-dynamics case." The dead-weight relation
gets a clearer interpretation.

## Open questions

- **Naming of sugar output.** Today: `Counter::Internal` + `Counter::Run`. With
  `state` first-class, do we keep these names? Or:
  - `state Counter` + `interface Counter::View` (the external is the derived
    one)? Reverses the user-facing register.
  - `state Counter::States` + `interface Counter`? Intent-named.
  - Keep `::Internal` for backwards compatibility, document as auto-generated.

  Probably keep `::Internal` for now; the convention is well-established and
  `state` lets us *justify* rather than rely on the suffix.

- **Can `state` declarations be parameterized?** Yes — same as interfaces.
  `state Grid::Internal[Width: Int, Height: Int]` is fine and follows from the
  $S \cdot y^S$ framing where $S$ may be parameterized.

- **Does `state` permit guards?** Yes — position-level guards. They constrain
  which positions are inhabited, which constrains which abstract transitions
  are valid.

- **Direction guards on `state`?** Awkward. Directions in `state` are
  $\{s \Rightarrow t : t \in S\}$, parameterized by *target*. A "direction
  guard" would be a per-target predicate, but it's hard to write without a
  binding for the target. Probably out of scope; ambient position guards on
  source and target give us enough.

- **Multiple `state` declarations sharing a defer?** Two state blocks could be
  related by a defer (a lens between two state spaces). Today not exercised;
  no change needed; the grammar already allows any `Decl` to be a defer
  source/target.

- **Interaction with multi-entry defers.** A `state` block targeting multiple
  external interfaces under separate `defer ... : state_block -> X` and
  `defer ... : state_block -> Y` — different realizations of the same
  dynamics. Already supported structurally; `state` just gives the source a
  legible name.

## Implementation stages

Each leaves the system in a working state.

**Stage 1: AST + parser.**
- Add `StateBlock<T>`, `Decl::State(StateBlock<T>)`.
- Parser accepts `state Name[params]` declarations with the same position
  body grammar as `interface`, *minus* directions.
- Lower interns symbols; engine stores in `Engine.states`.
- `fmt_state_block` mirrors source syntax.
- Existing `interface ... { ... -> ... }` sugar continues to produce
  `<X>::Internal` *as an interface* with empty directions for now — Stage 1
  doesn't touch the sugar.

**Stage 2: sugar rewrite.**
- Update sugar so it emits a `Decl::State(StateBlock { name: "<X>::Internal",
  ... })` instead of an `Interface` with empty directions.
- Update validation in `validate.rs`: the layering rule reads
  `engine.states.contains_key(&defer.source)` instead of name-suffix check.
- Update query.rs's `apply_realization` and `apply_via_defer_source` to walk
  `Engine.states` where appropriate.

**Stage 3: facts + uquery integration.**
- Decide between `state_block/2` separate relation vs. `position/5` with
  `kind` field. Pick.
- Drop `iface_internal/2` from the fact base.
- Update Q2 reductions in `unified_query.md` §2.

**Stage 4: optional cleanup.**
- Reconsider `transition/5` retention. With `state` distinguishing dynamics
  from interfaces, the case for direct transitions on `interface` weakens
  further — most agentic-system shapes go through realization defers.
- Possibly drop `transition/5` and the `Direction.transition` field entirely,
  forcing all transitions through `state -> interface` defers.

Stage 4 is genuinely optional and probably wants its own design discussion.

## Out of scope

- Any change to `state` directions (they're $S \cdot y^S$, period). If someone
  wants restricted dynamics, write a defer.
- Direction guards inside `state` blocks.
- Parameterized `state` *positions* whose param spaces depend on each other
  (e.g. dependent types). Same concern as interfaces; not adding it here.
- Higher-order `state` (a state block whose positions are themselves state
  blocks). Don't go here without a concrete need.

## Summary

`state` is a small, conservative grammar addition that makes structural
intent legible:

- Names the universal-state-machine primitive that already exists by
  convention.
- Replaces the `::Internal` suffix as the validation carrier for abstract
  direction refs.
- Cleans up the unified-query relation schema by dropping `iface_internal/2`.
- Doesn't change runtime semantics — `state` declarations represent the same
  $S \cdot y^S$ polynomial that `<X>::Internal` already does today.

The work is mostly grammatical and bookkeeping. The design payoff is mostly
clarity: the categorical primitive gets a name, validation gets a structural
basis instead of a string convention, and the layering rule reads cleanly.
