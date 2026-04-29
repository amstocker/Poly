use std::collections::BTreeMap;
use super::eval::{eval, eval_bool, Bindings, EvalError};
use super::{BinOp, DirRef, Engine, Expr, Param, Sym};

// ============================================================================
// Query result types
// ============================================================================

#[derive(Clone, Debug)]
pub enum QueryError {
    UnknownInterface(String),
    UnknownPosition { interface: String, position: String },
    UnknownAction { interface: String, position: String, action: String },
    NoTransition { interface: String, position: String, action: String },
    GuardFailed { interface: String, position: String, kind: GuardKind },
    ArityMismatch { interface: String, position: String, expected: usize, got: usize },
    EvalFailed(EvalError),
}

#[derive(Clone, Copy, Debug)]
pub enum GuardKind {
    Position,
    Direction,
    TargetPosition,
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
    pub params: Vec<Param<Sym>>,
    pub constraint: Option<Expr<Sym>>,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub interface: Sym,
    pub source_position: Sym,
    pub source_bindings: Bindings,
    pub action: Sym,
    pub target_position: Sym,
    pub target_bindings: Bindings,
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
                for entry in &d.entries {
                    if entry.source_pos != pos_sym {
                        continue;
                    }
                    let mut grouped: BTreeMap<Sym, Vec<Sym>> = BTreeMap::new();
                    for m in &entry.directions {
                        if let (Some(src), Some(tgt)) =
                            (named_dir(&m.source_dir), named_dir(&m.target_dir))
                        {
                            grouped.entry(src).or_default().push(tgt);
                        }
                    }
                    let action_groups = grouped
                        .into_iter()
                        .map(|(src, tgts)| ActionGroup {
                            source_action: src,
                            target_actions: tgts,
                        })
                        .collect();
                    forward.push(ForwardLink {
                        defer: d.name,
                        source: d.source,
                        target: d.target,
                        target_pos: entry.target_pos,
                        action_groups,
                    });
                }
            }
            if d.target == iface_sym {
                let preimage: Vec<PreimageEntry> = d
                    .entries
                    .iter()
                    .filter(|e| e.target_pos == pos_sym)
                    .map(|e| {
                        let correspondences = e
                            .directions
                            .iter()
                            .filter_map(|m| {
                                let tgt = named_dir(&m.target_dir)?;
                                let src = named_dir(&m.source_dir)?;
                                Some(ActionCorrespondence {
                                    target_action: tgt,
                                    source_action: src,
                                })
                            })
                            .collect();
                        PreimageEntry { source_pos: e.source_pos, correspondences }
                    })
                    .collect();
                if !preimage.is_empty() {
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

    pub fn next_position(
        &self,
        interface: &str,
        position: &str,
        action: &str,
        bindings: Bindings,
    ) -> Result<Step, QueryError> {
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

        if let Some(g) = &pos.guard {
            if !eval_bool(g, &bindings).map_err(QueryError::EvalFailed)? {
                return Err(QueryError::GuardFailed {
                    interface: interface.to_string(),
                    position: position.to_string(),
                    kind: GuardKind::Position,
                });
            }
        }

        let action_sym = self.interner.find(action).ok_or_else(|| QueryError::UnknownAction {
            interface: interface.to_string(),
            position: position.to_string(),
            action: action.to_string(),
        })?;
        let dir = pos
            .directions
            .iter()
            .find(|d| d.name == action_sym)
            .ok_or_else(|| QueryError::UnknownAction {
                interface: interface.to_string(),
                position: position.to_string(),
                action: action.to_string(),
            })?;

        if let Some(g) = &dir.guard {
            if !eval_bool(g, &bindings).map_err(QueryError::EvalFailed)? {
                return Err(QueryError::GuardFailed {
                    interface: interface.to_string(),
                    position: position.to_string(),
                    kind: GuardKind::Direction,
                });
            }
        }

        let (target_pos_sym, target_bindings) = if let Some(trans) = &dir.transition {
            self.apply_transition(interface, &bindings, &trans.target_pos, &trans.args)?
        } else {
            self.apply_realization(interface, position, action, iface_sym, pos_sym, action_sym, &bindings)?
        };
        let target_pos = iface.position(&target_pos_sym).ok_or_else(|| {
            QueryError::UnknownPosition {
                interface: interface.to_string(),
                position: self.resolve(target_pos_sym).to_string(),
            }
        })?;

        if let Some(g) = &target_pos.guard {
            if !eval_bool(g, &target_bindings).map_err(QueryError::EvalFailed)? {
                return Err(QueryError::GuardFailed {
                    interface: interface.to_string(),
                    position: self.resolve(target_pos_sym).to_string(),
                    kind: GuardKind::TargetPosition,
                });
            }
        }

        Ok(Step {
            interface: iface_sym,
            source_position: pos_sym,
            source_bindings: bindings,
            action: action_sym,
            target_position: target_pos_sym,
            target_bindings,
        })
    }

    fn apply_transition(
        &self,
        interface: &str,
        bindings: &Bindings,
        target_pos: &Sym,
        args: &[Expr<Sym>],
    ) -> Result<(Sym, Bindings), QueryError> {
        let iface = self.interfaces.get(&self.interner.find(interface).unwrap()).unwrap();
        let tgt_pos = iface.position(target_pos).ok_or_else(|| QueryError::UnknownPosition {
            interface: interface.to_string(),
            position: self.resolve(*target_pos).to_string(),
        })?;
        if args.len() != tgt_pos.params.len() {
            return Err(QueryError::ArityMismatch {
                interface: interface.to_string(),
                position: self.resolve(*target_pos).to_string(),
                expected: tgt_pos.params.len(),
                got: args.len(),
            });
        }
        let mut new_bindings: Bindings = BTreeMap::new();
        for (param, arg) in tgt_pos.params.iter().zip(args.iter()) {
            let v = eval(arg, bindings).map_err(QueryError::EvalFailed)?;
            new_bindings.insert(param.name, v);
        }
        Ok((*target_pos, new_bindings))
    }

    fn apply_realization(
        &self,
        interface: &str,
        _position: &str,
        _action: &str,
        iface_sym: Sym,
        pos_sym: Sym,
        action_sym: Sym,
        bindings: &Bindings,
    ) -> Result<(Sym, Bindings), QueryError> {
        let internal_name = format!("{interface}::Internal");
        for d in &self.defers {
            if d.target != iface_sym {
                continue;
            }
            if self.resolve(d.source) != internal_name {
                continue;
            }
            for entry in &d.entries {
                if entry.target_pos != pos_sym {
                    continue;
                }
                for m in &entry.directions {
                    if let DirRef::Named(name) = m.target_dir {
                        if name != action_sym {
                            continue;
                        }
                        if let DirRef::Abstract { tgt_pos, tgt_args, .. } = &m.source_dir {
                            return self.apply_transition(interface, bindings, tgt_pos, tgt_args);
                        }
                    }
                }
            }
        }
        Ok((pos_sym, bindings.clone()))
    }

    pub fn locate_action(&self, action: &str) -> ActionLocations {
        let mut locations = Vec::new();
        if let Some(action_sym) = self.interner.find(action) {
            for (iname, iface) in &self.interfaces {
                for pos in &iface.positions {
                    if let Some(dir) = pos.directions.iter().find(|d| d.name == action_sym) {
                        let constraint = conjoin(pos.guard.as_ref(), dir.guard.as_ref());
                        locations.push(ActionLocation {
                            interface: *iname,
                            position: pos.name,
                            params: pos.params.clone(),
                            constraint,
                        });
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
            QueryError::UnknownAction { interface, position, action } => {
                format!("unknown action: {interface}.{position}.{action}")
            }
            QueryError::NoTransition { interface, position, action } => {
                format!("no transition for {interface}.{position}.{action}")
            }
            QueryError::GuardFailed { interface, position, kind } => {
                let which = match kind {
                    GuardKind::Position => "position guard",
                    GuardKind::Direction => "direction guard",
                    GuardKind::TargetPosition => "target position guard",
                };
                format!("{which} failed at {interface}.{position}")
            }
            QueryError::ArityMismatch { interface, position, expected, got } => {
                format!(
                    "arity mismatch at {interface}.{position}: expected {expected} arg(s), got {got}"
                )
            }
            QueryError::EvalFailed(e) => format!("evaluation failed: {}", self.fmt_eval_error(e)),
        }
    }

    pub fn fmt_step(&self, step: &Step) -> String {
        let src = format!(
            "{}.{}{}",
            self.resolve(step.interface),
            self.resolve(step.source_position),
            self.fmt_bindings(&step.source_bindings),
        );
        let tgt = format!(
            "{}.{}{}",
            self.resolve(step.interface),
            self.resolve(step.target_position),
            self.fmt_bindings(&step.target_bindings),
        );
        format!("{src} --{}--> {tgt}\n", self.resolve(step.action))
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
            out.push_str(&format!("  {}.{}", self.resolve(l.interface), self.resolve(l.position)));
            if !l.params.is_empty() {
                out.push_str(&self.fmt_param_list(&l.params));
            }
            if let Some(c) = &l.constraint {
                out.push_str(&format!(" if ({})", self.fmt_expr(c, 0)));
            }
            out.push('\n');
        }
        out
    }
}


// ============================================================================
// Helpers
// ============================================================================

fn named_dir(r: &DirRef<Sym>) -> Option<Sym> {
    match r {
        DirRef::Named(s) => Some(*s),
        DirRef::Abstract { .. } => None,
    }
}

fn conjoin(a: Option<&Expr<Sym>>, b: Option<&Expr<Sym>>) -> Option<Expr<Sym>> {
    match (a, b) {
        (None, None) => None,
        (Some(e), None) | (None, Some(e)) => Some(e.clone()),
        (Some(x), Some(y)) => Some(Expr::BinOp(
            BinOp::And,
            Box::new(x.clone()),
            Box::new(y.clone()),
        )),
    }
}

fn brace_set(items: &[&str]) -> String {
    if items.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", items.join(", "))
    }
}
