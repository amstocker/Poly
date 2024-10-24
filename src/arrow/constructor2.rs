use std::hash::Hash;

use im::{HashMap, Vector};



#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Constructor<T: Clone> {
    Atom(T),
    Sum(Vector<Constructor<T>>),
    Product(Vector<Constructor<T>>),
    Sequence(Vector<Constructor<T>>)
}

impl<T: Clone> Constructor<T> {
    pub fn sum(iter: impl IntoIterator<Item = Constructor<T>>) -> Constructor<T> {
        Constructor::Sum(iter.into_iter().collect())
    }

    pub fn product(iter: impl IntoIterator<Item = Constructor<T>>) -> Constructor<T> {
        Constructor::Product(iter.into_iter().collect())
    }

    pub fn sequence(iter: impl IntoIterator<Item = Constructor<T>>) -> Constructor<T> {
        Constructor::Sequence(iter.into_iter().collect())
    }
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Constructor<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constructor::Atom(atom) => write!(f, "{}", atom),
            Constructor::Sum(elems)
                => write!(f, "{}", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" + ")),
            Constructor::Product(elems)
                => write!(f, "({})", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(", ")),
            Constructor::Sequence(elems)
                => write!(f, "[{}]", elems.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" -> ")),
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
                .map(|elem| elem.build())
                .reduce(|acc: A, elem| acc.add(elem)).unwrap(),
            Constructor::Product(elems) => elems.into_iter()
                .map(|elem| elem.build())
                .reduce(|acc: A, elem| acc.add(elem)).unwrap(),
            Constructor::Sequence(elems) => elems.into_iter()
                .map(|elem| elem.build())
                .reduce(|acc: A, elem| acc.add(elem)).unwrap()
        }
    }
}


#[derive(Clone)]
pub struct Arrow<T: Clone + Eq + Hash>(HashMap<Constructor<T>, Constructor<T>>);

impl<T: Clone + Eq + Hash> Arrow<T> {
    pub fn arrow(iter: impl IntoIterator<Item = (Constructor<T>, Constructor<T>)>) -> Arrow<T> {
        Arrow(iter.into_iter().collect())
    }
}

impl<T: Clone + Eq + Hash> Constructible<Arrow<T>> for Arrow<T> {
    fn atom(arrow: Arrow<T>) -> Self {
        arrow
    }

    fn add(self, other: Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut sum = left.clone();
        for (x, y) in right.iter() {
            sum.entry(x.clone())
               .and_modify(|y2| *y2 = Constructor::sum([y.clone(), y2.clone()]))
               .or_insert(y.clone());
        }
        Arrow(sum)
    }

    fn mult(self, other: Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut product = HashMap::new();
        for (x1, y1) in left.iter() {
            for (x2, y2) in right.iter() {
                product.insert(
                    Constructor::product([x1.clone(), x2.clone()]),
                    Constructor::product([y1.clone(), y2.clone()])
                );
            }
        }
        Arrow(product)
    }

    fn compose(self, other: Self) -> Self {
        let (Arrow(first), Arrow(second)) = (self, other);
        let mut composition = first.clone();
        'outer: for (x, y1) in first.iter() {
            for (y2, z) in second.iter() {
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