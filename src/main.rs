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



    // Alternate idea for engine.
    use engine::interaction::*;

    let span = Span {
        interactions: vec![
            Interaction {
                source: Operation { state: 0, action: 0 },
                target: Operation { state: 1, action: 0 }
            },
            Interaction {
                source: Operation { state: 0, action: 1 },
                target: Operation { state: 1, action: 0 }
            },
            Interaction {
                source: Operation { state: 2, action: 0 },
                target: Operation { state: 1, action: 0 }
            }
        ]
    };

    for interaction in span.interact(
        Interaction {
            source: Query::State { state: 0 },
            target: Query::Any
        }
    ) {
        println!("{:?}", interaction);
    }
}
