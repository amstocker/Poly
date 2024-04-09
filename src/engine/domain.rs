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

#[derive(Debug)]
pub enum Recognized {
  All,
  Partial,
  Error
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
    Iter { domain: self, index: Some(index) }
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

  pub fn iter_maximal(&self) -> impl Iterator<Item = ElemIndex> + '_ {
    self.elems.iter()
      .filter(|elem| elem.maximal)
      .map(|elem| elem.index)
  }

  pub fn recognize_at_index(&self, index: Option<ElemIndex>, mut values: impl Iterator<Item = T>) -> Recognized {
    match (
      values.next(),
      index.and_then(|index| self.elems.get(index))
    ) {
      (Some(value), Some(elem)) if elem.value == value =>
        self.recognize_at_index(elem.next, values),
      (None, None)    => Recognized::All,
      (Some(_), None) => Recognized::Partial,
      (_, _)          => Recognized::Error
    }
  }
}




