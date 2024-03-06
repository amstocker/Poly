pub mod action;
pub mod config;
pub mod label;
pub mod rule;
pub mod chain;


use std::collections::{HashMap, HashSet};

use self::config::{Config, GroupConfig, RuleConfig, StateConfig};
use self::label::{LabelMap, Labeled};
use self::rule::Rule;
use self::chain::{ChainContext, ChainIndex};



#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct State {
  index: StateIndex
}

#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct Action {
  index: ActionIndex,
  base: State
}

pub type StateIndex = usize;
pub type ActionIndex = usize;
pub type RuleIndex = usize;



#[derive(Default)]
pub struct Engine {
  states: Vec<State>,
  actions: Vec<Action>,
  pub chains: ChainContext<Action>,
  pub rules: Vec<Rule<ChainIndex>>,

  label_map: LabelMap,
  rule_map: HashMap<ChainIndex, HashSet<RuleIndex>>,
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_map = LabelMap::default();
    let mut rule_map = HashMap::new();

    for StateConfig { label, actions } in config.states {
      let state = engine.new_state();
      label_map.insert(label, state);

      for label in actions {
        let action = engine.new_action(state);
        label_map.insert(label, action);
      }
    }

    for GroupConfig { rules, .. } in config.groups {
      for RuleConfig { from, to } in rules {
        let index = engine.new_rule(
          from.into_iter().map(|label| label_map.get(label).unwrap()),
          to.into_iter().map(|label| label_map.get(label).unwrap())
        );
        engine.rules.get(index)
          .map(|rule|
            rule_map
              .entry(rule.from)
              .or_insert(HashSet::new())
              .insert(index)
          );
      }
    }

    engine.label_map = label_map;
    engine.rule_map = rule_map;

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
    let state = State {
      index: self.states.len()
    };
    self.states.push(state);
    state
  }

  fn new_action(&mut self, base: State) -> Action {
    let action = Action {
      index: self.actions.len(),
      base
    };
    self.actions.push(action);
    action
  }

  fn new_rule(
    &mut self,
    from: impl Iterator<Item = Action> + Clone,
    to: impl Iterator<Item = Action> + Clone
  ) -> RuleIndex {
    let index = self.rules.len();
    self.rules.push(Rule {
      from: self.chains.new_chain(from).unwrap(),
      to: self.chains.new_chain(to).unwrap()
    });
    index
  }

  pub fn reduce_labeled<'a, S: AsRef<str>>(
    &self,
    labels: impl Iterator<Item = S> + Clone
  ) -> Option<&String> {
    self.reduce(
      self.lookup_actions(labels)
    ).and_then(|action|
      self.lookup_label(action)
    )
  }

  // `reduce` expects a _stack_ of actions, so that the most recent action is first.
  pub fn reduce(
    &self,
    actions: impl Iterator<Item = Action> + Clone
  ) -> Option<Action> {
    self.chains.get_chain(actions)
      .and_then(|index| self.rule_map.get(&index))
      .and_then(|rules| rules.iter().next())
      .and_then(|&index| self.rules.get(index))
      .and_then(|rule| self.chains.get_action(rule.to))
  }
}