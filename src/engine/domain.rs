use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use super::util::PartialResult;


pub type ElemIndex = usize;

#[derive(Debug)]
pub struct Elem<T> {
  index: ElemIndex,
  value: T,
  next: Option<ElemIndex>,
  prev: HashSet<ElemIndex>,
  maximal: bool
}


#[derive(Clone, Copy)]
pub struct Iter<'a, T> {
  domain: &'a Domain<T>,
  index: Option<ElemIndex>
}

impl<T> Iterator for Iter<'_, T> where T: Copy {
  type Item = T;
  
  fn next(&mut self) -> Option<T> {
    self.index.and_then(|index|
      self.domain.elems.get(index)
    ).map(|elem| {
      self.index = elem.next;
      elem.value
    })
  }
}


#[derive(Default, Debug)]
pub struct Domain<T> {
  elems: Vec<Elem<T>>
}

impl<T> Domain<T>
where
  T: Eq + Copy + Hash
{
  pub fn get(&self, index: ElemIndex) -> Iter<T> {
    Iter { domain: &self, index: Some(index) }
  }
  
  pub fn new(&mut self, values: impl Iterator<Item = T>) -> Option<ElemIndex> {
    let index = self.new_with_next(values, None)?;
    self.elems.get_mut(index)?.maximal = true;
    Some(index)
  }

  pub fn new_with_next(&mut self, mut values: impl Iterator<Item = T>, next: Option<ElemIndex>) -> Option<ElemIndex> {
    values.next()
      .and_then(|value| {
        let index = if let Some(elem) = self.elems.iter().find(|&elem|
          elem.value == value
          && elem.next == next
        ) {
          elem.index
        } else {
          let index = self.elems.len();
          self.elems.push(Elem {
            index,
            value,
            next,
            prev: HashSet::new(),
            maximal: false
          });
          index
        };

        next.and_then(|next_index|
          self.elems.get_mut(next_index)
            .map(|elem| elem.prev.insert(index))
        );

        self.new_with_next(values, Some(index))
      })
      .or(next)
  }

  pub fn iter_maximal(&self) -> impl Iterator<Item = &Elem<T>> {
    self.elems.iter().filter(|elem| elem.maximal)
  }

  pub fn recognize(&self, mut stack: Vec<T>) -> PartialResult<ElemIndex, Vec<T>> {
    if let Some(value) = stack.pop() {
      for elem in self.iter_maximal() {
        stack = match self.recognize_at_index(stack, elem.next) {
          PartialResult::Error(stack) => stack,
          result =>
            return result.map(|_| elem.index)
        }
      }
      stack.push(value);
    }
    PartialResult::Error(stack)
  }

  pub fn recognize_at_index(&self, mut stack: Vec<T>, index: Option<ElemIndex>) -> PartialResult<(), Vec<T>> {
    match (
      stack.pop(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(value), Some(elem)) =>
        if elem.value == value {
          match self.recognize_at_index(stack, elem.next) {
            PartialResult::Error(mut stack) => {
              stack.push(value);
              PartialResult::Error(stack)
            },
            result => result
          }
        } else {
          stack.push(value);
          PartialResult::Error(stack)
      },
      (Some(value), None) => {
        stack.push(value);
        PartialResult::Partial((), stack)
      },
      (None, None) => {
        PartialResult::Ok((), stack)
      },
      (None, Some(_)) => {
        PartialResult::Error(stack)
      }
    }
  }
}




