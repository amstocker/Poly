mod engine;

use serde_json;

use self::engine::Engine;
use self::engine::config::Config;



fn reduce(engine: &Engine, actions: &[&str]) {
    println!("{:?} -> {:?}",
        actions,
        engine.reduce_labeled(actions.into_iter())
    );
}

fn main() {
    let config = serde_json::from_str::<Config>(include_str!("../test_config.json")).unwrap();
    let engine = Engine::from_config(config);

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
        reduce(&engine, &actions);
    }

    let chains = [
        ["AtoA", "AtoA", "AtoA", "AtoA"],
        ["AtoA", "AtoA", "AtoA", "AtoB"],
        ["AtoA", "AtoA", "AtoB", "BtoA"],
        ["AtoA", "AtoA", "AtoB", "BtoB"],
        ["AtoA", "AtoB", "BtoA", "AtoA"],
        ["AtoA", "AtoB", "BtoA", "AtoB"],
        ["AtoA", "AtoB", "BtoB", "BtoA"],
        ["AtoA", "AtoB", "BtoB", "BtoB"],
        ["AtoB", "BtoA", "AtoA", "AtoA"],
        ["AtoB", "BtoA", "AtoA", "AtoB"],
        ["AtoB", "BtoA", "AtoB", "BtoA"],
        ["AtoB", "BtoA", "AtoB", "BtoB"],
        ["AtoB", "BtoB", "BtoA", "AtoA"],
        ["AtoB", "BtoB", "BtoA", "AtoB"],
        ["AtoB", "BtoB", "BtoB", "BtoA"],
        ["AtoB", "BtoB", "BtoB", "BtoB"],
        ["BtoA", "AtoA", "AtoA", "AtoA"],
        ["BtoA", "AtoA", "AtoA", "AtoB"],
        ["BtoA", "AtoA", "AtoB", "BtoA"],
        ["BtoA", "AtoA", "AtoB", "BtoB"],
        ["BtoA", "AtoB", "BtoA", "AtoA"],
        ["BtoA", "AtoB", "BtoA", "AtoB"],
        ["BtoA", "AtoB", "BtoB", "BtoA"],
        ["BtoA", "AtoB", "BtoB", "BtoB"],
        ["BtoB", "BtoA", "AtoA", "AtoA"],
        ["BtoB", "BtoA", "AtoA", "AtoB"],
        ["BtoB", "BtoA", "AtoB", "BtoA"],
        ["BtoB", "BtoA", "AtoB", "BtoB"],
        ["BtoB", "BtoB", "BtoA", "AtoA"],
        ["BtoB", "BtoB", "BtoA", "AtoB"],
        ["BtoB", "BtoB", "BtoB", "BtoA"],
        ["BtoB", "BtoB", "BtoB", "BtoB"]
    ];

    for actions in chains.into_iter() {
        reduce(&engine, &actions);
    }
}
