pub mod action;
pub mod config;
pub mod transform;
pub mod sequence;


use std::collections::{HashMap, hash_map};

use self::config::{Config, StateConfig, TransformConfig};
use self::transform::Transform;
use self::sequence::{SequenceContext, SequenceIndex};



#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct State {
  index: usize
}

#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct Action {
  index: usize,
  base: State
}

pub type TransformIndex = usize;


#[derive(Default)]
pub struct LabelMap<T>(HashMap<String, T>);

impl<T> LabelMap<T> where T: Eq + Copy {
  pub fn insert(&mut self, label: String, value: T) {
    self.0.insert(label.into(), value);
  }

  pub fn get<S: AsRef<str>>(&self, label: S) -> Option<T> {
    self.0.get(label.as_ref()).copied()
  }

  pub fn reverse_lookup(&self, value: T) -> Option<&String> {
    self.iter()
      .find(|(_, &v)| v == value)
      .map(|(label, _)| label)
  }

  pub fn iter(&self) -> hash_map::Iter<String, T> {
    self.0.iter()
  }
}


#[derive(Default)]
pub struct Engine {
  states: Vec<State>,
  actions: Vec<Action>,
  pub sequence_context: SequenceContext<Action>,
  pub transforms: Vec<Transform<SequenceIndex>>,

  label_to_state: LabelMap<State>,
  label_to_action: LabelMap<Action>
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_to_state = LabelMap::default();
    let mut label_to_action = LabelMap::default();

    for StateConfig { label, actions } in config.states {
      let state = engine.new_state();
      label_to_state.insert(label, state);

      for label in actions {
        let action = engine.new_action(state);
        label_to_action.insert(label, action);
      }
    }

    for TransformConfig { from, to } in config.transforms {
      engine.new_transform(
        from.into_iter().map(|label| label_to_action.get(label).unwrap()),
        to.into_iter().map(|label| label_to_action.get(label).unwrap())
      );
    }

    engine.label_to_state = label_to_state;
    engine.label_to_action = label_to_action;

    engine
  }

  pub fn lookup_state_label(&self, state: State) -> Option<&String> {
    self.label_to_state.reverse_lookup(state)
  }

  pub fn lookup_action_label(&self, action: Action) -> Option<&String> {
    self.label_to_action.reverse_lookup(action)
  }

  pub fn lookup_actions<'a, S: AsRef<str>>(
    &'a self,
    labels: impl Iterator<Item = S> + Clone + 'a
  ) -> impl Iterator<Item = Action> + Clone + 'a {
    labels.map(|label| self.label_to_action.get(label).unwrap())
  }

  pub fn lookup_action_labels(&self, actions: impl Iterator<Item = Action>) -> Vec<&String> {
    actions
      .map(|action| self.lookup_action_label(action).unwrap())
      .collect() 
  }

  pub fn lookup_action_sequence_labels(&self, index: SequenceIndex) -> Vec<&String> {
    self.lookup_action_labels(
      self.sequence_context.get_action_sequence(index)
    )
  }

  pub fn labeled_transforms(&self) -> Vec<Transform<Vec<&String>>> {
    self.transforms.iter()
      .map(|&Transform { from, to }| Transform {
        from: self.lookup_action_sequence_labels(from),
        to: self.lookup_action_sequence_labels(to),
      })
      .collect()
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

  fn new_transform(
    &mut self,
    from: impl Iterator<Item = Action> + Clone,
    to: impl Iterator<Item = Action> + Clone
  ) -> TransformIndex {
    let index = self.transforms.len();
    self.transforms.push(Transform {
      from: self.sequence_context.new_sequence(from).unwrap(),
      to: self.sequence_context.new_sequence(to).unwrap()
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
      self.lookup_action_label(action)
    )
    
  }

  // `reduce` expects a _stack_ of actions, so that the most recent action is first.
  pub fn reduce(
    &self,
    actions: impl Iterator<Item = Action> + Clone
  ) -> Option<Action> {
    self.sequence_context.get_sequence(actions)
      .and_then(|index|
        self.transforms.iter()
          .find(|&transform|
            transform.from == index
          )
          .and_then(|transform|
            self.sequence_context.get_action(transform.to)
          )
      )
  }
}