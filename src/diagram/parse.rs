use std::hash::Hash;

use chumsky::prelude::*;

use super::arrow::Arrow;
use super::constructor::Constructor;
use super::query::Placeholder;


fn sum<T: Clone>(inner: impl Parser<char, Constructor<T>, Error = Simple<char>>) -> impl Parser<char, Constructor<T>, Error = Simple<char>> {
    inner
        .padded()
        .separated_by(just('+'))
        .map(|elems| Constructor::Sum(elems.into()))
}

fn product<T: Clone>(inner: impl Parser<char, Constructor<T>, Error = Simple<char>>) -> impl Parser<char, Constructor<T>, Error = Simple<char>> {
    inner
        .padded()
        .separated_by(just(','))
        .delimited_by(just('('), just(')'))
        .map(|elems| Constructor::Product(elems.into()))
}

fn sequence<T: Clone>(inner: impl Parser<char, Constructor<T>, Error = Simple<char>>) -> impl Parser<char, Constructor<T>, Error = Simple<char>> {
    inner
        .padded()
        .separated_by(just("->"))
        .delimited_by(just('['), just(']'))
        .map(|elems| Constructor::Sum(elems.into()))
}

fn atom<T: Clone + From<String>>() -> impl Parser<char, Constructor<T>, Error = Simple<char>> {
    text::ident()
        .padded()
        .map(|ident| Constructor::Atom(T::from(ident)))
}

pub fn constructor<T: Clone + From<String>>() -> impl Parser<char, Constructor<T>, Error = Simple<char>> {
    sum(product(atom()).or(atom()))
        .or(product(atom()))
        .or(atom())
}


fn arrow<T>() -> impl Parser<char, Arrow<T>, Error = Simple<char>>
where
    T: Clone + Eq + Hash + From<String>
{
    constructor()
        .then_ignore(just("=>"))
        .then(constructor())
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'))
        .map(|pairs| pairs.into())
}

fn arrow_decl() -> impl Parser<char, Arrow<String>, Error = Simple<char>> {
    text::keyword("arrow").padded()
        .ignore_then(arrow()) 
}

fn arrow_query() -> impl Parser<char, Arrow<Placeholder<String>>, Error = Simple<char>> {
    text::keyword("query").padded()
        .ignore_then(arrow())
}


pub fn parser() -> impl Parser<char, (Vec<Arrow<String>>, Arrow<Placeholder<String>>), Error = Simple<char>> {
    arrow_decl()
        .separated_by(text::newline().repeated())
        .then(arrow_query())
        .padded()
        .then_ignore(end())
}