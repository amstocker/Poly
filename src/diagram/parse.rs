use chumsky::prelude::*;
use text::{newline, whitespace};


pub type Label = String;

#[derive(Debug)]
pub struct Arrow<T> {
    from: T,
    to: T
}

#[derive(Debug)]
pub struct Action<T> {
    label: T
}

#[derive(Debug)]
pub struct State<T> {
    label: T,
    actions: Vec<Action<T>>
}

#[derive(Debug)]
pub enum Decl {
    Interface {
        label: Label,
        states: Vec<State<Label>>
    },
    Defer {
        label: Label,
        transitions: Vec<State<Arrow<Label>>>
    }
}

pub fn parser() -> impl Parser<char, Vec<Decl>, Error = Simple<char>> {
    let ident = text::ident().padded();

    let actions = ident
        .separated_by(whitespace())
        .delimited_by(just('{'), just('}'))
        .map(|actions| actions.into_iter()
            .map(|label| Action { label })
            .collect()
        );

    let states = ident
        .then(actions.clone())
        .separated_by(just(','))
        .map(|states| states.into_iter()
            .map(|(label, actions)| State { label, actions })
            .collect()
        );

    let interface = text::keyword("interface")
        .ignore_then(ident)
        .then_ignore(just(':'))
        .then(states.clone())
        .map(|(label, states)| Decl::Interface {
            label,
            states
        });

    interface
        .separated_by(newline().repeated())
        .then_ignore(end())
}