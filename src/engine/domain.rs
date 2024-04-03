use super::util::PartialResult;


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


pub struct RecognizedIter<'a, T, I: Iterator<Item = ElemIndex>> {
  domain: &'a Domain<T>,
  maximal_iter: I,
  stack: Vec<T>
}

impl<T: Eq + Copy, I: Iterator<Item = ElemIndex>> Iterator for RecognizedIter<'_, T, I> {
  type Item = ElemIndex;

  fn next(&mut self) -> Option<ElemIndex> {
    match self.maximal_iter.next() {
        Some(index) => self.domain.recognize(&mut self.stack).into(),
        None => todo!(),
    }
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

  pub fn recognize(&self, stack: &mut Vec<T>) -> PartialResult<ElemIndex> {
    for elem in self.iter_maximal() {
      match self.recognize_at_index(stack, Some(elem.index)) {
        PartialResult::Error => (),
        result =>
          return result.map(|_| elem.index)
      }
    }
    PartialResult::Error
  }

  pub fn recognize_at_index(&self, stack: &mut Vec<T>, index: Option<ElemIndex>) -> PartialResult<()> {
    match (
      stack.pop(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(value), Some(elem)) =>
        if elem.value == value {
          match self.recognize_at_index(stack, elem.next) {
            PartialResult::Error => {
              stack.push(value);
              PartialResult::Error
            },
            result => result
          }
        } else {
          stack.push(value);
          PartialResult::Error
        },
      (Some(value), None) => {
        stack.push(value);
        PartialResult::Partial(())
      },
      (None, None) => PartialResult::Ok(()),
      (None, Some(_)) => PartialResult::Error
    }
  }
}




