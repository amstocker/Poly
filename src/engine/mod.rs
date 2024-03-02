pub mod lens;
pub mod sequence;
pub mod config;


use std::collections::HashMap;

use chumsky::chain::Chain;

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
  pub lenses: Vec<Lens<State, SequenceIndex>>,

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

  pub fn new_state(&mut self) -> State {
    let state = State {
      index: self.states.len()
    };
    self.states.push(state);
    state
  }

  pub fn new_action(&mut self, base: State) -> Action {
    let action = Action {
      index: self.actions.len(),
      base
    };
    self.actions.push(action);
    action
  }

  pub fn new_lens(
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
}