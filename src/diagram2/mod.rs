pub mod object;
pub mod action;


pub use object::*;
pub use action::*;


pub trait Arrow<T> {
    fn source(&self) -> Object<T>;
    fn target(&self) -> Object<T>;
}

impl<T, A> Arrow<T> for Action<A> where A: Arrow<T> {
    fn source(&self) -> Object<T> {
        todo!()
    }

    fn target(&self) -> Object<T> {
        todo!()
    }
}
