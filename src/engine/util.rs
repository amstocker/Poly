

#[derive(Debug)]
pub enum PartialResult<A, B> {
  Ok(A, B),
  Partial(A, B),
  Error(B)
}

impl<A, B> PartialResult<A, B> {
  pub fn map<C>(self, f: impl FnOnce(A) -> C) -> PartialResult<C, B> {
    match self {
        PartialResult::Ok(a, b) => PartialResult::Ok(f(a), b),
        PartialResult::Partial(a, b) => PartialResult::Partial(f(a), b),
        PartialResult::Error(b) => PartialResult::Error(b),
    }
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