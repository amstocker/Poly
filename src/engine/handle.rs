use std::marker::PhantomData;


pub struct Handle<T> {
  id: usize,
  marker: PhantomData<T>
}

pub struct Store<T> {
  data: Vec<T>
}