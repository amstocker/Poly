use std::collections::{HashMap, HashSet};

use super::domain::{Domain, ElemIndex};
use super::rule::Rule;
use super::util::PartialResult;
use super::Engine;



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

  fn base_transduce(&self, mut queue: Vec<Action>) -> Result<Vec<Action>, Vec<Action>> {
    // Match target of diagram to see if it's Exactly or Any, then loop if any and once if otherwise.
    loop {
      queue = match self.targets.recognize(queue) {
        PartialResult::Partial(index, mut queue) => {
          queue.extend(self.iter_source(index));
          queue
        },
        PartialResult::Ok(index, mut queue) => {
          queue.extend(self.iter_source(index));
          return Ok(queue)
        },
        PartialResult::Error(queue) =>
          return Err(queue)
      }
    }
  }
}


impl Engine<Vec<Action>, Vec<Action>> for BaseEngine {
  fn transduce(&self, queue: Vec<Action>) -> Result<Vec<Action>, Vec<Action>> {
    self.base_transduce(queue)
  }
}

