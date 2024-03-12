use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type ChainIndex = usize;

#[derive(Debug)]
pub struct ChainElem<A> {
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
    ).map(|elem| {
      self.index = elem.prev;
      elem.action
    })
  }
}


#[derive(Debug)]
pub enum Recognized<A> {
  All {
    queue: Vec<A>
  },
  Partial {
    queue: Vec<A>
  },
  Error {
    queue: Vec<A>
  }
}

#[derive(Debug)]
pub enum RecognizedIndex<A> {
  All {
    index: ChainIndex,
    queue: Vec<A>
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
        Recognized::All { queue } => RecognizedIndex::All { index, queue },
        Recognized::Partial { queue } => RecognizedIndex::Partial { index, queue },
        Recognized::Error { queue } => RecognizedIndex::Error { queue }
    }
  }
}

impl<A> From<RecognizedIndex<A>> for Option<ChainIndex> {
  fn from(value: RecognizedIndex<A>) -> Self {
    match value {
      RecognizedIndex::All { index, .. } => Some(index),
      RecognizedIndex::Partial { index, .. } => Some(index),
      RecognizedIndex::Error { .. } => None
    }
  }
}


#[derive(Debug)]
pub enum ChainDirection {
  Forward,
  Backward
}


#[derive(Debug)]
pub struct ChainContext<A> {
  id: usize,
  data: Vec<ChainElem<A>>,
  direction: ChainDirection,
  ends: HashMap<A, HashSet<ChainIndex>>
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
        data: Vec::new(),
        direction: ChainDirection::Forward,
        ends: HashMap::new()
      }
    }
}

impl<A> ChainContext<A>
where
  A: Eq + Copy + Hash + Debug
{
  pub fn get_chain(&self, index: ChainIndex) -> ChainIter<A> {
    ChainIter { context: &self, index: Some(index) }
  }
  
  pub fn new_chain(&mut self, actions: impl Iterator<Item = A>) -> Option<ChainIndex> {
    let index = self.new_chain_with_prev(actions, None)?;
    let elem = self.data.get_mut(index)?;
    self.ends
      .entry(elem.action)
      .or_insert(HashSet::new())
      .insert(elem.index);
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
            next: HashSet::new()
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

  pub fn recognize_chain(&self, mut queue: Vec<A>) -> RecognizedIndex<A> {
    if let Some(action) = queue.pop() {
      let ends = self.ends.get(&action)
        .unwrap()
        .iter()
        .map(|&index| self.data.get(index).unwrap());
      for elem in ends {
        queue = match self.recognize_chain_at_index(queue, elem.prev) {
          Recognized::Error { queue } => queue,
          result => {
            return result.with_index(elem.index);
          }
        }
      }
      queue.push(action);
    }
    RecognizedIndex::Error { queue }
  }

  pub fn recognize_chain_at_index(&self, mut queue: Vec<A>, index: Option<ChainIndex>) -> Recognized<A> {
    match (
      queue.pop(),
      index.and_then(|index| self.data.get(index))
    ) {
      (Some(action), Some(elem)) =>
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
      },
      (Some(action), None) => {
        queue.push(action);
        Recognized::Partial { queue }
      },
      (None, None) => {
        Recognized::All { queue }
      },
      (None, Some(_)) => {
        Recognized::Error { queue }
      }
    }
  }
}




