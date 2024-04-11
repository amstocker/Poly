mod engine;


use serde_json;

use engine::label::LabelLayer;
use engine::config::Config;



fn transduce(engine: &LabelLayer, actions: &[&str]) {
    println!("{:?} -> {:?}",
        actions,
        engine.transduce(actions)
    );
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
        transduce(&engine, &actions);
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
        transduce(&engine, &actions);
    }

}
