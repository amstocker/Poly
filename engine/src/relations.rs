// Iterator helpers projecting the loaded program as flat relations.
//
// These replace the old `facts::Facts` denormalized cache: instead of
// projecting once into a Vec-per-relation, we walk the nested AST on
// demand. At Poly's current scale (a handful of interfaces) the cost is
// negligible, and the consolidation removes a parallel set of tuple types
// (`PositionFact`, `DirectionFact`, …) that duplicated the AST.

use crate::types::{
    Defer, DeferEntry, Direction, DirMapping, Interface, Param, Position, SchemaBody, Variant,
};
use crate::{Engine, Sym};

impl Engine {
    /// Every loaded interface.
    pub fn iface_relation(&self) -> impl Iterator<Item = &Interface<Sym>> {
        self.interfaces.values()
    }

    /// `(internal, external)` pairs derived from the `::Internal`
    /// suffix convention. An interface named `Foo::Internal` is paired
    /// with `Foo` iff `Foo` is also a loaded interface.
    pub fn iface_internal_relation(&self) -> impl Iterator<Item = (Sym, Sym)> + '_ {
        self.index.internal_to_external.iter().map(|(&int, &ext)| (int, ext))
    }

    /// `(schema_name, fields)` pairs for record-shaped schemas.
    pub fn schema_record_relation(&self) -> impl Iterator<Item = (Sym, &[Param<Sym>])> {
        self.schemas.values().filter_map(|s| match &s.body {
            SchemaBody::Record(fields) => Some((s.name, fields.as_slice())),
            SchemaBody::Sum(_) => None,
        })
    }

    /// `(schema_name, variants)` pairs for sum-shaped schemas.
    pub fn schema_sum_relation(&self) -> impl Iterator<Item = (Sym, &[Variant<Sym>])> {
        self.schemas.values().filter_map(|s| match &s.body {
            SchemaBody::Sum(variants) => Some((s.name, variants.as_slice())),
            SchemaBody::Record(_) => None,
        })
    }

    /// Every (iface, position) pair.
    pub fn position_relation(&self) -> impl Iterator<Item = (Sym, &Position<Sym>)> {
        self.interfaces
            .values()
            .flat_map(|iface| iface.positions.iter().map(move |p| (iface.name, p)))
    }

    /// Every (iface, position_name, direction) triple.
    pub fn direction_relation(&self) -> impl Iterator<Item = (Sym, Sym, &Direction<Sym>)> {
        self.interfaces.values().flat_map(|iface| {
            iface.positions.iter().flat_map(move |pos| {
                pos.directions.iter().map(move |dir| (iface.name, pos.name, dir))
            })
        })
    }

    /// Every defer declaration.
    pub fn defer_relation(&self) -> impl Iterator<Item = &Defer<Sym>> {
        self.defers.iter()
    }

    /// Every (defer_name, entry_idx, entry) triple.
    pub fn defer_entry_relation(&self) -> impl Iterator<Item = (Sym, usize, &DeferEntry<Sym>)> {
        self.defers.iter().flat_map(|d| {
            d.entries.iter().enumerate().map(move |(idx, e)| (d.name, idx, e))
        })
    }

    /// Every (defer_name, entry_idx, dir_mapping) triple.
    pub fn defer_dir_relation(&self) -> impl Iterator<Item = (Sym, usize, &DirMapping<Sym>)> {
        self.defers.iter().flat_map(|d| {
            d.entries.iter().enumerate().flat_map(move |(idx, e)| {
                e.directions.iter().map(move |m| (d.name, idx, m))
            })
        })
    }
}
