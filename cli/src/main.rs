use std::collections::BTreeSet;

use poly_engine::query::{
    Answer, Goal, IndexSlot, Query, Slot, Term, Value, VarGen, VarId,
};
use poly_engine::{Bindings, Engine, EngineError, Expr, Sym};

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
    print!("{}", eng.fmt_facts());
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
    let Some(eng) = load(path) else { return 1 };
    match rest.split_first() {
        Some((flag, tail)) if flag == "--explain" => match tail {
            [iface, pos] => run_explain(&eng, iface, pos),
            _ => {
                eprintln!("usage: poly query <file> --explain <interface> <position>");
                1
            }
        },
        Some((flag, tail)) if flag == "--locate" => match tail {
            [action] => run_locate(&eng, action),
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

fn run_explain(eng: &Engine, iface: &str, pos: &str) -> i32 {
    let Some(i_sym) = eng.interner.find(iface) else {
        eprintln!("unknown interface: {iface}");
        return 1;
    };
    if !eng.interfaces.contains_key(&i_sym) {
        eprintln!("unknown interface: {iface}");
        return 1;
    }
    let Some(p_sym) = eng.interner.find(pos) else {
        eprintln!("unknown position: {iface}.{pos}");
        return 1;
    };
    let env = Bindings::default();

    // Available actions at (iface, pos).
    let mut g = VarGen::new();
    let action_v = g.fresh();
    let actions_q = Query::single(vec![Goal::Direction {
        iface: Term::Sym(i_sym),
        position: Term::Sym(p_sym),
        action: Term::Var(action_v),
        params: Slot::Anon,
        guard: Slot::Anon,
    }]);
    let actions = eng.query(&actions_q, &env);

    // Forward defers (this iface as defer source).
    let mut g = VarGen::new();
    let fd = g.fresh();
    let f_tgt = g.fresh();
    let f_entry = g.fresh();
    let f_tgt_pos = g.fresh();
    let fwd_q = Query::single(vec![
        Goal::Defer {
            defer: Term::Var(fd),
            source: Term::Sym(i_sym),
            target: Term::Var(f_tgt),
        },
        Goal::DeferEntry {
            defer: Term::Var(fd),
            entry_idx: IndexSlot::Var(f_entry),
            source_pos: Term::Sym(p_sym),
            src_pattern: Slot::Anon,
            src_guard: Slot::Anon,
            target_pos: Term::Var(f_tgt_pos),
            target_args: Slot::Anon,
        },
    ]);
    let forward = eng.query(&fwd_q, &env);

    // Backward defers (this iface as defer target).
    let mut g = VarGen::new();
    let bd = g.fresh();
    let b_src = g.fresh();
    let b_entry = g.fresh();
    let b_src_pos = g.fresh();
    let bwd_q = Query::single(vec![
        Goal::Defer {
            defer: Term::Var(bd),
            source: Term::Var(b_src),
            target: Term::Sym(i_sym),
        },
        Goal::DeferEntry {
            defer: Term::Var(bd),
            entry_idx: IndexSlot::Var(b_entry),
            source_pos: Term::Var(b_src_pos),
            src_pattern: Slot::Anon,
            src_guard: Slot::Anon,
            target_pos: Term::Sym(p_sym),
            target_args: Slot::Anon,
        },
    ]);
    let backward = eng.query(&bwd_q, &env);

    println!("{iface} at {pos}");
    print!("  available actions: {{");
    let mut seen: BTreeSet<Sym> = BTreeSet::new();
    let mut first = true;
    for a in &actions {
        if let Some(Value::Sym(s)) = a.subst.get(&action_v) {
            if seen.insert(*s) {
                if !first { print!(", "); }
                print!("{}", eng.resolve(*s));
                first = false;
            }
        }
    }
    println!("}}");

    if !forward.is_empty() {
        println!();
        println!("  forward defers:");
        for ans in &forward {
            let d = sym_of(ans, fd);
            let t = sym_of(ans, f_tgt);
            let tp = sym_of(ans, f_tgt_pos);
            println!(
                "    {} : {} -> {} ({}.{} -> {}.{})",
                eng.resolve(d), eng.resolve(i_sym), eng.resolve(t),
                eng.resolve(i_sym), eng.resolve(p_sym),
                eng.resolve(t), eng.resolve(tp),
            );
            print_residual(eng, &ans.residual, "      ");
        }
    }

    if !backward.is_empty() {
        println!();
        println!("  backward defers:");
        for ans in &backward {
            let d = sym_of(ans, bd);
            let s = sym_of(ans, b_src);
            let sp = sym_of(ans, b_src_pos);
            println!(
                "    {} : {} -> {} ({}.{} -> {}.{})",
                eng.resolve(d), eng.resolve(s), eng.resolve(i_sym),
                eng.resolve(s), eng.resolve(sp),
                eng.resolve(i_sym), eng.resolve(p_sym),
            );
            print_residual(eng, &ans.residual, "      ");
        }
    }

    0
}

fn run_locate(eng: &Engine, action: &str) -> i32 {
    let Some(a_sym) = eng.interner.find(action) else {
        println!("action `{action}` is not available at any position");
        return 1;
    };
    let mut g = VarGen::new();
    let i_v = g.fresh();
    let p_v = g.fresh();
    let q = Query::single(vec![
        Goal::Direction {
            iface: Term::Var(i_v),
            position: Term::Var(p_v),
            action: Term::Sym(a_sym),
            params: Slot::Anon,
            guard: Slot::Anon,
        },
        Goal::Position {
            iface: Term::Var(i_v),
            position: Term::Var(p_v),
            params: Slot::Anon,
            guard: Slot::Anon,
        },
    ]);
    let answers = eng.query(&q, &Bindings::default());
    if answers.is_empty() {
        println!("action `{action}` is not available at any position");
        return 1;
    }
    println!("action `{action}` is available at:");
    for ans in &answers {
        let i = sym_of(ans, i_v);
        let p = sym_of(ans, p_v);
        print!("  {}.{}", eng.resolve(i), eng.resolve(p));
        print_residual_inline(eng, &ans.residual);
        println!();
    }
    0
}

fn sym_of(ans: &Answer, v: VarId) -> Sym {
    match ans.subst.get(&v) {
        Some(Value::Sym(s)) => *s,
        _ => panic!("expected Sym binding for variable"),
    }
}

fn print_residual(eng: &Engine, residual: &[Expr<Sym>], indent: &str) {
    if residual.is_empty() {
        return;
    }
    let parts: Vec<String> = residual.iter().map(|e| eng.fmt_expr(e, 0)).collect();
    println!("{indent}if ({})", parts.join(" and "));
}

fn print_residual_inline(eng: &Engine, residual: &[Expr<Sym>]) {
    if residual.is_empty() {
        return;
    }
    let parts: Vec<String> = residual.iter().map(|e| eng.fmt_expr(e, 0)).collect();
    print!(" if ({})", parts.join(" and "));
}
