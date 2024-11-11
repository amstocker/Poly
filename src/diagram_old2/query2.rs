use std::collections::BinaryHeap;

use im::Vector;



pub trait Arrow<T>: Sized + Clone + Eq {
    fn source(&self) -> T;
    fn target(&self) -> T;
    fn append_to(&self, path: &Path<Self>) -> impl IntoIterator<Item = Path<Self>>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path<A: Clone + Eq>(Vector<A>);

impl<A: Clone + Eq> PartialOrd for Path<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.depth().partial_cmp(&self.depth())
    }
}

impl<A: Clone + Eq> Ord for Path<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.depth().cmp(&self.depth())
    }
}

impl<A: Clone + Eq> Path<A> {
    pub fn singleton(arrow: A) -> Path<A> {
        Path(Vector::from_iter([arrow]))
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn append<'a, T: 'a>(&'a self, arrow: &'a A) -> impl IntoIterator<Item = Path<A>> + 'a
    where
        A: Arrow<T>
    {
        arrow.append_to(self)
    }

    pub fn target<T>(&self) -> Option<T> where A: Arrow<T> {
        self.0.last().map(|arrow| arrow.target())
    }

    pub fn push(&self, arrow: &A) -> Path<A> {
        let mut arrows = self.0.clone();
        arrows.push_back(arrow.clone());
        Path(arrows)
    }
}

#[derive(Debug)]
pub struct Query<T, A>
where
    A: Arrow<T> + Clone + Eq
{
    arrows: Vec<A>,
    paths: BinaryHeap<Path<A>>,
    target: T
}

impl<T, A> Query<T, A>
where
    T: Eq,
    A: Arrow<T> + Clone + Eq
{
    pub fn new(arrows: impl IntoIterator<Item = A>, source: T, target: T) -> Query<T, A> {
        let arrows = arrows.into_iter()
            .collect::<Vec<_>>();
        let paths = arrows.iter()
            .cloned()
            .filter(|arrow| arrow.source() == source)
            .map(|arrow| Path::singleton(arrow))
            .collect();
        Query {
            arrows,
            paths,
            target
        }
    }
}

impl<T, A> Iterator for Query<T, A>
where
    T: Eq,
    A: Arrow<T> + Clone + Eq
{
    type Item = Path<A>;

    fn next(&mut self) -> Option<Self::Item> {
        let path = self.paths.pop()?;

        for arrow in &self.arrows {
            self.paths.extend(path.append(arrow));
        }
        if let Some(target) = path.target() {
            if target == self.target {
                return Some(path);
            }
        }
        self.next()
    }
}