

pub struct Queue<A> {
  data: Vec<A>
}

impl<A> Queue<A> where A: Eq + Copy {
  pub fn pop(&mut self) -> Option<A> {
    self.data.pop()
  }

  pub fn push(&mut self, value: A) {
    self.data.push(value);
  }
}