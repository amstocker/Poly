use chumsky::prelude::*;
use std::collections::BTreeMap;

use super::{BinOp, Decl, Defer, Direction, Expr, Interface, Param, Position, Schema, SchemaBody,
    Transition, Type, UnOp, Variant};


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
    keyword("interface")
        .ignore_then(ident())
        .then(param_list())
        .then(position_parser().separated_by(just(',').padded_by(ws())))
        .map(|((name, params), positions)| {
            let iface = Interface { name, params, positions };
            desugar_interface(iface)
        })
}

fn desugar_interface(iface: Interface<String>) -> Vec<Decl<String>> {
    let plain = !iface.is_parameterized();
    let has_transitions = iface
        .positions
        .iter()
        .any(|p| p.directions.iter().any(|d| d.transition.is_some()));

    if !plain || !has_transitions {
        return vec![Decl::Interface(iface)];
    }

    use std::collections::BTreeSet;
    let mut states: BTreeSet<String> = iface.positions.iter().map(|p| p.name.clone()).collect();
    for p in &iface.positions {
        for d in &p.directions {
            if let Some(t) = &d.transition {
                states.insert(t.target_pos.clone());
            }
        }
    }

    let external_positions: Vec<Position<String>> = states
        .iter()
        .map(|s| {
            let directions = match iface.positions.iter().find(|p| &p.name == s) {
                Some(p) => p
                    .directions
                    .iter()
                    .map(|d| Direction {
                        name: d.name.clone(),
                        params: Vec::new(),
                        guard: None,
                        transition: None,
                    })
                    .collect(),
                None => Vec::new(),
            };
            Position {
                name: s.clone(),
                params: Vec::new(),
                guard: None,
                directions,
            }
        })
        .collect();
    let external = Interface {
        name: iface.name.clone(),
        params: Vec::new(),
        positions: external_positions,
    };

    let internal_positions: Vec<Position<String>> = states
        .iter()
        .map(|s| {
            let directions: Vec<Direction<String>> = states
                .iter()
                .map(|t| Direction {
                    name: format!("{s}=>{t}"),
                    params: Vec::new(),
                    guard: None,
                    transition: None,
                })
                .collect();
            Position {
                name: s.clone(),
                params: Vec::new(),
                guard: None,
                directions,
            }
        })
        .collect();
    let internal_name = format!("{}.internal", iface.name);
    let internal = Interface {
        name: internal_name.clone(),
        params: Vec::new(),
        positions: internal_positions,
    };

    let mut pos_map: BTreeMap<String, String> = BTreeMap::new();
    let mut dir_map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for s in &states {
        pos_map.insert(s.clone(), s.clone());
    }
    for p in &iface.positions {
        let mut inner: BTreeMap<String, String> = BTreeMap::new();
        for d in &p.directions {
            let dest = d
                .transition
                .as_ref()
                .map(|t| t.target_pos.clone())
                .unwrap_or_else(|| p.name.clone());
            inner.insert(d.name.clone(), format!("{}=>{}", p.name, dest));
        }
        dir_map.insert(p.name.clone(), inner);
    }
    let defer = Defer {
        name: format!("{}.run", iface.name),
        source: internal_name,
        target: iface.name.clone(),
        pos_map,
        dir_map,
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

fn dir_mapping() -> impl Parser<char, (Vec<String>, String), Error = Simple<char>> {
    ident()
        .separated_by(just('|').padded_by(ws()))
        .then_ignore(just("->").padded_by(ws()))
        .then(ident())
}

fn pos_mapping(
) -> impl Parser<char, (String, String, Vec<(Vec<String>, String)>), Error = Simple<char>> {
    ident()
        .then_ignore(just("->").padded_by(ws()))
        .then(ident())
        .then(
            dir_mapping()
                .separated_by(just(',').padded_by(ws()))
                .delimited_by(just('{').padded_by(ws()), just('}').padded_by(ws())),
        )
        .map(|((src, tgt), dirs)| (src, tgt, dirs))
}

fn defer_decl() -> impl Parser<char, Defer<String>, Error = Simple<char>> {
    keyword("defer")
        .ignore_then(ident())
        .then_ignore(just(':').padded_by(ws()))
        .then(ident())
        .then_ignore(just("->").padded_by(ws()))
        .then(ident())
        .then(pos_mapping().separated_by(just(',').padded_by(ws())))
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
