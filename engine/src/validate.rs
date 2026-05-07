use super::{Defer, DirRef, Engine, Interface, Sym};


// ============================================================================
// Validation errors
// ============================================================================

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
}


// ============================================================================
// Validator
// ============================================================================

impl Engine {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        for d in &self.defers {
            self.validate_defer(d, &mut errors);
        }
        errors
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
