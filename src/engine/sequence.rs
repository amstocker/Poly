use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};


pub type SequenceIndex = usize;

#[derive(Debug)]
pub struct SequenceElem<A> {
  pub index: SequenceIndex,
  prev: Option<SequenceIndex>,
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
    if let Some(index) = self.find(actions) {
      return Some(index);
    }
    
    let mut actions = actions.into_iter().copied().peekable();
    let index = actions.peek().map(|_| self.cursor);
    let mut prev = None;
    while let Some(action) = actions.next() {
      let seq = SequenceElem {
        index: self.cursor,
        prev,
        next: actions.peek().map(|_| self.cursor + 1),
        action
      };
      self.data.push(seq);
      prev = Some(self.cursor);
      self.cursor += 1;
    }

    index
  }

  pub fn find(&self, actions: &[A]) -> Option<SequenceIndex> {
    self.find_helper(
      actions,
      self.data.iter(),
      None,
      None
    )
  }

  fn find_helper<'a, I>(
    &self,
    actions: &[A],
    iter: I,
    first: Option<SequenceIndex>,
    prev: Option<SequenceIndex>
  ) -> Option<SequenceIndex>
  where
    A: 'a,
    I: Iterator<Item = &'a SequenceElem<A>> + Clone
  {
    if actions.len() == 0 {
      return None;
    }

    if let Some(&action) = actions.get(0) {
      if actions.len() == 1 {
        return iter.clone()
          .find(|&e|
            e.action == action
            && e.next.is_none()
            && e.prev == prev
          )
          .and_then(|e| first.or(Some(e.index)));
      } else {

        // TODO: Doesn't type check if we just clone the filtered iterator...
        let filtered = iter
          .clone()
          .filter(|&e| e.action == action && e.prev == prev)
          .filter_map(|e| e.next)
          .map(|index| self.data.get(index).unwrap())
          .collect::<Vec<_>>();
        for e in iter {
          if let Some(index) = self.find_helper(
            &actions[1..],
            filtered.iter().cloned(),
            first.or(Some(e.index)),
            Some(e.index)
          ) {
            return Some(index);
          }
        }
      }
    }

    None
  }

  pub fn get_action(&self, index: SequenceIndex) -> Option<A> {
    self.data.get(index).map(|e| e.action)
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





