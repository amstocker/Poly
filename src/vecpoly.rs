use crate::poly;


pub type StateIndex = usize;
impl poly::State for StateIndex {}

pub type MutationIndex = usize;
impl poly::Mutation for MutationIndex {}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Delegation {
  pub from: MutationIndex,
  pub to: MutationIndex
}

#[derive(Debug)]
pub struct Lens {
  pub source: StateIndex,
  pub target: StateIndex,
  pub data: Vec<Delegation>
}

impl poly::Lens<StateIndex, MutationIndex> for Lens {
  fn source(&self) -> StateIndex {
    self.source
  }

  fn target(&self) -> StateIndex {
    self.target
  }

  fn delegate_from(&self, mutation: MutationIndex) -> Option<MutationIndex> {
      self.data.iter()
        .find(|Delegation { from, .. }| *from == mutation)
        .map(|Delegation { to, .. }| to)
        .copied()
  }

  fn compose(&self, other: Self) -> Self {
      Lens {
        source: self.source,
        target: other.target,
        data: other.data.iter()
          .copied()
          .filter_map(|Delegation { from: other_from, to: other_to }| {
            if let Some(to) = self.delegate_from(other_to) {
              Some(Delegation { from: other_from, to })
            } else {
              None
            }
          })
          .collect()
      }
  }
}
