use std::{collections::BinaryHeap, fmt::Debug, hash::Hash};

use im::Vector;

use super::{arrow::{Arrow, Pair}, constructor::*};



const PLACEHOLDER: &'static str = "_";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Placeholder<T: Clone> {
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
pub struct Path<T: Clone> {
    depth: usize,
    sequence: Vector<Pair<T>>
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Path<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut elems: Vec<_> = self.sequence.iter().map(|pair| pair.source.to_string()).collect();
        if let Some(pair) = self.sequence.last() {
            elems.push(pair.target.to_string());
        }
        write!(f, "{}", elems.join(" => "))
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

    pub fn new(pair: &Pair<T>) -> Path<T> {
        Path::empty().push(pair)
    }

    pub fn push(&self, pair: &Pair<T>) -> Path<T> {
        let mut sequence = self.sequence.clone();
        sequence.push_back(pair.clone());
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
    pairs: Vec<Pair<T>>,
    source: Placeholder<T>,
    target: Placeholder<T>,
    queue: BinaryHeap<Path<T>>
}

impl<T: Clone + Eq + Hash> Query<T> {
    pub fn new(arrows: Vec<Arrow<T>>, source: Placeholder<T>, target: Placeholder<T>) -> Query<T> {
        let pairs: Vec<Pair<T>> = arrows.iter()
            .flat_map(|arrow| arrow.pairs())
            .collect();
        let queue = pairs.iter()
            .filter(|Pair { source: x, .. }|
                match &source {
                    Placeholder::Blank => true,
                    Placeholder::Atom(source) => x == source,
                }
            )
            .map(|pair| Path::new(&pair))
            .collect();
        Query {
            pairs,
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

        for pair in &self.pairs {
            if path.target() == pair.source {
                self.queue.push(path.push(pair));
            }
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