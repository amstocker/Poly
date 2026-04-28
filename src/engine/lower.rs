use std::collections::BTreeMap;
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

fn lower_defer(d: Defer<String>, interner: &mut Interner) -> Defer<Sym> {
    let mut pos_map = BTreeMap::new();
    for (k, v) in d.pos_map {
        pos_map.insert(interner.intern(&k), interner.intern(&v));
    }
    let mut dir_map: BTreeMap<Sym, BTreeMap<Sym, Sym>> = BTreeMap::new();
    for (k, inner) in d.dir_map {
        let k_sym = interner.intern(&k);
        let mut new_inner = BTreeMap::new();
        for (tk, tv) in inner {
            new_inner.insert(interner.intern(&tk), interner.intern(&tv));
        }
        dir_map.insert(k_sym, new_inner);
    }
    Defer {
        name: interner.intern(&d.name),
        source: interner.intern(&d.source),
        target: interner.intern(&d.target),
        pos_map,
        dir_map,
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
