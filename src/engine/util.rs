
pub trait IndexedIterator: Iterator {
  fn index(&self) -> usize;
}