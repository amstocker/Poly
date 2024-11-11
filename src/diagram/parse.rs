use chumsky::prelude::*;

use super::{Term, Transform};



fn atom<T: Clone + From<String>>() -> impl Parser<char, T, Error = Simple<char>> {
    text::ident()
        .padded()
        .map(|ident| T::from(ident))
}

fn term<T: Clone + From<String>>() -> impl Parser<char, Term<T>, Error = Simple<char>> {
    atom::<T>()
        .map(|atom| Term::from(atom))
        .or(
            atom::<T>()
                .padded()
                .separated_by(just(','))
                .delimited_by(just('('), just(')'))
                .map(|elems| Term(elems.into()))
        )
}

fn constructor<T: Clone + From<String>>() -> impl Parser<char, Vec<Term<T>>, Error = Simple<char>> {
    term()
        .padded()
        .separated_by(just('+'))
}


fn transform<T: Clone + From<String>>() -> impl Parser<char, Vec<Transform<T>>, Error = Simple<char>> {
    let constructors = constructor::<T>()
        .then_ignore(just("=>"))
        .then(constructor::<T>());
    constructors.map(|(sources, targets)| {
        let mut transforms = Vec::new();
        for source in sources {
            for target in targets.iter().cloned() {
                transforms.push(Transform {
                    source: source.clone(),
                    target
                });
            }
        }
        transforms
    })
}

fn arrow<T: Clone + From<String>>() -> impl Parser<char, Vec<Transform<T>>, Error = Simple<char>> {
    let all_transforms = transform()
        .separated_by(just(','))
        .delimited_by(just('{'), just('}'));
    all_transforms.map(|all_transforms| {
        let mut acc_transforms = Vec::new();
        for transforms in all_transforms {
            acc_transforms.extend(transforms);
        }
        acc_transforms
    })
}

pub fn parser<T: Clone + From<String>>() -> impl Parser<char, Vec<Transform<T>>, Error = Simple<char>> {
    arrow()
    //    .separated_by(text::newline().repeated())
    //    .then_ignore(end())
}