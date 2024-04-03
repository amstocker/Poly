use std::collections::{HashMap, HashSet};

use super::domain::{Domain, ElemIndex};
use super::rule::Rule;
use super::util::{Recognized, Stack};


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

  // TODO: We want to change this so that it "fans out" the recognition recursively.
  // So we take a stack and then recursively recognize and push _chunks_ to a stack-like data-structure.
  // This data structure should work with domains so that "extending" really just pushes a ref to the
  // respective chunk in the source domain, all while treating it as a "stack" seamlessly.
  // Hence, the transduce function should utilize both the source and target domains.
  pub fn base_transduce(&self, stack: &mut Stack<Action>) -> Result<(), ()> {
    loop {
      match self.targets.recognize(stack) {
        (Some(index), Recognized::Partial) =>
          stack.extend(self.iter_source(index)),
        (Some(index), Recognized::All) => {
          stack.extend(self.iter_source(index));
          return Ok(())
        },
        (_, Recognized::Error) | (_, _) =>
          return Err(())
      }
    }
  }
}

