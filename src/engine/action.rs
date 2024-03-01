use std::sync::atomic::{AtomicUsize, Ordering};



pub type SequenceIndex = usize;

#[derive(Debug)]
pub struct SequenceElem<'a, A> {
  index: SequenceIndex,
  prev: Option<SequenceIndex>,
  next: Option<SequenceIndex>,
  action: &'a A
}

#[derive(Debug)]
pub struct SequenceContext<'a, A> where A: Eq {
  id: usize,
  pub data: Vec<SequenceElem<'a, A>>,
  cursor: SequenceIndex
}

impl<'a, A> PartialEq for SequenceContext<'a, A> where A: Eq {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl<'a, A> SequenceContext<'a, A> where A: Eq {
  pub fn new() -> Self {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    Self {
      id: COUNTER.fetch_add(1, Ordering::Relaxed),
      data: Vec::new(),
      cursor: 0
    }
  }

  pub fn new_sequence<T: IntoIterator<Item = &'a A>>(&mut self, actions: T) -> Option<SequenceIndex> {
    let mut actions = actions.into_iter().peekable();
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
}





