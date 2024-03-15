

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Lens {

}

pub struct Diagram {
}

pub enum LensType {
  Category,
  Monad,
  Iso,
  Other
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<T> {
  pub from: T,
  pub to: T
}

