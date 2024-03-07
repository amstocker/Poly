mod engine;

use serde_json;

use self::engine::Engine;
use self::engine::config::Config;



fn reduce(engine: &Engine, actions: &[&str]) {
    println!("{:?} -> {:?}",
        actions,
        engine.reduce_labeled(actions.into_iter().rev())
    );
}

fn main() {
    let config = serde_json::from_str::<Config>(include_str!("test_config.json")).unwrap();
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
    

}
