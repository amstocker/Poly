use chumsky::prelude::*;
use chumsky::text::whitespace;


#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward
}

#[derive(Debug)]
pub struct Action {
    label: String
}

#[derive(Debug)]
pub struct State {
    label: String,
    actions: Vec<Action>
}

#[derive(Debug)]
pub struct ActionTransform {
    from: Vec<String>,
    to: String
}

#[derive(Debug)]
pub struct StateTransform {
    from: String,
    to: String,
    transforms: Vec<ActionTransform>
}

#[derive(Debug)]
pub enum Decl {
    Interface {
        label: String,
        states: Vec<State>
    },
    Defer {
        from: String,
        to: String,
        transforms: Vec<StateTransform>
    }
    // Query?
    // Context/focus?
}


impl From<String> for Action {
    fn from(label: String) -> Self {
        Action { label }
    }
}

impl From<(String, Vec<Action>)> for State {
    fn from((label, actions): (String, Vec<Action>)) -> Self {
        State { label, actions }
    }
}

impl From<(String, Vec<String>)> for ActionTransform {
    fn from((to, from): (String, Vec<String>)) -> Self {
        ActionTransform { from, to }
    }
}

impl From<((String, String), Vec<ActionTransform>)> for StateTransform {
    fn from(((from, to), transforms): ((String, String), Vec<ActionTransform>)) -> Self {
        StateTransform { from, to, transforms }
    }
}


fn arrow(direction: Direction) -> impl Parser<char, &'static str, Error = Simple<char>> {
    use Direction::*;
    just(match direction {
        Forward  => "->",
        Backward => "<-"
    })
}

fn interface() -> impl Parser<char, Decl, Error = Simple<char>> {
    let ident = text::ident().padded();

    let actions = ident
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'))
        .map(|actions| actions.into_iter()
            .map(Action::from)
            .collect()
        );

    let states = ident
        .then(actions)
        .separated_by(just(','))
        .map(|states| states.into_iter()
            .map(State::from)
            .collect()
        );

    text::keyword("interface")
        .ignore_then(ident)
        .then_ignore(just(':'))
        .then(states)
        .padded()
        .map(|(label, states)| Decl::Interface {
            label,
            states
        })
}

fn defer() -> impl Parser<char, Decl, Error = Simple<char>> {
    let ident = text::ident().padded();

    let action_transforms = ident
        .then_ignore(arrow(Direction::Backward))
        .then(ident.separated_by(just('|')))
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'))
        .map(|transforms| transforms.into_iter()
            .map(ActionTransform::from)
            .collect()
        );

    let state_transforms = ident
        .then_ignore(arrow(Direction::Forward))
        .then(ident)
        .then(action_transforms)
        .separated_by(just(','))
        .map(|transforms| transforms.into_iter()
            .map(StateTransform::from)
            .collect()
        );

    text::keyword("defer")
        .ignore_then(ident)
        .then_ignore(arrow(Direction::Forward))
        .then(ident)
        .then_ignore(just(':'))
        .then(state_transforms)
        .map(|((from, to), transforms)| Decl::Defer {
            from,
            to,
            transforms
        })
}

pub fn parser() -> impl Parser<char, Vec<Decl>, Error = Simple<char>> {
    interface().or(defer())
        .separated_by(whitespace())
        .padded()
        .then_ignore(end())
}

pub fn parse(src: String) -> Result<Vec<Decl>, Vec<Simple<char>>> {
    parser().parse(src.clone())
}