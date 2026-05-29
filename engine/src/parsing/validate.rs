use std::collections::HashSet;

use crate::engine::DeclKind;
use crate::{Defer, DirRef, Engine, Interface, Param, Sym};


// ============================================================================
// Validation errors
// ============================================================================

/// Lexical scopes in which a duplicate name can occur. Used to format the
/// "name shadowed earlier definition" error contextually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateScope {
    TopLevel(DeclKind),
    PositionsInInterface(Sym),
    DirectionsInPosition { interface: Sym, position: Sym },
    InterfaceParams(Sym),
    PositionParams { interface: Sym, position: Sym },
    DirectionParams { interface: Sym, position: Sym, direction: Sym },
    SchemaRecordFields(Sym),
    SchemaSumVariants(Sym),
    VariantParams { schema: Sym, variant: Sym },
}

#[derive(Clone, Debug)]
pub enum ValidationError {
    UnknownInterface(Sym),
    DeferUnknownPosition { defer: Sym, interface: Sym, position: Sym },
    DeferPatternArity { defer: Sym, interface: Sym, position: Sym, expected: usize, got: usize },
    DeferTargetArity { defer: Sym, interface: Sym, position: Sym, expected: usize, got: usize },
    DirRefUnknown { defer: Sym, interface: Sym, position: Sym, name: Sym },
    DirRefAbstractNotPermitted { defer: Sym, interface: Sym },
    AbstractUnknownPos { defer: Sym, interface: Sym, position: Sym },
    AbstractArity { defer: Sym, interface: Sym, position: Sym, expected: usize, got: usize },
    DuplicateName { scope: DuplicateScope, name: Sym },
}


// ============================================================================
// Validator
// ============================================================================

impl Engine {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        for &(kind, name) in &self.duplicate_decls {
            errors.push(ValidationError::DuplicateName {
                scope: DuplicateScope::TopLevel(kind),
                name,
            });
        }
        for iface in self.interfaces.values() {
            self.validate_interface_names(iface, &mut errors);
        }
        for schema in self.schemas.values() {
            self.validate_schema_names(schema, &mut errors);
        }
        for d in &self.defers {
            self.validate_defer(d, &mut errors);
        }
        errors
    }

    fn validate_interface_names(
        &self,
        iface: &Interface<Sym>,
        errors: &mut Vec<ValidationError>,
    ) {
        check_param_dups(
            &iface.params,
            DuplicateScope::InterfaceParams(iface.name),
            errors,
        );
        let mut seen_pos: HashSet<Sym> = HashSet::new();
        for pos in &iface.positions {
            if !seen_pos.insert(pos.name) {
                errors.push(ValidationError::DuplicateName {
                    scope: DuplicateScope::PositionsInInterface(iface.name),
                    name: pos.name,
                });
            }
            check_param_dups(
                &pos.params,
                DuplicateScope::PositionParams { interface: iface.name, position: pos.name },
                errors,
            );
            let mut seen_dir: HashSet<Sym> = HashSet::new();
            for dir in &pos.directions {
                if !seen_dir.insert(dir.name) {
                    errors.push(ValidationError::DuplicateName {
                        scope: DuplicateScope::DirectionsInPosition {
                            interface: iface.name,
                            position: pos.name,
                        },
                        name: dir.name,
                    });
                }
                check_param_dups(
                    &dir.params,
                    DuplicateScope::DirectionParams {
                        interface: iface.name,
                        position: pos.name,
                        direction: dir.name,
                    },
                    errors,
                );
            }
        }
    }

    fn validate_schema_names(
        &self,
        schema: &crate::Schema<Sym>,
        errors: &mut Vec<ValidationError>,
    ) {
        use crate::SchemaBody;
        match &schema.body {
            SchemaBody::Record(fields) => check_param_dups(
                fields,
                DuplicateScope::SchemaRecordFields(schema.name),
                errors,
            ),
            SchemaBody::Sum(variants) => {
                let mut seen: HashSet<Sym> = HashSet::new();
                for v in variants {
                    if !seen.insert(v.name) {
                        errors.push(ValidationError::DuplicateName {
                            scope: DuplicateScope::SchemaSumVariants(schema.name),
                            name: v.name,
                        });
                    }
                    check_param_dups(
                        &v.params,
                        DuplicateScope::VariantParams {
                            schema: schema.name,
                            variant: v.name,
                        },
                        errors,
                    );
                }
            }
        }
    }

    fn validate_defer(&self, d: &Defer<Sym>, errors: &mut Vec<ValidationError>) {
        let src_iface = self.interfaces.get(&d.source);
        let tgt_iface = self.interfaces.get(&d.target);
        if src_iface.is_none() {
            errors.push(ValidationError::UnknownInterface(d.source));
        }
        if tgt_iface.is_none() {
            errors.push(ValidationError::UnknownInterface(d.target));
        }
        let (Some(src_iface), Some(tgt_iface)) = (src_iface, tgt_iface) else {
            return;
        };

        let src_internal = self.resolve(d.source).ends_with("::Internal");
        let tgt_internal = self.resolve(d.target).ends_with("::Internal");

        for entry in &d.entries {
            let src_pos = src_iface.position(&entry.source_pos);
            let tgt_pos = tgt_iface.position(&entry.target_pos);
            if src_pos.is_none() {
                errors.push(ValidationError::DeferUnknownPosition {
                    defer: d.name,
                    interface: d.source,
                    position: entry.source_pos,
                });
            }
            if tgt_pos.is_none() {
                errors.push(ValidationError::DeferUnknownPosition {
                    defer: d.name,
                    interface: d.target,
                    position: entry.target_pos,
                });
            }
            if let Some(sp) = src_pos {
                if sp.params.len() != entry.source_pattern.len() {
                    errors.push(ValidationError::DeferPatternArity {
                        defer: d.name,
                        interface: d.source,
                        position: sp.name,
                        expected: sp.params.len(),
                        got: entry.source_pattern.len(),
                    });
                }
            }
            if let Some(tp) = tgt_pos {
                if tp.params.len() != entry.target_args.len() {
                    errors.push(ValidationError::DeferTargetArity {
                        defer: d.name,
                        interface: d.target,
                        position: tp.name,
                        expected: tp.params.len(),
                        got: entry.target_args.len(),
                    });
                }
            }

            for m in &entry.directions {
                self.validate_dir_ref(
                    d, &m.target_dir, tgt_iface, entry.target_pos, tgt_internal, errors,
                );
                self.validate_dir_ref(
                    d, &m.source_dir, src_iface, entry.source_pos, src_internal, errors,
                );
            }
        }
    }

    fn validate_dir_ref(
        &self,
        d: &Defer<Sym>,
        r: &DirRef<Sym>,
        iface: &Interface<Sym>,
        pos: Sym,
        iface_is_internal: bool,
        errors: &mut Vec<ValidationError>,
    ) {
        match r {
            DirRef::Named(name) => {
                if let Some(p) = iface.position(&pos) {
                    if !p.directions.iter().any(|dir| dir.name == *name) {
                        errors.push(ValidationError::DirRefUnknown {
                            defer: d.name,
                            interface: iface.name,
                            position: pos,
                            name: *name,
                        });
                    }
                }
            }
            DirRef::Abstract { src_pos, src_pattern, tgt_pos, tgt_args } => {
                if !iface_is_internal {
                    errors.push(ValidationError::DirRefAbstractNotPermitted {
                        defer: d.name,
                        interface: iface.name,
                    });
                    return;
                }
                if let Some(sp) = iface.position(src_pos) {
                    if sp.params.len() != src_pattern.len() {
                        errors.push(ValidationError::AbstractArity {
                            defer: d.name,
                            interface: iface.name,
                            position: *src_pos,
                            expected: sp.params.len(),
                            got: src_pattern.len(),
                        });
                    }
                } else {
                    errors.push(ValidationError::AbstractUnknownPos {
                        defer: d.name,
                        interface: iface.name,
                        position: *src_pos,
                    });
                }
                if let Some(tp) = iface.position(tgt_pos) {
                    if tp.params.len() != tgt_args.len() {
                        errors.push(ValidationError::AbstractArity {
                            defer: d.name,
                            interface: iface.name,
                            position: *tgt_pos,
                            expected: tp.params.len(),
                            got: tgt_args.len(),
                        });
                    }
                } else {
                    errors.push(ValidationError::AbstractUnknownPos {
                        defer: d.name,
                        interface: iface.name,
                        position: *tgt_pos,
                    });
                }
            }
        }
    }

    pub fn fmt_validation_error(&self, e: &ValidationError) -> String {
        match e {
            ValidationError::DuplicateName { scope, name } => {
                let n = self.resolve(*name);
                match scope {
                    DuplicateScope::TopLevel(kind) => {
                        let k = match kind {
                            DeclKind::Schema => "schema",
                            DeclKind::Interface => "interface",
                            DeclKind::Defer => "defer",
                        };
                        format!("duplicate {k}: `{n}` is defined more than once")
                    }
                    DuplicateScope::PositionsInInterface(i) => format!(
                        "interface {}: duplicate position `{n}`",
                        self.resolve(*i),
                    ),
                    DuplicateScope::DirectionsInPosition { interface, position } => format!(
                        "{}.{}: duplicate direction `{n}`",
                        self.resolve(*interface),
                        self.resolve(*position),
                    ),
                    DuplicateScope::InterfaceParams(i) => format!(
                        "interface {}: duplicate parameter `{n}`",
                        self.resolve(*i),
                    ),
                    DuplicateScope::PositionParams { interface, position } => format!(
                        "{}.{}: duplicate parameter `{n}`",
                        self.resolve(*interface),
                        self.resolve(*position),
                    ),
                    DuplicateScope::DirectionParams { interface, position, direction } => format!(
                        "{}.{}.{}: duplicate parameter `{n}`",
                        self.resolve(*interface),
                        self.resolve(*position),
                        self.resolve(*direction),
                    ),
                    DuplicateScope::SchemaRecordFields(s) => format!(
                        "schema {}: duplicate field `{n}`",
                        self.resolve(*s),
                    ),
                    DuplicateScope::SchemaSumVariants(s) => format!(
                        "schema {}: duplicate variant `{n}`",
                        self.resolve(*s),
                    ),
                    DuplicateScope::VariantParams { schema, variant } => format!(
                        "schema {}.{}: duplicate parameter `{n}`",
                        self.resolve(*schema),
                        self.resolve(*variant),
                    ),
                }
            }
            ValidationError::UnknownInterface(s) => {
                format!("unknown interface: {}", self.resolve(*s))
            }
            ValidationError::DeferUnknownPosition { defer, interface, position } => format!(
                "defer {}: position `{}` not found in interface `{}`",
                self.resolve(*defer),
                self.resolve(*position),
                self.resolve(*interface),
            ),
            ValidationError::DeferPatternArity {
                defer, interface, position, expected, got,
            } => format!(
                "defer {}: source pattern at {}.{} has {} arg(s), expected {}",
                self.resolve(*defer),
                self.resolve(*interface),
                self.resolve(*position),
                got,
                expected,
            ),
            ValidationError::DeferTargetArity {
                defer, interface, position, expected, got,
            } => format!(
                "defer {}: target args at {}.{} have arity {}, expected {}",
                self.resolve(*defer),
                self.resolve(*interface),
                self.resolve(*position),
                got,
                expected,
            ),
            ValidationError::DirRefUnknown { defer, interface, position, name } => format!(
                "defer {}: action `{}` is not a direction of {}.{}",
                self.resolve(*defer),
                self.resolve(*name),
                self.resolve(*interface),
                self.resolve(*position),
            ),
            ValidationError::DirRefAbstractNotPermitted { defer, interface } => format!(
                "defer {}: abstract transition is not a direction of `{}`; abstract refs require an `::Internal` interface",
                self.resolve(*defer),
                self.resolve(*interface),
            ),
            ValidationError::AbstractUnknownPos { defer, interface, position } => format!(
                "defer {}: abstract transition references unknown position `{}` in `{}`",
                self.resolve(*defer),
                self.resolve(*position),
                self.resolve(*interface),
            ),
            ValidationError::AbstractArity {
                defer, interface, position, expected, got,
            } => format!(
                "defer {}: abstract transition at {}.{} has arity {}, expected {}",
                self.resolve(*defer),
                self.resolve(*interface),
                self.resolve(*position),
                got,
                expected,
            ),
        }
    }
}

fn check_param_dups(
    params: &[Param<Sym>],
    scope: DuplicateScope,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen: HashSet<Sym> = HashSet::new();
    for p in params {
        if !seen.insert(p.name) {
            errors.push(ValidationError::DuplicateName { scope, name: p.name });
        }
    }
}
