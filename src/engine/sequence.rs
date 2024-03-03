use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type SequenceIndex = usize;

#[derive(Debug)]
pub struct SequenceElem<A> {
  pub index: SequenceIndex,
  pub prev: Option<SequenceIndex>,
  pub next: Option<SequenceIndex>,
  pub action: A
}

#[derive(Debug)]
pub struct SequenceContext<A> {
  id: usize,
  pub data: Vec<SequenceElem<A>>,
  cursor: SequenceIndex
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
        data: Vec::new(),
        cursor: 0
      }
    }
}

impl<A> SequenceContext<A>
where
  A: Eq + Copy
{
  pub fn new_sequence(&mut self, actions: &[A]) -> Option<SequenceIndex> {
    self.find(actions).or_else(|| {
      let mut actions = actions.into_iter().copied().peekable();
      let index = actions.peek().map(|_| self.cursor);
      let mut prev = None;
      while let Some(action) = actions.next() {
        let elem = SequenceElem {
          index: self.cursor,
          prev,
          next: actions.peek().map(|_| self.cursor + 1),
          action
        };
        self.data.push(elem);
        prev = Some(self.cursor);
        self.cursor += 1;
      }
      index
    })
  }

  pub fn find(&self, actions: &[A]) -> Option<SequenceIndex> {
    self.data.iter()
      .filter(|&elem| self.is_same_sequence(actions, elem.index, None))
      .map(|elem| elem.index)
      .next()
  }

  fn is_same_sequence(&self, actions: &[A], index: SequenceIndex, prev: Option<SequenceIndex>) -> bool {
    if actions.len() == 0 {
      false
    } else if let (Some(elem), Some(&action)) = (self.data.get(index), actions.get(0)) {
      elem.action == action
      && elem.prev == prev
      && if actions.len() == 1 {
        elem.next == None
      } else {
        if let Some(next_index) = elem.next {
          self.is_same_sequence(&actions[1..], next_index, Some(index))
        } else {
          false
        }
      }
    } else {
      false
    }
  }

  pub fn get_action(&self, index: SequenceIndex) -> Option<A> {
    self.data.get(index).map(|elem| elem.action)
  }

  pub fn get_action_sequence(&self, mut index: SequenceIndex) -> Vec<A> {
    let mut actions = Vec::new();
    while let Some(elem) = self.data.get(index) {
      actions.push(elem.action);
      index = match elem.next {
        Some(index) => index,
        None => break,
      }
    }
    actions
  }
}





