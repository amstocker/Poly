mod engine;

use engine::eval::{Bindings, Value};
use engine::{Engine, EngineError, SchemaBody};

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

  poly facts <file>
      Project <file> into the relation tuples used by the (in-progress)
      query layer. One Datalog-style fact per line.

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

fn cmd_facts(args: &[String]) -> i32 {
    let path = match args {
        [p] => p,
        _ => {
            eprintln!("usage: poly facts <file>");
            return 1;
        }
    };
    let Some(eng) = load(path) else { return 1 };
    let facts = eng.facts();
    print!("{}", eng.fmt_facts(&facts));
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
        match parse_value(&eng, v) {
            Ok(val) => { bindings.insert(key, val); }
            Err(msg) => {
                eprintln!("could not parse value for {k}: {msg}");
                return 1;
            }
        }
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

fn parse_value(eng: &Engine, s: &str) -> Result<Value, String> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i64>() {
        return Ok(Value::Int(n));
    }
    if s == "true" {
        return Ok(Value::Bool(true));
    }
    if s == "false" {
        return Ok(Value::Bool(false));
    }
    if let Some((name, args_str)) = parse_construct_head(s) {
        let key = eng
            .interner
            .find(name)
            .ok_or_else(|| format!("unknown schema: {name}"))?;
        let schema = eng
            .schemas
            .get(&key)
            .ok_or_else(|| format!("unknown schema: {name}"))?;
        let params = match &schema.body {
            SchemaBody::Record(ps) => ps,
            SchemaBody::Sum(_) => {
                return Err(format!("sum constructors not yet supported: {name}"));
            }
        };
        let arg_strs = split_top_commas(args_str)?;
        if arg_strs.len() != params.len() {
            return Err(format!(
                "{name} expects {} arg(s), got {}",
                params.len(),
                arg_strs.len(),
            ));
        }
        let mut fields: std::collections::BTreeMap<engine::Sym, Value> =
            std::collections::BTreeMap::new();
        for (p, arg) in params.iter().zip(arg_strs.iter()) {
            fields.insert(p.name, parse_value(eng, arg)?);
        }
        return Ok(Value::Record { schema: key, fields });
    }
    let trimmed = s.trim_matches('"');
    Ok(Value::Str(trimmed.to_string()))
}

fn parse_construct_head(s: &str) -> Option<(&str, &str)> {
    let open = s.find('(')?;
    if !s.ends_with(')') {
        return None;
    }
    let name = s[..open].trim();
    if name.is_empty() {
        return None;
    }
    if !name.chars().next().unwrap().is_alphabetic() {
        return None;
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let inner = &s[open + 1..s.len() - 1];
    Some((name, inner))
}

fn split_top_commas(s: &str) -> Result<Vec<&str>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err("unbalanced parentheses".to_string());
                }
            }
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("unbalanced parentheses".to_string());
    }
    out.push(s[start..].trim());
    Ok(out)
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
