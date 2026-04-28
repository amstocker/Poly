mod diagram;
mod diagram_old;
mod diagram_old2;
mod engine;
mod test;
mod object;


fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<String> = if args.is_empty() {
        vec![
            "examples/implemented/test2.poly".to_string(),
            "examples/implemented/graph.poly".to_string(),
            "examples/grid.poly".to_string(),
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
    let src = std::fs::read_to_string(path).expect("could not read source file");

    let eng = match engine::Engine::load(&src) {
        Ok(e) => e,
        Err(errs) => {
            for e in errs {
                eprintln!("parse error in {path}: {e:?}");
            }
            return;
        }
    };

    println!("=== Loaded from {path} ===");
    for s in eng.schemas.values() {
        println!("{}", eng.fmt_schema(s));
    }
    for iface in eng.interfaces.values() {
        println!("{}", eng.fmt_interface(iface));
    }
    for d in &eng.defers {
        println!("{}", eng.fmt_defer(d));
    }
    println!();

    println!("=== Query 1: explain each position ===");
    for (iname_sym, iface) in &eng.interfaces {
        let iname = eng.resolve(*iname_sym).to_string();
        for pos in &iface.positions {
            let pname = eng.resolve(pos.name).to_string();
            print!("{}", eng.explain_position(&iname, &pname));
            println!();
        }
    }

    let mut all_actions = std::collections::BTreeSet::new();
    for iface in eng.interfaces.values() {
        for pos in &iface.positions {
            for dir in &pos.directions {
                all_actions.insert(eng.resolve(dir.name).to_string());
            }
        }
    }
    println!("=== Query 3: locate each action ===");
    for action in &all_actions {
        print!("{}", eng.locate_action(action));
    }
}
