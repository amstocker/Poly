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


#[inline]
fn top_of_stack_eq<Action: Clone + PartialEq>(stack: &Vector<Action>, other: &Vec<Action>) -> bool {
  if stack.len() < other.len() {
    return false
  }
  let d = stack.len() - other.len();
  for i in 0..other.len() {
    if stack.get(i + d) != other.get(i) {
      return false
    }
  }
  true
}

impl<Action: Clone + PartialEq> Lens<Action> {
  pub fn transduce(&self, stack: Vector<Action>) -> impl Iterator<Item = Vector<Action>> + '_ {
    self.rules.iter()
      .filter_map(move |Rule { from, to }| {
        if top_of_stack_eq(&stack, from) {
          let mut stack = stack.clone();
          stack.truncate(stack.len() - from.len());
          stack.extend(to.iter().cloned());
          Some(stack)
        } else {
          None
        }
      })
  }
}

