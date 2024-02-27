
pub trait State {}

pub trait Mutation {}

pub trait Lens<S, M>
where
  S: State,
  M: Mutation
{
  fn source(&self) -> S;

  fn target(&self) -> S;

  // Delegates from a mutation based at the target state to a mutation based at the source state.
  fn delegate_from(&self, mutation: M) -> Option<M>;

  fn compose(&self, other: Self) -> Self;
}

pub trait Context<S, M>
where
  S: State,
  M: Mutation
{
  fn mutations(&self, state: S) -> impl Iterator<Item = M>;

  fn base(&self, mutation: M) -> S;
}
