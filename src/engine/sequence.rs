use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type SequenceIndex = usize;

#[derive(Debug)]
pub struct SequenceElem<A> {
  pub index: SequenceIndex,
  pub action: A,
  pub prev: Option<SequenceIndex>,
  pub next: HashSet<SequenceIndex>
}

#[derive(Clone, Copy)]
pub struct SequenceIter<'a, A> {
  context: &'a SequenceContext<A>,
  index: Option<SequenceIndex>
}

impl<'a, A> Iterator for SequenceIter<'a, A> where A: Copy {
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
pub struct SequenceContext<A> {
  id: usize,
  pub data: Vec<SequenceElem<A>>
}

impl<A> PartialEq for SequenceContext<A> {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl<A> Default for SequenceContext<A> {
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

impl<A> SequenceContext<A>
where
  A: Eq + Copy
{
  pub fn new_sequence<I: Iterator<Item = A> + Clone>(&mut self, actions: I) -> Option<SequenceIndex> {
    self.add_sequence(actions, None)
  }

  pub fn add_sequence<I: Iterator<Item = A> + Clone>(&mut self, mut actions: I, prev: Option<SequenceIndex>) -> Option<SequenceIndex> {
    actions.next()
      .and_then(|action| {
        let index = if let Some(elem) = self.data.iter().find(|&elem|
          elem.action == action
          && elem.prev == prev
        ) {
          elem.index
        } else {
          let index = self.data.len();
          self.data.push(SequenceElem { index, action, prev, next: HashSet::new() });
          index
        };

        prev.and_then(|next_index|
          self.data.get_mut(next_index)
            .map(|elem| elem.next.insert(index))
        );

        self.add_sequence(actions, Some(index))
      })
      .or(prev)
  }

  pub fn get_sequence<I: Iterator<Item = A> + Clone>(&self, actions: I) -> Option<SequenceIndex> {
    self.data.iter()
      .filter(|&elem| self.is_same_sequence(actions.clone(), Some(elem.index)))
      .map(|elem| elem.index)
      .next()
  }

  pub fn is_same_sequence<I: Iterator<Item = A> + Clone>(&self, mut actions: I, index: Option<SequenceIndex>) -> bool {
    match (actions.next(), index.and_then(|index| self.data.get(index))) {
        (Some(action), Some(elem)) =>
          elem.action == action
          && self.is_same_sequence(actions, elem.prev),
        (None, None) => true,
        (_, _) => false
    }
  }

  pub fn get_action(&self, index: SequenceIndex) -> Option<A> {
    self.data.get(index).map(|elem| elem.action)
  }

  pub fn get_action_sequence(&self, index: SequenceIndex) -> SequenceIter<A> {
    SequenceIter { context: &self, index: Some(index) }
  }
}





