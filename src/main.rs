mod engine;

use serde_json;


fn main() {
    use crate::engine::Engine;
    use crate::engine::config::Config;

    let config = serde_json::from_str::<Config>(include_str!("test_config.json")).unwrap();
    let engine = Engine::from_config(config);

    println!("States:");
    for (label, state) in engine.label_to_state.iter() {
        println!("\t{:?}: {:?}", label, state);
    }

    println!("Actions:");
    for (label, action) in engine.label_to_action.iter() {
        println!("\t{:?}: {:?}", label, action);
    }

    println!("Sequences:");
    for elem in &engine.sequence_context.data {
        println!("\t{:?}: action={:?}, next={:?}", elem.index, engine.lookup_action_label(elem.action).unwrap(), elem.next);
    }

    let actions = ["AtoA", "AtoB"];
    println!("{:?} -> {:?}",
        actions,
        engine.lookup_action_label(engine.reduce(&actions).unwrap()).unwrap());
}
