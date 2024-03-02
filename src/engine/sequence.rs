use std::sync::atomic::{AtomicUsize, Ordering};


pub type SequenceIndex = usize;

#[derive(Debug)]
pub struct SequenceElem<A> {
  index: SequenceIndex,
  prev: Option<SequenceIndex>,
  next: Option<SequenceIndex>,
  action: A
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
      static COUNTER: AtomicUsize = AtomicUsize::new(1);
      Self {
        id: COUNTER.fetch_add(1, Ordering::Relaxed),
        data: Vec::new(),
        cursor: 0
      }
    }
}

impl<'a, A> SequenceContext<A> where A: 'a + Eq + Copy {
  pub fn new_sequence<T: IntoIterator<Item = &'a A> + Clone>(&mut self, actions: T) -> Option<SequenceIndex> {
    if let Some(index) = self.find(actions.clone()) { return Some(index); }

    let mut actions = actions.into_iter().copied().peekable();
    let index = actions.peek().map(|_| self.cursor);
    let mut prev = None;
    loop {
      if let Some(action) = actions.next() {
        let seq = SequenceElem {
          index: self.cursor,
          prev,
          next: actions.peek().map(|_| self.cursor + 1),
          action
        };
        self.data.push(seq);
        prev = Some(self.cursor);
        self.cursor += 1;
      } else {
        break;
      }
    }

    index
  }

  pub fn find<T: IntoIterator<Item = &'a A>>(&self, actions: T) -> Option<SequenceIndex> {
    todo!()
  }
}





