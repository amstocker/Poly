use std::collections::HashMap;


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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<A, B> {
  pub from: A,
  pub to: B
}


#[derive(Default)]
pub struct Lens {
  pub base_state_map: HashMap<Action, State>,
  pub rules: Vec<Rule<Vec<Action>, Vec<Action>>>
}

impl Lens {
  pub fn recognize(&self, stack: Vec<Action>) -> impl Iterator<Item = Vec<Action>> + '_ {
    self.rules.iter()
      .filter_map(move |Rule { from, to }| {
        let mut stack = stack.clone();
        if from.len() <= stack.len() && &stack[(stack.len() - from.len())..] == from {
          stack.truncate(stack.len() - from.len());
          stack.extend(to);
          Some(stack)
        } else {
          None
        }
      })
  }
}

