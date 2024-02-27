use std::marker::PhantomData;


pub trait State {}

pub trait Mutation {}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation<M> where M: Mutation {
  pub from: M,
  pub to: M
}

#[derive(Debug)]
pub struct Lens<S, M, D> {
  pub source: S,
  pub target: S,
  pub data: D,
  _dummy: PhantomData<M>
}

impl<'a, S, M, D> Lens<S, M, D>
where
  S: State + Copy,
  M: 'a + Mutation + Eq + Copy,
  D: 'a + FromIterator<Delegation<M>>,
  &'a D: IntoIterator<Item = &'a Delegation<M>>
{
  pub fn new(source: S, target: S, data: D) -> Self {
    Self {
        source,
        target,
        data,
        _dummy: PhantomData,
    }
  }

  pub fn delegate_from(&'a self, mutation: M) -> Option<M> {
    self.data.into_iter()
      .find(|Delegation { from, .. }| *from == mutation)
      .map(|Delegation { to, .. }| to)
      .copied()
  }

  pub fn compose(&'a self, other: &'a Self) -> Self {
    Self::new(
      self.source,
      other.target,
      other.data.into_iter()
        .copied()
        .filter_map(|Delegation { from: other_from, to: other_to }| {
          if let Some(to) = self.delegate_from(other_to) {
            Some(Delegation { from: other_from, to })
          } else {
            None
          }
        })
        .collect(),
    )
  }
}

pub trait Context<S, M>
where
  S: State,
  M: Mutation
{
  fn mutations(&self, state: S) -> impl Iterator<Item = M>;

  fn base(&self, mutation: M) -> S;
}
