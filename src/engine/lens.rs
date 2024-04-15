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
  pub fn new<I1, I2>(rules: impl IntoIterator<Item = Rule<I1, I2>>) -> Self
  where
    I1: IntoIterator<Item = Action>,
    I2: IntoIterator<Item = Action>
  {
    Lens {
      rules: rules.into_iter()
        .map(|Rule { from, to }|
          Rule {
            from: from.into_iter().collect(),
            to: to.into_iter().collect()
          }
        ).collect()
    }
  }

  pub fn transduce(&self, stack: Vector<Action>) -> Result<impl Iterator<Item = Vector<Action>> + '_, Vector<Action>> {
    let ret_stack = stack.clone();
    let mut iter = self.rules.iter()
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
      .peekable();
    match iter.peek() {
      Some(_) => Ok(iter),
      None => Err(ret_stack),
    }
  }
}

