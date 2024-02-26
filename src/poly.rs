use std::marker::PhantomData;



pub trait State {}

pub trait Mutation<S>
where
  S: State
{
  fn base(&self) -> S;
}

pub trait Monomial<S, M>
where
  S: State,
  M: Mutation<S>
{
  fn state(&self) -> S;

  // Must be the case that `mutation.base() == state` for each `mutation` in `state.mutations()`.
  fn mutations(&self) -> impl Iterator<Item = M>;
}

pub trait Transformation<T>
{
  fn transform(&self, x: T) -> Option<T>;

  // NOTE: Composition of transformations should be contravariant!
  fn compose(&self, other: Self) -> Self;
}


// TODO: Better to frame this as depending on two monomials?
#[derive(Debug)]
pub struct Lens<S, M, T>
where
  S: State,
  M: Mutation<S>,
  T: Transformation<M>
{
  pub domain: S,
  pub codomain: S,

  // Need to ensure the following conditions are true:
  //    (1) `self.transformation(mutation)?.base == self.domain`
  //    (2) `self.transformation(mutation) == None` if and only if `mutation.base != self.codomain`
  pub transformation: T,

  _dummy: PhantomData<M>
}

impl<S, M, T> Lens<S, M, T>
where
  S: State + Copy,
  M: Mutation<S>,
  T: Transformation<M>
{
  pub fn new(domain: S, codomain: S, transformation: T) -> Self {
    Self {
      domain,
      codomain,
      transformation,
      _dummy: PhantomData
    }
  }

  pub fn compose(&self, other: Self) -> Self {
    Lens {
      domain: self.domain,
      codomain: other.codomain,
      transformation: self.transformation.compose(other.transformation),
      ..*self
    }
  }
}