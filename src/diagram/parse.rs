use chumsky::prelude::*;
use text::whitespace;


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
        states: Vec<State<Label>>,
        then: Option<Box<Decl>>
    },
    Defer {
        label: Label,
        transitions: Vec<State<Arrow<Label>>>,
        then: Option<Box<Decl>>
    }
}

impl Decl {
    pub fn then(&self) -> Option<&Decl> {
        match self {
            Decl::Interface { then, .. } => then.as_deref(),
            Decl::Defer { then, .. } => then.as_deref(),
        }
    }
}

pub fn parser() -> impl Parser<char, Decl, Error = Simple<char>> {
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
            states,
            then: None
        });

    interface
}