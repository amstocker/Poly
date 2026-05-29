use std::collections::{BTreeMap, HashMap};

use crate::interner::Interner;
use crate::parsing::{lower, parse};
use crate::types::{Decl, Defer, Interface, Schema};
use crate::Sym;


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

pub(crate) const INTERNAL_SUFFIX: &str = "::Internal";

/// Lookup tables built once at engine construction. Replace what would
/// otherwise be linear scans inside the query hot paths (BFS over defers,
/// realization-defer lookup per `Goal::Step`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Index {
    /// External iface name → its `::Internal` carrier, when both are loaded.
    pub(crate) external_to_internal: HashMap<Sym, Sym>,
    /// Inverse of `external_to_internal` — drives `iface_internal_relation`.
    pub(crate) internal_to_external: HashMap<Sym, Sym>,
    /// External iface → index into `Engine::defers` of the realization defer
    /// (the unique defer with `source = I::Internal, target = I`).
    pub(crate) realization_for_iface: HashMap<Sym, usize>,
    /// `(iface, pos) → reachable (iface', pos')` for one Forward defer hop.
    pub(crate) defer_forward: HashMap<(Sym, Sym), Vec<(Sym, Sym)>>,
    /// Same, reversed: each entry contributes `(target, target_pos) →
    /// (source, source_pos)`.
    pub(crate) defer_backward: HashMap<(Sym, Sym), Vec<(Sym, Sym)>>,
}

/// Top-level declaration kinds for duplicate-name reporting. Populated by
/// `Engine::new` when `BTreeMap::insert` clobbers an earlier binding (or two
/// defers share a name); surfaced as `ValidationError::DuplicateName` by
/// `validate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    Schema,
    Interface,
    Defer,
}

#[derive(Clone, Debug, Default)]
pub struct Engine {
    pub interner: Interner,
    pub schemas: BTreeMap<Sym, Schema<Sym>>,
    pub interfaces: BTreeMap<Sym, Interface<Sym>>,
    pub defers: Vec<Defer<Sym>>,
    pub(crate) index: Index,
    /// `(kind, name)` pairs detected during construction where a later decl
    /// shadowed an earlier one. Drained by `validate`.
    pub(crate) duplicate_decls: Vec<(DeclKind, Sym)>,
}

impl Engine {
    /// Construct an `Engine` from a pre-lowered decl list and the matching
    /// interner. `pub(crate)` because callers can't independently produce
    /// `Decl<Sym>` (the `Sym` type has no public constructor); the public
    /// path is `Engine::load`, which keeps the interner ↔ decls invariant.
    pub(crate) fn new(interner: Interner, decls: Vec<Decl<Sym>>) -> Engine {
        let mut engine = Engine { interner, ..Engine::default() };
        let mut seen_defers: std::collections::HashSet<Sym> = std::collections::HashSet::new();
        for decl in decls {
            match decl {
                Decl::Schema(s) => {
                    let name = s.name;
                    if engine.schemas.insert(name, s).is_some() {
                        engine.duplicate_decls.push((DeclKind::Schema, name));
                    }
                }
                Decl::Interface(i) => {
                    let name = i.name;
                    if engine.interfaces.insert(name, i).is_some() {
                        engine.duplicate_decls.push((DeclKind::Interface, name));
                    }
                }
                Decl::Defer(d) => {
                    if !seen_defers.insert(d.name) {
                        engine.duplicate_decls.push((DeclKind::Defer, d.name));
                    }
                    engine.defers.push(d);
                }
            }
        }
        engine.index = engine.build_index();
        engine
    }

    fn build_index(&self) -> Index {
        let mut idx = Index::default();
        // External ↔ internal pairing: name-suffix convention. Same logic as
        // `iface_internal_relation` but materialized.
        for iface in self.interfaces.values() {
            let name = self.interner.resolve(iface.name);
            let Some(stripped) = name.strip_suffix(INTERNAL_SUFFIX) else { continue };
            let Some(ext_sym) = self.interner.find(stripped) else { continue };
            if !self.interfaces.contains_key(&ext_sym) { continue }
            idx.internal_to_external.insert(iface.name, ext_sym);
            idx.external_to_internal.insert(ext_sym, iface.name);
        }
        // Realization defer per external iface and adjacency lists from defer
        // entries.
        for (defer_idx, d) in self.defers.iter().enumerate() {
            if let Some(ext) = idx.internal_to_external.get(&d.source) {
                if d.target == *ext {
                    idx.realization_for_iface.insert(*ext, defer_idx);
                }
            }
            for entry in &d.entries {
                idx.defer_forward
                    .entry((d.source, entry.source_pos))
                    .or_default()
                    .push((d.target, entry.target_pos));
                idx.defer_backward
                    .entry((d.target, entry.target_pos))
                    .or_default()
                    .push((d.source, entry.source_pos));
            }
        }
        idx
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
