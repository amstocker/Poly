pub mod lens;
pub mod sequence;

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
  lenses: Vec<Lens<State, SequenceIndex>>
}

impl Engine {
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
    delegations: &[
      (&[Action], &[Action])
    ]
  ) -> LensRef {
    let lens = Lens {
      source,
      target,
      data: delegations.iter()
        .map(|(from, to)| Delegation {
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