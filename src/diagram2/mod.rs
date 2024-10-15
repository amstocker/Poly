pub mod object;
pub mod arrow;


use object::*;
use arrow::*;


pub struct Monad<T> {
    object: Object<T>,
    mult: Arrow<T>,
    unit: Arrow<T>
}

pub struct Comonad<T> {
    object: Object<T>,
    comult: Arrow<T>,
    counit: Arrow<T>
}

pub struct Engine<T> {
    objects:  Vec<Object<T>>,
    arrows:   Vec<Arrow<T>>,
    monads:   Vec<Monad<T>>,
    comonads: Vec<Comonad<T>>
    
    // For Monads, we want to actually keep track of arrows that defer to
    // some power of the object of the monad.  If none of those exist, there is
    // no point in expanding powers of a monadic object in the Query.
}

pub struct QueryResult<'engine, T> {
    engine: &'engine Engine<T>,
    existential: Arrow<T>,
    stack: Vec<Arrow<T>>,
    history: ()
}

impl<'engine, T> QueryResult<'engine, T> {
    pub fn new(engine: &'engine Engine<T>, existential: Arrow<T>) -> QueryResult<T> {
        QueryResult {
            engine,
            existential,
            stack: Vec::new(),
            history: ()
        }
    }
}

impl<'engine, T> Iterator for QueryResult<'engine, T> {
    type Item = Arrow<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Implement a breadth-first search on Arrows, constructing new Arrows once a match has been found.
        // Hash on Domains/Arrows that uniquely characterizes?:
        //      https://stackoverflow.com/questions/1988665/hashing-a-tree-structure
        //let source = self.existential.source;

        // look for arrows whose target is this source.
        for arrow in &self.engine.arrows {
            
        }

        unimplemented!()
    }
}

// impl Engine {
//     pub fn query(&self, existential: Arrow) -> QueryResult {
//         QueryResult::new(self, existential)
//     }
// }