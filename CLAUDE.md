# Poly

## What this is

Poly is a Rust-implemented language/runtime for describing **agentic systems** as composed polynomial-functor interfaces. An interface is a polynomial `p = Σ_i y^{p[i]}` (positions/states with per-position direction sets / actions). Composition is via `defer` — a polynomial lens (forward map on positions, backward map on directions).

**Project mode is exploratory research, not product development.** The user has a math PhD and is genuinely enjoying the categorical analysis. Restarts and rethinks are normal output, not failures. Default to ideas/sketches/math-connections; don't push shipping pressure.

**Working hypothesis (sharpened over multiple sessions):** Poly is a *typed live knowledge base* for agentic systems — Datalog-flavored queries against a polynomial schema. Three layers worth keeping distinct: schema (the static type), configuration (the live realized state), trajectory (the append-only history). Agents and humans both read/write the same artifact. The polynomial-functor framing earns its keep by giving "structured non-determinism" / "bounded latitude": a schema declares a *space of possible systems*; agents operate freely within that space; the runtime mechanically enforces the boundary.

Reference: https://arxiv.org/pdf/2312.00990

## Where to look

- `planning/architecture.md` — current engine state (source language, sugar, fact base, query layer, simplifier, CLI, module layout). Read before making engine changes.
- `planning/roadmap.md` — open work, deferred items, vision-layer ideas.
- `planning/log/` — per-session design notes. Add new ones here.

## Repo layout (one-liner)

Cargo workspace with two crates:

- `engine/` — `poly-engine` (lib). **The active surface.** Public API at `engine/src/api.rs` (named ops on `Engine` + result types); typed AST at `engine/src/types.rs`; everything else (`uquery`, `eval`, `simplify`, `parse`, `lower`, `validate`, `relations`, `fmt`, `interner`, `engine`) is `pub(crate)`.
- `cli/` — `poly` (bin). Thin renderer; no engine logic.
- `examples/*.poly` — currently `counter.poly`, `graph.poly`, `grid.poly`, `test2.poly`; all parseable.
- Only dependency: `chumsky` (parser combinators).

## Build / run

```sh
cargo build
cargo test
cargo run -- show examples/graph.poly
cargo run -- facts examples/test2.poly
cargo run -- query examples/graph.poly --explain Graph A
cargo run -- query examples/graph.poly --locate X
```

Running `cargo run` with no args prints CLI usage.

## Working norms

- Lean into the polynomial-functor framing — the user is fluent, and clarity in the math beats hand-wavy paraphrase.
- Everything composable should be **named** so traces are legible (auto-names like `Counter::Run`).
- Small, working increments over speculative scaffolding. Restarts are normal — don't pile complexity onto an unsettled foundation.
- High-value artifact is often *prose-with-math-sketches* in `planning/`, not code. Land important conversation outcomes in `planning/log/` and update `architecture.md`/`roadmap.md` if the steady-state shifted.
- Don't optimize for human writability of the language; agents are a co-equal user. Uniform/regular grammar beats clever syntax.
- Open design questions to leave open until evidence forces an answer: data structures (interfaces vs parameter primitives vs sugar), parameter mutation semantics, multi-agent concurrency on shared state, exact shape of any future query surface syntax.

## Sibling repo (intuition source)

`/Users/andrew/Documents/Github/agent practice/` — six numbered Python projects (`01_tool_use` → `06_job_tracker`) the user is working through to build agentic-engineering intuition. When designing Poly features, reach for these for concrete examples, but don't treat them as a curriculum to complete in order.
