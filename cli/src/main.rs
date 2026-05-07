use poly_engine::api::{ActionLocation, ApiError, DeferLink, ExplainResult, Poly};
use poly_engine::{Engine, EngineError};

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
        "facts" => cmd_facts(rest),
        "query" => cmd_query(rest),
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

  poly facts <file>
      Project <file> into the relation tuples used by the query layer.
      One Datalog-style fact per line.

  poly query <file> --explain <interface> <position>
      Show what is determined elsewhere when <interface> is at <position>:
      available actions plus forward and backward defer links.

  poly query <file> --locate <action>
      List every (interface, position) where <action> is available, with
      its enabling residual constraint (if any).

  poly help
      Print this message."
    );
}

fn load(path: &str) -> Option<Poly> {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not read {path}: {e}");
            return None;
        }
    };
    match Poly::from_source(&src) {
        Ok(p) => Some(p),
        Err(EngineError::Parse(errs)) => {
            for e in errs {
                eprintln!("parse error in {path}: {e:?}");
            }
            None
        }
        Err(EngineError::Validate(msgs)) => {
            for m in msgs {
                eprintln!("validation error in {path}: {m}");
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
    let Some(poly) = load(path) else { return 1 };
    let eng = poly.engine();
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

fn cmd_facts(args: &[String]) -> i32 {
    let path = match args {
        [p] => p,
        _ => {
            eprintln!("usage: poly facts <file>");
            return 1;
        }
    };
    let Some(poly) = load(path) else { return 1 };
    print!("{}", poly.engine().fmt_facts(poly.facts()));
    0
}

fn cmd_query(args: &[String]) -> i32 {
    let (path, rest) = match args.split_first() {
        Some((p, r)) => (p.as_str(), r),
        None => {
            eprintln!("usage: poly query <file> --explain <interface> <position>");
            eprintln!("       poly query <file> --locate <action>");
            return 1;
        }
    };
    let Some(poly) = load(path) else { return 1 };
    match rest.split_first() {
        Some((flag, tail)) if flag == "--explain" => match tail {
            [iface, pos] => run_explain(&poly, iface, pos),
            _ => {
                eprintln!("usage: poly query <file> --explain <interface> <position>");
                1
            }
        },
        Some((flag, tail)) if flag == "--locate" => match tail {
            [action] => run_locate(&poly, action),
            _ => {
                eprintln!("usage: poly query <file> --locate <action>");
                1
            }
        },
        _ => {
            eprintln!("usage: poly query <file> --explain <interface> <position>");
            eprintln!("       poly query <file> --locate <action>");
            1
        }
    }
}

fn run_explain(poly: &Poly, iface: &str, pos: &str) -> i32 {
    let result = match poly.explain_position(iface, pos) {
        Ok(r) => r,
        Err(ApiError::UnknownInterface(name)) => {
            eprintln!("unknown interface: {name}");
            return 1;
        }
        Err(ApiError::UnknownPosition { iface, position }) => {
            eprintln!("unknown position: {iface}.{position}");
            return 1;
        }
    };
    print_explain(poly, &result, iface, pos);
    0
}

fn print_explain(poly: &Poly, r: &ExplainResult, iface_label: &str, pos_label: &str) {
    println!("{iface_label} at {pos_label}");

    print!("  available actions: {{");
    for (i, a) in r.actions.iter().enumerate() {
        if i > 0 { print!(", "); }
        print!("{}", poly.resolve(*a));
    }
    println!("}}");

    if !r.forward.is_empty() {
        println!();
        println!("  forward defers:");
        for link in &r.forward {
            print_defer_link(poly, link);
        }
    }

    if !r.backward.is_empty() {
        println!();
        println!("  backward defers:");
        for link in &r.backward {
            print_defer_link(poly, link);
        }
    }
}

fn print_defer_link(poly: &Poly, link: &DeferLink) {
    println!(
        "    {} : {} -> {} ({}.{} -> {}.{})",
        poly.resolve(link.defer),
        poly.resolve(link.source_iface),
        poly.resolve(link.target_iface),
        poly.resolve(link.source_iface),
        poly.resolve(link.source_pos),
        poly.resolve(link.target_iface),
        poly.resolve(link.target_pos),
    );
    print_residual(poly.engine(), &link.residual, "      ");
}

fn run_locate(poly: &Poly, action: &str) -> i32 {
    let answers = poly.locate_action(action);
    if answers.is_empty() {
        println!("action `{action}` is not available at any position");
        return 1;
    }
    println!("action `{action}` is available at:");
    for ans in &answers {
        print_action_location(poly, ans);
    }
    0
}

fn print_action_location(poly: &Poly, loc: &ActionLocation) {
    print!("  {}.{}", poly.resolve(loc.iface), poly.resolve(loc.position));
    print_residual_inline(poly.engine(), &loc.residual);
    println!();
}

fn print_residual(eng: &Engine, residual: &[poly_engine::Expr<poly_engine::Sym>], indent: &str) {
    if residual.is_empty() {
        return;
    }
    let parts: Vec<String> = residual.iter().map(|e| eng.fmt_expr(e, 0)).collect();
    println!("{indent}if ({})", parts.join(" and "));
}

fn print_residual_inline(eng: &Engine, residual: &[poly_engine::Expr<poly_engine::Sym>]) {
    if residual.is_empty() {
        return;
    }
    let parts: Vec<String> = residual.iter().map(|e| eng.fmt_expr(e, 0)).collect();
    print!(" if ({})", parts.join(" and "));
}
