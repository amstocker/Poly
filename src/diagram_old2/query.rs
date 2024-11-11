use std::{collections::BinaryHeap, fmt::Debug, hash::Hash};

use im::Vector;

use super::{arrow::{Arrow, Transform}, constructor::*};



const PLACEHOLDER: &'static str = "_";

// TODO: This should really just be isomorphic to "Option",
//       then we should be able to parse any Constructor of Placeholders
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Placeholder<T: Clone + Eq + Hash> {
    Blank,
    Atom(Constructor<T>)
}

impl From<String> for Placeholder<String> {
    fn from(value: String) -> Self {
        if value == PLACEHOLDER {
            Placeholder::Blank
        } else {
            Placeholder::Atom(value.into())
        }
    }
}

impl std::fmt::Display for Placeholder<String> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Placeholder::Blank => write!(f, "{}", PLACEHOLDER),
            Placeholder::Atom(constructor) => write!(f, "{}", constructor),
        }
    }
}




#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<T: Clone + Eq + Hash> {
    depth: usize,
    sequence: Vector<Transform<T>>
}

impl<T: Clone + Eq + Hash + std::fmt::Display> std::fmt::Display for Path<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let elems = self.sequence.iter()
            .map(|Transform { source, target }|
                format!("{} => {}", source, target)
            )
            .collect::<Vec<_>>();
        write!(f, "[ {} ]", elems.join(", "))
    }
}

impl<T: Clone + Eq + Hash> PartialOrd for Path<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.depth.partial_cmp(&self.depth)
    }
}

impl<T: Clone + Eq + Hash> Ord for Path<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.depth.cmp(&self.depth)
    }
}

impl<T: Clone + Eq + Hash> Path<T> {
    pub fn empty() -> Path<T> {
        Path {
            depth: 0,
            sequence: Vector::new()
        }
    }

    pub fn new(transform: &Transform<T>) -> Path<T> {
        Path::empty().push(transform)
    }

    pub fn push(&self, transform: &Transform<T>) -> Path<T> {
        let mut sequence = self.sequence.clone();
        sequence.push_back(transform.clone());
        Path {
            depth: self.depth + 1,
            sequence
        }
    }

    pub fn target(&self) -> Constructor<T> {
        self.sequence.last().unwrap().target.clone()
    }
}


pub struct Query<T: Clone + Eq + Hash> {
    transforms: Vec<Transform<T>>,
    source: Placeholder<T>,
    target: Placeholder<T>,
    queue: BinaryHeap<Path<T>>
}

impl<T: Clone + Eq + Hash> Query<T> {
    pub fn new(arrows: Vec<Arrow<T>>, source: Placeholder<T>, target: Placeholder<T>) -> Query<T> {
        let transforms: Vec<Transform<T>> = arrows.iter()
            .flat_map(|arrow| arrow.transforms())
            .collect();
        let queue = transforms.iter()
            .filter(|Transform { source: x, .. }|
                match &source {
                    Placeholder::Blank => true,
                    Placeholder::Atom(source) => x == source,
                }
            )
            .map(|transform| Path::new(&transform))
            .collect();
        Query {
            transforms,
            source,
            target,
            queue
        }
    }
}

impl<T: Clone + Eq + Hash> Iterator for Query<T> {
    type Item = Path<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let path = self.queue.pop()?;
        
        // TODO: This should be more nuanced.  If path.target() is a...
        //  - Sum, then we should push any pair to the path if the source matches any of the summands
        //  - Product, then we should push all possible compositions from pairs that match a (subset of?) the product.
        //  - Sequence?
        //  - Atom is easy.
        match path.target() {
            Constructor::Atom(_) => todo!(),
            Constructor::Sum(elems) => todo!(),
            Constructor::Product(elems) => todo!()
        }

        match &self.target {
            Placeholder::Blank => Some(path),
            Placeholder::Atom(target) => if *target == path.target() {
                Some(path)
            } else {
                self.next()
            },
        }
    }
}