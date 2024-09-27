use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use chumsky::text::{newline, whitespace};


#[derive(Debug)]
pub struct Arrow {
    from: String,
    to: String
}

#[derive(Debug)]
pub struct Action {
    label: String
}

impl From<String> for Action {
    fn from(label: String) -> Self {
        Action { label }
    }
}

#[derive(Debug)]
pub struct State {
    label: String,
    actions: Vec<Action>
}

impl From<(String, Vec<Action>)> for State {
    fn from((label, actions): (String, Vec<Action>)) -> Self {
        State { label, actions }
    }
}

#[derive(Debug)]
pub struct ActionTransform {
    from: Vec<String>,
    to: String
}

impl From<(String, Vec<String>)> for ActionTransform {
    fn from((to, from): (String, Vec<String>)) -> Self {
        ActionTransform { from, to }
    }
}

#[derive(Debug)]
pub struct StateTransform {
    from: String,
    to: String,
    transforms: Vec<ActionTransform>
}

impl From<((String, String), Vec<ActionTransform>)> for StateTransform {
    fn from(((from, to), transforms): ((String, String), Vec<ActionTransform>)) -> Self {
        StateTransform { from, to, transforms }
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward
}

fn arrow(direction: Direction) -> impl Parser<char, &'static str, Error = Simple<char>> {
    use Direction::*;
    just(match direction {
        Forward  => "->",
        Backward => "<-"
    })
}

pub fn parser() -> impl Parser<char, Vec<Decl>, Error = Simple<char>> {
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

    let interface = text::keyword("interface")
        .ignore_then(ident)
        .then_ignore(just(':'))
        .then(states)
        .padded()
        .map(|(label, states)| Decl::Interface {
            label,
            states
        });

    use Direction::*;
    let action_transforms = ident
        .then_ignore(arrow(Backward))
        .then(ident.separated_by(just('|')))
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'))
        .map(|transforms| transforms.into_iter()
            .map(ActionTransform::from)
            .collect()
        );

    let state_transforms = ident
        .then_ignore(arrow(Forward))
        .then(ident)
        .then(action_transforms)
        .separated_by(just(','))
        .map(|transforms| transforms.into_iter()
            .map(StateTransform::from)
            .collect()
        );

    let defer = text::keyword("defer")
        .ignore_then(ident)
        .then_ignore(arrow(Forward))
        .then(ident)
        .then_ignore(just(':'))
        .then(state_transforms)
        .map(|((from, to), transforms)| Decl::Defer {
            from,
            to,
            transforms
        });

    interface.or(defer)
        .separated_by(whitespace())
        .then_ignore(whitespace())
        .then_ignore(end())
}

pub fn parse(src: String) -> Option<Vec<Decl>> {
    match parser().parse(src.clone()) {
        Ok(decls) => Some(decls),
        Err(errs) => {
            for err in errs {
                Report::build(ReportKind::Error, (), err.span().start)
                    .with_code(3)
                    .with_message(err.to_string())
                    .with_label(
                        Label::new(err.span())
                            .with_message(format!("{:?}", err))
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint(Source::from(src.clone()))
                    .unwrap();
            }
            None
        }
    }
}