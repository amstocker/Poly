pub mod action;
pub mod config;
pub mod label;
pub mod transform;
pub mod sequence;


use std::collections::{HashMap, HashSet};

use self::config::{Config, StateConfig, TransformConfig};
use self::label::{LabelMap, Labeled};
use self::transform::Transform;
use self::sequence::{SequenceContext, SequenceIndex};



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
pub type TransformIndex = usize;



#[derive(Default)]
pub struct Engine {
  states: Vec<State>,
  actions: Vec<Action>,
  pub sequence_context: SequenceContext<Action>,
  pub transforms: Vec<Transform<SequenceIndex>>,

  label_map: LabelMap,
  transform_map: HashMap<SequenceIndex, HashSet<TransformIndex>>,
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_map = LabelMap::default();
    let mut transform_map = HashMap::new();

    for StateConfig { label, actions } in config.states {
      let state = engine.new_state();
      label_map.insert(label, state);

      for label in actions {
        let action = engine.new_action(state);
        label_map.insert(label, action);
      }
    }

    for TransformConfig { from, to } in config.transforms {
      let index = engine.new_transform(
        from.into_iter().map(|label| label_map.get(label).unwrap()),
        to.into_iter().map(|label| label_map.get(label).unwrap())
      );
      engine.transforms.get(index)
        .map(|transform|
          transform_map
            .entry(transform.from)
            .or_insert(HashSet::new())
            .insert(index)
        );
    }

    engine.label_map = label_map;
    engine.transform_map = transform_map;

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
      self.lookup_label(action)
    )
  }

  // `reduce` expects a _stack_ of actions, so that the most recent action is first.
  pub fn reduce(
    &self,
    actions: impl Iterator<Item = Action> + Clone
  ) -> Option<Action> {
    self.sequence_context.get_sequence(actions)
      .and_then(|index| self.transform_map.get(&index))
      .and_then(|transforms| transforms.iter().next())
      .and_then(|&index| self.transforms.get(index))
      .and_then(|transform| self.sequence_context.get_action(transform.to))
  }
}