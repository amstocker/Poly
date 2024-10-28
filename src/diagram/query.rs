use std::{collections::BinaryHeap, hash::Hash};

use im::{HashSet, Vector};

use super::{arrow::Arrow, constructor::*};



const PLACEHOLDER: &'static str = "_";

#[derive(Clone, PartialEq, Eq, Hash)]
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


pub type Index = usize;

#[derive(Debug, PartialEq, Eq)]
pub struct Path {
    depth: usize,
    path: Vector<Index>
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.depth.partial_cmp(&self.depth)
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.depth.cmp(&self.depth)
    }
}

impl Path {
    pub fn new(index: Index) -> Path {
        Path {
            depth: 0,
            path: [index].into_iter().collect()
        }
    }
}

// impl Constructible for Path?


pub struct Query<T: Clone + Eq + Hash> {
    arrows: Vec<Arrow<T>>,
    sources: Vec<HashSet<Constructor<T>>>,
    targets: Vec<HashSet<Constructor<T>>>,
    source: Placeholder<T>,
    target: Placeholder<T>,
    queue: BinaryHeap<Path>
}

impl<T: Clone + Eq + Hash> Query<T> {
    pub fn new(arrows: Vec<Arrow<T>>, source: Placeholder<T>, target: Placeholder<T>) -> Query<T> {
        let queue = arrows.iter()
            .enumerate()
            .map(|(index, _)| Path::new(index)).collect();
        let sources = arrows.iter()
            .map(|arrow| arrow.source()).collect();
        let targets = arrows.iter()
            .map(|arrow| arrow.target()).collect();
        Query {
            arrows,
            sources,
            targets,
            source,
            target,
            queue
        }
    }
}

impl<T: Clone + Eq + Hash> Iterator for Query<T> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(path) = self.queue.pop() {
            Some(())
        } else {
            None
        }
    }
}




// How to actually query?  Maintain HashSet of history, then check also possible paths of arrows.
// We check each path in a loop, then push all extensions of the path into a queue.
// If we have a path and we try all possible extensions, and they are all in the history, we can pop that path.
// Do this until we've exhausted all possible paths (this could be infinite).