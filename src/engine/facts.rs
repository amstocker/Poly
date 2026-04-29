use super::*;


// ============================================================================
// Relation tuples
// ============================================================================

#[derive(Clone, Debug)]
pub struct SchemaRecordFact {
    pub schema: Sym,
    pub fields: Vec<Param<Sym>>,
}

#[derive(Clone, Debug)]
pub struct SchemaSumFact {
    pub schema: Sym,
    pub variants: Vec<Variant<Sym>>,
}

#[derive(Clone, Debug)]
pub struct IfaceFact {
    pub iface: Sym,
    pub params: Vec<Param<Sym>>,
}

#[derive(Clone, Debug)]
pub struct IfaceInternalFact {
    pub internal: Sym,
    pub external: Sym,
}

#[derive(Clone, Debug)]
pub struct PositionFact {
    pub iface: Sym,
    pub position: Sym,
    pub params: Vec<Param<Sym>>,
    pub guard: Option<Expr<Sym>>,
}

#[derive(Clone, Debug)]
pub struct DirectionFact {
    pub iface: Sym,
    pub position: Sym,
    pub action: Sym,
    pub params: Vec<Param<Sym>>,
    pub guard: Option<Expr<Sym>>,
}

#[derive(Clone, Debug)]
pub struct TransitionFact {
    pub iface: Sym,
    pub position: Sym,
    pub action: Sym,
    pub target_pos: Sym,
    pub args: Vec<Expr<Sym>>,
}

#[derive(Clone, Debug)]
pub struct DeferFact {
    pub defer: Sym,
    pub source: Sym,
    pub target: Sym,
}

#[derive(Clone, Debug)]
pub struct DeferEntryFact {
    pub defer: Sym,
    pub entry_idx: usize,
    pub source_pos: Sym,
    pub src_pattern: Vec<Pattern<Sym>>,
    pub src_guard: Option<Expr<Sym>>,
    pub target_pos: Sym,
    pub target_args: Vec<Expr<Sym>>,
}

#[derive(Clone, Debug)]
pub struct DeferDirFact {
    pub defer: Sym,
    pub entry_idx: usize,
    pub target_dir: DirRef<Sym>,
    pub source_dir: DirRef<Sym>,
}


// ============================================================================
// Container
// ============================================================================

#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub schema_records: Vec<SchemaRecordFact>,
    pub schema_sums: Vec<SchemaSumFact>,
    pub ifaces: Vec<IfaceFact>,
    pub iface_internals: Vec<IfaceInternalFact>,
    pub positions: Vec<PositionFact>,
    pub directions: Vec<DirectionFact>,
    pub transitions: Vec<TransitionFact>,
    pub defers: Vec<DeferFact>,
    pub defer_entries: Vec<DeferEntryFact>,
    pub defer_dirs: Vec<DeferDirFact>,
}


// ============================================================================
// Projection
// ============================================================================

const INTERNAL_SUFFIX: &str = "::Internal";

const PREC_TOP: u8 = 0;

impl Engine {
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();

        for s in self.schemas.values() {
            match &s.body {
                SchemaBody::Record(fields) => f.schema_records.push(SchemaRecordFact {
                    schema: s.name,
                    fields: fields.clone(),
                }),
                SchemaBody::Sum(variants) => f.schema_sums.push(SchemaSumFact {
                    schema: s.name,
                    variants: variants.clone(),
                }),
            }
        }

        for iface in self.interfaces.values() {
            f.ifaces.push(IfaceFact {
                iface: iface.name,
                params: iface.params.clone(),
            });

            let name = self.resolve(iface.name);
            if let Some(stripped) = name.strip_suffix(INTERNAL_SUFFIX) {
                if let Some(ext_sym) = self.interner.find(stripped) {
                    if self.interfaces.contains_key(&ext_sym) {
                        f.iface_internals.push(IfaceInternalFact {
                            internal: iface.name,
                            external: ext_sym,
                        });
                    }
                }
            }

            for pos in &iface.positions {
                f.positions.push(PositionFact {
                    iface: iface.name,
                    position: pos.name,
                    params: pos.params.clone(),
                    guard: pos.guard.clone(),
                });
                for dir in &pos.directions {
                    f.directions.push(DirectionFact {
                        iface: iface.name,
                        position: pos.name,
                        action: dir.name,
                        params: dir.params.clone(),
                        guard: dir.guard.clone(),
                    });
                    if let Some(t) = &dir.transition {
                        f.transitions.push(TransitionFact {
                            iface: iface.name,
                            position: pos.name,
                            action: dir.name,
                            target_pos: t.target_pos,
                            args: t.args.clone(),
                        });
                    }
                }
            }
        }

        for d in &self.defers {
            f.defers.push(DeferFact {
                defer: d.name,
                source: d.source,
                target: d.target,
            });
            for (idx, entry) in d.entries.iter().enumerate() {
                f.defer_entries.push(DeferEntryFact {
                    defer: d.name,
                    entry_idx: idx,
                    source_pos: entry.source_pos,
                    src_pattern: entry.source_pattern.clone(),
                    src_guard: entry.source_guard.clone(),
                    target_pos: entry.target_pos,
                    target_args: entry.target_args.clone(),
                });
                for m in &entry.directions {
                    f.defer_dirs.push(DeferDirFact {
                        defer: d.name,
                        entry_idx: idx,
                        target_dir: m.target_dir.clone(),
                        source_dir: m.source_dir.clone(),
                    });
                }
            }
        }

        f
    }

    pub fn fmt_facts(&self, facts: &Facts) -> String {
        let mut out = String::new();
        let mut emit = |buf: &mut String, lines: Vec<String>| {
            if lines.is_empty() {
                return;
            }
            if !buf.is_empty() {
                buf.push('\n');
            }
            for line in lines {
                buf.push_str(&line);
                buf.push('\n');
            }
        };

        let lines: Vec<String> = facts
            .schema_records
            .iter()
            .map(|r| {
                format!(
                    "schema_record({}, {}).",
                    self.resolve(r.schema),
                    self.fmt_param_list(&r.fields),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .schema_sums
            .iter()
            .map(|s| {
                let parts: Vec<String> = s
                    .variants
                    .iter()
                    .map(|v| {
                        if v.params.is_empty() {
                            self.resolve(v.name).to_string()
                        } else {
                            format!("{}{}", self.resolve(v.name), self.fmt_param_list(&v.params))
                        }
                    })
                    .collect();
                format!("schema_sum({}, [{}]).", self.resolve(s.schema), parts.join(", "))
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .ifaces
            .iter()
            .map(|i| {
                format!(
                    "iface({}, {}).",
                    self.resolve(i.iface),
                    self.fmt_param_list(&i.params),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .iface_internals
            .iter()
            .map(|i| {
                format!(
                    "iface_internal({}, {}).",
                    self.resolve(i.internal),
                    self.resolve(i.external),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .positions
            .iter()
            .map(|p| {
                format!(
                    "position({}, {}, {}, {}).",
                    self.resolve(p.iface),
                    self.resolve(p.position),
                    self.fmt_param_list(&p.params),
                    self.fmt_opt_expr(&p.guard),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .directions
            .iter()
            .map(|d| {
                format!(
                    "direction({}, {}, {}, {}, {}).",
                    self.resolve(d.iface),
                    self.resolve(d.position),
                    self.resolve(d.action),
                    self.fmt_param_list(&d.params),
                    self.fmt_opt_expr(&d.guard),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .transitions
            .iter()
            .map(|t| {
                let args: Vec<String> =
                    t.args.iter().map(|a| self.fmt_expr(a, PREC_TOP)).collect();
                format!(
                    "transition({}, {}, {}, {}, [{}]).",
                    self.resolve(t.iface),
                    self.resolve(t.position),
                    self.resolve(t.action),
                    self.resolve(t.target_pos),
                    args.join(", "),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .defers
            .iter()
            .map(|d| {
                format!(
                    "defer({}, {}, {}).",
                    self.resolve(d.defer),
                    self.resolve(d.source),
                    self.resolve(d.target),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .defer_entries
            .iter()
            .map(|e| {
                let tgt_args: Vec<String> =
                    e.target_args.iter().map(|a| self.fmt_expr(a, PREC_TOP)).collect();
                format!(
                    "defer_entry({}, {}, {}, {}, {}, {}, [{}]).",
                    self.resolve(e.defer),
                    e.entry_idx,
                    self.resolve(e.source_pos),
                    self.fmt_pattern_list(&e.src_pattern),
                    self.fmt_opt_expr(&e.src_guard),
                    self.resolve(e.target_pos),
                    tgt_args.join(", "),
                )
            })
            .collect();
        emit(&mut out, lines);

        let lines: Vec<String> = facts
            .defer_dirs
            .iter()
            .map(|m| {
                format!(
                    "defer_dir({}, {}, {}, {}).",
                    self.resolve(m.defer),
                    m.entry_idx,
                    self.fmt_dir_ref(&m.target_dir),
                    self.fmt_dir_ref(&m.source_dir),
                )
            })
            .collect();
        emit(&mut out, lines);

        out
    }

    fn fmt_opt_expr(&self, e: &Option<Expr<Sym>>) -> String {
        match e {
            None => "_".to_string(),
            Some(e) => self.fmt_expr(e, PREC_TOP),
        }
    }
}
