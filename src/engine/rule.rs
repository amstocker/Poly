

pub enum Group {
  Category,
  Monad,
  Iso,
  Other
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<T> {
  pub from: T,
  pub to: T
}

