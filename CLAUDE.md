# Poly

## What this is

Poly is a Rust-implemented language/runtime for describing **agentic systems** as composed polynomial-functor interfaces. An interface is a polynomial `p = Σ_i y^{p[i]}` (positions/states with per-position direction sets / actions). Composition is via `defer` — a polynomial lens (forward map on positions, backward map on directions).

**Project mode is exploratory research, not product development.** The user has a math PhD and is genuinely enjoying the categorical analysis. Restarts and rethinks are normal output, not failures. Default to ideas/sketches/math-connections; don't push shipping pressure.

**Working hypothesis (sharpened over multiple sessions):** Poly is a *typed live knowledge base* for agentic systems — Datalog-flavored queries against a polynomial schema. Three layers worth keeping distinct: schema (the static type), configuration (the live realized state), trajectory (the append-only history). Agents and humans both read/write the same artifact. The polynomial-functor framing earns its keep by giving "structured non-determinism" / "bounded latitude": a schema declares a *space of possible systems*; agents operate freely within that space; the runtime mechanically enforces the boundary.

Reference: https://arxiv.org/pdf/2312.00990

## Repo layout

- `src/engine/` — **the active engine.** Start here.
  - `mod.rs` — typed model (`Interface`, `Defer`, `Engine`, `Decl`), queries, `Display` impls.
  - `parse.rs` — chumsky parser for the implemented language subset, plus state-machine-sugar desugaring.
- `src/diagram/`, `src/diagram_old/`, `src/diagram_old2/` — earlier algebraic primitives (terms, transforms, paths, constructors, BFS query). Kept for reference but not on the engine path. Don't edit unless asked.
- `src/test/mod.rs` — `logru` (Prolog-style) experimentation. Possibly load-bearing later if Poly grows a Datalog query layer.
- `src/object.rs` — stub, unused.
- `src/main.rs` — driver. `cargo run` walks both implemented examples; `cargo run -- <path>` targets a specific file.
- `examples/implemented/` — `.poly` files inside the implemented subset (currently `test2.poly`, `graph.poly`).
- `examples/*.poly` — sketches outside the implemented subset (parameterized states, guards, schemas, agents, Kleene). Source of truth for *intended* syntax, not all parseable yet.
- `examples/01_tool_use/` — exploratory `tool` / `agent` block sketches; design-only, not parseable.
- `planning/` — design notes. New session notes go here.

## Implemented language subset

What the parser currently accepts:

- **`interface Name` block**: comma-separated positions, each `Name` or `Name { Dir, Dir, ... }`.
- **State-machine sugar**: any direction may carry a transition `Action -> NextPos`. If any direction has a transition, the interface is desugared into three declarations:
  1. **`Name`** — the *external* interface. Positions = declared states; directions = action names (transitions stripped).
  2. **`Name.internal`** — the *universal state machine* polynomial on the same states. At each state `s`, directions are `{s=>t : t ∈ states}` — every conceivable outgoing transition. This is `S · y^S` where S is the state set.
  3. **`Name.run : Name.internal -> Name`** — the realization defer. Identity on positions; at each state `s`, each declared action `a -> dest` is realized as the abstract direction `s=>dest` in `Name.internal[s]`.
- **`defer Name : A -> B` block**: comma-separated position mappings `SrcPos -> TgtPos { TgtDir | TgtDir -> SrcDir, ... }`.
- **Line comments**: `# ...` to end of line. Stripped in a pre-pass before parsing.

What the parser does **not** yet accept (sketched in `examples/`, not implemented):

- Parameterized positions: `Count[n: Int]`, `Cell[i: Int, j: Int]`.
- Guards: `if (n > 0)` or `(n > 0)` prefix.
- `schema` (algebraic data types).
- `external` keyword.
- `agent` blocks with prompt templates.
- Kleene `*` for variable arity.
- `view` is deprecated, do not reintroduce.

## Engine queries (current)

- `explain_position(interface, position)` — *(Q1)* if I is at S, what does that determine about every other interface connected by a defer? Walks every defer touching I, projecting forward and backward.
- `locate_action(action)` — *(Q3)* every `(interface, position)` pair where `action` is a direction.
- *(Q2 unimplemented)* "given current position + action, what's the next position" — needs the state-machine semantics in the queries themselves; for now, the same information falls out by inspecting `Foo.run`'s `dir_map`.
- *(future)* Pattern-matching / unification-style query that subsumes Q1/Q2/Q3. Datalog-shaped; should be designed *after* the explicit queries make their needs visible, not before.

## Naming conventions for sugar-derived names

- External interface keeps the user-given name: `Graph`.
- Internal universal-state-machine interface: `Graph.internal`.
- Realization defer: `Graph.run`.
- Dotted names are stored as plain strings; they're not currently parseable from user-typed defer references (the `text::ident` parser stops at the dot). User-typed code never writes them today, so this is fine for v0.

## Display impls

`Interface` and `Defer` have `Display` impls that mirror source syntax — printing them produces text that looks the way it would be written by hand. Use them in driver code (`println!("{iface}")`, `println!("{d}")`) instead of ad-hoc formatting.

## Dependencies

- `chumsky = "0.9.3"` — parser combinators.
- `im = "15.1.0"` — persistent collections (used by older `diagram` modules).
- `logru = "0.4.1"` — Prolog-style logic engine (experimentation only; possibly central later).

## Build / run

```sh
cargo build
cargo run                                     # walks examples/implemented/test2.poly + graph.poly
cargo run -- examples/implemented/graph.poly  # one specific file
```

## Working norms

- Active surface: `src/engine/`. Target it for changes; leave `diagram*` alone unless asked.
- Lean into the polynomial-functor framing — the user is fluent, and clarity in the math beats hand-wavy paraphrase.
- Everything composable should be **named** so traces are legible (auto-names like `Counter.run`).
- Small, working increments over speculative scaffolding. Restarts are normal — don't pile complexity onto an unsettled foundation.
- High-value artifact is often *prose-with-math-sketches* in `planning/`, not code. Suggest landing important conversation outcomes there.
- Don't optimize for human writability of the language; agents are a co-equal user. Uniform/regular grammar beats clever syntax.
- Open design questions to leave open until evidence forces an answer: data structures (interfaces vs parameter primitives vs sugar), parameter mutation semantics, multi-agent concurrency on shared state, exact shape of any future query language.

## Sibling repo (intuition source)

`/Users/andrew/Documents/Github/agent practice/` — six numbered Python projects (`01_tool_use` → `06_job_tracker`) the user is working through to build agentic-engineering intuition. When designing Poly features, reach for these for concrete examples, but don't treat them as a curriculum to complete in order.
