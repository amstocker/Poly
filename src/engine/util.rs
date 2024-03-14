

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