use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type ChainIndex = usize;

#[derive(Debug)]
pub struct Chain<A> {
  index: ChainIndex,
  action: A,
  prev: Option<ChainIndex>,
  next: HashSet<ChainIndex>
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
      self.context.data.get(index)
        .map(|elem| {
          self.index = elem.prev;
          elem.action
        })
    )
  }
}


#[derive(Debug)]
pub struct ChainContext<A> {
  id: usize,
  data: Vec<Chain<A>>
}

impl<A> PartialEq for ChainContext<A> {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl<A> Default for ChainContext<A> {
    fn default() -> Self {
      Self {
        id: (|| {
          static COUNTER: AtomicUsize = AtomicUsize::new(1);
          COUNTER.fetch_add(1, Ordering::Relaxed)
        })(),
        data: Vec::new()
      }
    }
}

impl<A> ChainContext<A>
where
  A: Eq + Copy
{
  pub fn new_chain(&mut self, actions: impl Iterator<Item = A> + Clone) -> Option<ChainIndex> {
    self.new_chain_with_prev(actions, None)
  }

  pub fn new_chain_with_prev(
    &mut self,
    mut actions: impl Iterator<Item = A> + Clone,
    prev: Option<ChainIndex>
  ) -> Option<ChainIndex> {
    actions.next()
      .and_then(|action| {
        let index = if let Some(elem) = self.data.iter().find(|&elem|
          elem.action == action
          && elem.prev == prev
        ) {
          elem.index
        } else {
          let index = self.data.len();
          self.data.push(Chain { index, action, prev, next: HashSet::new() });
          index
        };

        prev.and_then(|prev_index|
          self.data.get_mut(prev_index)
            .map(|elem| elem.next.insert(index))
        );

        self.new_chain_with_prev(actions, Some(index))
      })
      .or(prev)
  }

  // `get_chain` expects a _stack_ of actions (i.e. most recent action first!)
  pub fn get_chain(
    &self,
    actions: impl Iterator<Item = A> + Clone
  ) -> Option<ChainIndex> {
    self.data.iter()
      .filter(|&elem| self.is_same_chain(actions.clone(), Some(elem.index)))
      .map(|elem| elem.index)
      .next()
  }

  // ``is_same_chain` also expects a _stack_ of actions.
  pub fn is_same_chain(
    &self,
    mut actions: impl Iterator<Item = A> + Clone,
    index: Option<ChainIndex>
  ) -> bool {
    match (actions.next(), index.and_then(|index| self.data.get(index))) {
        (Some(action), Some(elem)) =>
          elem.action == action
          && self.is_same_chain(actions, elem.prev),
        (None, None) => true,
        (_, _) => false
    }
  }

  pub fn get_action(&self, index: ChainIndex) -> Option<A> {
    self.data.get(index).map(|elem| elem.action)
  }

  pub fn get_action_chain(&self, index: ChainIndex) -> ChainIter<A> {
    ChainIter { context: &self, index: Some(index) }
  }
}





