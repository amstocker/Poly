use super::*;

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
        self.defers.iter().find(|d| d.name == sym).map(|d| self.fmt_defer(d))
    }

    pub fn fmt_schema(&self, s: &Schema<Sym>) -> String {
        let mut out = format!("schema {}", self.resolve(s.name));
        match &s.body {
            SchemaBody::Record(fields) => {
                for (i, p) in fields.iter().enumerate() {
                    let sep = if i + 1 < fields.len() { "," } else { "" };
                    out.push_str(&format!("\n    {}{}", self.fmt_param(p), sep));
                }
            }
            SchemaBody::Sum(variants) => {
                for (i, v) in variants.iter().enumerate() {
                    let sep = if i + 1 < variants.len() { "," } else { "" };
                    out.push_str(&format!("\n    {}{}", self.fmt_variant(v), sep));
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
            let sep = if i + 1 < iface.positions.len() { "," } else { "" };
            out.push_str(&format!("\n    {}{}", self.fmt_position(pos), sep));
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
        for (i, src_pos) in d.pos_map.keys().enumerate() {
            let tgt_pos = &d.pos_map[src_pos];
            let body = match d.dir_map.get(src_pos) {
                Some(map) if !map.is_empty() => {
                    use std::collections::BTreeMap;
                    let mut grouped: BTreeMap<Sym, Vec<Sym>> = BTreeMap::new();
                    for (tgt_dir, src_dir) in map {
                        grouped.entry(*src_dir).or_default().push(*tgt_dir);
                    }
                    let lines: Vec<String> = grouped
                        .iter()
                        .map(|(src_dir, tgt_dirs)| {
                            let parts: Vec<&str> =
                                tgt_dirs.iter().map(|s| self.resolve(*s)).collect();
                            format!("        {} -> {}", parts.join(" | "), self.resolve(*src_dir))
                        })
                        .collect();
                    format!(" {{\n{}\n    }}", lines.join(",\n"))
                }
                _ => " {}".to_string(),
            };
            let sep = if i + 1 < d.pos_map.len() { "," } else { "" };
            out.push_str(&format!(
                "\n    {} -> {}{}{}",
                self.resolve(*src_pos),
                self.resolve(*tgt_pos),
                body,
                sep,
            ));
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
            let all_simple = pos
                .directions
                .iter()
                .all(|d| d.params.is_empty() && d.guard.is_none() && d.transition.is_none());
            if all_simple {
                let names: Vec<&str> =
                    pos.directions.iter().map(|d| self.resolve(d.name)).collect();
                out.push_str(&format!(" {{ {} }}", names.join(", ")));
            } else {
                out.push_str(" {");
                for (i, dir) in pos.directions.iter().enumerate() {
                    let sep = if i + 1 < pos.directions.len() { "," } else { "" };
                    out.push_str(&format!("\n        {}{}", self.fmt_direction(dir), sep));
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

    fn fmt_type(&self, ty: &Type<Sym>) -> String {
        match ty {
            Type::Int => "Int".to_string(),
            Type::Str => "String".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Named(s) => self.resolve(*s).to_string(),
        }
    }

    pub fn fmt_expr(&self, e: &Expr<Sym>, parent_prec: u8) -> String {
        match e {
            Expr::LitInt(n) => n.to_string(),
            Expr::LitStr(s) => format!("\"{s}\""),
            Expr::LitBool(b) => b.to_string(),
            Expr::Var(s) => self.resolve(*s).to_string(),
            Expr::Field(base, name) => {
                format!("{}.{}", self.fmt_expr(base, PREC_ATOM), self.resolve(*name))
            }
            Expr::Construct(name, args) => {
                let parts: Vec<String> =
                    args.iter().map(|a| self.fmt_expr(a, PREC_TOP)).collect();
                format!("{}({})", self.resolve(*name), parts.join(", "))
            }
            Expr::BinOp(op, l, r) => {
                let p = bin_prec(*op);
                let s = format!(
                    "{} {} {}",
                    self.fmt_expr(l, p),
                    bin_str(*op),
                    self.fmt_expr(r, p),
                );
                if p < parent_prec { format!("({s})") } else { s }
            }
            Expr::UnOp(op, inner) => {
                let s = match op {
                    UnOp::Neg => format!("-{}", self.fmt_expr(inner, PREC_UNARY)),
                    UnOp::Not => format!("not {}", self.fmt_expr(inner, PREC_UNARY)),
                };
                if PREC_UNARY < parent_prec { format!("({s})") } else { s }
            }
        }
    }
}
