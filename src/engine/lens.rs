
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation<T> {
  pub from: T,
  pub to: T
}

#[derive(Debug)]
pub struct Lens<S, T> {
  pub source: S,
  pub target: S,
  pub data: Vec<Delegation<T>>,
}

impl<S, T> Lens<S, T> where S: Copy, T: Eq + Copy {
  pub fn delegate_from(&self, t: T) -> Option<T> {
    self.data.iter()
      .find(|&&Delegation { from, .. }| from == t)
      .map(|&Delegation { to, .. }| to)
  }

  pub fn compose(&self, other: &Self) -> Self
  {
    Self {
      source: self.source,
      target: other.target,
      data: other.data.iter()
        .filter_map(|&Delegation { from: other_from, to: other_to }| {
          self.delegate_from(other_to).map(|to| {
            Delegation { from: other_from, to }
          })
        })
        .collect(),
    }
  }
}
