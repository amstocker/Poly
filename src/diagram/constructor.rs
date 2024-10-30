use std::hash::Hash;

use im::Vector;



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor<T: Clone> {
    Atom(T),
    Sum(Vector<Constructor<T>>),
    Product(Vector<Constructor<T>>),
    Sequence(Vector<Constructor<T>>)
}

impl<T: Clone> From<T> for Constructor<T> {
    fn from(value: T) -> Self {
        Constructor::Atom(value)
    }
}

impl<'t, T: Clone + 't> From<&'t T> for Constructor<T> {
    fn from(value: &'t T) -> Self {
        Constructor::atom(value)
    }
}

pub trait Constructible<T> {
    fn new(t: &T) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn mult(&self, other: &Self) -> Self;
    fn then(&self, other: &Self) -> Self;
}

impl<'t, T: Clone + 't> Constructor<T> {
    pub fn atom(elem: &'t T) -> Constructor<T> {
        Constructor::Atom(elem.clone())
    }

    pub fn sum(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Sum(elems.into_iter().cloned().collect())
    }

    pub fn product(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Product(elems.into_iter().cloned().collect())
    }

    pub fn sequence(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Sequence(elems.into_iter().cloned().collect())
    }

    pub fn build<A: Constructible<T>>(&self) -> A {
        match self {
            Constructor::Atom(t) => Constructible::new(t),
            Constructor::Sum(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.add(&elem)).unwrap(),
            Constructor::Product(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.mult(&elem)).unwrap(),
            Constructor::Sequence(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.then(&elem)).unwrap()
        }
    }
}


use std::fmt::{Display, Formatter, Result};

impl<T: Clone + Display> Display for Constructor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Constructor::Atom(atom) => write!(f, "{}", atom),
            Constructor::Sum(elems) =>
                write!(f, "{}", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" + ")),
            Constructor::Product(elems) =>
                write!(f, "({})", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(", ")),
            Constructor::Sequence(elems) =>
                write!(f, "[{}]", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" -> ")),
        }
    }
}
