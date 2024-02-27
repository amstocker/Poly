use std::marker::PhantomData;



// TODO: Context is responsible for creation of new data...
// for example lenses should deal with references to actions, because actions can also be sequences of steps.
// however composition of lenses should only create new delegations?
// ... so basically Context is in charge of memory management.
pub trait Context<S, A> {
  fn mutations(&self, state: S) -> impl Iterator<Item = A>;
  fn base(&self, action: A) -> S;
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation<A> {
  pub from: A,
  pub to: A
}

#[derive(Debug)]
pub struct Lens<S, D> {
  pub source: S,
  pub target: S,
  pub data: D,
}

impl<S, D> Lens<S, D> {
  pub fn new(source: S, target: S, data: D) -> Self {
    Self {
        source,
        target,
        data,
    }
  }

  pub fn delegate_from<'a, A>(&'a self, action: A) -> Option<A>
  where
    A: 'a + Eq + Copy,
    &'a D: IntoIterator<Item = &'a Delegation<A>>
  {
    self.data.into_iter()
      .find(|Delegation { from, .. }| *from == action)
      .map(|Delegation { to, .. }| to)
      .copied()
  }

  pub fn compose<'a, A>(&'a self, other: &'a Self) -> Self
  where
    S: Copy,
    A: 'a + Eq + Copy,
    D: 'a + FromIterator<Delegation<A>>,
    &'a D: IntoIterator<Item = &'a Delegation<A>>
  {
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
