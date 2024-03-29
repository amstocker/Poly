
#[derive(Debug)]
pub struct Operation<S, A> {
  pub state: S,
  pub action: A
}

#[derive(Debug)]
pub struct Interaction<T> {
  pub source: T,
  pub target: T
}

pub struct Span<T> {
  pub interactions: Vec<Interaction<T>>
}


#[derive(Debug)]
pub enum Query<S, A> {
  Any,
  State {
    state: S
  },
  Operation {
    state: S,
    action: A
  }
}

impl<S, A> Query<S, A> where A: Eq, S: Eq {
  pub fn matches(&self, other: &Operation<S, A>) -> bool {
    match self {
        Query::Any =>
          true,
        Query::State { state } =>
          *state == other.state,
        Query::Operation { action, state } =>
          *state == other.state && *action == other.action,
    }
  }
}

impl<S, A> Interaction<Query<S, A>> where A: Eq, S: Eq {
  pub fn matches(&self, other: &Interaction<Operation<S, A>>) -> bool {
    self.source.matches(&other.source) && self.target.matches(&other.target)
  }
}

impl<S, A> Span<Operation<S, A>> where A: Eq, S: Eq {
  pub fn interact(&self, query: Interaction<Query<S, A>>) -> impl Iterator<Item = &Interaction<Operation<S, A>>> {
    self.interactions.iter()
      .filter(move |interaction| query.matches(interaction))
  }
}