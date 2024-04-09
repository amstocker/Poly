use std::collections::HashMap;

use super::domain::{Domain, ElemIndex, Recognized};
use super::rule::Rule;
use super::tree::{Tree, NodeIndex};


pub type StateIndex = usize;
pub type ActionIndex = usize;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct State {
  pub index: StateIndex
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct Action {
  pub index: ActionIndex
}


#[derive(Default)]
pub struct Lens {
  pub targets: Domain<Action>,
  pub sources: Domain<Action>,

  pub base_state_map: HashMap<Action, State>,
  pub rules: Vec<Rule<ElemIndex, ElemIndex>>
}

impl Lens {
  fn iter_to(&self, from: ElemIndex) -> impl Iterator<Item = ElemIndex> + '_ {
    self.rules.iter()
      .filter(move |rule| rule.from == from)
      .map(|rule| rule.to)
  }

  fn recognize<'a>(
    &'a self,
    tree: &'a Tree<Action>,
    parent: Option<NodeIndex>
  ) -> impl Iterator<Item = (Recognized, ElemIndex, Option<NodeIndex>)> + 'a {
    self.targets.iter_maximal()
      .filter_map(move |index| {
        let mut branch = tree.branch(parent);
        match self.targets.recognize_at_index(Some(index), &mut branch) {
            Recognized::All     => Some((Recognized::All, index, branch.parent())),
            Recognized::Partial => Some((Recognized::Partial, index, branch.parent())),
            Recognized::Error   => None,
        }
      })
  }

  pub fn transduce_once(&self, tree: &mut Tree<Action>, parent: Option<NodeIndex>) -> Vec<Option<NodeIndex>> {
    let recognitions = self.recognize(tree, parent).collect::<Vec<_>>();
    
    let mut indices = Vec::new();
    for (_, from_index, parent) in recognitions {
      for to_index in self.iter_to(from_index) {
        indices.push(tree.extend(parent, self.sources.get(to_index)));
      }
    }
    indices
  }

}

