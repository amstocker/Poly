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
  prev: HashSet<ElemIndex>
}


#[derive(Clone, Copy)]
pub struct Iter<'a, T> {
  domain: &'a Domain<T>,
  index: Option<ElemIndex>
}

impl<'a, T> Iterator for Iter<'a, T> where T: Copy {
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
  elems: Vec<Elem<T>>,
  maximal_elems: HashMap<T, HashSet<ElemIndex>>
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
    let elem = self.elems.get_mut(index)?;
    self.maximal_elems
      .entry(elem.value)
      .or_insert(HashSet::new())
      .insert(elem.index);
    Some(index)
  }

  pub fn new_with_next(
    &mut self,
    mut values: impl Iterator<Item = T>,
    next: Option<ElemIndex>
  ) -> Option<ElemIndex> {
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
            prev: HashSet::new()
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

  pub fn recognize(&self, mut queue: Vec<T>) -> PartialResult<ElemIndex, Vec<T>> {
    if let Some(value) = queue.pop() {
      for elem in match self.maximal_elems.get(&value) {
        Some(elems) =>
          elems.iter().map(|&index| self.elems.get(index).unwrap()),
        None =>
          return PartialResult::Error(queue)
      } {
        queue = match self.recognize_at_index(queue, elem.next) {
          PartialResult::Error(queue) => queue,
          result =>
            return result.map(|_| elem.index)
        }
      }
      queue.push(value);
    }
    PartialResult::Error(queue)
  }

  pub fn recognize_at_index(
    &self,
    mut queue: Vec<T>,
    index: Option<ElemIndex>
  ) -> PartialResult<(), Vec<T>> {
    match (
      queue.pop(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(value), Some(elem)) =>
        if elem.value == value {
          match self.recognize_at_index(queue, elem.next) {
            PartialResult::Error(mut queue) => {
              queue.push(value);
              PartialResult::Error(queue)
            },
            result => result
          }
        } else {
          queue.push(value);
          PartialResult::Error(queue)
      },
      (Some(value), None) => {
        queue.push(value);
        PartialResult::Partial((), queue)
      },
      (None, None) => {
        PartialResult::Ok((), queue)
      },
      (None, Some(_)) => {
        PartialResult::Error(queue)
      }
    }
  }
}




