use std::hash::Hash;

use im::{HashMap, HashSet};
use super::constructor::*;



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrow<T: Clone + Eq + Hash>(
    HashMap<Constructor<T>, Constructor<T>>
);

impl<T: Clone + Eq + Hash, I> From<I> for Arrow<T>
where
    I: Iterator<Item = (Constructor<T>, Constructor<T>)>
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

    pub fn source(&self) -> HashSet<Constructor<T>> {
        self.0.keys().cloned().collect()
    }

    pub fn target(&self) -> HashSet<Constructor<T>> {
        self.0.values().cloned().collect()
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