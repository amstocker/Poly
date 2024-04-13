use im_rc::Vector;


pub type StateIndex = usize;
pub type ActionIndex = usize;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct StateHandle {
  pub index: StateIndex
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct ActionHandle {
  pub index: ActionIndex
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<A, B> {
  pub from: A,
  pub to: B
}

pub type Interface<Action> = Vec<Vec<Action>>;


#[derive(Default)]
pub struct Lens<Action> {
  source: Interface<Action>,
  target: Interface<Action>,

  // TODO: The rules should be checked at run-time to ensure that they conform to the lens rules!
  pub rules: Vec<Rule<Vec<Action>, Vec<Action>>>
}

impl<Action: Copy + Eq> Lens<Action> {
  pub fn transduce(&self, stack: impl Into<Vector<Action>>) -> impl Iterator<Item = Vector<Action>> + '_ {
    let stack = stack.into();
    self.rules.iter()
      .filter_map(move |Rule { from, to }| {
        let mut stack = stack.clone();
        if from.len() <= stack.len()
          && stack.slice(stack.len() - from.len()..).iter().eq(from)
        {
          stack.extend(to.iter().copied());
          Some(stack)
        } else {
          None
        }
      })
  }
}

