pub mod object;
pub mod action;


pub use object::*;
pub use action::*;


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arrow<T> {
    pub source: Object<T>,
    pub target: Object<T>
}

impl<T> Sequence<Arrow<T>> {
    pub fn source(&self) -> Object<T> {

    }

    pub fn target(&self) -> Object<T> {
        // actually want to fold..., but need "zero" and "unit" object/arrow...
        self.0.iter().reduce(|x, y| x.0.source * y.0.source)
    }
}

impl<T> Parallel<Arrow<T>> {
    pub fn source(&self) -> Object<T> {
        self.0.iter().reduce(|x, y| )
    }

    pub fn target(&self) -> Object<T> {
        
    }
}


pub struct Monad<T> {
    object: Object<T>,
    mult: Parallel<T>,
    unit: Parallel<T>
}

pub struct Comonad<T> {
    object: Object<T>,
    comult: Parallel<T>,
    counit: Parallel<T>
}

pub struct Engine<T> {
    objects:  Vec<Object<T>>,
    arrows:   Vec<Parallel<T>>,
    monads:   Vec<Monad<T>>,
    comonads: Vec<Comonad<T>>
    
    // For Monads, we want to actually keep track of arrows that defer to
    // some power of the object of the monad.  If none of those exist, there is
    // no point in expanding powers of a monadic object in the Query.
}

pub struct QueryResult<'engine, T> {
    engine: &'engine Engine<T>,
    existential: Parallel<T>,
    stack: Vec<Parallel<T>>,
    history: ()
}

impl<'engine, T> QueryResult<'engine, T> {
    pub fn new(engine: &'engine Engine<T>, existential: Parallel<T>) -> QueryResult<T> {
        QueryResult {
            engine,
            existential,
            stack: Vec::new(),
            history: ()
        }
    }
}

impl<'engine, T> Iterator for QueryResult<'engine, T> {
    type Item = Parallel<T>;

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