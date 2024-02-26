use crate::poly;



pub type StateIndex = usize;

impl poly::State for StateIndex {}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct MutationIndex {
  pub index: usize,
  pub base: StateIndex
}

impl poly::Mutation<StateIndex> for MutationIndex {
  fn base(&self) -> StateIndex {
      self.base
  }
}

pub struct Monomial {
  pub state: StateIndex,
  pub mutations: Vec<MutationIndex>
}

impl poly::Monomial<StateIndex, MutationIndex> for Monomial {
  fn state(&self) -> StateIndex {
    self.state
  }

  fn mutations(&self) -> impl Iterator<Item = MutationIndex> {
    self.mutations.iter().copied()
  }
}


#[derive(Debug)]
pub struct Transformation {
  pub data: Vec<(MutationIndex, MutationIndex)>
}

impl poly::Transformation<MutationIndex> for Transformation {
  fn transform(&self, mutation: MutationIndex) -> Option<MutationIndex> {
      self.data.iter()
        .find(|(x, _)| *x == mutation)
        .map(|(_, y)| y)
        .copied()
  }

  // NOTE: Composition of relations needs to be contravariant!
  fn compose(&self, other: Self) -> Self {
      Transformation {
        data: other.data.iter()
          .copied()
          .filter_map(|(x, y)| {
            if let Some(z) = self.transform(y) {
              Some((x, z))
            } else {
              None
            }
          })
          .collect()
      }
  }
}
