pub mod action;
pub mod object;
pub mod query;

pub use action::*;
pub use object::*;
pub use query::*;



pub trait Arrow<T> {
    fn source(&self) -> Object<T>;
    fn target(&self) -> Object<T>;
}

impl<T, A> Arrow<T> for Operation<A> where A: Arrow<T> {
    fn source(&self) -> Object<T> {
        match self {
            Operation::Value(arrow) => arrow.source(),
            Operation::Identity => Object::unit(),
        }
    }

    fn target(&self) -> Object<T> {
        match self {
            Operation::Value(arrow) => arrow.target(),
            Operation::Identity => Object::unit(),
        }
    }
}

impl<T: Eq, A> Arrow<T> for Sequence<A> where A: Arrow<T> {
    fn source(&self) -> Object<T> {
        if self.data.len() == 0 {
            return Object::zero();
        }
        let first_source = self.data[0].source();
        let mut target = self.data[0].target();
        for i in 1..self.data.len() {
            if self.data[i].source() != target {
                return Object::zero();
            } else {
                target = self.data[i].target();
            }
        }
        first_source
    }

    fn target(&self) -> Object<T> {
        if self.data.len() == 0 {
            return Object::zero();
        }
        let last_target = self.data[self.data.len() - 1].target();
        let mut source = self.data[self.data.len() - 1].source();
        for i in (0..(self.data.len()-1)).rev() {
            if self.data[i].target() != source {
                return Object::zero();
            } else {
                source = self.data[i].source();
            }
        }
        last_target
    }
}

impl<T: Eq + Ord + Clone, A> Arrow<T> for Parallel<A> where A: Arrow<T> {
    fn source(&self) -> Object<T> {
        self.data.iter().fold(Object::unit(), |prod, seq| prod * seq.source())
    }

    fn target(&self) -> Object<T> {
        self.data.iter().fold(Object::unit(), |prod, seq| prod * seq.target())
    }
}

impl<T: Eq + Ord + Clone, A> Arrow<T> for Action<A> where A: Arrow<T> {
    fn source(&self) -> Object<T> {
        match self {
            Action::Operation(operation) => operation.source(),
            Action::Sequence(sequence) => sequence.source(),
            Action::Parallel(parallel) => parallel.source(),
        }
    }

    fn target(&self) -> Object<T> {
        match self {
            Action::Operation(operation) => operation.target(),
            Action::Sequence(sequence) => sequence.target(),
            Action::Parallel(parallel) => parallel.target(),
        }
    }
}
