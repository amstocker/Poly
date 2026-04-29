use std::collections::BTreeMap;
use super::eval::{eval, eval_bool, Bindings, EvalError};
use super::{BinOp, DirMapping, DirRef, Engine, Expr, Param, Pattern, Position, Sym};

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
    pub source_pattern: Vec<Pattern<Sym>>,
    pub target_pos: Sym,
    pub target_args: Vec<Expr<Sym>>,
    pub mappings: Vec<DirMapping<Sym>>,
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
    pub source_pattern: Vec<Pattern<Sym>>,
    pub target_args: Vec<Expr<Sym>>,
    pub mappings: Vec<DirMapping<Sym>>,
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
                    forward.push(ForwardLink {
                        defer: d.name,
                        source: d.source,
                        target: d.target,
                        source_pattern: entry.source_pattern.clone(),
                        target_pos: entry.target_pos,
                        target_args: entry.target_args.clone(),
                        mappings: entry.directions.clone(),
                    });
                }
            }
            if d.target == iface_sym {
                let preimage: Vec<PreimageEntry> = d
                    .entries
                    .iter()
                    .filter(|e| e.target_pos == pos_sym)
                    .map(|e| PreimageEntry {
                        source_pos: e.source_pos,
                        source_pattern: e.source_pattern.clone(),
                        target_args: e.target_args.clone(),
                        mappings: e.directions.clone(),
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
            if !eval_bool(self, g, &bindings).map_err(QueryError::EvalFailed)? {
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
        let dir_opt = pos.directions.iter().find(|d| d.name == action_sym);

        let (target_pos_sym, target_bindings) = match dir_opt {
            Some(dir) => {
                if let Some(g) = &dir.guard {
                    if !eval_bool(self, g, &bindings).map_err(QueryError::EvalFailed)? {
                        return Err(QueryError::GuardFailed {
                            interface: interface.to_string(),
                            position: position.to_string(),
                            kind: GuardKind::Direction,
                        });
                    }
                }
                if let Some(trans) = &dir.transition {
                    self.apply_transition(interface, &bindings, &trans.target_pos, &trans.args)?
                } else {
                    self.apply_realization(
                        interface, position, action, iface_sym, pos_sym, action_sym, &bindings,
                    )?
                }
            }
            None => self.apply_via_defer_source(
                interface, iface_sym, pos, pos_sym, action_sym, &bindings,
            )?,
        };
        let target_pos = iface.position(&target_pos_sym).ok_or_else(|| {
            QueryError::UnknownPosition {
                interface: interface.to_string(),
                position: self.resolve(target_pos_sym).to_string(),
            }
        })?;

        if let Some(g) = &target_pos.guard {
            if !eval_bool(self, g, &target_bindings).map_err(QueryError::EvalFailed)? {
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
        for p in &iface.params {
            if let Some(v) = bindings.get(&p.name) {
                new_bindings.insert(p.name, v.clone());
            }
        }
        for (param, arg) in tgt_pos.params.iter().zip(args.iter()) {
            let v = eval(self, arg, bindings).map_err(QueryError::EvalFailed)?;
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

    fn apply_via_defer_source(
        &self,
        interface: &str,
        iface_sym: Sym,
        pos: &Position<Sym>,
        pos_sym: Sym,
        action_sym: Sym,
        bindings: &Bindings,
    ) -> Result<(Sym, Bindings), QueryError> {
        for d in &self.defers {
            if d.source != iface_sym {
                continue;
            }
            for entry in &d.entries {
                if entry.source_pos != pos_sym {
                    continue;
                }
                for m in &entry.directions {
                    let DirRef::Named(name) = m.target_dir else {
                        continue;
                    };
                    if name != action_sym {
                        continue;
                    }
                    if let DirRef::Abstract { src_pos, src_pattern, tgt_pos, tgt_args } =
                        &m.source_dir
                    {
                        if *src_pos != pos_sym {
                            continue;
                        }
                        let local = bind_pattern(pos, bindings, src_pattern);
                        return self.apply_transition(interface, &local, tgt_pos, tgt_args);
                    }
                }
            }
        }
        Err(QueryError::UnknownAction {
            interface: interface.to_string(),
            position: self.resolve(pos_sym).to_string(),
            action: self.resolve(action_sym).to_string(),
        })
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
            let src_shape = self.fmt_pos_shape(e.position, &f.source_pattern);
            let tgt_shape = self.fmt_pos_shape_args(f.target_pos, &f.target_args);
            out.push_str(&format!(
                "    {}.{} -> {}.{}\n",
                self.resolve(f.source),
                src_shape,
                self.resolve(f.target),
                tgt_shape,
            ));
            for m in &f.mappings {
                out.push_str(&format!(
                    "      {}  <-  {}\n",
                    self.fmt_dir_ref(&m.target_dir),
                    self.fmt_dir_ref(&m.source_dir),
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
            for p in &b.preimage {
                let src_shape = self.fmt_pos_shape(p.source_pos, &p.source_pattern);
                let tgt_shape = self.fmt_pos_shape_args(e.position, &p.target_args);
                out.push_str(&format!(
                    "    {}.{} -> {}.{}\n",
                    self.resolve(b.source),
                    src_shape,
                    self.resolve(b.target),
                    tgt_shape,
                ));
                for m in &p.mappings {
                    out.push_str(&format!(
                        "      {}  <-  {}\n",
                        self.fmt_dir_ref(&m.target_dir),
                        self.fmt_dir_ref(&m.source_dir),
                    ));
                }
            }
        }

        out
    }

    fn fmt_pos_shape(&self, pos: Sym, pattern: &[Pattern<Sym>]) -> String {
        let mut out = self.resolve(pos).to_string();
        if !pattern.is_empty() {
            out.push_str(&self.fmt_pattern_list(pattern));
        }
        out
    }

    fn fmt_pos_shape_args(&self, pos: Sym, args: &[Expr<Sym>]) -> String {
        let mut out = self.resolve(pos).to_string();
        if !args.is_empty() {
            let parts: Vec<String> = args.iter().map(|a| self.fmt_expr(a, 0)).collect();
            out.push_str(&format!("[{}]", parts.join(", ")));
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

fn bind_pattern(pos: &Position<Sym>, bindings: &Bindings, pat: &[Pattern<Sym>]) -> Bindings {
    let mut new_bindings = bindings.clone();
    for (param, p) in pos.params.iter().zip(pat.iter()) {
        if let Pattern::Bind(name) = p {
            if let Some(v) = bindings.get(&param.name) {
                new_bindings.insert(*name, v.clone());
            }
        }
    }
    new_bindings
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
