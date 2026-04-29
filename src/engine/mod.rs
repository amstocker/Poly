pub mod eval;
pub mod facts;
pub mod fmt;
pub mod interner;
pub mod lower;
pub mod parse;
pub mod query;
pub mod types;
pub mod uquery;
pub mod validate;

pub use interner::{Interner, Sym};
pub use types::*;

use std::collections::BTreeMap;


// ============================================================================
// Engine load errors
// ============================================================================

#[derive(Debug)]
pub enum EngineError {
    Parse(Vec<chumsky::error::Simple<char>>),
    Validate(Vec<String>),
}


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

    pub fn load(src: &str) -> Result<Engine, EngineError> {
        use chumsky::Parser;
        let raw: Vec<Decl<String>> =
            parse::file().parse(src.to_string()).map_err(EngineError::Parse)?;
        let mut interner = Interner::new();
        let decls = lower::lower_decls(raw, &mut interner);
        let engine = Engine::new(interner, decls);
        let errors = engine.validate();
        if !errors.is_empty() {
            let formatted: Vec<String> =
                errors.iter().map(|e| engine.fmt_validation_error(e)).collect();
            return Err(EngineError::Validate(formatted));
        }
        Ok(engine)
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        self.interner.resolve(sym)
    }
}
