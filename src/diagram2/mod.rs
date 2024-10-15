pub mod constructor;


pub mod object {
    use std::hash::{Hash, Hasher};


    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Identifier {
        pub value: usize
    }

    impl From<usize> for Identifier {
        fn from(value: usize) -> Self {
            Identifier { value }
        }
    }

    #[derive(Debug)]
    pub enum Constructor {
        Atom(Identifier),
        Sum(Vec<Identifier>),
        Product(Vec<Identifier>)
    }

    impl Constructor {
        pub fn atom<I: Into<Identifier>>(id: I) -> Constructor {
            Self::Atom(id.into())
        }

        pub fn sum<T, I>(ids: T) -> Constructor
        where
            T: IntoIterator<Item = I>,
            I: Into<Identifier>
        {
            Self::Sum(ids.into_iter().map(|id| id.into()).collect())
        }

        pub fn product<T, I>(ids: T) -> Constructor
        where
            T: IntoIterator<Item = I>,
            I: Into<Identifier>
        {
            Self::Product(ids.into_iter().map(|id| id.into()).collect())
        }
    }

    impl Hash for Constructor {
        fn hash<H: Hasher>(&self, state: &mut H) {
            core::mem::discriminant(self).hash(state);
            match self {
                Constructor::Atom(id) => {
                    id.value.hash(state);
                },
                Constructor::Sum(ids) => {
                    let mut ids = ids.clone();
                    ids.sort();
                    ids.hash(state);
                },
                Constructor::Product(ids) => {
                    let mut ids = ids.clone();
                    ids.sort();
                    ids.hash(state);
                },
            }
        }
    }
    
    #[derive(Debug, Hash)]
    pub struct Object {
        pub id: Identifier,
        pub constructor: Constructor
    }
}


pub mod arrow {
    use super::object;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Identifier {
        value: usize
    }

    #[derive(Debug)]
    pub enum Constructor {
        Atom,
        Parallel(Vec<Identifier>),
        Sequential(Vec<Identifier>)
    }

    #[derive(Debug)]
    pub struct Arrow {
        // `source` and `target` object should also be able to be recursively
        // built from the constructor...
        pub id: Identifier,
        pub source: object::Identifier,
        pub target: object::Identifier,
        pub constructor: Constructor
    }
}



use std::collections::HashMap;

use arrow::Arrow;
use object::Object;


pub struct Monad {
    object: object::Identifier,
    mult: arrow::Identifier,
    unit: arrow::Identifier
}

pub struct Comonad {
    object: object::Identifier,
    comult: arrow::Identifier,
    counit: arrow::Identifier
}

pub struct Engine {
    objects:  HashMap<u64, Object>,
    arrows:   Vec<Arrow>,
    monads:   Vec<Monad>,
    comonads: Vec<Comonad>
    
    // For Monads, we want to actually keep track of arrows that defer to
    // some power of the object of the monad.  If none of those exist, there is
    // no point in expanding powers of a monadic object in the Query.
}

pub struct QueryResult<'engine> {
    engine: &'engine Engine,
    existential: Arrow,
    stack: Vec<arrow::Identifier>,
    history: ()
}

impl<'engine> QueryResult<'engine> {
    pub fn new(engine: &'engine Engine, existential: Arrow) -> QueryResult {
        QueryResult {
            engine,
            existential,
            stack: Vec::new(),
            history: ()
        }
    }
}

impl<'engine> Iterator for QueryResult<'engine> {
    type Item = arrow::Identifier;

    fn next(&mut self) -> Option<Self::Item> {
        // Implement a breadth-first search on Arrows, constructing new Arrows once a match has been found.
        // Hash on Domains/Arrows that uniquely characterizes?:
        //      https://stackoverflow.com/questions/1988665/hashing-a-tree-structure
        let source = self.existential.source;

        // look for arrows whose target is this source.
        for arrow in &self.engine.arrows {
            
        }

        unimplemented!()
    }
}

impl Engine {
    pub fn query(&self, existential: Arrow) -> QueryResult {
        QueryResult::new(self, existential)
    }
}