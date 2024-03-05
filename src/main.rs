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
    for elem in &engine.sequence_context.data {
        println!("\t{:?}: action={:?}, next={:?}, prev={:?}", elem.index, engine.lookup_action_label(elem.action).unwrap(), elem.next, elem.prev);
    }

    println!("Transforms:");
    for transform in &engine.transforms {
        println!("\t{:?}", transform);
    }

    let sequences = [
        ["AtoA", "AtoA"],
        ["AtoA", "AtoB"],
        ["AtoB", "BtoA"],
        ["AtoB", "BtoB"],
        ["BtoA", "AtoA"],
        ["BtoA", "AtoB"],
        ["BtoB", "BtoA"],
        ["BtoB", "BtoB"]
    ];

    for actions in sequences.into_iter() {
        reduce(&engine, &actions);
    }
    

}
