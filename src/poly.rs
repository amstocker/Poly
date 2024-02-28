

pub trait Context<S, A> {
  fn actions(&self, state: S) -> impl Iterator<Item = A>;
  fn base(&self, action: A) -> S;
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation<'a, T> {
  pub from: &'a T,
  pub to: &'a T
}

#[derive(Debug)]
pub struct Lens<'a, S, D> {
  pub source: &'a S,
  pub target: &'a S,
  pub data: D,
}

impl<'a, S, D, T> Lens<'a, S, D>
where
  T: 'a + Eq,
  D: 'a + FromIterator<Delegation<'a, T>>,
  &'a D: IntoIterator<Item = &'a Delegation<'a, T>>
{
  pub fn new(source: &'a S, target: &'a S, data: D) -> Self {
    Self {
        source,
        target,
        data,
    }
  }

  pub fn delegate_from(&'a self, t: &T) -> Option<&T> {
    self.data.into_iter()
      .find(|&&Delegation { from, .. }| from == t)
      .map(|&Delegation { to, .. }| to)
  }

  pub fn compose(&'a self, other: &'a Self) -> Self {
    Self::new(
      self.source,
      other.target,
      other.data.into_iter()
        .filter_map(|&Delegation { from: other_from, to: other_to }| {
          self.delegate_from(other_to).map(|to| {
            Delegation { from: other_from, to }
          })
        })
        .collect(),
    )
  }
}
