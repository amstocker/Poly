use std::hash::{Hash, Hasher};


#[derive(Debug)]
pub enum Constructor<T> {
    Atom(T),
    Sum(Vec<T>),
    Product(Vec<T>)
}

pub type CanonicalConstructor<T> = Constructor<Constructor<Constructor<T>>>;


impl<T> Constructor<T> {
    pub fn sum<I: IntoIterator<Item = T>>(ids: I) -> Constructor<T> {
        Self::Sum(ids.into_iter().collect())
    }

    pub fn product<I: IntoIterator<Item = T>>(ids: I) -> Constructor<T> {
        Self::Product(ids.into_iter().collect())
    }

    pub fn canonical_repr(self) -> CanonicalConstructor<T> {
        unimplemented!()
    }

    pub fn distribute(&self) {
        match self {
            Constructor::Product(vec) => {
                todo!()
            },
            _ => {}
        }
    }
}

impl<T> Hash for Constructor<T> where T: Hash + Ord + Clone {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Atom(id) => id.hash(state),
            Self::Sum(ids) | Self::Product(ids) => {
                let mut ids = ids.clone();
                ids.sort();
                ids.hash(state);
            }
        }
    }
}