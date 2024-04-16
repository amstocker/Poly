use std::default;

use im_rc::Vector;



#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Rule<A, B> {
  pub from: A,
  pub to: B
}

pub enum Iteration {
  Any,
  Exactly(usize),
  AtLeast(usize),
  AtMost(usize)
}

impl Default for Iteration {
  fn default() -> Self { Iteration::Exactly(1) }
}

#[derive(Default)]
pub enum Attribute {
  #[default] Unchecked,
  Checked(bool)
}


#[derive(Default, Debug)]
pub struct Lens<A> {
  pub rules: Vec<Rule<Vec<A>, Vec<A>>>
}

pub struct ArrayLens<A, const N: usize, const M: usize> {
  pub rules: Vec<Rule<[A; N], [A; M]>>
}

impl<A: Clone + PartialEq, const N: usize, const M: usize> From<ArrayLens<A, N, M>> for Lens<A> {
  fn from(value: ArrayLens<A, N, M>) -> Self { Lens::new(value.rules) }
}


#[inline]
fn top_of_stack_eq<A: Clone + PartialEq>(stack: &Vector<A>, other: &Vec<A>) -> bool {
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

impl<A: Clone + PartialEq> Lens<A> {
  pub fn new<I1, I2>(rules: impl IntoIterator<Item = Rule<I1, I2>>) -> Self
  where
    I1: IntoIterator<Item = A>,
    I2: IntoIterator<Item = A>
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

  pub fn transduce(&self, stack: Vector<A>) -> Result<impl Iterator<Item = Vector<A>> + '_, Vector<A>> {
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

