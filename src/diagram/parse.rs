use std::hash::Hash;

use chumsky::prelude::*;
use chumsky::text::newline;

use super::arrow::*;
use super::constructor::*;
use super::query::*;



fn arrow<T>() -> impl Parser<char, Arrow<T>, Error = Simple<char>>
where
    T: Clone + Eq + Hash + From<String>
{
    let ident = text::ident().padded();

    ident
        .then_ignore(just("=>"))
        .then(ident)
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'))
        .map(|pairs| pairs.into_iter().map(|(x, y)|
            (Constructor::Atom(x.into()), Constructor::Atom(y.into()))))
        .map(|pairs| pairs.into())
}

fn arrow_decl() -> impl Parser<char, Arrow<String>, Error = Simple<char>> {
    text::keyword("arrow")
        .padded()
        .ignore_then(arrow::<String>()) 
}

fn arrow_query() -> impl Parser<char, Arrow<Placeholder<String>>, Error = Simple<char>> {
    text::keyword("query")
        .padded()
        .ignore_then(arrow::<Placeholder<String>>())
}


pub fn parser() -> impl Parser<char, (Vec<Arrow<String>>, Arrow<Placeholder<String>>), Error = Simple<char>> {
    arrow_decl().separated_by(newline().repeated())
        .then(arrow_query())
        .then_ignore(end())
}