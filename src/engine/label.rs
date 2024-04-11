use std::collections::{hash_map, HashMap};

use crate::engine::config::InterfaceConfig;

use super::base::{Action, Lens, State};
use super::config::{Config, LensConfig, RuleConfig, StateConfig};
use super::rule::Rule;



#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Labeled {
  Action(Action),
  State(State),
}

impl Into<Labeled> for State {
  fn into(self) -> Labeled {
    Labeled::State(self)
  }
}

impl Into<Labeled> for Action {
  fn into(self) -> Labeled {
    Labeled::Action(self)
  }
}

impl From<Labeled> for Option<State> {
  fn from(value: Labeled) -> Option<State> {
    match value {
      Labeled::State(state) => Some(state),
      _ => None
    }
  }
}

impl From<Labeled> for Option<Action> {
  fn from(value: Labeled) -> Option<Action> {
    match value {
      Labeled::Action(action) => Some(action),
      _ => None
    }
  }
}



#[derive(Default)]
pub struct LabelMap(HashMap<String, Labeled>);

impl LabelMap {
  pub fn insert(&mut self, label: String, value: impl Into<Labeled>) {
    self.0.insert(label.into(), value.into());
  }

  pub fn get<T>(&self, label: impl AsRef<str>) -> Option<T>
  where
    Option<T>: From<Labeled>
  {
    self.0.get(label.as_ref()).copied()?.into()
  }

  pub fn reverse_lookup(&self, value: impl Into<Labeled> + Copy) -> Option<&String> {
    self.0.iter()
      .find(|(_, &v)| v == value.into())
      .map(|(label, _)| label)
  }

  pub fn iter<'a, T>(&'a self) -> impl Iterator<Item = T> + 'a
  where
    Option<T>: From<Labeled>
  {
    self.0.iter()
      .filter_map(|(_, &value)| value.into())
  }

  pub fn iter_all(&self) -> hash_map::Iter<String, Labeled> {
    self.0.iter()
  }
}



pub trait Indexed {
  fn build_with_index(index: usize) -> Self;
}

impl Indexed for State {
  fn build_with_index(index: usize) -> Self {
    State { index }
  }
}

impl Indexed for Action {
  fn build_with_index(index: usize) -> Self {
    Action { index } 
  }
}

#[derive(Default)]
pub struct IndexedHandler<T> {
  data: Vec<T>
}

impl<T> IndexedHandler<T> where T: Indexed + Copy {
  pub fn new(&mut self) -> T {
    let value = T::build_with_index(self.data.len());
    self.data.push(value);
    value
  }
}


pub struct LabelLayer {
  states: IndexedHandler<State>,
  actions: IndexedHandler<Action>,
  pub label_map: LabelMap,

  pub engine: Lens 
}

impl LabelLayer {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Lens::default();

    let mut states = IndexedHandler::default();
    let mut actions = IndexedHandler::default();
    let mut label_map = LabelMap::default();


    println!("diagram: {:?}", config.diagram);

    for InterfaceConfig { states: state_configs, .. } in config.interfaces {
      for StateConfig { label, actions: action_labels } in state_configs {
        let state = states.new();
        label_map.insert(label, state);
  
        for label in action_labels {
          let action = actions.new();
          label_map.insert(label, action);
          engine.base_state_map.insert(action, state);
        }
      }
    }

    for LensConfig { rules, .. } in config.lenses {
      for RuleConfig { from, to } in rules {
        let rule = Rule {
          from: engine.targets.insert(
            from.into_iter().map(|label| label_map.get(label).unwrap())
          ).unwrap(),
          to: engine.sources.insert(
            to.into_iter().rev().map(|label| label_map.get(label).unwrap())
          ).unwrap()
        };
        engine.rules.push(rule)
      }
    }

    Self {
      states,
      actions,
      label_map,
      engine
    }
  }

  fn translate(&self, stack: &[&str]) -> Vec<Action> {
    stack.as_ref().iter()
      .map(|label| self.label_map.get(label).unwrap())
      .collect()
  }

  fn untranslate(&self, stack: Vec<Action>) -> Vec<String> {
    stack.iter()
      .map(|action|
        self.label_map.reverse_lookup(*action).unwrap()
      )
      .cloned()
      .collect()
  }

  pub fn transduce(&self, stack: &[&str]) -> Vec<Vec<String>> {
    self.engine.recognize(self.translate(stack))
      .map(|stack| self.untranslate(stack))
      .collect()
  }
}