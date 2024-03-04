mod engine;

use serde_json;



fn reduce(engine: &crate::engine::Engine, actions: &[&str]) {
    println!("{:?} -> {:?}",
        actions,
        engine.lookup_action_label(
            engine.reduce(
                // Most recent actions should be _first_.
                actions.into_iter().copied().rev()
            ).unwrap()
        ).unwrap()
    );
}

fn main() {
    use crate::engine::Engine;
    use crate::engine::config::Config;

    let config = serde_json::from_str::<Config>(include_str!("test_config.json")).unwrap();
    let engine = Engine::from_config(config);

    println!("Sequences:");
    for elem in &engine.sequence_context.data {
        println!("\t{:?}: action={:?}, next={:?}, prev={:?}", elem.index, engine.lookup_action_label(elem.action).unwrap(), elem.next, elem.prev);
    }

    println!("Lenses:");
    for lens in &engine.lenses {
        println!("\t{:?}", lens);
    }

    let sequences = [
        ["AtoA", "AtoA"],
        ["AtoA", "AtoB"],
        ["AtoB", "BtoA"],
        ["AtoB", "BtoB"],
        ["BtoA", "AtoA"],
        ["BtoA", "AtoB"],
        ["BtoB", "BtoA"],
        ["BtoB", "BtoB"],
    ];

    for seq in sequences.into_iter() {
        reduce(&engine, &seq);
    }
    

}
