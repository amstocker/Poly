use std::hash::Hash;

use im::HashMap;
use super::constructor::*;



// TODO: Arrow should really be a trait that is Constructible from Self
//       and has a source and target.  It should also be able to produce
//       an iterator of pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrow<T: Clone + Eq + Hash>(
    HashMap<Constructor<T>, Constructor<T>>
);

// TODO: Pair should implelement the Arrow trait
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair<T: Clone> {
    pub source: Constructor<T>,
    pub target: Constructor<T>
}

impl<T: Clone + Eq + Hash, I> From<I> for Arrow<T>
where
    I: IntoIterator<Item = (Constructor<T>, Constructor<T>)>
{
    fn from(pairs: I) -> Self {
        Arrow(pairs.into_iter().collect())
    }
}

impl<'t, T: Clone + Eq + Hash + 't> Arrow<T> {
    pub fn new(map: impl IntoIterator<Item = (&'t Constructor<T>, &'t Constructor<T>)>) -> Arrow<T> {
        Arrow(map.into_iter()
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect())
    }

    pub fn dup(elems: impl IntoIterator<Item = &'t Constructor<T>>) -> Arrow<T> {
        Arrow(elems.into_iter()
            .map(|x| (x.clone(), Constructor::product([x, x])))
            .collect())
    }

    pub fn apply(&self, x: &Constructor<T>) -> Option<Constructor<T>> {
        self.0.get(x).cloned()
    }

    pub fn pairs<'a>(&'a self) -> impl Iterator<Item = Pair<T>> + 'a {
        self.0.iter()
            .map(|(x, y)|
                Pair {
                    source: x.clone(),
                    target: y.clone()
                }
            )
    }
}

impl<T: Clone + Eq + Hash> Constructible<Arrow<T>> for Arrow<T> {
    fn new(arrow: &Arrow<T>) -> Self {
        arrow.clone()
    }

    fn add(&self, other: &Self) -> Self {
        let (Arrow(left), Arrow(right)) = (self, other);
        let mut sum = left.clone();
        for (x, y2) in right.iter() {
            sum.entry(x.clone())
               .and_modify(|y1| *y1 = Constructor::sum([y1, y2]))
               .or_insert(y2.clone());
        }
        Arrow(sum)
    }

    fn mult(&self, other: &Self) -> Self {
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

    fn then(&self, other: &Self) -> Self {
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


use std::fmt::{Display, Formatter, Result};

impl<T: Clone + Eq + Hash + Display> Display for Arrow<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let pairs = self.0.iter()
            .map(|(x, y)| format!("{} => {}", x.to_string(), y.to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{{{}}}", pairs)
    }
}