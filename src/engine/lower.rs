use super::*;

pub fn lower_decls(raw: Vec<Decl<String>>, interner: &mut Interner) -> Vec<Decl<Sym>> {
    raw.into_iter().map(|d| lower_decl(d, interner)).collect()
}

fn lower_decl(d: Decl<String>, interner: &mut Interner) -> Decl<Sym> {
    match d {
        Decl::Interface(i) => Decl::Interface(lower_interface(i, interner)),
        Decl::Defer(d) => Decl::Defer(lower_defer(d, interner)),
        Decl::Schema(s) => Decl::Schema(lower_schema(s, interner)),
    }
}

fn lower_type(ty: Type<String>, interner: &mut Interner) -> Type<Sym> {
    match ty {
        Type::Int => Type::Int,
        Type::Str => Type::Str,
        Type::Bool => Type::Bool,
        Type::Named(s) => Type::Named(interner.intern(&s)),
    }
}

fn lower_param(p: Param<String>, interner: &mut Interner) -> Param<Sym> {
    Param {
        name: interner.intern(&p.name),
        ty: lower_type(p.ty, interner),
    }
}

fn lower_params(ps: Vec<Param<String>>, interner: &mut Interner) -> Vec<Param<Sym>> {
    ps.into_iter().map(|p| lower_param(p, interner)).collect()
}

fn lower_expr(e: Expr<String>, interner: &mut Interner) -> Expr<Sym> {
    match e {
        Expr::LitInt(n) => Expr::LitInt(n),
        Expr::LitStr(s) => Expr::LitStr(s),
        Expr::LitBool(b) => Expr::LitBool(b),
        Expr::Var(s) => Expr::Var(interner.intern(&s)),
        Expr::Field(base, name) => {
            Expr::Field(Box::new(lower_expr(*base, interner)), interner.intern(&name))
        }
        Expr::BinOp(op, l, r) => Expr::BinOp(
            op,
            Box::new(lower_expr(*l, interner)),
            Box::new(lower_expr(*r, interner)),
        ),
        Expr::UnOp(op, inner) => Expr::UnOp(op, Box::new(lower_expr(*inner, interner))),
        Expr::Construct(name, args) => Expr::Construct(
            interner.intern(&name),
            args.into_iter().map(|a| lower_expr(a, interner)).collect(),
        ),
    }
}

fn lower_transition(t: Transition<String>, interner: &mut Interner) -> Transition<Sym> {
    Transition {
        target_pos: interner.intern(&t.target_pos),
        args: t.args.into_iter().map(|e| lower_expr(e, interner)).collect(),
    }
}

fn lower_direction(d: Direction<String>, interner: &mut Interner) -> Direction<Sym> {
    Direction {
        name: interner.intern(&d.name),
        params: lower_params(d.params, interner),
        guard: d.guard.map(|g| lower_expr(g, interner)),
        transition: d.transition.map(|t| lower_transition(t, interner)),
    }
}

fn lower_position(p: Position<String>, interner: &mut Interner) -> Position<Sym> {
    Position {
        name: interner.intern(&p.name),
        params: lower_params(p.params, interner),
        guard: p.guard.map(|g| lower_expr(g, interner)),
        directions: p
            .directions
            .into_iter()
            .map(|d| lower_direction(d, interner))
            .collect(),
    }
}

fn lower_interface(i: Interface<String>, interner: &mut Interner) -> Interface<Sym> {
    Interface {
        name: interner.intern(&i.name),
        params: lower_params(i.params, interner),
        positions: i
            .positions
            .into_iter()
            .map(|p| lower_position(p, interner))
            .collect(),
    }
}

fn lower_pattern(p: Pattern<String>, interner: &mut Interner) -> Pattern<Sym> {
    match p {
        Pattern::Wildcard => Pattern::Wildcard,
        Pattern::Bind(name) => Pattern::Bind(interner.intern(&name)),
    }
}

fn lower_dir_ref(r: DirRef<String>, interner: &mut Interner) -> DirRef<Sym> {
    match r {
        DirRef::Named(name) => DirRef::Named(interner.intern(&name)),
        DirRef::Abstract { src_pos, src_pattern, tgt_pos, tgt_args } => DirRef::Abstract {
            src_pos: interner.intern(&src_pos),
            src_pattern: src_pattern
                .into_iter()
                .map(|p| lower_pattern(p, interner))
                .collect(),
            tgt_pos: interner.intern(&tgt_pos),
            tgt_args: tgt_args.into_iter().map(|e| lower_expr(e, interner)).collect(),
        },
    }
}

fn lower_dir_mapping(m: DirMapping<String>, interner: &mut Interner) -> DirMapping<Sym> {
    DirMapping {
        target_dir: lower_dir_ref(m.target_dir, interner),
        source_dir: lower_dir_ref(m.source_dir, interner),
    }
}

fn lower_defer_entry(e: DeferEntry<String>, interner: &mut Interner) -> DeferEntry<Sym> {
    DeferEntry {
        source_pos: interner.intern(&e.source_pos),
        source_pattern: e
            .source_pattern
            .into_iter()
            .map(|p| lower_pattern(p, interner))
            .collect(),
        source_guard: e.source_guard.map(|g| lower_expr(g, interner)),
        target_pos: interner.intern(&e.target_pos),
        target_args: e.target_args.into_iter().map(|a| lower_expr(a, interner)).collect(),
        directions: e.directions.into_iter().map(|m| lower_dir_mapping(m, interner)).collect(),
    }
}

fn lower_defer(d: Defer<String>, interner: &mut Interner) -> Defer<Sym> {
    Defer {
        name: interner.intern(&d.name),
        source: interner.intern(&d.source),
        target: interner.intern(&d.target),
        entries: d.entries.into_iter().map(|e| lower_defer_entry(e, interner)).collect(),
    }
}

fn lower_variant(v: Variant<String>, interner: &mut Interner) -> Variant<Sym> {
    Variant {
        name: interner.intern(&v.name),
        params: lower_params(v.params, interner),
    }
}

fn lower_schema(s: Schema<String>, interner: &mut Interner) -> Schema<Sym> {
    let body = match s.body {
        SchemaBody::Record(fields) => SchemaBody::Record(lower_params(fields, interner)),
        SchemaBody::Sum(variants) => {
            SchemaBody::Sum(variants.into_iter().map(|v| lower_variant(v, interner)).collect())
        }
    };
    Schema {
        name: interner.intern(&s.name),
        body,
    }
}
