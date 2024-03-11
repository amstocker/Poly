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
  pub targets: ChainContext<Action>,
  pub sources: ChainContext<Action>,

  label_map: LabelMap,
  base_state_map: HashMap<Action, State>,
  pub rule_source_map: HashMap<ChainIndex, HashSet<Rule<ChainIndex>>>,
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_map = LabelMap::default();
    let mut rule_map = HashMap::new();
    let mut base_state_map = HashMap::new();

    for StateConfig { label, actions } in config.states {
      let state = engine.new_state();
      label_map.insert(label, state);

      for label in actions {
        let action = engine.new_action();
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
    engine.rule_source_map = rule_map;
    engine
  }

  pub fn lookup_label<T: Into<Labeled> + Copy>(&self, labeled: T) -> Option<&String> {
    self.label_map.reverse_lookup(labeled)
  }

  pub fn lookup_actions<'a, S: AsRef<str>>(
    &'a self,
    labels: impl Iterator<Item = S> + Clone + 'a
  ) -> impl Iterator<Item = Action> + Clone + 'a {
    labels.map(|label| self.label_map.get(label).unwrap())
  }

  fn new_state(&mut self) -> State {
    self.states.new()
  }

  fn new_action(&mut self) -> Action {
    self.actions.new()
  }

  pub fn reduce_labeled<'a, S: AsRef<str>>(
    &self,
    labels: impl Iterator<Item = S> + Clone
  ) -> Option<&String> {
    self.reduce(
      self.lookup_actions(labels).collect()
    ).and_then(|action|
      self.lookup_label(action)
    )
  }

  fn reduce(&self, queue: Vec<Action>) -> Option<Action> {
    if let (Some(index), _) = self.targets.recognize_chain(queue) {
      self.rule_source_map.get(&index)
        .and_then(|rules| rules.iter().next())
        .and_then(|rule| self.sources.get_action(rule.to)) 
    } else {
      None
    }
  }

}
