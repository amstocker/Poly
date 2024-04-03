use super::util::{Recognized, ReversibleStack};


pub type ElemIndex = usize;

#[derive(Debug)]
pub struct Elem<T> {
  index: ElemIndex,
  value: T,
  next: Option<ElemIndex>,
  maximal: bool
}


pub struct Iter<'a, T> {
  domain: &'a Domain<T>,
  index: Option<ElemIndex>
}

impl<T: Copy> Iterator for Iter<'_, T> {
  type Item = T;
  
  fn next(&mut self) -> Option<T> {
    self.index.and_then(|index|
      self.domain.elems.get(index)
    ).map(|elem| {
      self.index = elem.next;
      elem.value
    })
  }
}


pub struct RecognizedIter<'a, T, I> {
  domain: &'a Domain<T>,
  elem_iter: I,
  stack: ReversibleStack<T>
}

impl<T: Eq + Copy, I: Iterator<Item = ElemIndex>> Iterator for RecognizedIter<'_, T, I> {
  type Item = ElemIndex;

  fn next(&mut self) -> Option<ElemIndex> {
    self.elem_iter.next().map(|index| {
      loop {
        match self.domain.recognize_at_index(&mut self.stack, Some(index)) {
          Recognized::All => todo!(),
          Recognized::Partial => todo!(),
          Recognized::Error => todo!(),
        }
      }
    })
  }
}


#[derive(Default, Debug)]
pub struct Domain<T> {
  elems: Vec<Elem<T>>
}

impl<T> Domain<T>
where
  T: Eq + Copy
{
  pub fn get(&self, index: ElemIndex) -> Iter<T> {
    Iter { domain: &self, index: Some(index) }
  }

  // Expects values to be an iterator with values in order from from first to last. 
  pub fn insert(&mut self, values: impl Iterator<Item = T>) -> Option<ElemIndex> {
    self.insert_with_next(values, None)
  }

  pub fn insert_with_next(&mut self, mut values: impl Iterator<Item = T>, next: Option<ElemIndex>) -> Option<ElemIndex> {
    match values.next() {
      Some(value) => {
        let index = if let Some(elem) = self.elems.iter().find(|&elem|
          elem.value == value && elem.next == next
        ) {
          elem.index
        } else {
          let index = self.elems.len();
          self.elems.push(Elem { index, value, next, maximal: false });
          index
        };
        self.insert_with_next(values, Some(index))
      },
      None => {
        if let Some(next_index) = next {
          self.elems.get_mut(next_index).map(|elem| elem.maximal = true);
        }
        next
      },
    }
  }

  pub fn iter_maximal(&self) -> impl Iterator<Item = &Elem<T>> {
    self.elems.iter().filter(|elem| elem.maximal)
  }

  pub fn recognize_all(&self, stack: ReversibleStack<T>) -> RecognizedIter<T, impl Iterator<Item = ElemIndex> + '_> {
    RecognizedIter {
      domain: &self,
      elem_iter: self.iter_maximal().map(|elem| elem.index),
      stack
    }
  }

  pub fn recognize(&self, stack: &mut ReversibleStack<T>) -> (Option<ElemIndex>, Recognized) {
    for elem in self.iter_maximal() {
      match self.recognize_at_index(stack, Some(elem.index)) {
        Recognized::Error => continue,
        result =>
          return (Some(elem.index), result)
      }
    }
    (None, Recognized::Error)
  }

  pub fn recognize_at_index(&self, stack: &mut ReversibleStack<T>, index: Option<ElemIndex>) -> Recognized {
    match (
      stack.pop(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(value), Some(elem)) =>
        if elem.value == value {
          match self.recognize_at_index(stack, elem.next) {
            Recognized::Error => {
              stack.undo();
              Recognized::Error
            },
            result => result
          }
        } else {
          stack.undo();
          Recognized::Error
        },
      (Some(value), None) => {
        stack.undo();
        Recognized::Partial
      },
      (None, None) =>
        Recognized::All,
      (None, Some(_)) =>
        Recognized::Error
    }
  }
}




