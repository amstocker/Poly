use crate::poly;


pub type StateIndex = usize;
impl poly::State for StateIndex {}

pub type MutationIndex = usize;
impl poly::Mutation for MutationIndex {}
