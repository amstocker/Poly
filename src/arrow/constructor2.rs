use std::hash::Hash;

use im::{HashMap, Vector};



#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Constructor<T: Clone> {
    Atom(T),
    Sum(Vector<Constructor<T>>),
    Product(Vector<Constructor<T>>),
    Sequence(Vector<Constructor<T>>)
}

impl<'t, T: Clone + 't> Constructor<T> {
    pub fn sum(iter: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Sum(iter.into_iter().cloned().collect())
    }

    pub fn product(iter: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Product(iter.into_iter().cloned().collect())
    }

    pub fn sequence(iter: impl IntoIterator<Item = &'t Constructor<T>>) -> Constructor<T> {
        Constructor::Sequence(iter.into_iter().cloned().collect())
    }
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Constructor<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

pub trait Constructible<T> {
    fn atom(t: T) -> Self;
    fn add(self, other: Self) -> Self;
    fn mult(self, other: Self) -> Self;
    fn compose(self, other: Self) -> Self;
}

impl<T: Clone> Constructor<T> {
    pub fn build<A: Constructible<T>>(self) -> A {
        match self {
            Constructor::Atom(t) => Constructible::atom(t),
            Constructor::Sum(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.add(elem)).unwrap(),
            Constructor::Product(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.add(elem)).unwrap(),
            Constructor::Sequence(elems) => elems.into_iter()
                .map(|elem| elem.build::<A>())
                .reduce(|acc, elem| acc.add(elem)).unwrap()
        }
    }
}


#[derive(Clone)]
pub struct Arrow<T: Clone + Eq + Hash>(HashMap<Constructor<T>, Constructor<T>>);

impl<'t, T: Clone + Eq + Hash + 't> Arrow<T> {
    pub fn arrow(iter: impl IntoIterator<Item = (&'t Constructor<T>, &'t Constructor<T>)>) -> Arrow<T> {
        Arrow(iter.into_iter().map(|(x, y)| (x.clone(), y.clone())).collect())
    }
}

impl<T: Clone + Eq + Hash> Constructible<Arrow<T>> for Arrow<T> {
    fn atom(arrow: Arrow<T>) -> Self {
        arrow
    }

    fn add(self, other: Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut sum = left.clone();
        for (x, y2) in right.iter() {
            sum.entry(x.clone())
               .and_modify(|y| *y = Constructor::sum([y, y2]))
               .or_insert(y2.clone());
        }
        Arrow(sum)
    }

    fn mult(self, other: Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut product = HashMap::new();
        for (x1, y1) in left.iter() {
            for (x2, y2) in right.iter() {
                product.insert(
                    Constructor::product([x1, x2]),
                    Constructor::product([y1, y2])
                );
            }
        }
        Arrow(product)
    }

    fn compose(self, other: Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut composition = left.clone();
        'outer: for (x, y1) in left.iter() {
            for (y2, z) in right.iter() {
                if y1 == y2 {
                    composition.insert(x.clone(), z.clone());
                    continue 'outer;
                }
            }
            composition.remove(x);
        }
        Arrow(composition)
    }
}

impl<T: Clone + Eq + Hash + std::fmt::Display> std::fmt::Display for Arrow<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pairs = self.0.iter()
            .map(|(x, y)| format!("{} => {}", x.to_string(), y.to_string()))
            .collect::<Vec<_>>()
            .join(" , ");
        write!(f, "{{ {} }}", pairs)
    }
}