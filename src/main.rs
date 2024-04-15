mod engine;


use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicUsize, Ordering};

use engine::lens::{Lens, Rule};
use im_rc::Vector;
use serde_json;

use engine::label::LabelLayer;
use engine::config::Config;



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Test {
    A, B, C
}


#[derive(Debug)]
struct QueueItem<Action: Clone> {
    id: usize,
    level: usize,
    stack: Vector<Action>,
    last_id: Option<usize>
}

pub fn new_id() -> usize {
    static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<Action: Clone> QueueItem<Action> {
    pub fn new(stack: impl Into<Vec<Action>>) -> Self {
        QueueItem {
            id: new_id(),
            level: 0,
            stack: stack.into().into(),
            last_id: None
        }
    }
}

impl<Action: Clone> PartialEq for QueueItem<Action> {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level
    }
}

impl<Action: Clone> Eq for QueueItem<Action> {}

impl<Action: Clone> PartialOrd for QueueItem<Action> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.level.partial_cmp(&other.level).map(|ord| ord.reverse())
    }
}

impl<Action: Clone> Ord for QueueItem<Action> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level.cmp(&other.level).reverse()
    }
}

fn main() {
    let config = serde_json::from_str::<Config>(include_str!("../test_config.json")).unwrap();
    let engine = LabelLayer::from_config(config);

    let chains = [
        ["AtoA", "AtoA"],
        ["AtoA", "AtoB"],
        ["AtoB", "BtoA"],
        ["AtoB", "BtoB"],
        ["BtoA", "AtoA"],
        ["BtoA", "AtoB"],
        ["BtoB", "BtoA"],
        ["BtoB", "BtoB"]
    ];

    for actions in chains.into_iter() {
        println!("{:?} -> {:?}", actions, engine.transduce(actions));
    }

    let chains = [
        ["AtoA", "AtoA", "AtoA"],
        ["AtoA", "AtoA", "AtoB"],
        ["AtoA", "AtoB", "BtoA"],
        ["AtoA", "AtoB", "BtoB"],
        ["AtoB", "BtoA", "AtoA"],
        ["AtoB", "BtoA", "AtoB"],
        ["AtoB", "BtoB", "BtoA"],
        ["AtoB", "BtoB", "BtoB"],
        ["BtoA", "AtoA", "AtoA"],
        ["BtoA", "AtoA", "AtoB"],
        ["BtoA", "AtoB", "BtoA"],
        ["BtoA", "AtoB", "BtoB"],
        ["BtoB", "BtoA", "AtoA"],
        ["BtoB", "BtoA", "AtoB"],
        ["BtoB", "BtoB", "BtoA"],
        ["BtoB", "BtoB", "BtoB"]
    ];

    for actions in chains.into_iter() {
        println!("{:?} -> {:?}", actions, engine.transduce(actions));
    }


    // Testing lenses...

    let lens1 = Lens::new([
        Rule { from: [Test::A, Test::B], to: [Test::C] },
        Rule { from: [Test::A, Test::A], to: [Test::B] },
        Rule { from: [Test::A, Test::C], to: [Test::B] }
    ]);

    let lens2 = Lens::new([
        Rule { from: [Test::B], to: [Test::C] }
    ]);

    let lenses = [lens1, lens2];

    let mut queue = BinaryHeap::default();

    queue.push(QueueItem::new([Test::A, Test::A, Test::A]));
    queue.push(QueueItem::new([Test::A, Test::A, Test::A, Test::A]));

    while let Some(item) = queue.pop() {
        println!("{:?}", item);
        let QueueItem { id, level, stack, .. } = item;
        for lens in lenses.iter() {
            match lens.transduce(stack.clone()) {
                Ok(iter) =>
                    queue.extend(iter.map(|stack|
                        QueueItem { id: new_id(), level: level + 1, stack, last_id: Some(id) })
                    ),
                Err(_) => ()
            }
        }
    }
}
