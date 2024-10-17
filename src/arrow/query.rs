use super::{Action, Arrow, Atom, Object};



type Index = usize;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrowIndex {
    source: usize,
    target: usize
}

impl Arrow<Index> for ArrowIndex {
    fn source(&self) -> Object<Index> {
        Atom::Value(self.source).into()
    }

    fn target(&self) -> Object<Index> {
        Atom::Value(self.target).into()
    }
}

pub struct Engine<T> {
    atoms: Vec<Atom<T>>,
    objects: Vec<Object<T>>,
    actions: Vec<Action<ArrowIndex>>
}

pub struct QueryResult<'engine, T> {
    engine: &'engine Engine<T>,
    existential: Action<ArrowIndex>,
    stack: Vec<Action<ArrowIndex>>,
    history: ()
}

impl<'engine, T> QueryResult<'engine, T> {
    pub fn new(engine: &'engine Engine<T>, existential: Action<ArrowIndex>) -> QueryResult<T> {
        QueryResult {
            engine,
            existential,
            stack: Vec::new(),
            history: ()
        }
    }
}

impl<'engine, T> Iterator for QueryResult<'engine, T> {
    type Item = Action<ArrowIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        // Implement a breadth-first search on Arrows, constructing new Arrows once a match has been found.
        // Hash on Domains/Arrows that uniquely characterizes?:
        //      https://stackoverflow.com/questions/1988665/hashing-a-tree-structure
        //let source = self.existential.source;

        // look for arrows whose target is this source.
        //for arrow in &self.engine.arrows {
        //    
        //}

        // add actions/objects to 

        unimplemented!()
    }
}

// impl Engine {
//     pub fn query(&self, existential: Arrow) -> QueryResult {
//         QueryResult::new(self, existential)
//     }
// }