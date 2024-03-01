use crate::engine::action::SequenceContext;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation<T> {
  pub from: T,
  pub to: T
}

#[derive(Debug)]
pub struct Lens<S, D> {
  pub source: S,
  pub target: S,
  pub data: D,
}

impl<S, D> Lens<S, D> {
  pub fn delegate_from<'a, T>(&'a self, t: T) -> Option<T>
  where
    T: 'a + Eq + Copy,
    D: 'a + FromIterator<Delegation<T>>,
    &'a D: IntoIterator<Item = &'a Delegation<T>>
  {
    self.data.into_iter()
      .find(|&&Delegation { from, .. }| from == t)
      .map(|&Delegation { to, .. }| to)
  }

  pub fn compose<'a, T>(&'a self, other: &'a Self) -> Self
  where
    S: Copy,
    T: 'a + Eq + Copy,
    D: 'a + FromIterator<Delegation<T>>,
    &'a D: IntoIterator<Item = &'a Delegation<T>>
  {
    Self {
      source: self.source,
      target: other.target,
      data: other.data.into_iter()
        .filter_map(|&Delegation { from: other_from, to: other_to }| {
          self.delegate_from(other_to).map(|to| {
            Delegation { from: other_from, to }
          })
        })
        .collect(),
    }
  }
}
