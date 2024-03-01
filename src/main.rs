mod engine;


const TEST: &str = r#"
(interface X { 0, 1 }, Y { 0 })

(lens
    A -> X {
        0 -> A
        1 -> B
    },
    B -> Y {
        0 -> A
    }
)
"#;    


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct State {
    index: usize
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct Action<S> {
    index: usize,
    base: S
}


fn main() {
    use crate::engine::lens::{Lens, Delegation};
    use crate::engine::action::SequenceContext;
    

    let unit_state = State { index: 0 };
    let unit_action = Action { index: 0, base: &unit_state };

    let stateA = State { index: 1 };
    let stateB = State { index: 2 };

    let actionAtoA = Action { index: 1, base: &stateA };
    let actionAtoB = Action { index: 2, base: &stateA };
    let actionBtoA = Action { index: 3, base: &stateB };
    let actionBtoB = Action { index: 4, base: &stateB };

    let mut context = SequenceContext::new();

    let AtoAtoA = context.new_sequence([&actionAtoA, &actionAtoA]).unwrap();
    let AtoBtoA = context.new_sequence([&actionAtoB, &actionBtoA]).unwrap();
    let AtoAtoB = context.new_sequence([&actionAtoA, &actionAtoB]).unwrap();
    let AtoBtoB = context.new_sequence([&actionAtoB, &actionBtoB]).unwrap();

    let BtoAtoA = context.new_sequence([&actionBtoA, &actionAtoA]).unwrap();
    let BtoAtoB = context.new_sequence([&actionBtoA, &actionAtoB]).unwrap();
    let BtoBtoA = context.new_sequence([&actionBtoB, &actionBtoA]).unwrap();
    let BtoBtoB = context.new_sequence([&actionBtoB, &actionBtoB]).unwrap();

    let AtoA = context.new_sequence([&actionAtoA]).unwrap();
    let AtoB = context.new_sequence([&actionAtoB]).unwrap();
    let BtoA = context.new_sequence([&actionBtoA]).unwrap();
    let BtoB = context.new_sequence([&actionBtoB]).unwrap();

    let identity = context.new_sequence([&unit_action]).unwrap();

    for elem in &context.data {
        println!("{:?}", elem);
    }


    // The comonad structures on states A and B determine how sequences of actions fold together.
    let comonad_structure_for_stateA  = Lens {
        source: stateA,
        target: stateA,
        data: vec![
            Delegation { from: AtoAtoA, to: AtoA },
            Delegation { from: AtoAtoB, to: AtoB },
            Delegation { from: AtoBtoA, to: AtoA },
            Delegation { from: AtoBtoB, to: AtoB }
        ]
    };

    let comonad_structure_for_stateA  = Lens {
        source: stateB,
        target: stateB,
        data: vec![
            Delegation { from: BtoAtoA, to: BtoA },
            Delegation { from: BtoAtoB, to: BtoB },
            Delegation { from: BtoBtoA, to: BtoA },
            Delegation { from: BtoBtoB, to: BtoB }
        ]
    };

    // The counit structures on states A and B imply that each of these states has an identity (i.e. "do nothing") action.
    let counit_structure_for_stateA = Lens {
        source: stateA,
        target: unit_state,
        data: vec![
            Delegation { from: identity,  to: AtoA },
        ]
    };

    let counit_structure_for_stateB = Lens {
        source: stateA,
        target: unit_state,
        data: vec![
            Delegation { from: identity,  to: BtoB },
        ]
    };

}
