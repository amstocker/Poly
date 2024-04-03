

#[derive(Debug)]
pub enum Recognized {
  All,
  Partial,
  Error
}

pub struct ReversibleStack<T> {
  pub inner: Vec<T>,
  remainder: Vec<T>
}

impl<T: Copy> ReversibleStack<T> {
  pub fn pop(&mut self) -> Option<T> {
    let value = self.inner.pop();
    if let Some(value) = value {
      self.remainder.push(value);
    }
    value
  }

  pub fn undo(&mut self) {
    if let Some(value) = self.remainder.pop() {
      self.inner.push(value);
    }
  }

  pub fn undo_all(&mut self) {
    while let Some(value) = self.remainder.pop() {
      self.inner.push(value);
    }
  }
}

impl<T> Into<ReversibleStack<T>> for Vec<T> {
  fn into(self) -> ReversibleStack<T> {
    ReversibleStack {
      inner: self,
      remainder: Vec::new()
    }
  }
}

impl<T> Into<Vec<T>> for ReversibleStack<T> {
  fn into(self) -> Vec<T> {
    self.inner
  }
}




pub enum Orientation {
  Forward,
  Backward
}

pub enum OrientedIterator<I1, I2> {
  Forward(I1),
  Backward(I2)
}

impl<T, I1, I2> Iterator for OrientedIterator<I1, I2>
where
  I1: Iterator<Item = T>, I2: Iterator<Item = T>
{
  type Item = I1::Item;
  
  fn next(&mut self) -> Option<T> {
    match self {
        OrientedIterator::Forward(iter) => iter.next(),
        OrientedIterator::Backward(iter) => iter.next(),
    }
  }
}

pub struct OrientedVec<T> {
  data: Vec<T>,
  orientation: Orientation
}

impl<T> OrientedVec<T> {
  fn iter(&self) -> impl Iterator<Item = &T> {
    match self.orientation {
      Orientation::Forward =>
        OrientedIterator::Forward(self.data.iter()),
      Orientation::Backward =>
        OrientedIterator::Backward(self.data.iter().rev()),
    }
  }
}