use std::hash::Hash;

use im::{HashMap, Vector};



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor<T: Clone> {
    Atom(T),
    Sum(Vector<Constructor<T>>),
    Product(Vector<Constructor<T>>),
    Sequence(Vector<Constructor<T>>)
}

pub trait Constructible<T> {
    fn atom(t: T) -> Self;
    fn zero() -> Self;
    fn unit() -> Self;
    fn identity() -> Self;
    fn add(self, other: Self) -> Self;
    fn mult(self, other: Self) -> Self;
    fn compose(self, other: Self) -> Self;
}

impl<T: Clone> Constructor<T> {
    pub fn build<A: Constructible<T>>(self) -> A {
        match self {
            Constructor::Atom(t) => Constructible::atom(t),
            Constructor::Sum(elems) => elems.into_iter()
                .fold(Constructible::zero(), |acc, other| acc.add(other.build())),
            Constructor::Product(elems) => elems.into_iter()
                .fold(Constructible::unit(), |acc, other| acc.mult(other.build())),
            Constructor::Sequence(elems) => elems.into_iter()
                .fold(Constructible::identity(), |acc, other| acc.compose(other.build())),
        }
    }
}


pub enum Arrow<T: Clone> {
    Zero,
    Identity,
    Arrow(HashMap<Constructor<T>, Constructor<T>>)
}

impl<T: Clone + Eq + Hash> Constructible<Arrow<T>> for Arrow<T> {
    fn atom(arrow: Arrow<T>) -> Self {
        arrow
    }

    fn zero() -> Self {
        Arrow::Zero
    }

    fn unit() -> Self {
        Arrow::Identity
    }

    fn identity() -> Self {
        Arrow::Identity
    }

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Arrow::Arrow(left), Arrow::Arrow(right)) => todo!(),
            (Arrow::Identity, arrow) | (arrow, Arrow::Identity) => arrow,
            (Arrow::Zero, _) | (_, Arrow::Zero) => Arrow::Zero
        }
    }

    fn mult(self, other: Self) -> Self {
        match (self, other) {
            (Arrow::Arrow(left), Arrow::Arrow(right)) => todo!(),
            (Arrow::Identity, arrow) | (arrow, Arrow::Identity) => arrow,
            (Arrow::Zero, _) | (_, Arrow::Zero) => Arrow::Zero
        }
    }

    fn compose(self, other: Self) -> Self {
        match (self, other) {
            (Arrow::Arrow(first), Arrow::Arrow(second)) => {
                let mut composition = HashMap::new();
                for (x, y1) in first.iter() {
                    for (y2, z) in second.iter() {
                        if y1 == y2 {
                            composition.insert(x.clone(), z.clone());
                        }
                    }
                }
                Arrow::Arrow(composition)
            },
            (Arrow::Identity, arrow) | (arrow, Arrow::Identity) => arrow,
            (Arrow::Zero, _) | (_, Arrow::Zero) => Arrow::Zero
        }
    }
}