pub mod action;
pub mod chain;
pub mod config;
pub mod rule;
pub mod util;


use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use self::config::{Config, LensConfig, RuleConfig, StateConfig};
use self::util::{LabelMap, Labeled, IndexedHandler};
use self::rule::{Rule, Lens};
use self::chain::{ChainContext, ChainIndex, Recognized};



pub type StateIndex = usize;
pub type ActionIndex = usize;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct State {
  index: StateIndex
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct Action {
  index: ActionIndex
}


#[derive(Default)]
pub struct Engine {
  states: IndexedHandler<State>,
  actions: IndexedHandler<Action>,
  targets: ChainContext<Action>,
  sources: ChainContext<Action>,

  label_map: LabelMap,
  base_state_map: HashMap<Action, State>,
  rule_map: HashMap<ChainIndex, HashSet<Rule<ChainIndex>>>,
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_map = LabelMap::default();
    let mut rule_map = HashMap::new();
    let mut base_state_map = HashMap::new();

    for StateConfig { label, actions } in config.states {
      let state = engine.states.new();
      label_map.insert(label, state);

      for label in actions {
        let action = engine.actions.new();
        label_map.insert(label, action);
        base_state_map.insert(action, state);
      }
    }

    for LensConfig { label, rules, .. } in config.lenses {
      label_map.insert(label, Lens {});
      for RuleConfig { from, to } in rules {
        let rule = Rule {
          from: engine.targets.new_chain(
            from.into_iter().map(|label| label_map.get(label).unwrap())
          ).unwrap(),
          to: engine.sources.new_chain(
            to.into_iter().map(|label| label_map.get(label).unwrap())
          ).unwrap()
        };
        rule_map
          .entry(rule.from)
          .or_insert(HashSet::new())
          .insert(rule);
      }
    }

    engine.label_map = label_map;
    engine.base_state_map = base_state_map;
    engine.rule_map = rule_map;
    engine
  }

  // TODO: This should really be handled by some kind of middleware.
  pub fn reduce_labeled<'a, S: AsRef<str>>(
    &self,
    labels: impl Iterator<Item = S> + Clone
  ) -> Vec<&String> {
    let queue = self.reduce(labels.map(|label| self.label_map.get(label).unwrap()).collect());
    queue.unwrap().iter().map(|&action| self.label_map.reverse_lookup(action).unwrap()).collect::<Vec<_>>()
  }

  // TODO: Ensure that 
  fn iter_source<'a>(&'a self, target: ChainIndex) -> impl Iterator<Item = Action> + 'a {
    self.rule_map.get(&target)

      // Pick the first rule that has `from` equal to the target.
      .and_then(|rules| rules.iter().next())

      // Use that to iterate over the actions at the source.
      .map(|rule| self.sources.get_chain(rule.to))
      .unwrap()
  }

  fn reduce(&self, mut queue: Vec<Action>) -> Result<Vec<Action>, Vec<Action>> {
    loop {
      queue = match self.targets.recognize_chain(queue) {
        chain::RecognizedIndex::All { index, mut queue } => {
          queue.extend(self.iter_source(index));
          return Ok(queue);
        },
        chain::RecognizedIndex::Partial { index, mut queue } => {
          queue.extend(self.iter_source(index));
          queue
        },
        chain::RecognizedIndex::Error { queue } => {
          return Err(queue);
        },
      }
    }
  }

}
