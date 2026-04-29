use chumsky::prelude::*;

use super::{BinOp, Decl, Defer, DeferEntry, DirMapping, DirRef, Direction, Expr, Interface, Param,
    Pattern, Position, Schema, SchemaBody, Transition, Type, UnOp, Variant};


// ============================================================================
// Lexical helpers
// ============================================================================

fn ws() -> impl Parser<char, (), Error = Simple<char>> + Clone {
    let line_comment = just('#')
        .ignore_then(none_of::<_, _, Simple<char>>("\n").repeated())
        .ignored();
    let space = filter(|c: &char| c.is_whitespace()).ignored();
    line_comment.or(space).repeated().ignored()
}

fn ident() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    text::ident().padded_by(ws())
}

fn qualified_ident() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    text::ident()
        .then(just("::").ignore_then(text::ident()).repeated())
        .map(|(head, rest): (String, Vec<String>)| {
            if rest.is_empty() {
                head
            } else {
                let mut s = head;
                for part in rest {
                    s.push_str("::");
                    s.push_str(&part);
                }
                s
            }
        })
        .padded_by(ws())
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
        .padded_by(ws())
}


// ============================================================================
// Types and parameters
// ============================================================================

fn type_parser() -> impl Parser<char, Type<String>, Error = Simple<char>> + Clone {
    ident().map(|s: String| match s.as_str() {
        "Int" => Type::Int,
        "String" => Type::Str,
        "Bool" => Type::Bool,
        _ => Type::Named(s),
    })
}

fn param() -> impl Parser<char, Param<String>, Error = Simple<char>> + Clone {
    ident()
        .then_ignore(just(':').padded_by(ws()))
        .then(type_parser())
        .map(|(name, ty)| Param { name, ty })
}

fn param_list() -> impl Parser<char, Vec<Param<String>>, Error = Simple<char>> + Clone {
    param()
        .separated_by(just(',').padded_by(ws()))
        .delimited_by(just('[').padded_by(ws()), just(']').padded_by(ws()))
        .or_not()
        .map(|opt| opt.unwrap_or_default())
}


// ============================================================================
// Expression parser
// ============================================================================

fn expr_parser() -> impl Parser<char, Expr<String>, Error = Simple<char>> + Clone {
    recursive(|expr| {
        let lit_int = text::int::<_, Simple<char>>(10)
            .padded_by(ws())
            .try_map(|s: String, span| {
                s.parse::<i64>()
                    .map(Expr::LitInt)
                    .map_err(|e| Simple::custom(span, e.to_string()))
            });

        let lit_str = none_of::<_, _, Simple<char>>("\"")
            .repeated()
            .collect::<String>()
            .delimited_by(just('"'), just('"'))
            .map(Expr::LitStr)
            .padded_by(ws());

        let constructor = text::ident()
            .then(
                expr.clone()
                    .separated_by(just(',').padded_by(ws()))
                    .delimited_by(just('(').padded_by(ws()), just(')').padded_by(ws())),
            )
            .map(|(name, args): (String, Vec<Expr<String>>)| Expr::Construct(name, args))
            .padded_by(ws());

        let id_or_kw = text::ident::<_, Simple<char>>()
            .padded_by(ws())
            .map(|s: String| match s.as_str() {
                "true" => Expr::LitBool(true),
                "false" => Expr::LitBool(false),
                _ => Expr::Var(s),
            });

        let parens = expr
            .clone()
            .delimited_by(just('(').padded_by(ws()), just(')').padded_by(ws()));

        let atom = constructor.or(id_or_kw).or(lit_int).or(lit_str).or(parens);

        let postfix = atom
            .then(just('.').padded_by(ws()).ignore_then(text::ident()).repeated())
            .foldl(|base, field: String| Expr::Field(Box::new(base), field));

        let unary = recursive(|unary| {
            choice((
                just('-')
                    .padded_by(ws())
                    .ignore_then(unary.clone())
                    .map(|e| Expr::UnOp(UnOp::Neg, Box::new(e))),
                keyword("not")
                    .ignore_then(unary)
                    .map(|e| Expr::UnOp(UnOp::Not, Box::new(e))),
                postfix.clone(),
            ))
        });

        let mul_op = choice((
            just('*').padded_by(ws()).to(BinOp::Mul),
            just('/').padded_by(ws()).to(BinOp::Div),
            just('%').padded_by(ws()).to(BinOp::Mod),
        ));
        let mul = unary
            .clone()
            .then(mul_op.then(unary).repeated())
            .foldl(|l, (op, r)| Expr::BinOp(op, Box::new(l), Box::new(r)));

        let add_op = choice((
            just('+').padded_by(ws()).to(BinOp::Add),
            just('-').padded_by(ws()).to(BinOp::Sub),
        ));
        let add = mul
            .clone()
            .then(add_op.then(mul).repeated())
            .foldl(|l, (op, r)| Expr::BinOp(op, Box::new(l), Box::new(r)));

        let cmp_op = choice((
            just("==").padded_by(ws()).to(BinOp::Eq),
            just("!=").padded_by(ws()).to(BinOp::Neq),
            just("<=").padded_by(ws()).to(BinOp::Le),
            just(">=").padded_by(ws()).to(BinOp::Ge),
            just("<").padded_by(ws()).to(BinOp::Lt),
            just(">").padded_by(ws()).to(BinOp::Gt),
        ));
        let cmp = add
            .clone()
            .then(cmp_op.then(add).repeated())
            .map(|(first, rest)| {
                if rest.is_empty() {
                    return first;
                }
                let mut parts: Vec<Expr<String>> = Vec::new();
                let mut prev = first;
                for (op, next) in rest {
                    let cmp_node =
                        Expr::BinOp(op, Box::new(prev.clone()), Box::new(next.clone()));
                    parts.push(cmp_node);
                    prev = next;
                }
                parts
                    .into_iter()
                    .reduce(|a, b| Expr::BinOp(BinOp::And, Box::new(a), Box::new(b)))
                    .unwrap()
            });

        let and_lvl = cmp
            .clone()
            .then(keyword("and").to(BinOp::And).then(cmp).repeated())
            .foldl(|l, (op, r)| Expr::BinOp(op, Box::new(l), Box::new(r)));

        and_lvl
            .clone()
            .then(keyword("or").to(BinOp::Or).then(and_lvl).repeated())
            .foldl(|l, (op, r)| Expr::BinOp(op, Box::new(l), Box::new(r)))
    })
}


// ============================================================================
// Transition target
// ============================================================================

fn transition_parser() -> impl Parser<char, Transition<String>, Error = Simple<char>> + Clone {
    let arg_list = expr_parser()
        .separated_by(just(',').padded_by(ws()))
        .delimited_by(just('[').padded_by(ws()), just(']').padded_by(ws()))
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    ident()
        .then(arg_list)
        .map(|(target_pos, args)| Transition { target_pos, args })
}


// ============================================================================
// Direction
// ============================================================================

fn direction_parser() -> impl Parser<char, Direction<String>, Error = Simple<char>> + Clone {
    ident()
        .then(param_list())
        .then(keyword("if").ignore_then(expr_parser()).or_not())
        .then(
            just("->")
                .padded_by(ws())
                .ignore_then(transition_parser())
                .or_not(),
        )
        .map(|(((name, params), guard), transition)| Direction {
            name,
            params,
            guard,
            transition,
        })
}


// ============================================================================
// Position
// ============================================================================

fn position_parser() -> impl Parser<char, Position<String>, Error = Simple<char>> + Clone {
    ident()
        .then(param_list())
        .then(keyword("if").ignore_then(expr_parser()).or_not())
        .then(
            direction_parser()
                .separated_by(just(',').padded_by(ws()))
                .delimited_by(just('{').padded_by(ws()), just('}').padded_by(ws()))
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .map(|(((name, params), guard), directions)| Position {
            name,
            params,
            guard,
            directions,
        })
}


// ============================================================================
// Interface (with state-machine sugar desugaring for the plain case)
// ============================================================================

fn interface_decls() -> impl Parser<char, Vec<Decl<String>>, Error = Simple<char>> {
    let single_state_body = direction_parser()
        .separated_by(just(',').padded_by(ws()))
        .delimited_by(just('{').padded_by(ws()), just('}').padded_by(ws()));

    enum Body {
        Positions(Vec<Position<String>>),
        SingleState(Vec<Direction<String>>),
    }

    let body = single_state_body
        .map(Body::SingleState)
        .or(position_parser().separated_by(just(',').padded_by(ws())).map(Body::Positions));

    keyword("interface")
        .ignore_then(ident())
        .then(param_list())
        .then(body)
        .map(|((name, params), body)| {
            let positions = match body {
                Body::Positions(ps) => ps,
                Body::SingleState(directions) => vec![Position {
                    name: name.clone(),
                    params: Vec::new(),
                    guard: None,
                    directions,
                }],
            };
            let iface = Interface { name, params, positions };
            desugar_interface(iface)
        })
}

fn desugar_interface(iface: Interface<String>) -> Vec<Decl<String>> {
    let has_transitions = iface
        .positions
        .iter()
        .any(|p| p.directions.iter().any(|d| d.transition.is_some()));

    if !has_transitions {
        return vec![Decl::Interface(iface)];
    }

    let external_positions: Vec<Position<String>> = iface
        .positions
        .iter()
        .map(|p| Position {
            name: p.name.clone(),
            params: p.params.clone(),
            guard: p.guard.clone(),
            directions: p
                .directions
                .iter()
                .map(|d| Direction {
                    name: d.name.clone(),
                    params: d.params.clone(),
                    guard: d.guard.clone(),
                    transition: None,
                })
                .collect(),
        })
        .collect();
    let external = Interface {
        name: iface.name.clone(),
        params: iface.params.clone(),
        positions: external_positions,
    };

    let internal_positions: Vec<Position<String>> = iface
        .positions
        .iter()
        .map(|p| Position {
            name: p.name.clone(),
            params: p.params.clone(),
            guard: p.guard.clone(),
            directions: Vec::new(),
        })
        .collect();
    let internal_name = format!("{}::Internal", iface.name);
    let internal = Interface {
        name: internal_name.clone(),
        params: iface.params.clone(),
        positions: internal_positions,
    };

    let mut entries: Vec<DeferEntry<String>> = Vec::new();
    for p in &iface.positions {
        let source_pattern: Vec<Pattern<String>> = p
            .params
            .iter()
            .map(|param| Pattern::Bind(param.name.clone()))
            .collect();
        let target_args: Vec<Expr<String>> = p
            .params
            .iter()
            .map(|param| Expr::Var(param.name.clone()))
            .collect();
        let directions: Vec<DirMapping<String>> = p
            .directions
            .iter()
            .filter_map(|d| {
                d.transition.as_ref().map(|trans| DirMapping {
                    target_dir: DirRef::Named(d.name.clone()),
                    source_dir: DirRef::Abstract {
                        src_pos: p.name.clone(),
                        src_pattern: source_pattern.clone(),
                        tgt_pos: trans.target_pos.clone(),
                        tgt_args: trans.args.clone(),
                    },
                })
            })
            .collect();
        entries.push(DeferEntry {
            source_pos: p.name.clone(),
            source_pattern,
            source_guard: None,
            target_pos: p.name.clone(),
            target_args,
            directions,
        });
    }
    let defer = Defer {
        name: format!("{}::Run", iface.name),
        source: internal_name,
        target: iface.name.clone(),
        entries,
    };

    vec![
        Decl::Interface(external),
        Decl::Interface(internal),
        Decl::Defer(defer),
    ]
}


// ============================================================================
// Schema
// ============================================================================

enum SchemaEntry {
    Field(Param<String>),
    Variant(Variant<String>),
}

fn schema_entry() -> impl Parser<char, SchemaEntry, Error = Simple<char>> + Clone {
    let field = ident()
        .then_ignore(just(':').padded_by(ws()))
        .then(type_parser())
        .map(|(name, ty)| SchemaEntry::Field(Param { name, ty }));

    let variant = ident()
        .then(param_list())
        .map(|(name, params)| SchemaEntry::Variant(Variant { name, params }));

    field.or(variant)
}

fn schema_decl() -> impl Parser<char, Schema<String>, Error = Simple<char>> {
    keyword("schema")
        .ignore_then(ident())
        .then(schema_entry().separated_by(just(',').padded_by(ws())))
        .try_map(|(name, entries), span| {
            let mut fields: Vec<Param<String>> = Vec::new();
            let mut variants: Vec<Variant<String>> = Vec::new();
            for e in entries {
                match e {
                    SchemaEntry::Field(p) => fields.push(p),
                    SchemaEntry::Variant(v) => variants.push(v),
                }
            }
            let body = match (fields.is_empty(), variants.is_empty()) {
                (false, true) => SchemaBody::Record(fields),
                (true, false) => SchemaBody::Sum(variants),
                (true, true) => SchemaBody::Sum(Vec::new()),
                (false, false) => {
                    return Err(Simple::custom(
                        span,
                        format!("schema {name} mixes record fields and sum variants"),
                    ))
                }
            };
            Ok(Schema { name, body })
        })
}


// ============================================================================
// Defer
// ============================================================================

fn pattern() -> impl Parser<char, Pattern<String>, Error = Simple<char>> + Clone {
    let wildcard = just('_').padded_by(ws()).to(Pattern::Wildcard);
    let bind = ident().map(Pattern::Bind);
    wildcard.or(bind)
}

fn pattern_list() -> impl Parser<char, Vec<Pattern<String>>, Error = Simple<char>> + Clone {
    pattern()
        .separated_by(just(',').padded_by(ws()))
        .delimited_by(just('[').padded_by(ws()), just(']').padded_by(ws()))
        .or_not()
        .map(|opt| opt.unwrap_or_default())
}

fn arg_list() -> impl Parser<char, Vec<Expr<String>>, Error = Simple<char>> + Clone {
    expr_parser()
        .separated_by(just(',').padded_by(ws()))
        .delimited_by(just('[').padded_by(ws()), just(']').padded_by(ws()))
        .or_not()
        .map(|opt| opt.unwrap_or_default())
}

fn abstract_dir_ref() -> impl Parser<char, DirRef<String>, Error = Simple<char>> + Clone {
    ident()
        .then(pattern_list())
        .then_ignore(just("=>").padded_by(ws()))
        .then(ident())
        .then(arg_list())
        .map(|(((src_pos, src_pattern), tgt_pos), tgt_args)| DirRef::Abstract {
            src_pos,
            src_pattern,
            tgt_pos,
            tgt_args,
        })
}

fn dir_ref() -> impl Parser<char, DirRef<String>, Error = Simple<char>> + Clone {
    abstract_dir_ref().or(ident().map(DirRef::Named))
}

fn dir_mapping() -> impl Parser<char, Vec<DirMapping<String>>, Error = Simple<char>> + Clone {
    dir_ref()
        .separated_by(just('|').padded_by(ws()))
        .then_ignore(just("->").padded_by(ws()))
        .then(dir_ref())
        .map(|(target_dirs, source_dir)| {
            target_dirs
                .into_iter()
                .map(|target_dir| DirMapping {
                    target_dir,
                    source_dir: source_dir.clone(),
                })
                .collect()
        })
}

fn defer_entry() -> impl Parser<char, DeferEntry<String>, Error = Simple<char>> {
    ident()
        .then(pattern_list())
        .then(keyword("if").ignore_then(expr_parser()).or_not())
        .then_ignore(just("->").padded_by(ws()))
        .then(ident().or_not())
        .then(arg_list())
        .then(
            dir_mapping()
                .separated_by(just(',').padded_by(ws()))
                .delimited_by(just('{').padded_by(ws()), just('}').padded_by(ws()))
                .or_not()
                .map(|opt| opt.unwrap_or_default()),
        )
        .map(
            |(((((source_pos, source_pattern), source_guard), target_pos), target_args), groups)| {
                let directions: Vec<DirMapping<String>> = groups.into_iter().flatten().collect();
                DeferEntry {
                    source_pos,
                    source_pattern,
                    source_guard,
                    target_pos: target_pos.unwrap_or_default(),
                    target_args,
                    directions,
                }
            },
        )
}

fn defer_decl() -> impl Parser<char, Defer<String>, Error = Simple<char>> {
    keyword("defer")
        .ignore_then(ident())
        .then_ignore(just(':').padded_by(ws()))
        .then(qualified_ident())
        .then_ignore(just("->").padded_by(ws()))
        .then(qualified_ident())
        .then(defer_entry().separated_by(just(',').padded_by(ws())))
        .map(|(((name, source), target), entries)| {
            let entries = entries
                .into_iter()
                .map(|mut e| {
                    if e.target_pos.is_empty() {
                        e.target_pos = target.clone();
                    }
                    e
                })
                .collect();
            Defer { name, source, target, entries }
        })
}


// ============================================================================
// File-level
// ============================================================================

pub fn file() -> impl Parser<char, Vec<Decl<String>>, Error = Simple<char>> {
    let interface = interface_decls();
    let defer = defer_decl().map(|d| vec![Decl::Defer(d)]);
    let schema = schema_decl().map(|s| vec![Decl::Schema(s)]);
    let decl = interface.or(defer).or(schema);
    decl.padded_by(ws())
        .repeated()
        .map(|chunks: Vec<Vec<Decl<String>>>| chunks.into_iter().flatten().collect())
        .then_ignore(end())
}
