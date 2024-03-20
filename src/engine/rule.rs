

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Lens {

}

pub struct Diagram {
}

pub enum LensTypeHint {
  Category,
  Monad,
  Iso,
  Other
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<A, B> {
  pub from: A,
  pub to: B
}

