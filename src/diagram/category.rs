/*
 *  Notes:
 *      - The `Relation` here, categorically speaking, is a span in the category of finite sets.
 *        In other words, they behave like integer-valued matrices.
 *      - Furthermore, it is technically a double-category, and we should implement the
 *        double-categorical machinery.
 */

 use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering}
};


pub type Value = usize;

pub fn next_value() -> Value {
    static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}


// We use a Vec instead of HashSet because we want to allow for the possibility of duplicates.
#[derive(Debug, Default)]
pub struct Relation {
    data: Vec<(Value, Value)>,
}

impl<T> From<T> for Relation where T: IntoIterator<Item = (usize, usize)> {
    fn from(value: T) -> Self {
        Relation {
            data: value.into_iter().collect()
        }
    }
}

impl<'a> Relation {
    pub fn items(&'a self) -> impl Iterator<Item = (Value, Value)> + 'a {
        self.data.iter().copied()
    }

    pub fn domain(&self) -> HashSet<Value> {
        self.items()
            .map(|(x, _)| x)
            .collect()
    }

    pub fn codomain(&self) -> HashSet<Value> {
        self.items()
            .map(|(_, y)| y)
            .collect()
    }

    pub fn forward_eval(&self, value: Value) -> HashMap<Value, usize> {
        self.items()
            .fold(
                HashMap::new(),
                |mut counts, (x, y)| {
                    if x == value {
                        *counts.entry(y).or_insert(0) += 1;
                    }
                    counts
                }
            ) 
    }

    pub fn backward_eval(&self, value: Value) -> HashMap<Value, usize> {
        self.items()
            .fold(
                HashMap::new(),
                |mut counts, (x, y)| {
                    if y == value {
                        *counts.entry(x).or_insert(0) += 1;
                    }
                    counts
                }
            ) 
    }

    pub fn dual(&self) -> Relation {
        self.data.iter().copied().map(|(x, y)| (y, x)).into()
    }

    pub fn identity(&self, values: impl IntoIterator<Item = Value>) -> Relation {
        values.into_iter().map(|x| (x, x)).into()
    }

    pub fn compose(&self, other: &Relation) -> Relation {
        let mut rel = Relation::default();
        for (x, y1) in self.items() {
            for (y2, z) in other.items() {
                if y1 == y2 {
                    rel.data.push((x, z));
                }
            }
        }
        rel
    }

    // This produces the two projection maps from the product of the
    // respective domains of `self` and `other`.
    pub fn product(&self, other: &Relation) -> (Relation, Relation) {
        (
            self.domain().iter().map(|&x| (next_value(), x)).into(),
            other.domain().iter().map(|&x| (next_value(), x)).into()
        )
    }

    // This produces the two inclusion maps from the coproduct of the
    // respective codomains of `self` and `other`.
    pub fn coproduct(&self, other: &Relation) -> (Relation, Relation) {
        (
            self.codomain().iter().map(|&y| (y, next_value())).into(),
            other.codomain().iter().map(|&y| (y, next_value())).into()
        )
    }

    pub fn equalizer(&self, other: &Relation) -> Relation {
        unimplemented!()
    }

    pub fn coequalizer(&self, other: &Relation) -> Relation {
        unimplemented!()
    }

    pub fn pullback(&self, other: &Relation) -> (Relation, Relation) {
        unimplemented!()
    }

    pub fn pushout(&self, other: &Relation) -> (Relation, Relation) {
        unimplemented!()
    }
}
