# 2026-05-08 — Runtime, configuration layer sketch

Date: 2026-05-08
Status: design sketch — pre-implementation, exploring the problem
Reads alongside: `architecture.md` "working hypothesis", `roadmap.md`
"Vision-layer items", `2026-05-07-embedding-api.md`

## Framing

The engine is now stable enough that the next layer is the **runtime** —
the thing that holds live state and dispatches agents. Vision-layer notes
have been consistent on the high-level shape:

- The engine stays **stateless**. State is passed in.
- A separate Rust process imports `poly-engine` as a crate and owns the
  live state.
- Three layers, of which the engine implements one: **schema** (static
  type, built), **configuration** (live realized state, this doc),
  **trajectory** (append-only history, next).

This log focuses on **configuration** because trajectory and the agent
surface both presuppose it. Configuration is the smallest piece that
gives "the system has a current state" semantics; everything else
(history, agent actions, permission scopes) layers on top.

The aim is to make the design space legible, not to commit. Several
choices below are flagged explicitly as open.

## Categorical framing

A schema declares a polynomial $p = \Sigma_i y^{p[i]}$ (per interface),
plus lenses $D : p \to q$ between interfaces (defers). A **configuration**
is, roughly, a *section* of this structure:

- For each interface $I$ in scope: a chosen position $i \in I$, with
  concrete values for its position parameters.
- For each defer $D : A \to B$ between in-scope interfaces: the chosen
  positions must be coherent under $D$'s forward map. I.e. if $A$ is at
  $a$, then $B$ is at $D_{\mathrm{pos}}(a)$.

So a configuration is *not* a free choice across interfaces; the defer
network constrains it. In the polynomial-functor literature this is the
natural notion: a configuration of a composed system is a section of the
composite polynomial.

There is a degree of freedom in how we present it:

1. **Free + checked.** Configuration is a free assignment (one position
   per interface) and the runtime validates coherence under defers on
   every change.
2. **Quotient.** Configuration is the assignment of positions to a set of
   "free" interfaces (roots of the defer DAG), and positions of derived
   interfaces are *computed* by walking defers.
3. **Hybrid.** Stored at the granularity the user finds useful, derived
   on read. (Probably what falls out in practice.)

(2) is mathematically tightest and avoids storing redundant state. (1) is
operationally simplest. Worth explicitly choosing once we have a real
example with overlapping defer coverage.

## What needs to be in a configuration

Putting aside *how* it's stored, a configuration must answer at least:

- For interface $I$ (or instance thereof — see "Instances" below): what
  position is it at, with what argument values?
- For position $P$ at interface $I$ with concrete args: are the position
  guards satisfied? (The simplifier already handles this — a coherent
  configuration is one where every position guard reduces to true.)
- For directions out of the current position: are the direction guards
  satisfied? (This is *the* query the agent surface needs: "what can I
  do right now?" — already expressible via existing `Goal::Direction`
  composed with `Goal::Position`.)

The point worth stressing: **the engine already speaks this language.**
Configuration is, structurally, a populated `Bindings` plus a chosen
`(iface, position, args)` per instance. The runtime is the thing that
*owns* and *mutates* the current configuration; the engine is the thing
that *answers questions* about it.

## Instances vs singletons

Open question, not yet forced:

- **Singleton interpretation.** A schema declares a single fixed system;
  each interface has exactly one position-state at a time. Simpler.
  Matches the React analogy at the top level (one rendered tree).
- **Instance interpretation.** Each interface can be instantiated zero or
  more times; the configuration is a multiset of `(instance_id,
  iface, position, args)`. Necessary for "many counters" or "agents as
  interfaces."

The agent-tool surface (item 7 in `project_poly_vision.md`) implicitly
assumes instances — "list interfaces" only makes sense if there are
multiple. The "agent is an interface" framing forces it: every running
agent is an instance.

Probably commit to instances early. Defer the schema-level question of
"is this interface single-instance or multi-instance" until an example
forces it.

## Operations the runtime exposes

Working list. Not the public API yet — sketching what an agent actually
needs:

- **`current() -> Configuration`** — read the live configuration.
- **`enabled(instance_id) -> Vec<Action>`** — directions whose guards
  reduce to true at the instance's current position. Built on
  `Engine::query`.
- **`propose(instance_id, action, args?) -> Result<Configuration,
  Violation>`** — attempt to take an action. Returns the next
  configuration if the schema admits the transition; otherwise a
  structured `Violation`. This is where "bounded latitude" becomes
  operational.
- **`spawn(iface, args?) -> instance_id`** — instantiate a new interface
  in the configuration.
- **`despawn(instance_id)`** — remove. (Or: lifetime is governed by
  schema-level rules eventually; out of scope for v0.)
- **`query(query, config_env) -> Answers`** — pass-through to
  `Engine::query` with the live configuration folded into the env. The
  agent's read interface to the world.

Worth noting that `propose` is mostly a thin wrapper around the existing
`Goal::Step`: the runtime asks the engine "what's the destination of this
action from here?", checks the residual reduces to true under the live
env, and commits if it does. The engine already does the math; the
runtime is bookkeeping.

## Validation as a query

The runtime never re-implements schema semantics. Every check is a query:

- "Is configuration C coherent?" — for each defer-linked pair, ask
  whether the derived position matches; for each instance, ask whether
  the position guard holds under the args.
- "Is action A valid at instance I?" — `Goal::Step` from the current
  position, action = A; accept if any answer survives with `true`
  residual under the live env.
- "What's the resulting state?" — same query, read the destination.

This keeps the runtime small and the engine the single source of truth
for what is and isn't allowed by the schema.

## What changes in the engine

Probably nothing, in v0. The named goals already cover the queries the
runtime needs. Things to watch for:

- **Bindings ergonomics.** The runtime will fold a lot of the live config
  into the env. If the same `Bindings` is reused across many queries,
  there may be a case for pre-interning or otherwise caching. Premature
  to optimize.
- **Position identity.** Today positions are `(iface, position_name,
  args)`. The runtime needs to attach `instance_id` somewhere — almost
  certainly outside the engine, in a runtime-side `Instance` struct. The
  engine shouldn't grow an instance concept.
- **Trajectory hooks.** When `propose` accepts, something needs to log.
  The runtime owns the log. The engine doesn't.

## Things explicitly *not* in this sketch

- **Trajectory layer.** Append-only history, replay, time-travel queries.
  Next log, not this one.
- **Multi-agent concurrency.** Two agents proposing simultaneously. v0
  is single-writer; concurrency control lives above (locks, queues,
  optimistic with retry). Worth its own design discussion.
- **Permission / privilege scoping.** Who can propose what. Lives above
  the runtime; the runtime doesn't need to know.
- **Schema-driven instance lifecycle.** "This interface can be
  instantiated up to N times" or "this interface must always have at
  least one instance." Schema-extension territory; defer.
- **The agent-tool surface itself.** What the LLM's tool-call shape
  looks like. Designed once the runtime ops are stable.

## Open questions to chew on

1. **Free vs quotient configuration.** Pick one, or admit hybrid is
   inevitable. Probably wants a concrete example with two overlapping
   defers before deciding.
2. **Singleton vs instances.** Lean instances; confirm against a real
   use case before committing.
3. **Configuration as a value vs as a database.** A small system can
   hand around `Configuration` by value. A larger one wants persistent
   storage and incremental updates. v0 should probably be a value;
   structure it so a database backend slots in later.
4. **What does `propose` return on success?** The whole new config? A
   diff? A trajectory entry? Probably the trajectory entry, with
   `current()` re-derived; defer until trajectory layer is sketched.
5. **Where does the schema live at runtime?** The `Engine` already owns
   it. The `Runtime` probably owns an `Engine` and adds the
   configuration on top. One process, one schema, one configuration —
   for now.
6. **Coherence on schema reload.** If the schema changes while a
   configuration is live, what happens? Out of scope for v0; loud error
   probably suffices.

## Next steps

Not committing to a build order. Possible directions in roughly
ascending commitment:

- **(a) Sit with this.** Rewrite once the framing settles. Probably
  worth sharpening the categorical story (what *is* a configuration
  in poly-functor terms, precisely) before writing any code.
- **(b) Write a tiny example by hand.** Take `examples/counter.poly`,
  describe its configuration space and a few transitions in prose,
  see if the data-shape questions answer themselves.
- **(c) Draft a `runtime/` crate skeleton.** New workspace member. No
  ops yet; just `Runtime` struct holding `Engine + Configuration` and a
  `current()` reader. Forces the data-shape question concretely.
- **(d) Build `enabled()` first.** It's the smallest op that's actually
  useful — given a config, what can the agent do? Pure read; no
  mutation; exercises the engine-as-validator pattern.

(a) and (b) cost nothing. (c) and (d) want the open questions at least
provisionally answered first.
