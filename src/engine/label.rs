use std::collections::{hash_map, HashMap};

use crate::engine::config::InterfaceConfig;

use super::lens::{ActionHandle, Lens, StateHandle, Rule};
use super::config::{Config, LensConfig, RuleConfig, StateConfig};



#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Labeled {
  Action(ActionHandle),
  State(StateHandle),
}

impl From<StateHandle> for Labeled {
  fn from(state: StateHandle) -> Labeled {
    Labeled::State(state)
  }
}

impl From<ActionHandle> for Labeled {
  fn from(action: ActionHandle) -> Labeled {
    Labeled::Action(action)
  }
}

impl From<Labeled> for Option<StateHandle> {
  fn from(value: Labeled) -> Option<StateHandle> {
    match value {
      Labeled::State(state) => Some(state),
      _ => None
    }
  }
}

impl From<Labeled> for Option<ActionHandle> {
  fn from(value: Labeled) -> Option<ActionHandle> {
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

impl Indexed for StateHandle {
  fn build_with_index(index: usize) -> Self {
    StateHandle { index }
  }
}

impl Indexed for ActionHandle {
  fn build_with_index(index: usize) -> Self {
    ActionHandle { index } 
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
  states: IndexedHandler<StateHandle>,
  actions: IndexedHandler<ActionHandle>,
  pub label_map: LabelMap,

  pub engine: Lens<ActionHandle> 
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
        }
      }
    }

    for LensConfig { rules, .. } in config.lenses {
      for RuleConfig { from, to } in rules {
        let rule = Rule {
          from: from.into_iter().map(|label| label_map.get(label).unwrap()).collect(),
          to: to.into_iter().map(|label| label_map.get(label).unwrap()).collect()
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

  fn translate<S: AsRef<str>>(&self, stack: impl IntoIterator<Item = S>) -> Vec<ActionHandle> {
    stack.into_iter()
      .map(|label| self.label_map.get(label).unwrap())
      .collect()
  }

  fn untranslate(&self, stack: impl IntoIterator<Item = ActionHandle>) -> Vec<String> {
    stack.into_iter()
      .map(|action|
        self.label_map.reverse_lookup(action).unwrap()
      )
      .cloned()
      .collect()
  }

  pub fn transduce<S: AsRef<str>>(&self, stack: impl IntoIterator<Item = S>) -> Vec<Vec<String>> {
    self.engine.transduce(self.translate(stack).into())
      .map(|stack| self.untranslate(stack))
      .collect()
  }
}