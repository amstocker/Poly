mod diagram;
mod diagram_old;
mod diagram_old2;
mod engine;
mod test;
mod object;

use chumsky::Parser;


fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<String> = if args.is_empty() {
        vec![
            "examples/implemented/test2.poly".to_string(),
            "examples/implemented/graph.poly".to_string(),
        ]
    } else {
        args
    };

    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            println!();
        }
        run(path);
    }
}

fn run(path: &str) {
    let raw = std::fs::read_to_string(path).expect("could not read source file");
    let src = engine::parse::strip_comments(&raw);

    let decls = match engine::parse::file().parse(src) {
        Ok(d) => d,
        Err(errs) => {
            for e in errs {
                eprintln!("parse error in {path}: {e:?}");
            }
            return;
        }
    };

    let eng = engine::Engine::from_decls(decls);

    println!("=== Loaded from {path} ===");
    for iface in eng.interfaces.values() {
        println!("{iface}");
    }
    for d in &eng.defers {
        println!("{d}");
    }
    println!();

    println!("=== Query 1: explain each position ===");
    for (iname, iface) in &eng.interfaces {
        for pname in iface.positions.keys() {
            print!("{}", eng.explain_position(iname, pname));
            println!();
        }
    }

    let mut all_actions = std::collections::BTreeSet::new();
    for iface in eng.interfaces.values() {
        for dirs in iface.positions.values() {
            for d in dirs {
                all_actions.insert(d.clone());
            }
        }
    }
    println!("=== Query 3: locate each action ===");
    for action in &all_actions {
        print!("{}", eng.locate_action(action));
    }
}
