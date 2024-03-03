pub mod lens;
pub mod sequence;
pub mod config;


use std::collections::HashMap;

use self::config::{Config, DelegationConfig};
use self::lens::{Delegation, Lens};
use self::sequence::{SequenceContext, SequenceIndex};



#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct State {
  index: usize
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Action {
  index: usize,
  base: State
}

pub struct LensRef {
  index: usize
}


#[derive(Default)]
pub struct Engine {
  states: Vec<State>,
  actions: Vec<Action>,
  sequence_context: SequenceContext<Action>,
  lenses: Vec<Lens<State, SequenceIndex>>,

  label_to_state: HashMap<String, State>,
  label_to_action: HashMap<String, Action>
}


impl Engine {
  pub fn from_config(config: Config) -> Self {
    let mut engine = Engine::default();
    
    for state_config in config.states {
      let state = engine.new_state();
      engine.label_to_state.insert(state_config.label, state);

      for action_label in state_config.actions {
        let action = engine.new_action(state);
        engine.label_to_action.insert(action_label, action);
      }
    }

    for lens_config in config.lenses {
      engine.new_lens(
        engine.label_to_state.get(&lens_config.source).copied().unwrap(),
        engine.label_to_state.get(&lens_config.target).copied().unwrap(),
        lens_config.delegations.iter()
          .map(|DelegationConfig { from, to }| Delegation {
            from: from.into_iter()
              .map(|label| engine.label_to_action.get(label).copied().unwrap())
              .collect(),
            to: to.into_iter()
              .map(|label| engine.label_to_action.get(label).copied().unwrap())
              .collect()
          })
          .collect()
      );
    }

    engine
  }

  pub fn lookup_state_label(&self, state: State) -> Option<&String> {
    self.label_to_state.iter()
      .find(|(_, &value)| value == state)
      .map(|(label, _)| label)
  }

  pub fn lookup_action_label(&self, action: Action) -> Option<&String> {
    self.label_to_action.iter()
      .find(|(_, &value)| value == action)
      .map(|(label, _)| label)
  }

  pub fn lookup_action_labels(&self, actions: &[Action]) -> Vec<&String> {
    actions.iter()
      .map(|&action| self.lookup_action_label(action).unwrap())
      .collect() 
  }

  pub fn lookup_action_sequence_labels(&self, index: SequenceIndex) -> Vec<&String> {
    self.lookup_action_labels(
      &self.sequence_context.get_action_sequence(index)
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

  fn new_lens(
    &mut self,
    source: State,
    target: State,
    delegations:  Vec<Delegation<Vec<Action>>> 
  ) -> LensRef {
    let lens = Lens {
      source,
      target,
      data: delegations.iter()
        .map(|Delegation { from, to }| Delegation {
          from: self.sequence_context.new_sequence(from).unwrap(),
          to: self.sequence_context.new_sequence(to).unwrap()
        })
        .collect::<Vec<_>>()
    };
    
    let lens_ref = LensRef {
      index: self.lenses.len()
    };
    self.lenses.push(lens);
    lens_ref
  }

  pub fn reduce(&self, actions: &[&str]) -> Option<Action> {
    let actions = actions.iter()
      .map(|&label| self.label_to_action.get(label).copied().unwrap())
      .collect::<Vec<_>>();
    
    if let Some(index) = self.sequence_context.find(&actions) {
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