use chumsky::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use super::{Decl, Defer, Interface};


fn ident() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    text::ident().padded()
}

fn keyword(kw: &'static str) -> impl Parser<char, (), Error = Simple<char>> + Clone {
    text::ident()
        .try_map(move |s: String, span| {
            if s.as_str() == kw {
                Ok(())
            } else {
                Err(Simple::custom(span, format!("expected `{kw}`, got `{s}`")))
            }
        })
        .padded()
}

// One direction in an interface body — `Name` (plain) or `Name -> NextPos` (transition).
fn direction() -> impl Parser<char, (String, Option<String>), Error = Simple<char>> {
    ident()
        .then(just("->").padded().ignore_then(ident()).or_not())
}

#[derive(Clone)]
struct ParsedPosition {
    name: String,
    dirs: Vec<(String, Option<String>)>,
}

fn position() -> impl Parser<char, ParsedPosition, Error = Simple<char>> {
    ident()
        .then(
            direction()
                .separated_by(just(',').padded())
                .delimited_by(just('{').padded(), just('}').padded())
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .map(|(name, dirs)| ParsedPosition { name, dirs })
}

// An interface declaration. If any direction has a transition (`Action -> NextPos`),
// the interface is treated as state-machine sugar and desugared into:
//   - the external interface `<name>` (positions = states, directions = the
//     declared action names, transitions stripped)
//   - the internal interface `<name>.internal` — the universal state machine
//     polynomial on these states. Same positions, but at each state s the
//     directions are {s=>t : t in states}: one direction per conceivable
//     outgoing transition.
//   - a defer `<name>.run : <name>.internal -> <name>` realizing each declared
//     action `a -> dest` at state s as the abstract transition `s=>dest`.
// The space of all such defers (with the internal universal fixed) is the
// space of state machines on this state set; the source interface declares
// the structural possibility space, the defer picks out one realization.
fn interface_decls() -> impl Parser<char, Vec<Decl>, Error = Simple<char>> {
    keyword("interface")
        .ignore_then(ident())
        .then(position().separated_by(just(',').padded()))
        .map(|(name, entries)| desugar_interface(name, entries))
}

fn desugar_interface(name: String, entries: Vec<ParsedPosition>) -> Vec<Decl> {
    let has_transitions = entries.iter().any(|e| e.dirs.iter().any(|(_, t)| t.is_some()));

    // Universe of states: declared positions plus any transition target that
    // was referenced but not declared (treated as a terminal position).
    let mut states: BTreeSet<String> = entries.iter().map(|e| e.name.clone()).collect();
    if has_transitions {
        for e in &entries {
            for (_action, transition) in &e.dirs {
                if let Some(t) = transition {
                    states.insert(t.clone());
                }
            }
        }
    }

    // External interface: positions are states; directions are the declared
    // action names (transitions stripped). Inferred (target-only) states get
    // empty direction sets.
    let mut target_positions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for s in &states {
        target_positions.insert(s.clone(), BTreeSet::new());
    }
    for e in &entries {
        let dirs: BTreeSet<String> = e.dirs.iter().map(|(d, _)| d.clone()).collect();
        target_positions.insert(e.name.clone(), dirs);
    }
    let target = Interface { name: name.clone(), positions: target_positions };

    if !has_transitions {
        return vec![Decl::Interface(target)];
    }

    // Internal interface — the universal state machine on `states`.
    let mut internal_positions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for s in &states {
        let mut dirs = BTreeSet::new();
        for t in &states {
            dirs.insert(format!("{s}=>{t}"));
        }
        internal_positions.insert(s.clone(), dirs);
    }

    // Defer Graph.internal -> Graph: identity on positions; at each state s,
    // each declared action `a -> dest` is realized as the transition `s=>dest`.
    let mut pos_map: BTreeMap<String, String> = BTreeMap::new();
    let mut dir_map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for s in &states {
        pos_map.insert(s.clone(), s.clone());
    }
    for e in &entries {
        let mut inner: BTreeMap<String, String> = BTreeMap::new();
        for (action, transition) in &e.dirs {
            let dest = transition.clone().unwrap_or_else(|| e.name.clone());
            inner.insert(action.clone(), format!("{}=>{}", e.name, dest));
        }
        dir_map.insert(e.name.clone(), inner);
    }

    let internal_name = format!("{name}.internal");
    let internal = Interface { name: internal_name.clone(), positions: internal_positions };
    let defer = Defer {
        name: format!("{name}.run"),
        source: internal_name,
        target: name,
        pos_map,
        dir_map,
    };

    vec![Decl::Interface(target), Decl::Interface(internal), Decl::Defer(defer)]
}

fn dir_mapping() -> impl Parser<char, (Vec<String>, String), Error = Simple<char>> {
    ident()
        .separated_by(just('|').padded())
        .then_ignore(just("->").padded())
        .then(ident())
}

fn pos_mapping(
) -> impl Parser<char, (String, String, Vec<(Vec<String>, String)>), Error = Simple<char>> {
    ident()
        .then_ignore(just("->").padded())
        .then(ident())
        .then(
            dir_mapping()
                .separated_by(just(',').padded())
                .delimited_by(just('{').padded(), just('}').padded()),
        )
        .map(|((src, tgt), dirs)| (src, tgt, dirs))
}

fn defer() -> impl Parser<char, Defer, Error = Simple<char>> {
    keyword("defer")
        .ignore_then(ident())
        .then_ignore(just(':').padded())
        .then(ident())
        .then_ignore(just("->").padded())
        .then(ident())
        .then(pos_mapping().separated_by(just(',').padded()))
        .map(|(((name, source), target), mappings)| {
            let mut pos_map = BTreeMap::new();
            let mut dir_map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
            for (src, tgt, dirs) in mappings {
                pos_map.insert(src.clone(), tgt);
                let mut inner = BTreeMap::new();
                for (tgt_dirs, src_dir) in dirs {
                    for tgt_dir in tgt_dirs {
                        inner.insert(tgt_dir, src_dir.clone());
                    }
                }
                dir_map.insert(src, inner);
            }
            Defer { name, source, target, pos_map, dir_map }
        })
}

pub fn file() -> impl Parser<char, Vec<Decl>, Error = Simple<char>> {
    let interface = interface_decls();
    let defer_decl = defer().map(|d| vec![Decl::Defer(d)]);
    let decl = interface.or(defer_decl);
    decl.padded()
        .repeated()
        .map(|chunks: Vec<Vec<Decl>>| chunks.into_iter().flatten().collect())
        .then_ignore(end())
}

// Strips `# ...` line comments. Naive (does not understand strings) but sufficient
// for the current language subset.
pub fn strip_comments(src: &str) -> String {
    src.lines()
        .map(|line| match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
