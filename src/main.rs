mod engine;

use serde_json;

use crate::engine::Engine;
use crate::engine::config::Config;



fn reduce(engine: &Engine, actions: &[&str]) {
    println!("{:?} -> {:?}",
        actions,
        engine.reduce_labeled(actions.into_iter().rev())
    );
}

fn main() {
    let config = serde_json::from_str::<Config>(include_str!("test_config.json")).unwrap();
    let engine = Engine::from_config(config);

    println!("Sequence Elements:");
    for elem in &engine.chains.data {
        println!("\t{:?}: action={:?}, next={:?}, prev={:?}", elem.index, engine.lookup_label(elem.action).unwrap(), elem.next, elem.prev);
    }

    println!("Transforms:");
    for transform in &engine.rules {
        println!("\t{:?}", transform);
    }

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
        reduce(&engine, &actions);
    }
    

}
