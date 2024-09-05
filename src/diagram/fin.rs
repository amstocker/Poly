// Notes:
//  - Might be good to have some kind of "universe" of objects to ensure no overlap...
//  - Another goal is to create a way to describe diagrams, and then compute limits and colimits.
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering}
};


pub type Value = usize;

pub fn next_value() -> Value {
    static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}


#[derive(Debug)]
pub struct Arrow(pub HashMap<Value, Value>);

pub type Object = HashSet<Value>;




impl Arrow {
    pub fn items<'a>(&'a self) -> impl Iterator<Item = (Value, Value)> + 'a {
        self.0.iter().map(|(&x, &y)| (x, y))
    }

    pub fn identity(&self, values: impl IntoIterator<Item = Value>) -> Arrow {
        Arrow(values.into_iter().map(|x| (x, x)).collect())
    }

    pub fn domain(&self) -> Object {
        self.items().map(|(x, _)| x).collect()
    }

    pub fn codomain(&self) -> Object {
        self.items().map(|(_, y)| y).collect()
    }

    pub fn apply(&self, value: &Value) -> Option<Value> {
        self.items().find(|(x, _)| x == value).map(|(_, y)| y)
    }

    pub fn image(&self, value: Value) -> Object {
        self.items()
            .filter_map(|(x, y)| if x == value { Some(y) } else { None })
            .collect()
    }

    pub fn preimage(&self, value: Value) -> Object {
        self.items()
            .filter_map(|(x, y)| if y == value { Some(x) } else { None })
            .collect()
    }

    pub fn compose(&self, other: &Arrow) -> Arrow {
        let mut data = HashMap::new();
        for (x, y1) in self.items() {
            for (y2, z) in other.items() {
                if y1 == y2 {
                    data.insert(x, z);
                }
            }
        }
        Arrow(data)
    }

    pub fn product(&self, other: &Arrow) -> (Arrow, Arrow) {
        let mut data_self = HashMap::new();
        let mut data_other = HashMap::new();
        for (x1, _) in self.items() {
            for (x2, _) in other.items() {
                let i = next_value();
                data_self.insert(i, x1);
                data_other.insert(i, x2);
            }
        }
        (Arrow(data_self), Arrow(data_other))
    }

    pub fn coproduct(&self, other: &Arrow) -> (Arrow, Arrow) {
        let mut data_self = HashMap::new();
        let mut data_other = HashMap::new();
        for (_, y1) in self.items() {
            data_self.insert(y1, next_value());
        }
        for (_, y2) in other.items() {
            data_other.insert(y2, next_value());
        }
        (Arrow(data_self), Arrow(data_other)) 
    }

    pub fn equalize(&self, other: &Arrow) -> Arrow {
        let mut data = HashMap::new();
        for &x in self.domain().intersection(&other.domain()) {
            if self.image(x) == other.image(x) {
                data.insert(x, x);
            }
        }
        Arrow(data)
    }

    pub fn coequalize(&self, other: &Arrow) -> Arrow {
        let mut components: Vec<HashSet<Value>> = Vec::new();
        for edge in self.domain().intersection(&other.domain()) {
            let source = self.apply(edge).unwrap();
            let target = other.apply(edge).unwrap();
            if let Some(component) = components.iter_mut().find(|component| component.contains(&source)) {
                component.insert(target);
            } else {
                components.push([source, target].into());
            }
        }
        let mut data = HashMap::new();
        for component in components {
            let i = next_value();
            for vertex in component {
                data.insert(vertex, i);
            }
        }
        Arrow(data)
    }

    pub fn pullback(&self, other: &Arrow) -> (Arrow, Arrow) {
        let (p1, p2) = self.product(other);
        let e = p1.compose(self).equalize(&p2.compose(other));
        (
            e.compose(&p1),
            e.compose(&p2)
        )
    }

    pub fn pushout(&self, other: &Arrow) -> (Arrow, Arrow) {
        let (i1, i2) = self.coproduct(other);
        let e = self.compose(&i1).coequalize(&other.compose(&i2));
        (
            i1.compose(&e),
            i2.compose(&e)
        )
    }
}