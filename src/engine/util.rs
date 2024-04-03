

#[derive(Debug)]
pub enum PartialResult<A> {
  Ok(A),
  Partial(A),
  Error
}

impl<A> PartialResult<A> {
  pub fn map<B>(self, f: impl FnOnce(A) -> B) -> PartialResult<B> {
    match self {
        PartialResult::Ok(a) => PartialResult::Ok(f(a)),
        PartialResult::Partial(a) => PartialResult::Partial(f(a)),
        PartialResult::Error => PartialResult::Error,
    }
  }
}

impl<A> Into<Option<A>> for PartialResult<A> {
    fn into(self) -> Option<A> {
      match self {
        PartialResult::Ok(a) => Some(a),
        PartialResult::Partial(a) => Some(a),
        PartialResult::Error => None,
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