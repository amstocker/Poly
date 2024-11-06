use std::hash::Hash;

use im::{HashSet, Vector};

use super::constructor::Constructor;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Transform<T: Clone + Eq + Hash> {
    pub source: Constructor<T>,
    pub target: Constructor<T>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path<T: Clone + Eq + Hash>(Vector<Transform<T>>);

impl<T: Clone + Eq + Hash> Path<T> {
    pub fn empty() -> Path<T> {
        Path(Vector::new())
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn target(&self) -> Option<&Constructor<T>> {
        self.0.last().map(|Transform { target, .. }| target)
    }
}

impl<T: Clone + Eq + Hash> PartialOrd for Path<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.depth().partial_cmp(&self.depth())
    }
}

impl<T: Clone + Eq + Hash> Ord for Path<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.depth().cmp(&self.depth())
    }
}

// TODO: IDK if HashSet is really necessary here... might just need to iter?
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arrow<T: Clone + Eq + Hash>(HashSet<Path<T>>);

impl<T: Clone + Eq + Hash> Arrow<T> {
    pub fn push(&self, transform: &Transform<T>) -> Arrow<T> {
        let mut paths = HashSet::new();

        match &transform.source {
            elem @ Constructor::Atom(atom) => for path in &self.0 {
                if match path.target() {
                    Some(Constructor::Atom(target)) =>
                        target == atom,
                    Some(Constructor::Sum(elems)) =>
                        elems.contains(elem),
                    Some(Constructor::Product(elems)) => if elems.len() == 1 {
                        &elems[0] == elem
                    } else {
                        continue
                    },
                    None => continue
                } {
                    let mut path = path.clone();
                    path.0.push_back(transform.clone());
                    paths.insert(path);
                }
            },
            Constructor::Sum(source_elems) => for path in &self.0 {
                if match path.target() {
                    Some(elem@ Constructor::Atom(_)) |
                    Some(elem @ Constructor::Product(_)) =>
                        source_elems.contains(elem),
                    Some(Constructor::Sum(target_elems)) =>
                        !source_elems.clone().intersection(target_elems.clone()).is_empty(),
                    None => continue
                } {
                    let mut path = path.clone();
                    path.0.push_back(transform.clone());
                    paths.insert(path);
                }
            },
            Constructor::Product(elems) => for elem in elems {
                for path in &self.0 {
                    // recursively match elem?
                }
            },
        }

        Arrow(paths)
    }
}