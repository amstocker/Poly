mod engine;

use serde_json;


fn main() {
    use crate::engine::Engine;
    use crate::engine::config::Config;

    let config = serde_json::from_str::<Config>(include_str!("test_config.json")).unwrap();
    let engine = Engine::from_config(config);

    let actions = ["AtoB", "BtoA"];
    println!("{:?} -> {:?}",
        actions,
        engine.lookup_action_label(
            engine.reduce(
                actions.into_iter()
            ).unwrap()
        ).unwrap()
    );

}
