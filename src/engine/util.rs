use std::collections::{HashMap, hash_map};
use std::marker::PhantomData;

use crate::engine::{Action, State};
use crate::engine::rule::Lens;



#[derive(Debug)]
pub enum PartialResult<A, B> {
  Ok(A, B),
  Partial(A, B),
  Error(B)
}

pub trait Transducer<A, B> {
  fn map<C>(self, f: impl FnOnce(A) -> C) -> PartialResult<C, B>;
  fn transducer(eval: impl Fn(B) -> PartialResult<A, B>, update: impl Fn(A, B) -> B) -> impl Fn(B) -> Result<B, B>;
}

impl<A, B> Transducer<A, B> for PartialResult<A, B> {
  fn map<C>(self, f: impl FnOnce(A) -> C) -> PartialResult<C, B> {
    match self {
        PartialResult::Ok(a, b) => PartialResult::Ok(f(a), b),
        PartialResult::Partial(a, b) => PartialResult::Partial(f(a), b),
        PartialResult::Error(b) => PartialResult::Error(b),
    }
  }

  fn transducer(
    eval: impl Fn(B) -> PartialResult<A, B>,
    update: impl Fn(A, B) -> B
  ) -> impl Fn(B) -> Result<B, B> {
    move |mut data| {
      loop {
        data = match eval(data) {
          PartialResult::Partial(intermediate, data) => update(intermediate, data),
          PartialResult::Ok(intermediate, data) => return Ok(update(intermediate, data)),
          PartialResult::Error(data) => return Err(data),
        }
      }
    }
  }
}

pub trait Middleware<A, B, C, D> {
  
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Labeled {
  Action(Action),
  State(State),
  Lens(Lens)
}

impl Into<Labeled> for State {
  fn into(self) -> Labeled {
    Labeled::State(self)
  }
}

impl Into<Labeled> for Action {
  fn into(self) -> Labeled {
    Labeled::Action(self)
  }
}

impl Into<Labeled> for Lens {
  fn into(self) -> Labeled {
    Labeled::Lens(self)
  }
}

impl From<Labeled> for Option<State> {
  fn from(value: Labeled) -> Option<State> {
    match value {
      Labeled::State(state) => Some(state),
      _ => None
    }
  }
}

impl From<Labeled> for Option<Action> {
  fn from(value: Labeled) -> Option<Action> {
    match value {
      Labeled::Action(action) => Some(action),
      _ => None
    }
  }
}

impl From<Labeled> for Option<Lens> {
  fn from(value: Labeled) -> Self {
    match value {
      Labeled::Lens(lens) => Some(lens),
      _ => None
    }
  }
}


#[derive(Default)]
pub struct LabelMap(HashMap<String, Labeled>);

impl LabelMap {
  pub fn insert(&mut self, label: String, value: impl Into<Labeled>) {
    self.0.insert(label.into(), value.into());
  }

  pub fn get<T>(&self, label: impl AsRef<str>) -> Option<T>
  where
    Option<T>: From<Labeled>
  {
    self.0.get(label.as_ref()).copied()?.into()
  }

  pub fn reverse_lookup(&self, value: impl Into<Labeled> + Copy) -> Option<&String> {
    self.0.iter()
      .find(|(_, &v)| v == value.into())
      .map(|(label, _)| label)
  }

  pub fn iter<'a, T>(&'a self) -> impl Iterator<Item = T> + 'a
  where
    Option<T>: From<Labeled>
  {
    self.0.iter()
      .filter_map(|(_, &value)| value.into())
  }

  pub fn iter_all(&self) -> hash_map::Iter<String, Labeled> {
    self.0.iter()
  }
}



pub trait Indexed {
  fn build_with_index(index: usize) -> Self;
}

pub struct Index<T> {
  index: usize,
  marker: PhantomData<T>
}


impl Indexed for State {
  fn build_with_index(index: usize) -> Self {
    State { index }
  }
}

impl Indexed for Action {
  fn build_with_index(index: usize) -> Self {
    Action { index } 
  }
}

#[derive(Default)]
pub struct IndexedHandler<T> {
  data: Vec<T>
}

impl<T> IndexedHandler<T> where T: Indexed + Copy {
  pub fn new(&mut self) -> T {
    let value = T::build_with_index(self.data.len());
    self.data.push(value);
    value
  }
}