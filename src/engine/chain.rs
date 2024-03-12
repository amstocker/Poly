use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use super::util::{PartialResult, Transducer};


pub type ChainIndex = usize;

#[derive(Debug)]
pub struct ChainElem<A> {
  index: ChainIndex,
  action: A,
  next: Option<ChainIndex>,
  prev: HashSet<ChainIndex>
}


#[derive(Clone, Copy)]
pub struct ChainIter<'a, A> {
  context: &'a ChainContext<A>,
  index: Option<ChainIndex>
}

impl<'a, A> Iterator for ChainIter<'a, A> where A: Copy {
  type Item = A;
  
  fn next(&mut self) -> Option<A> {
    self.index.and_then(|index|
      self.context.elems.get(index)
    ).map(|elem| {
      self.index = elem.next;
      elem.action
    })
  }
}


#[derive(Default, Debug)]
pub struct ChainContext<A> {
  elems: Vec<ChainElem<A>>,
  maximal_elems: HashMap<A, HashSet<ChainIndex>>
}

impl<A> ChainContext<A>
where
  A: Eq + Copy + Hash + Debug
{
  pub fn get_chain(&self, index: ChainIndex) -> ChainIter<A> {
    ChainIter { context: &self, index: Some(index) }
  }
  
  pub fn new_chain(&mut self, actions: impl Iterator<Item = A>) -> Option<ChainIndex> {
    let index = self.new_chain_with_next(actions, None)?;
    let elem = self.elems.get_mut(index)?;
    self.maximal_elems
      .entry(elem.action)
      .or_insert(HashSet::new())
      .insert(elem.index);
    Some(index)
  }

  pub fn new_chain_with_next(
    &mut self,
    mut actions: impl Iterator<Item = A>,
    next: Option<ChainIndex>
  ) -> Option<ChainIndex> {
    actions.next()
      .and_then(|action| {
        let index = if let Some(elem) = self.elems.iter().find(|&elem|
          elem.action == action
          && elem.next == next
        ) {
          elem.index
        } else {
          let index = self.elems.len();
          self.elems.push(ChainElem {
            index,
            action,
            next,
            prev: HashSet::new()
          });
          index
        };

        next.and_then(|next_index|
          self.elems.get_mut(next_index)
            .map(|elem| elem.prev.insert(index))
        );

        self.new_chain_with_next(actions, Some(index))
      })
      .or(next)
  }

  pub fn recognize_chain(&self, mut queue: Vec<A>) -> PartialResult<ChainIndex, Vec<A>> {
    if let Some(action) = queue.pop() {
      for elem in match self.maximal_elems.get(&action) {
        Some(elems) =>
          elems.iter().map(|&index| self.elems.get(index).unwrap()),
        None =>
          return PartialResult::Error(queue)
      } {
        queue = match self.recognize_chain_at_index(queue, elem.next) {
          PartialResult::Error(queue) => queue,
          result =>
            return result.map(|_| elem.index)
        }
      }
      queue.push(action);
    }
    PartialResult::Error(queue)
  }

  pub fn recognize_chain_at_index(
    &self,
    mut queue: Vec<A>,
    index: Option<ChainIndex>
  ) -> PartialResult<(), Vec<A>> {
    match (
      queue.pop(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(action), Some(elem)) =>
        if elem.action == action {
          match self.recognize_chain_at_index(queue, elem.next) {
            PartialResult::Error(mut queue) => {
              queue.push(action);
              PartialResult::Error(queue)
            },
            result => result
          }
        } else {
          queue.push(action);
          PartialResult::Error(queue)
      },
      (Some(action), None) => {
        queue.push(action);
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




