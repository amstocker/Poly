mod engine;


use std::collections::BinaryHeap;

use engine::lens::{Lens, Rule};
use im_rc::Vector;
use serde_json;

use engine::Engine;
use engine::config::Config;



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Test {
    A, B, C
}


#[derive(PartialEq, Eq, Debug)]
struct QueueItem<Action: Clone + PartialEq + Eq> {
    level: usize,
    stack: Vector<Action>,
}



impl<Action: Clone + Eq> QueueItem<Action> {
    pub fn new(stack: impl Into<Vec<Action>>) -> Self {
        QueueItem {
            level: 0,
            stack: stack.into().into(),
        }
    }
}

impl<Action: Clone + Eq> PartialOrd for QueueItem<Action> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Action: Clone + Eq> Ord for QueueItem<Action> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level.cmp(&other.level).reverse()
    }
}

fn main() {
    let config = serde_json::from_str::<Config>(include_str!("../test_config.json")).unwrap();
    let engine = Engine::build_from_config(config);

    let lens = engine.lenses.get(0).unwrap();
    let lens2 = engine.lenses.get(3).unwrap();

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
        println!("{:?} -> {:?}", actions, engine.transduce(lens, actions));
        println!("\t{:?} -> {:?}", actions, engine.transduce(lens2, actions));
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
        println!("{:?} -> {:?}", actions, engine.transduce(lens, actions));
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

    while let Some(QueueItem { level, stack }) = queue.pop() {
        for lens in lenses.iter() {
            match lens.transduce(stack.clone()) {
                Ok(iter) =>
                    queue.extend(iter.map(|stack|
                        QueueItem { level: level + 1, stack })
                    ),
                Err(stack) => ()
            }
        }
    }
}
