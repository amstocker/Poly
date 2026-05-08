// Source → validated AST: the three sequential steps `Engine::load`
// runs.
//
//   1. `parse::file()`         — chumsky grammar; sugar rewrite
//   2. `lower::lower_decls()`  — Decl<String> → Decl<Sym>
//   3. `Engine::validate()`    — defer arity + abstract-ref checks
//
// The submodules are `pub(crate)` so other parts of the engine (only
// `engine.rs` today) can call into them; consumers of the library
// don't see this layer.

pub(crate) mod lower;
pub(crate) mod parse;
pub(crate) mod validate;
