pub mod parse;

use std::collections::{BTreeMap, BTreeSet};


// A polynomial interface: positions, each carrying a set of directions (actions).
#[derive(Clone, Debug)]
pub struct Interface {
    pub name: String,
    pub positions: BTreeMap<String, BTreeSet<String>>,
}

impl std::fmt::Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "interface {}", self.name)?;
        let entries: Vec<String> = self
            .positions
            .iter()
            .map(|(p, dirs)| {
                if dirs.is_empty() {
                    p.clone()
                } else {
                    format!(
                        "{p} {{ {} }}",
                        dirs.iter().cloned().collect::<Vec<_>>().join(", ")
                    )
                }
            })
            .collect();
        for (i, e) in entries.iter().enumerate() {
            let suffix = if i + 1 < entries.len() { "," } else { "" };
            write!(f, "\n    {e}{suffix}")?;
        }
        Ok(())
    }
}

// A defer is a polynomial lens p -> q:
//   forward map on positions: pos_map[src_pos] = tgt_pos
//   backward map on directions: dir_map[src_pos][tgt_dir] = src_dir
// (For each source position, every target direction at the image position
//  is mapped back to a source direction at the original position.)
#[derive(Clone, Debug)]
pub struct Defer {
    pub name: String,
    pub source: String,
    pub target: String,
    pub pos_map: BTreeMap<String, String>,
    pub dir_map: BTreeMap<String, BTreeMap<String, String>>,
}

impl std::fmt::Display for Defer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "defer {} : {} -> {}", self.name, self.source, self.target)?;
        let mappings: Vec<&String> = self.pos_map.keys().collect();
        for (i, src_pos) in mappings.iter().enumerate() {
            let tgt_pos = &self.pos_map[*src_pos];
            let body = match self.dir_map.get(*src_pos) {
                Some(map) if !map.is_empty() => {
                    // Group target directions that share the same source direction
                    // so they can be rendered as `Tgt | Tgt -> Src`.
                    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
                    for (tgt_dir, src_dir) in map {
                        grouped.entry(src_dir.clone()).or_default().push(tgt_dir.clone());
                    }
                    let lines: Vec<String> = grouped
                        .iter()
                        .map(|(src_dir, tgt_dirs)| {
                            format!("        {} -> {}", tgt_dirs.join(" | "), src_dir)
                        })
                        .collect();
                    format!(" {{\n{}\n    }}", lines.join(",\n"))
                }
                _ => " {}".to_string(),
            };
            let suffix = if i + 1 < mappings.len() { "," } else { "" };
            write!(f, "\n    {} -> {}{}{}", src_pos, tgt_pos, body, suffix)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Decl {
    Interface(Interface),
    Defer(Defer),
}

#[derive(Clone, Debug, Default)]
pub struct Engine {
    pub interfaces: BTreeMap<String, Interface>,
    pub defers: Vec<Defer>,
}

impl Engine {
    pub fn from_decls(decls: Vec<Decl>) -> Engine {
        let mut engine = Engine::default();
        for decl in decls {
            match decl {
                Decl::Interface(i) => { engine.interfaces.insert(i.name.clone(), i); }
                Decl::Defer(d) => engine.defers.push(d),
            }
        }
        engine
    }

    // Query (1): if `interface` is at `position`, what does that determine
    // about every other interface connected by a defer?
    pub fn explain_position(&self, interface: &str, position: &str) -> String {
        let mut out = String::new();
        let iface = match self.interfaces.get(interface) {
            Some(i) => i,
            None => return format!("unknown interface: {interface}\n"),
        };
        let actions = match iface.positions.get(position) {
            Some(a) => a,
            None => return format!("unknown position: {interface}.{position}\n"),
        };

        out.push_str(&format!("{interface} at {position}\n"));
        out.push_str(&format!("  available actions: {}\n", fmt_set(actions)));

        for d in &self.defers {
            if d.source == interface {
                if let Some(tgt) = d.pos_map.get(position) {
                    out.push_str(&format!(
                        "\n  via defer {} ({} -> {}):\n",
                        d.name, d.source, d.target
                    ));
                    out.push_str(&format!("    {} must be at {}\n", d.target, tgt));
                    if let Some(dm) = d.dir_map.get(position) {
                        let by_src = group_by_value(dm);
                        for (src_dir, tgt_dirs) in &by_src {
                            out.push_str(&format!(
                                "    action {} <- {} action(s) {{{}}}\n",
                                src_dir,
                                d.target,
                                comma_join(tgt_dirs)
                            ));
                        }
                    }
                }
            }

            if d.target == interface {
                let preimage: Vec<&String> = d
                    .pos_map
                    .iter()
                    .filter_map(|(s, t)| if t == position { Some(s) } else { None })
                    .collect();
                if !preimage.is_empty() {
                    out.push_str(&format!(
                        "\n  via defer {} ({} -> {}):\n",
                        d.name, d.source, d.target
                    ));
                    out.push_str(&format!(
                        "    {} could be at any of: {{{}}}\n",
                        d.source,
                        preimage.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ));
                    for s in &preimage {
                        if let Some(dm) = d.dir_map.get(*s) {
                            for (tgt_dir, src_dir) in dm {
                                out.push_str(&format!(
                                    "    if {}={}: choosing {}.{} corresponds to {}.{}\n",
                                    d.source, s, interface, tgt_dir, d.source, src_dir
                                ));
                            }
                        }
                    }
                }
            }
        }

        out
    }

    // Query (3): for what (interface, position) pairs is `action` available?
    pub fn locate_action(&self, action: &str) -> String {
        let mut hits = Vec::new();
        for (iname, iface) in &self.interfaces {
            for (pname, dirs) in &iface.positions {
                if dirs.contains(action) {
                    hits.push(format!("  {iname}.{pname}"));
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


fn group_by_value(m: &BTreeMap<String, String>) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (k, v) in m {
        out.entry(v.clone()).or_default().insert(k.clone());
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
