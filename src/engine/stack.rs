use super::util::IndexedIterator;






pub type Cursor = usize;

#[derive(Debug)]
pub struct Stack<T> {
  pub inner: Vec<T>,
  cursor: Cursor
}

impl<T: Copy> Stack<T> {
  pub fn pop(&mut self) -> Option<T> {
    if self.cursor > 0 {
      self.cursor -= 1;
      self.inner.get(self.cursor).copied()
    } else {
      None
    }
  }

  pub fn undo(&mut self) {
    self.cursor += 1;
  }

  pub fn reset(&mut self) {
    self.cursor = self.inner.len();
  }

  pub fn extend(&mut self, other: impl Iterator<Item = T>) {
    self.inner.truncate(self.cursor);
    self.inner.extend(other);
    self.reset()
  }
}


impl<T> Into<Stack<T>> for Vec<T> {
  fn into(self) -> Stack<T> {
    let cursor = self.len();
    Stack {
      inner: self,
      cursor
    }
  }
}

impl<T> Into<Vec<T>> for Stack<T> {
  fn into(self) -> Vec<T> {
    self.inner
  }
}

