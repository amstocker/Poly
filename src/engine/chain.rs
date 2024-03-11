use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type ChainIndex = usize;

#[derive(Debug)]
pub struct ChainElem<A> {
  pub index: ChainIndex,
  pub action: A,
  pub prev: Option<ChainIndex>,
  pub next: HashSet<ChainIndex>,
  pub is_end: bool
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


pub enum Recognized<A> {
  All,
  Partial {
    queue: Vec<A>
  },
  Error {
    queue: Vec<A>
  }
}

pub enum RecognizedIndex<A> {
  All {
    index: ChainIndex
  },
  Partial {
    index: ChainIndex,
    queue: Vec<A>
  },
  Error {
    queue: Vec<A>
  }
}

impl<A> Recognized<A> {
  pub fn with_index(self, index: ChainIndex) -> RecognizedIndex<A> {
    match self {
        Recognized::All => RecognizedIndex::All { index },
        Recognized::Partial { queue } => RecognizedIndex::Partial { index, queue },
        Recognized::Error { queue } => RecognizedIndex::Error { queue }
    }
  }
}

impl<A> From<RecognizedIndex<A>> for Option<ChainIndex> {
  fn from(value: RecognizedIndex<A>) -> Self {
    match value {
      RecognizedIndex::All { index } => Some(index),
      RecognizedIndex::Partial { index, .. } => Some(index),
      RecognizedIndex::Error { .. } => None
    }
  }
}


#[derive(Debug)]
pub struct ChainContext<A> {
  id: usize,
  pub data: Vec<ChainElem<A>>
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
  pub fn new_chain(&mut self, actions: impl Iterator<Item = A>) -> Option<ChainIndex> {
    let index = self.new_chain_with_prev(actions, None)?;
    let elem = self.data.get_mut(index)?;
    elem.is_end = true;
    Some(index)
  }

  pub fn new_chain_with_prev(
    &mut self,
    mut actions: impl Iterator<Item = A>,
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
          self.data.push(ChainElem {
            index,
            action,
            prev,
            next: HashSet::new(),
            is_end: false
          });
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

  fn iter_ends(&self) -> impl Iterator<Item = &ChainElem<A>> {
    self.data.iter().filter(|elem| elem.is_end)
  }

  pub fn recognize_chain(&self, mut queue: Vec<A>) -> RecognizedIndex<A> {
    for elem in self.iter_ends() {
      queue = match self.recognize_chain_at_index(queue, Some(elem.index)) {
        Recognized::Error { queue } => queue,
        result => {
          return result.with_index(elem.index);
        }
      }
    }
    RecognizedIndex::Error { queue }
  }

  pub fn recognize_chain_at_index(&self, mut queue: Vec<A>, index: Option<ChainIndex>) -> Recognized<A> {
    match (
      queue.pop(),
      index.and_then(|index| self.data.get(index))
    ) {
      (Some(action), Some(elem)) => {
        if elem.action == action {
          match self.recognize_chain_at_index(queue, elem.prev) {
            Recognized::Error { mut queue } => {
              queue.push(action);
              Recognized::Error { queue }
            },
            result => result
          }
        } else {
          queue.push(action);
          Recognized::Error { queue }
        }
      },
      (Some(action), None) => {
        queue.push(action);
        Recognized::Error { queue }
      },
      (None, Some(_)) => {
        Recognized::Partial { queue }
      },
      (None, None) => {
        Recognized::All
      }
    }
  }

  pub fn get_action(&self, index: ChainIndex) -> Option<A> {
    self.data.get(index).map(|elem| elem.action)
  }

  pub fn get_action_chain(&self, index: ChainIndex) -> ChainIter<A> {
    ChainIter { context: &self, index: Some(index) }
  }
}




