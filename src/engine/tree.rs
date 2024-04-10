
pub type NodeIndex = usize;

#[derive(Debug)]
pub struct Node<T> {
  value: T,
  parent: Option<NodeIndex>
}

#[derive(Debug)]
pub struct Tree<T> {
  pub nodes: Vec<Node<T>>
}

pub struct Branch<'a, T> {
  tree: &'a Tree<T>,
  parent: Option<NodeIndex>,
  current: Option<NodeIndex>
}

impl<T> Branch<'_, T> {
  pub fn index(&self) -> Option<NodeIndex> {
    self.current
  }
}

impl<T: Copy> Iterator for Branch<'_, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current = self.parent;
    self.parent
      .and_then(|index| self.tree.nodes.get(index))
      .map(|node| {
        self.parent = node.parent;
        node.value
      })
  }
}


impl<T: Copy> Tree<T> {
  pub fn new() -> Tree<T> {
    Tree { nodes: Vec::new() }
  }

  pub fn branch(&self, parent: Option<NodeIndex>) -> Branch<T> {
    Branch { tree: self, parent, current: None }
  }

  pub fn push(&mut self, parent: Option<NodeIndex>, value: T) -> Option<NodeIndex> {
    let index = self.nodes.len();
    self.nodes.push(Node {
      value,
      parent
    });
    Some(index)
  }

  pub fn extend(&mut self, parent: Option<NodeIndex>, values: impl Iterator<Item = T>) -> Option<NodeIndex> {
    values.fold(parent, 
      |parent, value| self.push(parent, value)
    )
  }

  pub fn top(&self) -> Option<NodeIndex> {
    match self.nodes.len() {
      0 => None,
      n => Some(n - 1)
    }
  }
}

impl<T: Copy> Into<Tree<T>> for Vec<T> {
  fn into(self) -> Tree<T> {
    let mut stack = Tree::new();
    stack.extend(None, self.into_iter());
    stack
  }
}
