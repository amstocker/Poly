# Wiring diagrams for distributed systems

A survey of traditions that use "wiring diagrams" — broadly construed: boxes with
ports, nodes with channels, places with links — to describe distributed and
concurrent systems. Each tradition makes different commitments about what the
*nodes* are, what the *wires* are, and what the wiring algebra *is*. Read with
Poly's design space in mind: where the categorical framing is shared, what is
borrowable, where the traditions diverge in ways that matter for an agentic
runtime.

---

## 1. Operadic wiring diagrams (Spivak; Vagner–Spivak–Lerman)

**Atom:** a black box with input and output ports.
**Wire:** an identification of one box's output port with another's input port.
**Algebra:** wiring diagrams are morphisms of an operad `O(W)`; composition is
hierarchical nesting. A semantics is an `O(W)`-algebra assigning to each box a
mathematical system (e.g. a system of ODEs, a state machine, a database schema).

Spivak's 2013 paper introduced the operad for databases, recursion, and
plug-and-play circuits. Vagner–Spivak–Lerman (2015) extended it to open
dynamical systems, defining two algebras `G` (general systems) and `L` (linear
systems) over the same operad. The operad framing is what gives "compose two
sub-systems to get a system" its precise meaning: it is operad multiplication.

**Closest tradition to Poly.** The polynomial-functor formalism inherits the
"boxes-with-ports" intuition and replaces "set of input/output ports" with
"polynomial = position with a direction set per position." Wiring becomes a
*lens* rather than a bare wire-identification.

References:
- Spivak, *The operad of wiring diagrams* (2013), arXiv:1305.0297
- Vagner, Spivak, Lerman, *Algebras of open dynamical systems on the operad of
  wiring diagrams* (2015), arXiv:1408.1598

## 2. Polynomial functors (Niu–Spivak; Poly)

**Atom:** a polynomial `p = Σᵢ y^{p[i]}` — a set of positions, each tagged with
a set of directions.
**Wire:** a polynomial lens — forward map on positions, backward map on directions.
**Algebra:** the category `Poly` is closed monoidal under several products
(`+`, `×`, `⊗`, `◁`). Wiring diagrams in Niu–Spivak §4.3.2 are exactly lenses
of the form `(⊗ᵢ pᵢ) → q` — the tensor of *interior* interfaces routed to a
single *exterior* interface.

The lens form is structurally richer than bare wiring: the backward map says
*which inner directions implement each outer direction*, capturing
synchronization and joint action, not just passive plumbing.

This is the formalism Poly implements. The current Poly inverts the
section-4.3.2 order: it is *composite-first* (define the joint interface, then
project to views via defers), whereas the wiring-diagram framing is
*parts-first* (define the parts, then say how they are wired to form a composite).

References:
- Niu, Spivak, *Polynomial Functors: A Mathematical Theory of Interaction*,
  arXiv:2312.00990 — the canonical reference Poly is built from.
- Spivak, *Poly: an abundant categorical setting for mode-dependent dynamics*,
  arXiv:2005.01894.

## 3. Bigraphs (Milner)

**Atom:** a node with a *control* (a sort).
**Wire:** two independent graph structures over the same node set —
  - *place graph* (forest): nesting / containment / locality.
  - *link graph* (hypergraph): channels / names connecting node ports.
**Algebra:** bigraphical reactive systems — a set of reaction rules rewrites
bigraphs. Gives a *behavioural* semantics for free.

Designed specifically to model *ubiquitous and distributed* computing: the key
move is that **place and link are independent** — a node may be physically
nested inside one parent while its links cross arbitrarily many boundaries,
exactly mirroring how wireless links route around spatial structure. Milner
proposed bigraphs as a "Ubiquitous Abstract Machine," a successor to π-calculus
for distributed concurrency.

**Lesson for Poly:** Poly's current model conflates "where a thing is" with
"what it can do." A bigraph-style split — interfaces describe *what can happen*,
a separate place structure says *where the agents are* — may be worth
considering if Poly grows to model physical or organizational locality.

References:
- Milner, *The Space and Motion of Communicating Agents* (2009 monograph;
  draft https://www.cl.cam.ac.uk/archive/rm135/Bigraphs-draft.pdf)
- Milner, *Bigraphical reactive systems: basic theory*, UCAM-CL-TR-523.

## 4. String diagrams in symmetric monoidal categories

**Atom:** a morphism `f : A → B` in a symmetric monoidal category.
**Wire:** an object — a type carried along the wire.
**Algebra:** the SMC laws — sequential composition `;`, parallel composition `⊗`,
symmetry `σ`, plus any extra structure (compact closed, traced, hypergraph,…).

This is the most general graphical calculus, and most of the other traditions
on this list factor through it. Concurrency interpretations are well developed:
*Mazurkiewicz trace languages* coincide with symmetric monoidal languages over
"monoidal distributed alphabets," and Zielonka's asynchronous automata coincide
with the corresponding monoidal automata. So the SMC layer is where
*concurrency-as-permutation-of-independent-events* lives natively.

**Lesson for Poly:** the tensor `⊗` is exactly what the `(A, B)` notation in
`examples/tensor.poly` is reaching for. Promoting `⊗` to first-class status —
so wiring diagrams are lenses `(⊗ᵢ Aᵢ) → B` rather than only single-source
defers — aligns Poly with the standard SMC string-diagram view.

References:
- Piedeleu & Zanasi, *An Introduction to String Diagrams for Computer Scientists*
  (Cambridge Elements, 2023).
- Bonchi et al., *String Diagrammatic Trace Theory*, arXiv:2306.16341 — direct
  link between SMCs and Mazurkiewicz / Zielonka concurrency.

## 5. Petri nets — compositional and open

**Atom:** a *place* (state) or a *transition* (event); a bipartite graph
connecting them. Tokens occupy places; transitions consume/produce tokens.
**Wire:** an arc with a multiplicity, or — in the compositional framing —
a *boundary place* through which two nets glue.
**Algebra:** Baez–Master *open Petri nets*: a symmetric monoidal double
category whose 1-cells are nets equipped with open interfaces. Composition is
gluing along shared boundary places. Bonchi–Sobociński–Zanasi *resource
calculus* gives a string-diagrammatic syntax for Petri nets, sharing the
calculus used for signal-flow graphs.

Petri nets commit to a *resource / token* reading of state: state is a
multiset over places, and a transition is fundamentally about *consumption and
production*. This is a different commitment than Poly's position / direction
reading.

**Lesson for Poly:** worth thinking about whether some configurations are
better expressed as tokens-in-places than as joint-positions. If multiple
identical agents share a workflow, a Petri-style multiset view collapses the
combinatorial blowup that pure joint-positions incur.

References:
- Baez, Master, *Open Petri Nets* (2018), arXiv:1808.05415.
- Bonchi, Sobociński, Zanasi, *Diagrammatic Algebra: From Linear to
  Concurrent Systems* (POPL 2019).

## 6. Process calculi (CCS, CSP, π-calculus)

**Atom:** a process expression.
**Wire:** a channel / action name. In π-calculus, channel names are *first-class
data values* — they can be sent over channels and used to rewire the system at
runtime.
**Algebra:** structural operational semantics over an algebraic syntax. Parallel
composition `P | Q` is primitive; communication is by name-matching rendezvous
(CCS), event-set synchronization (CSP), or channel passing (π).

Strictly speaking, process calculi are *syntactic* rather than diagrammatic,
but the channel structure is the wiring: a CCS network drawn as boxes connected
by labelled channels is functionally a wiring diagram.

**Lesson for Poly:** the user has already flagged CCS-style name-matching as
the natural synchronization-inference mechanism for `examples/tensor.poly`. The
π-calculus extension — channels as first-class values that can be passed at
runtime — is worth considering if Poly ever wants *dynamic wiring* (agents
rewiring the system as it runs), not just static schemas.

References:
- Milner, *Communication and Concurrency* (CCS, 1989).
- Hoare, *Communicating Sequential Processes* (CSP, 1985).
- Milner, Parrow, Walker, *A Calculus of Mobile Processes* (π, 1992).

## 7. Reo (channel-based coordination)

**Atom:** a *component* (opaque process) or a *channel* (typed wire).
**Wire:** channels are first-class, *with behavior* — `Sync` (atomic pass-through),
`LossySync` (drops on backpressure), `FIFO(n)` (buffered), etc.
**Algebra:** *connectors* are compositional graphs of channels (a labelled
directed hypergraph). Coordination is *exogenous*: components know nothing of
each other; all coordination logic lives in the wiring.

A distinguishing feature of Reo: the channels are not passive plumbing — they
encode the coordination protocol itself. Buffer sizes, synchronicity, lossiness
are all wired in. The Dreams framework distributes the connector across nodes,
showing the wiring itself can be physically distributed without a central
arbiter.

**Lesson for Poly:** Reo's "components-as-black-boxes + wiring-is-coordination"
is structurally the same agenda as Poly's. The two differences worth flagging:
(a) Reo gives channels rich types (synchrony, buffering); Poly currently treats
all directions uniformly. (b) Reo has decades of tooling for *distributed
execution* of the coordination layer itself — a useful prior art if Poly ever
needs to run agents across machines.

References:
- Arbab, *Reo: a channel-based coordination model for component composition*
  (Math. Struct. Comp. Sci., 2004).
- Wikipedia: *Reo Coordination Language*.

## 8. Flow-based programming (Morrison)

**Atom:** a *component* (process) with *named ports*.
**Wire:** a *connection* — a bounded buffer between an output port and an input
port. Information packets (IPs) flow through.
**Algebra:** networks are flat graphs; there is no formal operad / categorical
structure, but practical decomposability is good (Morrison: "draw it broadly,
subdivide afterwards").

The engineering tradition rather than the mathematical one — predates the
categorical work by decades (Morrison, ~1970, at IBM Canada). Worth knowing
because it nails certain practical decisions: named ports, bounded buffers
forcing backpressure, parallelism as the default, components reusable across
networks.

**Lesson for Poly:** if Poly ever wants a *running* system rather than a
schema-and-query layer, the FBP design choices are the well-trodden path:
named ports, bounded buffers, backpressure, late-bound network configuration
files separate from component code.

References:
- Morrison, *Flow-Based Programming* (1994/2010 book; site
  https://jpaulm.github.io/fbp/).

---

## Cross-cutting design axes

The traditions above can be sorted along a few axes that matter for Poly's
design space:

| Axis | One end | Other end |
|---|---|---|
| Wires carry | identity-of-port | typed behavior |
| Composition is | hierarchical (operadic, nesting) | flat (gluing, parallel) |
| Coordination is | endogenous (in the components) | exogenous (in the wiring) |
| Wires are | static (schema) | dynamic (rewireable at runtime) |
| State semantics | position (poly) | resource / token (Petri) |
| Locality is | implicit in wiring | a separate dimension (bigraph) |
| Synchronization is | named rendezvous (CCS, CSP, Reo `Sync`) | sequential / async (FBP) |

Poly is currently: operadic-ish wires-carry-direction-sets, hierarchical
composition via lenses, *both* endogenous (positions hold local action sets)
*and* potentially exogenous (lenses can rewire), static, position-based, no
locality dimension, sync semantics implicit but un-inferred.

The two most actionable shifts visible from this survey:

1. **Promote `⊗` to first-class** (the user's `(A, B)` notation). Brings Poly
   in line with the SMC string-diagram tradition, and brings parts-first
   wiring-diagram thinking (Niu–Spivak §4.3.2) into the surface syntax.
2. **Pick an explicit synchronization rule.** All the rendezvous-based
   traditions (CCS, CSP, Reo `Sync`) make this an *explicit* design choice
   rather than something the user reads off ad-hoc. The lens backward-map in
   §2 is already half of this — it identifies an outer direction with a
   *tuple* of inner directions — but Poly doesn't yet declare *what* the
   composition rule is when several lenses act on overlapping inner positions.

A third, longer-horizon shift if Poly ever wants to model agents-with-physical-
location or organizational nesting: **adopt the place/link split from bigraphs.**
Worth deferring until a use case demands it.

## Sources

- [Spivak (2013): The operad of wiring diagrams](https://arxiv.org/abs/1305.0297)
- [Vagner, Spivak, Lerman (2015): Algebras of Open Dynamical Systems on the Operad of Wiring Diagrams](https://arxiv.org/abs/1408.1598)
- [Niu, Spivak: Polynomial Functors](https://arxiv.org/pdf/2312.00990)
- [Spivak (2020): Poly — An abundant categorical setting for mode-dependent dynamics](https://arxiv.org/abs/2005.01894)
- [Milner: The Space and Motion of Communicating Agents (draft)](https://www.cl.cam.ac.uk/archive/rm135/Bigraphs-draft.pdf)
- [Milner: Bigraphical reactive systems: basic theory (UCAM-CL-TR-523)](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-523.pdf)
- [Wikipedia: Bigraph](https://en.wikipedia.org/wiki/Bigraph)
- [Piedeleu, Zanasi: An Introduction to String Diagrams for Computer Scientists](https://www.cambridge.org/core/elements/an-introduction-to-string-diagrams-for-computer-scientists/3CDAF8F57D2299F0EACA3354E9757CFD)
- [Bonchi et al.: String Diagrammatic Trace Theory](https://arxiv.org/html/2306.16341)
- [Baez: Open Petri Nets](https://math.ucr.edu/home/baez/petri.pdf)
- [Bonchi, Sobociński, Zanasi: Diagrammatic Algebra — From Linear to Concurrent Systems](https://dl.acm.org/doi/10.1145/3290338)
- [Arbab: Reo — a channel-based coordination model](https://homepages.cwi.nl/~farhad/MSCS03Reo.pdf)
- [Wikipedia: Reo Coordination Language](https://en.wikipedia.org/wiki/Reo_Coordination_Language)
- [Morrison: Flow-Based Programming](https://jpaulm.github.io/fbp/)
- [Wikipedia: Flow-based programming](https://en.wikipedia.org/wiki/Flow-based_programming)
- [Topos Institute: Neural wiring diagrams for message passing in multiscale organizations](https://topos.institute/blog/2024-11-08-neural-wiring-diagrams/)
