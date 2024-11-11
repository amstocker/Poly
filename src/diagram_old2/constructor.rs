use std::hash::Hash;

use im::{HashSet, Vector};


// TODO: Might want to separate these into different types, and have different traits for each one?
//       Product should also be able to specify: Wildcard (*), at-least-one (+)


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor<T: Clone + Eq + Hash> {
    Atom(T),
    Sum(HashSet<Constructor<T>>),
    Product(Vector<Constructor<T>>)
}

impl<T: Clone + Eq + Hash> From<T> for Constructor<T> {
    fn from(value: T) -> Self {
        Constructor::Atom(value)
    }
}

pub trait Constructible<T> {
    fn new(t: &T) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn mult(&self, other: &Self) -> Self;
}

impl<'t, T: Clone + Eq + Hash + 't> Constructor<T> {
    pub fn sum(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Sum(elems.into_iter().cloned().collect())
    }

    pub fn product(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Product(elems.into_iter().cloned().collect())
    }

    pub fn build<A: Constructible<T>>(&self) -> A {
        match self {
            Constructor::Atom(t) => Constructible::new(t),
            Constructor::Sum(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.add(&elem)).unwrap(),
            Constructor::Product(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.mult(&elem)).unwrap()
        }
    }
}


use std::fmt::{Display, Formatter, Result};

impl<T: Clone + Eq + Hash + Display> Display for Constructor<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Constructor::Atom(atom) => write!(f, "{}", atom),
            Constructor::Sum(elems) =>
                write!(f, "{}", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" + ")),
            Constructor::Product(elems) =>
                write!(f, "({})", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(", "))
        }
    }
}
