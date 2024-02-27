use crate::poly;


pub type StateIndex = usize;
impl poly::State for StateIndex {}

pub type MutationIndex = usize;
impl poly::Mutation for MutationIndex {}

#[derive(Debug)]
pub struct Lens {
  pub source: StateIndex,
  pub target: StateIndex,
  pub data: Vec<(MutationIndex, MutationIndex)>
}

impl poly::Lens<StateIndex, MutationIndex> for Lens {
  fn source(&self) -> StateIndex {
    self.source
  }

  fn target(&self) -> StateIndex {
    self.target
  }

  fn delegate(&self, mutation: MutationIndex) -> Option<MutationIndex> {
      self.data.iter()
        .find(|(x, _)| *x == mutation)
        .map(|(_, y)| y)
        .copied()
  }

  // NOTE: Composition of relations needs to be contravariant!
  fn compose(&self, other: Self) -> Self {
      Lens {
        source: self.source,
        target: other.target,
        data: other.data.iter()
          .copied()
          .filter_map(|(x, y)| {
            if let Some(z) = self.delegate(y) {
              Some((x, z))
            } else {
              None
            }
          })
          .collect()
      }
  }
}
