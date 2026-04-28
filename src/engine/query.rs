use std::collections::BTreeMap;
use super::{Engine, Sym};

// ============================================================================
// Query result types
// ============================================================================

#[derive(Clone, Debug)]
pub enum QueryError {
    UnknownInterface(String),
    UnknownPosition { interface: String, position: String },
}

#[derive(Clone, Debug)]
pub struct PositionExplanation {
    pub interface: Sym,
    pub position: Sym,
    pub parameterized: bool,
    pub actions: Vec<Sym>,
    pub forward: Vec<ForwardLink>,
    pub backward: Vec<BackwardLink>,
}

#[derive(Clone, Debug)]
pub struct ForwardLink {
    pub defer: Sym,
    pub source: Sym,
    pub target: Sym,
    pub target_pos: Sym,
    pub action_groups: Vec<ActionGroup>,
}

#[derive(Clone, Debug)]
pub struct ActionGroup {
    pub source_action: Sym,
    pub target_actions: Vec<Sym>,
}

#[derive(Clone, Debug)]
pub struct BackwardLink {
    pub defer: Sym,
    pub source: Sym,
    pub target: Sym,
    pub preimage: Vec<PreimageEntry>,
}

#[derive(Clone, Debug)]
pub struct PreimageEntry {
    pub source_pos: Sym,
    pub correspondences: Vec<ActionCorrespondence>,
}

#[derive(Clone, Debug)]
pub struct ActionCorrespondence {
    pub target_action: Sym,
    pub source_action: Sym,
}

#[derive(Clone, Debug)]
pub struct ActionLocations {
    pub action: String,
    pub locations: Vec<ActionLocation>,
}

#[derive(Clone, Debug)]
pub struct ActionLocation {
    pub interface: Sym,
    pub position: Sym,
}


// ============================================================================
// Queries
// ============================================================================

impl Engine {
    pub fn explain_position(
        &self,
        interface: &str,
        position: &str,
    ) -> Result<PositionExplanation, QueryError> {
        let iface_sym = self
            .interner
            .find(interface)
            .ok_or_else(|| QueryError::UnknownInterface(interface.to_string()))?;
        let iface = self
            .interfaces
            .get(&iface_sym)
            .ok_or_else(|| QueryError::UnknownInterface(interface.to_string()))?;
        let pos_sym = self.interner.find(position).ok_or_else(|| QueryError::UnknownPosition {
            interface: interface.to_string(),
            position: position.to_string(),
        })?;
        let pos = iface.position(&pos_sym).ok_or_else(|| QueryError::UnknownPosition {
            interface: interface.to_string(),
            position: position.to_string(),
        })?;

        let actions: Vec<Sym> = pos.directions.iter().map(|d| d.name).collect();
        let mut forward = Vec::new();
        let mut backward = Vec::new();

        for d in &self.defers {
            if d.source == iface_sym {
                if let Some(&tgt) = d.pos_map.get(&pos_sym) {
                    let action_groups = d
                        .dir_map
                        .get(&pos_sym)
                        .map(|dm| {
                            group_by_value(dm)
                                .into_iter()
                                .map(|(src, tgts)| ActionGroup {
                                    source_action: src,
                                    target_actions: tgts.into_iter().collect(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    forward.push(ForwardLink {
                        defer: d.name,
                        source: d.source,
                        target: d.target,
                        target_pos: tgt,
                        action_groups,
                    });
                }
            }
            if d.target == iface_sym {
                let preimage_syms: Vec<Sym> = d
                    .pos_map
                    .iter()
                    .filter_map(|(s, t)| if *t == pos_sym { Some(*s) } else { None })
                    .collect();
                if !preimage_syms.is_empty() {
                    let preimage = preimage_syms
                        .into_iter()
                        .map(|s| {
                            let correspondences = d
                                .dir_map
                                .get(&s)
                                .map(|dm| {
                                    dm.iter()
                                        .map(|(t, src)| ActionCorrespondence {
                                            target_action: *t,
                                            source_action: *src,
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            PreimageEntry { source_pos: s, correspondences }
                        })
                        .collect();
                    backward.push(BackwardLink {
                        defer: d.name,
                        source: d.source,
                        target: d.target,
                        preimage,
                    });
                }
            }
        }

        Ok(PositionExplanation {
            interface: iface_sym,
            position: pos_sym,
            parameterized: iface.is_parameterized(),
            actions,
            forward,
            backward,
        })
    }

    pub fn locate_action(&self, action: &str) -> ActionLocations {
        let mut locations = Vec::new();
        if let Some(action_sym) = self.interner.find(action) {
            for (iname, iface) in &self.interfaces {
                for pos in &iface.positions {
                    if pos.directions.iter().any(|d| d.name == action_sym) {
                        locations.push(ActionLocation { interface: *iname, position: pos.name });
                    }
                }
            }
        }
        ActionLocations { action: action.to_string(), locations }
    }
}


// ============================================================================
// Query result formatting
// ============================================================================

impl Engine {
    pub fn fmt_query_error(&self, err: &QueryError) -> String {
        match err {
            QueryError::UnknownInterface(name) => format!("unknown interface: {name}"),
            QueryError::UnknownPosition { interface, position } => {
                format!("unknown position: {interface}.{position}")
            }
        }
    }

    pub fn fmt_position_explanation(&self, e: &PositionExplanation) -> String {
        let mut out = format!(
            "{} at {}\n",
            self.resolve(e.interface),
            self.resolve(e.position),
        );
        if e.parameterized {
            out.push_str(
                "  (parameterized — query reports shape only; concrete answers require bindings)\n",
            );
        }
        let action_names: Vec<&str> = e.actions.iter().map(|s| self.resolve(*s)).collect();
        out.push_str(&format!("  available actions: {}\n", brace_set(&action_names)));

        for f in &e.forward {
            out.push_str(&format!(
                "\n  via defer {} ({} -> {}):\n",
                self.resolve(f.defer),
                self.resolve(f.source),
                self.resolve(f.target),
            ));
            out.push_str(&format!(
                "    {} must be at {}\n",
                self.resolve(f.target),
                self.resolve(f.target_pos),
            ));
            for g in &f.action_groups {
                let tgt_names: Vec<&str> =
                    g.target_actions.iter().map(|s| self.resolve(*s)).collect();
                out.push_str(&format!(
                    "    action {} <- {} action(s) {}\n",
                    self.resolve(g.source_action),
                    self.resolve(f.target),
                    brace_set(&tgt_names),
                ));
            }
        }

        for b in &e.backward {
            out.push_str(&format!(
                "\n  via defer {} ({} -> {}):\n",
                self.resolve(b.defer),
                self.resolve(b.source),
                self.resolve(b.target),
            ));
            let pre_names: Vec<&str> =
                b.preimage.iter().map(|p| self.resolve(p.source_pos)).collect();
            out.push_str(&format!(
                "    {} could be at any of: {}\n",
                self.resolve(b.source),
                brace_set(&pre_names),
            ));
            for p in &b.preimage {
                for c in &p.correspondences {
                    out.push_str(&format!(
                        "    if {}={}: choosing {}.{} corresponds to {}.{}\n",
                        self.resolve(b.source),
                        self.resolve(p.source_pos),
                        self.resolve(b.target),
                        self.resolve(c.target_action),
                        self.resolve(b.source),
                        self.resolve(c.source_action),
                    ));
                }
            }
        }

        out
    }

    pub fn fmt_action_locations(&self, locs: &ActionLocations) -> String {
        if locs.locations.is_empty() {
            return format!("action `{}` is not available at any position\n", locs.action);
        }
        let mut out = format!("action `{}` is available at:\n", locs.action);
        for l in &locs.locations {
            out.push_str(&format!(
                "  {}.{}\n",
                self.resolve(l.interface),
                self.resolve(l.position),
            ));
        }
        out
    }
}


// ============================================================================
// Helpers
// ============================================================================

fn group_by_value(m: &BTreeMap<Sym, Sym>) -> BTreeMap<Sym, Vec<Sym>> {
    let mut out: BTreeMap<Sym, Vec<Sym>> = BTreeMap::new();
    for (k, v) in m {
        out.entry(*v).or_default().push(*k);
    }
    out
}

fn brace_set(items: &[&str]) -> String {
    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", items.join(", "))
    }
}
