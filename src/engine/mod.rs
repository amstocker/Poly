pub mod domain;
pub mod config;
pub mod rule;
pub mod util;


use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use self::config::{Config, LensConfig, RuleConfig, StateConfig};
use self::util::{IndexedHandler, LabelMap, PartialResult};
use self::rule::{Rule, Lens};
use self::domain::{Domain, ElemIndex};



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
pub struct BaseEngine {
  targets: Domain<Action>,
  sources: Domain<Action>,

  base_state_map: HashMap<Action, State>,
  rule_map: HashMap<ElemIndex, HashSet<Rule<ElemIndex>>>,
}

impl BaseEngine {
  fn iter_source<'a>(&'a self, target: ElemIndex) -> impl Iterator<Item = Action> + 'a {
    self.rule_map.get(&target)
      .and_then(|rules| rules.iter().next())
      .map(|rule| self.sources.get(rule.to))
      .unwrap()
  }

  fn base_transduce(&self, mut queue: Vec<Action>) -> Result<Vec<Action>, Vec<Action>> {
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


pub trait Engine<'a, In, Out> {
  fn transduce(&'a self, input: In) -> Result<Out, Out>;
}

impl Engine<'_, Vec<Action>, Vec<Action>> for BaseEngine {
  fn transduce(&self, queue: Vec<Action>) -> Result<Vec<Action>, Vec<Action>> {
    self.base_transduce(queue)
  }
}

pub trait Middleware<'a, In, Out> {
  type InnerIn;
  type InnerOut: 'a;
  type InnerEngine: Engine<'a, Self::InnerIn, Self::InnerOut> + 'a;

  fn inner(&self) -> &Self::InnerEngine;
  fn translate(&self, input: In) -> Self::InnerIn;
  fn untranslate(&'a self, output: Self::InnerOut) -> Out;
}

impl<'a, T, In, Out> Engine<'a, In, Out> for T
where
  T: Middleware<'a, In, Out>
{
  fn transduce(&'a self, input: In) -> Result<Out, Out> {
    self.inner().transduce(self.translate(input))
      .map(|output| self.untranslate(output))
      .map_err(|output| self.untranslate(output))
  }
}


pub struct LabelMiddleware {
  states: IndexedHandler<State>,
  actions: IndexedHandler<Action>,
  label_map: LabelMap,

  engine: BaseEngine 
}

impl LabelMiddleware {
  pub fn from_config(config: Config) -> Self {
    let mut engine = BaseEngine::default();

    let mut states = IndexedHandler::default();
    let mut actions = IndexedHandler::default();
    let mut label_map = LabelMap::default();

    for StateConfig { label, actions: action_labels } in config.states {
      let state = states.new();
      label_map.insert(label, state);

      for label in action_labels {
        let action = actions.new();
        label_map.insert(label, action);
        engine.base_state_map.insert(action, state);
      }
    }

    for LensConfig { label, rules, .. } in config.lenses {
      label_map.insert(label, Lens {});
      for RuleConfig { from, to } in rules {
        let rule = Rule {
          from: engine.targets.new(
            from.into_iter().map(|label| label_map.get(label).unwrap())
          ).unwrap(),
          to: engine.sources.new(
            to.into_iter().rev().map(|label| label_map.get(label).unwrap())
          ).unwrap()
        };
        engine.rule_map
          .entry(rule.from)
          .or_insert(HashSet::new())
          .insert(rule);
      }
    }

    Self {
      states,
      actions,
      label_map,
      engine
    }
  }
}

impl<'a> Middleware<'a, &[&str], Vec<&'a String>> for LabelMiddleware {
  type InnerIn = Vec<Action>;
  type InnerOut = Vec<Action>;
  type InnerEngine = BaseEngine;

  fn inner(&self) -> &Self::InnerEngine {
    &self.engine
  }

  fn translate(&self, queue: &[&str]) -> Vec<Action> {
    queue.as_ref().iter()
      .map(|label| self.label_map.get(label).unwrap())
      .collect()
  }

  fn untranslate(&'a self, queue: Vec<Action>) -> Vec<&'a String> {
    queue.iter()
      .map(|&action|
        self.label_map.reverse_lookup(action).unwrap()
      )
      .collect()
  }
}