use std::collections::{HashMap, HashSet};

use super::domain::{Domain, ElemIndex};
use super::rule::Rule;
use super::util::{Recognized, ReversibleStack};


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
pub struct BaseEngine {
  pub targets: Domain<Action>,
  pub sources: Domain<Action>,

  pub base_state_map: HashMap<Action, State>,
  pub rule_map: HashMap<ElemIndex, HashSet<Rule<ElemIndex, ElemIndex>>>,
}

impl BaseEngine {
  fn iter_source<'a>(&'a self, target: ElemIndex) -> impl Iterator<Item = Action> + 'a {
    self.rule_map.get(&target)
      .and_then(|rules| rules.iter().next())
      .map(|rule| self.sources.get(rule.to))
      .unwrap()
  }

  pub fn base_transduce(&self, stack: &mut ReversibleStack<Action>) -> Result<(), ()> {
    loop {
      match self.targets.recognize(stack) {
        (Some(index), Recognized::Partial) =>
          stack.inner.extend(self.iter_source(index)),
        (Some(index), Recognized::All) => {
          stack.inner.extend(self.iter_source(index));
          return Ok(())
        },
        (_, Recognized::Error) | (_, _) =>
          return Err(())
      }
    }
  }
}

