mod engine;

use engine::eval::{Bindings, Value};
use engine::Engine;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run_cli(&args);
    std::process::exit(code);
}

fn run_cli(args: &[String]) -> i32 {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (c.as_str(), r),
        None => {
            print_usage();
            return 1;
        }
    };
    match cmd {
        "show" => cmd_show(rest),
        "explain" => cmd_explain(rest),
        "locate" => cmd_locate(rest),
        "actions" => cmd_actions(rest),
        "step" => cmd_step(rest),
        "help" | "-h" | "--help" => {
            print_usage();
            0
        }
        _ => {
            eprintln!("unknown command: {cmd}\n");
            print_usage();
            1
        }
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  poly show <file>
      Print all schemas, interfaces, and defers in <file>.

  poly explain <file> <interface> <position>
      Show what is determined elsewhere when <interface> is at <position>.

  poly locate <file> <action>
      List every (interface, position) where <action> is available.

  poly actions <file> <interface> <position>
      List the actions available at <interface>.<position>.

  poly step <file> <interface> <position> <action> [name=value ...]
      Apply <action> at <interface>.<position> with the given parameter
      bindings; print the resulting position and bindings. Values may be
      integers, true/false, or quoted strings.

  poly help
      Print this message."
    );
}

fn load(path: &str) -> Option<Engine> {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not read {path}: {e}");
            return None;
        }
    };
    match Engine::load(&src) {
        Ok(e) => Some(e),
        Err(errs) => {
            for e in errs {
                eprintln!("parse error in {path}: {e:?}");
            }
            None
        }
    }
}

fn cmd_show(args: &[String]) -> i32 {
    let path = match args {
        [p] => p,
        _ => {
            eprintln!("usage: poly show <file>");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    for s in eng.schemas.values() {
        println!("{}", eng.fmt_schema(s));
    }
    for iface in eng.interfaces.values() {
        println!("{}", eng.fmt_interface(iface));
    }
    for d in &eng.defers {
        println!("{}", eng.fmt_defer(d));
    }
    0
}

fn cmd_explain(args: &[String]) -> i32 {
    let (path, iface, pos) = match args {
        [p, i, q] => (p, i, q),
        _ => {
            eprintln!("usage: poly explain <file> <interface> <position>");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    match eng.explain_position(iface, pos) {
        Ok(exp) => {
            print!("{}", eng.fmt_position_explanation(&exp));
            0
        }
        Err(err) => {
            eprintln!("{}", eng.fmt_query_error(&err));
            1
        }
    }
}

fn cmd_locate(args: &[String]) -> i32 {
    let (path, action) = match args {
        [p, a] => (p, a),
        _ => {
            eprintln!("usage: poly locate <file> <action>");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    let locs = eng.locate_action(action);
    print!("{}", eng.fmt_action_locations(&locs));
    if locs.locations.is_empty() {
        1
    } else {
        0
    }
}

fn cmd_step(args: &[String]) -> i32 {
    let (path, iface, pos, action, rest) = match args {
        [p, i, q, a, rest @ ..] => (p, i, q, a, rest),
        _ => {
            eprintln!("usage: poly step <file> <interface> <position> <action> [name=value ...]");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    let mut bindings: Bindings = std::collections::BTreeMap::new();
    for kv in rest {
        let Some((k, v)) = kv.split_once('=') else {
            eprintln!("expected name=value, got: {kv}");
            return 1;
        };
        let Some(key) = eng.interner.find(k) else {
            eprintln!("unknown parameter: {k}");
            return 1;
        };
        bindings.insert(key, parse_value(v));
    }
    match eng.next_position(iface, pos, action, bindings) {
        Ok(step) => {
            print!("{}", eng.fmt_step(&step));
            0
        }
        Err(err) => {
            eprintln!("{}", eng.fmt_query_error(&err));
            1
        }
    }
}

fn parse_value(s: &str) -> Value {
    if let Ok(n) = s.parse::<i64>() {
        return Value::Int(n);
    }
    match s {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            let trimmed = s.trim_matches('"');
            Value::Str(trimmed.to_string())
        }
    }
}

fn cmd_actions(args: &[String]) -> i32 {
    let (path, iface, pos) = match args {
        [p, i, q] => (p, i, q),
        _ => {
            eprintln!("usage: poly actions <file> <interface> <position>");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    match eng.explain_position(iface, pos) {
        Ok(exp) => {
            for a in &exp.actions {
                println!("{}", eng.resolve(*a));
            }
            0
        }
        Err(err) => {
            eprintln!("{}", eng.fmt_query_error(&err));
            1
        }
    }
}
