# `::` Namespaces, `_` Wildcards, and Abstract Transitions

Date: 2026-04-28
Status: design pinned, implementation in stages

## Motivation

`examples/counter.poly` currently fails to parse because it relies on three
syntactic forms the language doesn't yet support:

```poly
interface Counter
    Count[n: Int] if (n >= 0) {
        Increment -> Count[n + 1],
        Decrement if (n > 0) -> Count[n - 1]
    }

interface Button
    X { Press }

defer SetTo10 : Counter::Internal -> Button
    Count[_] -> X {
        Press -> (Count[_] => Count[10])
    }
```

The example expresses something real: a button (Button.X.Press) is wired to a
specific abstract state-machine action of Counter — "go directly from any
valid Count to Count[10]" — even though Counter's *external* interface only
exposes Increment/Decrement. The mechanism that lets us reference such an
action is `Counter::Internal`, the universal state machine of Counter.

This doc pins three design moves that together make the example legible.

## (1) `::` for sugar-generated subnames

State-machine sugar generates two subnames per interface:
- `Counter::Internal` — the universal state machine `S · y^S`.
- `Counter::Run : Counter::Internal -> Counter` — the realization defer.

Today's code uses `Counter.internal` and `Counter.run`, but `.` is taken
(field access on schema records eventually). Switch to `::`:
- `::` is reserved for sugar-generated subnames. No user-written declarations
  use `::`.
- `.` stays free for field access.
- Lexically: identifiers may contain `::` separators between idents, e.g.
  `Foo::Bar::Baz` parses as a single dotted identifier.

## (2) `_` is a fresh anonymous binder with ambient constraints

Inside any pattern (a position pattern in a defer mapping, or a transition
pattern in a direction reference), `_` introduces a fresh variable that:
- is anonymous (cannot be referenced),
- inherits the type of the slot it sits in,
- automatically picks up the ambient guard from the position declaration.

So `Count[_]` against `Count[n: Int] if (n >= 0)` is shorthand for
`Count[fresh] if (fresh >= 0)`. The user does not write the `if`.

Different `_`s in the same expression are different variables. So
`Count[_] => Count[_]` is "any state to any other state" — *not* "same state."

Named binders (`n` in `Count[n]`) are identical except they can be referenced
later in the same scope (e.g. in the target expression or in directions).

## (3) `(SrcPat => TgtExpr)` is direction-reference syntax for `::Internal`

A direction in `Counter::Internal` is an abstract transition between two Counter
states. Syntactically:

```
( Count[_] => Count[10] )
( Count[n] => Count[n + 1] )
( Count[n] => Count[m] if (m > n) )
```

This form is permitted wherever a direction name is expected in a defer body,
*provided the defer's source interface is `Counter::Internal`* (or the principle
generalizes — see layering rule below).

Semantically, `(SrcPat => TgtExpr)` denotes: the abstract direction in
Counter::Internal at source position whose pattern matches `SrcPat`, leading to
the position-and-args described by `TgtExpr`.

## Layering rule

A defer's body may only reference directions that exist in its declared source
interface. Therefore:

- `defer Foo : Counter -> Button` may only reference `Increment`, `Decrement`
  (the user-declared directions of Counter).
- `defer Foo : Counter::Internal -> Button` may reference abstract transitions
  like `(Count[_] => Count[10])`.

The user must explicitly write `Counter::Internal` when they want abstract
transitions. The engine errors with:

> abstract transition `(Count[_] => Count[10])` is not a direction of `Counter`;
> did you mean `Counter::Internal`?

This keeps the layering legible at the defer signature.

## Sugar generalization

State-machine sugar today fires only on *finite, non-parameterized* state sets.
With parameterized states, the universal state machine has uncountably many
directions per position; we cannot enumerate them.

Resolution: generate `Counter::Internal` as an interface whose `positions`
mirror Counter's positions (same names, params, guards) but whose `directions`
are *empty* in the data structure. This represents "any abstract direction
permitted by the source/target guards." The engine handles abstract direction
references symbolically: it doesn't look them up in `directions`, it
pattern-matches against the position structure.

`Counter::Run : Counter::Internal -> Counter` realizes each user-declared
direction as the corresponding abstract transition. For parameterized Counter,
this becomes a *parameterized* defer entry — directions like `Increment` map
to `(Count[n] => Count[n+1])`, with `n` bound by the entry's source pattern.

This forces a parallel change to `Defer`: the AST must carry binders on the
source position, expressions on the target position, and a richer direction
reference type.

## AST changes

**Defer becomes:**

```rust
pub struct Defer<T> {
    pub name: T,
    pub source: T,
    pub target: T,
    pub entries: Vec<DeferEntry<T>>,  // was: pos_map + dir_map
}

pub struct DeferEntry<T> {
    pub source_pos: T,
    pub source_pattern: Vec<Pattern<T>>,    // [_, n, _, ...]
    pub source_guard: Option<Expr<T>>,      // extra constraint on binders
    pub target_pos: T,
    pub target_args: Vec<Expr<T>>,
    pub directions: Vec<DirMapping<T>>,
}

pub enum Pattern<T> {
    Wildcard,                // `_`
    Bind(T),                 // `n`
}

pub struct DirMapping<T> {
    pub target_dir: DirRef<T>,  // name in target interface
    pub source_dir: DirRef<T>,  // name in source interface (or abstract transition)
}

pub enum DirRef<T> {
    Named(T),                // bare action name (Press, Increment)
    Abstract {               // (SrcPat => TgtExpr)
        src_pos: T,
        src_pattern: Vec<Pattern<T>>,
        tgt_pos: T,
        tgt_args: Vec<Expr<T>>,
    },
}
```

`Pattern<T>` is intentionally minimal. Direction parameters (e.g. an action
that takes args) are deferred until we have an example that needs them.

## Implementation stages

Each stage compiles cleanly and leaves the system in a usable state.

**Stage 1: lexical & sugar.**
- Allow `::` in identifiers (single dotted identifier token).
- Generalize state-machine sugar to fire on parameterized interfaces:
  generate `Counter::Internal` (positions mirror, directions empty) and
  `Counter::Run` (one entry per user-declared transition, with parameterized
  source pattern and target args).
- Update existing display / lower / sugar code paths.

**Stage 2: defer body grammar.**
- Refactor `Defer` AST to the shape above.
- Parse position patterns on defer source: `Count[_]`, `Count[n]`,
  `Count[_] if (...)`.
- Parse target position with arg expressions: `Count[10]`, `Count[n+1]`.
- Parse direction references: bare names *or* `(SrcPat => TgtExpr)`.
- Update `lower::lower_defer` and `fmt::fmt_defer`.
- Update `query::explain_position` to walk new defer entries (preserve
  current behavior — no validation of abstract transitions yet).

**Stage 3: engine validation (deferred from this work, future session).**
- Validate that direction references in a defer body exist in the declared
  source/target interfaces.
- Reject abstract transitions when the source isn't `::Internal`.
- Validate that abstract transition source/target patterns satisfy ambient
  guards (with `_` desugared to fresh binders).
- Surface these via `QueryError`.

**Stage 4: query upgrades (future).**
- Make `next_position` and `explain_position` aware of parameterized defer
  entries. When source binders are bound concretely, evaluate target args.
- Allow `locate_action` and friends to traverse abstract transitions —
  e.g. "where does Press eventually take Counter?"

## Out of scope for this work

- Parameterized direction names (action-with-args). counter.poly doesn't
  exercise this; deferring until an example demands it.
- Multiple defer entries sharing a source position with different guards
  (the earlier `Count[n] if (n <= 10) -> Off, Count[n] if (n >= 11) -> On`
  draft). Tractable as an extension once the simple form lands; would require
  parser & lowering to allow repeated source-pos entries differentiated by
  source guards. Flag in stage 2 as a known follow-up.
