use std::collections::HashMap;

use super::domain::{Domain, ElemIndex, Recognized};
use super::rule::Rule;



pub type StateIndex = usize;
pub type ActionIndex = usize;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct State {
  pub index: StateIndex
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct Action {
  pub index: ActionIndex
}



#[derive(Default)]
pub struct Lens {
  pub targets: Domain<Action>,
  pub sources: Domain<Action>,

  pub base_state_map: HashMap<Action, State>,
  pub rules: Vec<Rule<ElemIndex, ElemIndex>>
}

impl Lens {
  fn iter_to(&self, from: ElemIndex) -> impl Iterator<Item = ElemIndex> + '_ {
    self.rules.iter()
      .filter(move |rule| rule.from == from)
      .map(|rule| rule.to)
  }

  pub fn recognize(&self, stack: Vec<Action>) -> impl Iterator<Item = Vec<Action>> + '_ {
    self.targets.iter_maximal()
      .filter_map(move |index| {
        let mut stack = stack.clone();
        match self.targets.recognize_at_index(Some(index), &mut stack) {
          Recognized::Error => None,
          _                 => Some((index, stack))
        }
      })
      .flat_map(move |(index, stack)| {
        self.iter_to(index)
          .map(move |index| {
            let mut stack = stack.clone();
            stack.extend(self.sources.get(index));
            stack
          })
      })
  }

}

