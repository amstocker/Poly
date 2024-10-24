use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering}
};


pub type Value = usize;

pub fn next_value() -> Value {
    static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}


#[derive(Debug, Default)]
pub struct Relation {
    data: HashMap<(Value, Value), usize>,
}

#[derive(Debug, Default)]
pub struct Monomial {
    data: HashMap<Value, usize>
}

pub struct Item {
    x: Value,
    y: Value,
    multiplicity: usize
}

impl<T> From<T> for Relation where T: IntoIterator<Item = (usize, usize)> {
    fn from(value: T) -> Self {
        Relation {
            data: value.into_iter().fold(
                HashMap::new(),
                |mut data, pair| {
                    *data.entry(pair).or_insert(0) += 1;
                    data
                }
            )
        }
    }
}

impl Relation {
    pub fn items<'a>(&'a self) -> impl Iterator<Item = Item> + 'a {
        self.data.iter()
            .map(|(&(x, y), &multiplicity)|
                Item { x, y, multiplicity }
            )
    }

    pub fn compose(&self, other: &Relation) -> Relation {
        let mut rel = Relation::default();
        for Item { x, y: y1, multiplicity: m1 } in self.items() {
            for Item { x: y2, y: z, multiplicity: m2 } in other.items() {
                if y1 == y2 {
                    *rel.data.entry((x, z)).or_insert(0) += m1 * m2;
                }
            }
        }
        rel
    }

}