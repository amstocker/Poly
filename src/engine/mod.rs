pub mod fmt;
pub mod interner;
pub mod lower;
pub mod parse;
pub mod query;
pub mod types;

pub use interner::{Interner, Sym};
pub use types::*;

use std::collections::BTreeMap;


// ============================================================================
// Engine
// ============================================================================

#[derive(Clone, Debug, Default)]
pub struct Engine {
    pub interner: Interner,
    pub schemas: BTreeMap<Sym, Schema<Sym>>,
    pub interfaces: BTreeMap<Sym, Interface<Sym>>,
    pub defers: Vec<Defer<Sym>>,
}

impl Engine {
    pub fn new(interner: Interner, decls: Vec<Decl<Sym>>) -> Engine {
        let mut engine = Engine { interner, ..Engine::default() };
        for decl in decls {
            match decl {
                Decl::Schema(s) => { engine.schemas.insert(s.name, s); }
                Decl::Interface(i) => { engine.interfaces.insert(i.name, i); }
                Decl::Defer(d) => engine.defers.push(d),
            }
        }
        engine
    }

    pub fn load(src: &str) -> Result<Engine, Vec<chumsky::error::Simple<char>>> {
        use chumsky::Parser;
        let raw: Vec<Decl<String>> = parse::file().parse(src.to_string())?;
        let mut interner = Interner::new();
        let decls = lower::lower_decls(raw, &mut interner);
        Ok(Engine::new(interner, decls))
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        self.interner.resolve(sym)
    }
}
