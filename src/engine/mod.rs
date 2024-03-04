pub mod lens;
pub mod sequence;
pub mod config;


use std::collections::{HashMap, hash_map};

use chumsky::combinator::Label;

use self::config::{Config, DelegationConfig};
use self::lens::{Delegation, Lens};
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

pub struct LensRef {
  index: usize
}


#[derive(Default)]
pub struct LabelMap<T>(HashMap<String, T>);

impl<T> LabelMap<T> where T: Eq + Copy {
  pub fn insert(&mut self, label: String, value: T) {
    self.0.insert(label.into(), value);
  }

  pub fn get(&self, label: &str) -> Option<T> {
    self.0.get(label.into()).copied()
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
  pub lenses: Vec<Lens<State, SequenceIndex>>,

  label_to_state: LabelMap<State>,
  label_to_action: LabelMap<Action>
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    let mut label_to_state = LabelMap::default();
    let mut label_to_action = LabelMap::default();

    for state_config in config.states {
      let state = engine.new_state();
      label_to_state.insert(state_config.label, state);

      for action_label in state_config.actions {
        let action = engine.new_action(state);
        label_to_action.insert(action_label, action);
      }
    }

    for lens_config in config.lenses {
      engine.new_lens(
        label_to_state.get(&lens_config.source).unwrap(),
        label_to_state.get(&lens_config.target).unwrap(),
        lens_config.delegations.into_iter()
          .map(|DelegationConfig { from, to }| Delegation {
            from: from.into_iter()
              .map(|label| label_to_action.get(&label).unwrap())
              .collect(),
            to: to.into_iter()
              .map(|label| label_to_action.get(&label).unwrap())
              .collect()
          })
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

  pub fn lookup_action_labels<I: Iterator<Item = Action>>(&self, actions: I) -> Vec<&String> {
    actions
      .map(|action| self.lookup_action_label(action).unwrap())
      .collect() 
  }

  pub fn lookup_action_sequence_labels(&self, index: SequenceIndex) -> Vec<&String> {
    self.lookup_action_labels(
      self.sequence_context.get_action_sequence(index)
    )
  }

  pub fn labeled_lenses(&self) -> Vec<Lens<&String, Vec<&String>>> {
    self.lenses.iter()
      .map(|Lens { source, target, data }| Lens {
        source: self.lookup_state_label(*source).unwrap(),
        target: self.lookup_state_label(*target).unwrap(),
        data: data.iter().map(|&Delegation { from, to }| Delegation {
            from: self.lookup_action_sequence_labels(from),
            to: self.lookup_action_sequence_labels(to),
          })
          .collect()
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

  fn new_lens<I: Iterator<Item = Delegation<Vec<Action>>>>(
    &mut self,
    source: State,
    target: State,
    delegations: I 
  ) -> LensRef {
    let lens = Lens {
      source,
      target,
      data: delegations.map(|Delegation { from, to }| Delegation {
          from: self.sequence_context.new_sequence(from.into_iter()).unwrap(),
          to: self.sequence_context.new_sequence(to.into_iter()).unwrap()
        })
        .collect::<Vec<_>>()
    };
    
    let lens_ref = LensRef {
      index: self.lenses.len()
    };
    self.lenses.push(lens);
    lens_ref
  }

  // `reduce` expects a _stack_ of actions, so that the most recent action is first.
  pub fn reduce<'a, I: Iterator<Item = &'a str> + Clone>(&self, actions: I) -> Option<Action> {
    let actions = actions.map(|label|
      self.label_to_action.get(label).unwrap()
    );
    if let Some(index) = self.sequence_context.get_sequence(actions) {
      for lens in &self.lenses {
        for delegation in &lens.data {
          if delegation.from == index {
            return self.sequence_context.get_action(delegation.to);
          }
        }
      }
    } 

    None
  }
}