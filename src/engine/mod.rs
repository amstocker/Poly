pub mod parse;

use std::collections::{BTreeMap, BTreeSet, HashMap};


// ============================================================================
// Symbols and interner
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

#[derive(Clone, Debug, Default)]
pub struct Interner {
    forward: HashMap<String, Sym>,
    backward: Vec<String>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> Sym {
        if let Some(&sym) = self.forward.get(s) {
            return sym;
        }
        let sym = Sym(self.backward.len() as u32);
        self.backward.push(s.to_string());
        self.forward.insert(s.to_string(), sym);
        sym
    }

    pub fn find(&self, s: &str) -> Option<Sym> {
        self.forward.get(s).copied()
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        &self.backward[sym.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.backward.len()
    }
}


// ============================================================================
// Types and parameters  (generic over the name representation T)
// ============================================================================

#[derive(Clone, Debug)]
pub enum Type<T> {
    Int,
    Str,
    Bool,
    Named(T),
}

#[derive(Clone, Debug)]
pub struct Param<T> {
    pub name: T,
    pub ty: Type<T>,
}


// ============================================================================
// Expression AST  (T parameterizes name references; LitStr stays String)
// ============================================================================

#[derive(Clone, Debug)]
pub enum Expr<T> {
    LitInt(i64),
    LitStr(String),
    LitBool(bool),
    Var(T),
    Field(Box<Expr<T>>, T),
    BinOp(BinOp, Box<Expr<T>>, Box<Expr<T>>),
    UnOp(UnOp, Box<Expr<T>>),
    Construct(T, Vec<Expr<T>>),
}

#[derive(Clone, Copy, Debug)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Clone, Copy, Debug)]
pub enum UnOp { Neg, Not }


// ============================================================================
// Schema declarations
// ============================================================================

#[derive(Clone, Debug)]
pub struct Schema<T> {
    pub name: T,
    pub body: SchemaBody<T>,
}

#[derive(Clone, Debug)]
pub enum SchemaBody<T> {
    Record(Vec<Param<T>>),
    Sum(Vec<Variant<T>>),
}

#[derive(Clone, Debug)]
pub struct Variant<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
}


// ============================================================================
// Interface declarations
// ============================================================================

#[derive(Clone, Debug)]
pub struct Interface<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub positions: Vec<Position<T>>,
}

#[derive(Clone, Debug)]
pub struct Position<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub guard: Option<Expr<T>>,
    pub directions: Vec<Direction<T>>,
}

#[derive(Clone, Debug)]
pub struct Direction<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub guard: Option<Expr<T>>,
    pub transition: Option<Transition<T>>,
}

#[derive(Clone, Debug)]
pub struct Transition<T> {
    pub target_pos: T,
    pub args: Vec<Expr<T>>,
}

impl<T> Interface<T> {
    pub fn is_parameterized(&self) -> bool {
        !self.params.is_empty()
            || self.positions.iter().any(|p| {
                !p.params.is_empty()
                    || p.guard.is_some()
                    || p.directions.iter().any(|d| !d.params.is_empty() || d.guard.is_some())
            })
    }
}

impl<T: PartialEq> Interface<T> {
    pub fn position(&self, name: &T) -> Option<&Position<T>> {
        self.positions.iter().find(|p| &p.name == name)
    }
}


// ============================================================================
// Defer declarations
// ============================================================================

#[derive(Clone, Debug)]
pub struct Defer<T> {
    pub name: T,
    pub source: T,
    pub target: T,
    pub pos_map: BTreeMap<T, T>,
    pub dir_map: BTreeMap<T, BTreeMap<T, T>>,
}


// ============================================================================
// Top-level
// ============================================================================

#[derive(Clone, Debug)]
pub enum Decl<T> {
    Interface(Interface<T>),
    Defer(Defer<T>),
    Schema(Schema<T>),
}


// ============================================================================
// Engine  (concrete; always uses Sym internally)
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
        let mut engine = Engine {
            interner,
            ..Engine::default()
        };
        for decl in decls {
            match decl {
                Decl::Schema(s) => {
                    engine.schemas.insert(s.name, s);
                }
                Decl::Interface(i) => {
                    engine.interfaces.insert(i.name, i);
                }
                Decl::Defer(d) => engine.defers.push(d),
            }
        }
        engine
    }

    /// Parse Poly source and lower into an Engine in one step.
    pub fn load(src: &str) -> Result<Engine, Vec<chumsky::error::Simple<char>>> {
        use chumsky::Parser;
        let raw: Vec<Decl<String>> = parse::file().parse(src.to_string())?;
        let mut interner = Interner::new();
        let decls = parse::lower_decls(raw, &mut interner);
        Ok(Engine::new(interner, decls))
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        self.interner.resolve(sym)
    }
}


// ============================================================================
// Queries  (boundary takes &str, returns String — interner stays internal)
// ============================================================================

impl Engine {
    pub fn explain_position(&self, interface: &str, position: &str) -> String {
        let iface_sym = match self.interner.find(interface) {
            Some(s) => s,
            None => return format!("unknown interface: {interface}\n"),
        };
        let iface = match self.interfaces.get(&iface_sym) {
            Some(i) => i,
            None => return format!("unknown interface: {interface}\n"),
        };
        let pos_sym = match self.interner.find(position) {
            Some(s) => s,
            None => return format!("unknown position: {interface}.{position}\n"),
        };
        let pos = match iface.position(&pos_sym) {
            Some(p) => p,
            None => return format!("unknown position: {interface}.{position}\n"),
        };

        let mut out = String::new();
        out.push_str(&format!("{interface} at {position}\n"));
        if iface.is_parameterized() {
            out.push_str(
                "  (parameterized — query reports shape only; concrete answers require bindings)\n",
            );
        }

        let action_set: BTreeSet<String> = pos
            .directions
            .iter()
            .map(|d| self.resolve(d.name).to_string())
            .collect();
        out.push_str(&format!("  available actions: {}\n", fmt_set(&action_set)));

        for d in &self.defers {
            if d.source == iface_sym {
                if let Some(&tgt) = d.pos_map.get(&pos_sym) {
                    out.push_str(&format!(
                        "\n  via defer {} ({} -> {}):\n",
                        self.resolve(d.name),
                        self.resolve(d.source),
                        self.resolve(d.target),
                    ));
                    out.push_str(&format!(
                        "    {} must be at {}\n",
                        self.resolve(d.target),
                        self.resolve(tgt),
                    ));
                    if let Some(dm) = d.dir_map.get(&pos_sym) {
                        let by_src = group_by_value(dm);
                        for (src_dir, tgt_dirs) in &by_src {
                            let tgt_names: BTreeSet<String> = tgt_dirs
                                .iter()
                                .map(|s| self.resolve(*s).to_string())
                                .collect();
                            out.push_str(&format!(
                                "    action {} <- {} action(s) {{{}}}\n",
                                self.resolve(*src_dir),
                                self.resolve(d.target),
                                comma_join(&tgt_names),
                            ));
                        }
                    }
                }
            }
            if d.target == iface_sym {
                let preimage: Vec<Sym> = d
                    .pos_map
                    .iter()
                    .filter_map(|(s, t)| if *t == pos_sym { Some(*s) } else { None })
                    .collect();
                if !preimage.is_empty() {
                    out.push_str(&format!(
                        "\n  via defer {} ({} -> {}):\n",
                        self.resolve(d.name),
                        self.resolve(d.source),
                        self.resolve(d.target),
                    ));
                    let names: Vec<String> =
                        preimage.iter().map(|s| self.resolve(*s).to_string()).collect();
                    out.push_str(&format!(
                        "    {} could be at any of: {{{}}}\n",
                        self.resolve(d.source),
                        names.join(", "),
                    ));
                    for &s in &preimage {
                        if let Some(dm) = d.dir_map.get(&s) {
                            for (tgt_dir, src_dir) in dm {
                                out.push_str(&format!(
                                    "    if {}={}: choosing {}.{} corresponds to {}.{}\n",
                                    self.resolve(d.source),
                                    self.resolve(s),
                                    interface,
                                    self.resolve(*tgt_dir),
                                    self.resolve(d.source),
                                    self.resolve(*src_dir),
                                ));
                            }
                        }
                    }
                }
            }
        }

        out
    }

    pub fn locate_action(&self, action: &str) -> String {
        let action_sym = match self.interner.find(action) {
            Some(s) => s,
            None => return format!("action `{action}` is not available at any position\n"),
        };
        let mut hits = Vec::new();
        for (iname, iface) in &self.interfaces {
            for pos in &iface.positions {
                if pos.directions.iter().any(|d| d.name == action_sym) {
                    hits.push(format!(
                        "  {}.{}",
                        self.resolve(*iname),
                        self.resolve(pos.name),
                    ));
                }
            }
        }
        if hits.is_empty() {
            format!("action `{action}` is not available at any position\n")
        } else {
            format!("action `{action}` is available at:\n{}\n", hits.join("\n"))
        }
    }
}


// ============================================================================
// Formatting (replaces Display impls — needs the interner)
// ============================================================================

const PREC_TOP: u8 = 0;
const PREC_OR: u8 = 10;
const PREC_AND: u8 = 20;
const PREC_CMP: u8 = 30;
const PREC_ADD: u8 = 40;
const PREC_MUL: u8 = 50;
const PREC_UNARY: u8 = 60;
const PREC_ATOM: u8 = 100;

fn bin_prec(op: BinOp) -> u8 {
    use BinOp::*;
    match op {
        Or => PREC_OR,
        And => PREC_AND,
        Eq | Neq | Lt | Le | Gt | Ge => PREC_CMP,
        Add | Sub => PREC_ADD,
        Mul | Div | Mod => PREC_MUL,
    }
}

fn bin_str(op: BinOp) -> &'static str {
    use BinOp::*;
    match op {
        Add => "+", Sub => "-", Mul => "*", Div => "/", Mod => "%",
        Eq => "==", Neq => "!=", Lt => "<", Le => "<=", Gt => ">", Ge => ">=",
        And => "and", Or => "or",
    }
}

impl Engine {
    pub fn show_schema(&self, sym: Sym) -> Option<String> {
        self.schemas.get(&sym).map(|s| self.fmt_schema(s))
    }

    pub fn show_interface(&self, sym: Sym) -> Option<String> {
        self.interfaces.get(&sym).map(|i| self.fmt_interface(i))
    }

    pub fn show_defer(&self, sym: Sym) -> Option<String> {
        self.defers
            .iter()
            .find(|d| d.name == sym)
            .map(|d| self.fmt_defer(d))
    }

    fn fmt_type(&self, ty: &Type<Sym>) -> String {
        match ty {
            Type::Int => "Int".to_string(),
            Type::Str => "String".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Named(s) => self.resolve(*s).to_string(),
        }
    }

    fn fmt_param(&self, p: &Param<Sym>) -> String {
        format!("{}: {}", self.resolve(p.name), self.fmt_type(&p.ty))
    }

    fn fmt_param_list(&self, params: &[Param<Sym>]) -> String {
        let parts: Vec<String> = params.iter().map(|p| self.fmt_param(p)).collect();
        format!("[{}]", parts.join(", "))
    }

    fn fmt_variant(&self, v: &Variant<Sym>) -> String {
        let mut out = self.resolve(v.name).to_string();
        if !v.params.is_empty() {
            out.push_str(&self.fmt_param_list(&v.params));
        }
        out
    }

    pub fn fmt_schema(&self, s: &Schema<Sym>) -> String {
        let mut out = format!("schema {}", self.resolve(s.name));
        match &s.body {
            SchemaBody::Record(fields) => {
                for (i, p) in fields.iter().enumerate() {
                    let suffix = if i + 1 < fields.len() { "," } else { "" };
                    out.push_str(&format!("\n    {}{}", self.fmt_param(p), suffix));
                }
            }
            SchemaBody::Sum(variants) => {
                for (i, v) in variants.iter().enumerate() {
                    let suffix = if i + 1 < variants.len() { "," } else { "" };
                    out.push_str(&format!("\n    {}{}", self.fmt_variant(v), suffix));
                }
            }
        }
        out
    }

    pub fn fmt_interface(&self, iface: &Interface<Sym>) -> String {
        let mut out = format!("interface {}", self.resolve(iface.name));
        if !iface.params.is_empty() {
            out.push_str(&self.fmt_param_list(&iface.params));
        }
        for (i, pos) in iface.positions.iter().enumerate() {
            let suffix = if i + 1 < iface.positions.len() { "," } else { "" };
            out.push_str(&format!("\n    {}{}", self.fmt_position(pos), suffix));
        }
        out
    }

    fn fmt_position(&self, pos: &Position<Sym>) -> String {
        let mut out = self.resolve(pos.name).to_string();
        if !pos.params.is_empty() {
            out.push_str(&self.fmt_param_list(&pos.params));
        }
        if let Some(g) = &pos.guard {
            out.push_str(&format!(" if ({})", self.fmt_expr(g, PREC_TOP)));
        }
        if !pos.directions.is_empty() {
            let all_simple = pos.directions.iter().all(|d| {
                d.params.is_empty() && d.guard.is_none() && d.transition.is_none()
            });
            if all_simple {
                let names: Vec<String> = pos
                    .directions
                    .iter()
                    .map(|d| self.resolve(d.name).to_string())
                    .collect();
                out.push_str(&format!(" {{ {} }}", names.join(", ")));
            } else {
                out.push_str(" {");
                for (i, dir) in pos.directions.iter().enumerate() {
                    let suffix = if i + 1 < pos.directions.len() { "," } else { "" };
                    out.push_str(&format!("\n        {}{}", self.fmt_direction(dir), suffix));
                }
                out.push_str("\n    }");
            }
        }
        out
    }

    fn fmt_direction(&self, dir: &Direction<Sym>) -> String {
        let mut out = self.resolve(dir.name).to_string();
        if !dir.params.is_empty() {
            out.push_str(&self.fmt_param_list(&dir.params));
        }
        if let Some(g) = &dir.guard {
            out.push_str(&format!(" if ({})", self.fmt_expr(g, PREC_TOP)));
        }
        if let Some(t) = &dir.transition {
            out.push_str(&format!(" -> {}", self.fmt_transition(t)));
        }
        out
    }

    fn fmt_transition(&self, t: &Transition<Sym>) -> String {
        let mut out = self.resolve(t.target_pos).to_string();
        if !t.args.is_empty() {
            let parts: Vec<String> = t.args.iter().map(|e| self.fmt_expr(e, PREC_TOP)).collect();
            out.push_str(&format!("[{}]", parts.join(", ")));
        }
        out
    }

    pub fn fmt_defer(&self, d: &Defer<Sym>) -> String {
        let mut out = format!(
            "defer {} : {} -> {}",
            self.resolve(d.name),
            self.resolve(d.source),
            self.resolve(d.target),
        );
        let mappings: Vec<&Sym> = d.pos_map.keys().collect();
        for (i, src_pos) in mappings.iter().enumerate() {
            let tgt_pos = &d.pos_map[*src_pos];
            let body = match d.dir_map.get(*src_pos) {
                Some(map) if !map.is_empty() => {
                    let mut grouped: BTreeMap<Sym, Vec<Sym>> = BTreeMap::new();
                    for (tgt_dir, src_dir) in map {
                        grouped.entry(*src_dir).or_default().push(*tgt_dir);
                    }
                    let lines: Vec<String> = grouped
                        .iter()
                        .map(|(src_dir, tgt_dirs)| {
                            let parts: Vec<&str> =
                                tgt_dirs.iter().map(|s| self.resolve(*s)).collect();
                            format!(
                                "        {} -> {}",
                                parts.join(" | "),
                                self.resolve(*src_dir)
                            )
                        })
                        .collect();
                    format!(" {{\n{}\n    }}", lines.join(",\n"))
                }
                _ => " {}".to_string(),
            };
            let suffix = if i + 1 < mappings.len() { "," } else { "" };
            out.push_str(&format!(
                "\n    {} -> {}{}{}",
                self.resolve(**src_pos),
                self.resolve(*tgt_pos),
                body,
                suffix,
            ));
        }
        out
    }

    fn fmt_expr(&self, e: &Expr<Sym>, parent_prec: u8) -> String {
        match e {
            Expr::LitInt(n) => n.to_string(),
            Expr::LitStr(s) => format!("\"{s}\""),
            Expr::LitBool(b) => b.to_string(),
            Expr::Var(s) => self.resolve(*s).to_string(),
            Expr::Field(base, name) => {
                format!("{}.{}", self.fmt_expr(base, PREC_ATOM), self.resolve(*name))
            }
            Expr::Construct(name, args) => {
                let parts: Vec<String> = args.iter().map(|a| self.fmt_expr(a, PREC_TOP)).collect();
                format!("{}({})", self.resolve(*name), parts.join(", "))
            }
            Expr::BinOp(op, l, r) => {
                let p = bin_prec(*op);
                let s = format!(
                    "{} {} {}",
                    self.fmt_expr(l, p),
                    bin_str(*op),
                    self.fmt_expr(r, p)
                );
                if p < parent_prec {
                    format!("({s})")
                } else {
                    s
                }
            }
            Expr::UnOp(op, inner) => {
                let s = match op {
                    UnOp::Neg => format!("-{}", self.fmt_expr(inner, PREC_UNARY)),
                    UnOp::Not => format!("not {}", self.fmt_expr(inner, PREC_UNARY)),
                };
                if PREC_UNARY < parent_prec {
                    format!("({s})")
                } else {
                    s
                }
            }
        }
    }
}


// ============================================================================
// Misc helpers
// ============================================================================

fn group_by_value(m: &BTreeMap<Sym, Sym>) -> BTreeMap<Sym, BTreeSet<Sym>> {
    let mut out: BTreeMap<Sym, BTreeSet<Sym>> = BTreeMap::new();
    for (k, v) in m {
        out.entry(*v).or_default().insert(*k);
    }
    out
}

fn fmt_set(s: &BTreeSet<String>) -> String {
    if s.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", s.iter().cloned().collect::<Vec<_>>().join(", "))
    }
}

fn comma_join(s: &BTreeSet<String>) -> String {
    s.iter().cloned().collect::<Vec<_>>().join(", ")
}
