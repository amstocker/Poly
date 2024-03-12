pub mod action;
pub mod chain;
pub mod config;
pub mod rule;
pub mod util;


use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use self::config::{Config, LensConfig, RuleConfig, StateConfig};
use self::util::{IndexedHandler, LabelMap, PartialResult};
use self::rule::{Rule, Lens};
use self::chain::{ChainContext, ChainIndex};



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

    for StateConfig { label, actions } in config.states {
      let state = engine.states.new();
      engine.label_map.insert(label, state);

      for label in actions {
        let action = engine.actions.new();
        engine.label_map.insert(label, action);
        engine.base_state_map.insert(action, state);
      }
    }

    for LensConfig { label, rules, .. } in config.lenses {
      engine.label_map.insert(label, Lens {});
      for RuleConfig { from, to } in rules {
        let rule = Rule {
          from: engine.targets.new_chain(
            from.into_iter().map(|label| engine.label_map.get(label).unwrap())
          ).unwrap(),
          to: engine.sources.new_chain(
            to.into_iter().rev().map(|label| engine.label_map.get(label).unwrap())
          ).unwrap()
        };
        engine.rule_map
          .entry(rule.from)
          .or_insert(HashSet::new())
          .insert(rule);
      }
    }

    engine
  }

  // TODO: This should really be handled by some kind of middleware.
  pub fn transduce_labeled<'a, S: AsRef<str>>(
    &self,
    labels: impl Iterator<Item = S> + Clone
  ) -> Vec<&String> {
    let queue = self.transducer()(labels.map(|label| self.label_map.get(label).unwrap()).collect());
    queue.unwrap().iter().map(|&action| self.label_map.reverse_lookup(action).unwrap()).collect::<Vec<_>>()
  }

  fn iter_source<'a>(&'a self, target: ChainIndex) -> impl Iterator<Item = Action> + 'a {
    self.rule_map.get(&target)
      .and_then(|rules| rules.iter().next())
      .map(|rule| self.sources.get_chain(rule.to))
      .unwrap()
  }

  fn transducer<'a>(&'a self) -> impl Fn(Vec<Action>) -> Result<Vec<Action>, Vec<Action>> + 'a {
    let recognizer = |queue| self.targets.recognize_chain(queue);
    let extender = |index, mut queue: Vec<Action>| {
      queue.extend(self.iter_source(index));
      queue
    };
    PartialResult::transducer(recognizer, extender, extender)
  }

}
