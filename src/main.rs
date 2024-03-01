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



#[derive(PartialEq, Eq, Debug)]
struct State(usize);

#[derive(PartialEq, Eq, Debug)]
struct Action<S> {
    index: usize,
    base: S
}


type SequenceIndex = usize;

// Might want to add field for sequence length?
#[derive(PartialEq, Eq, Debug)]
struct Sequence<A> {
    next: Option<SequenceIndex>,
    action: A
}


fn main() {
    use crate::engine::{Lens, Delegation};
    

    let unit_state = State(0);
    let identity_action = Action { index: 0, base: &unit_state };

    let stateA = State(1);
    let stateB = State(2);

    let actionAtoA = Action { index: 1, base: &stateA };
    let actionAtoB = Action { index: 2, base: &stateA };
    let actionBtoA = Action { index: 3, base: &stateB };
    let actionBtoB = Action { index: 4, base: &stateB };

    let sequences = vec![

        // Length 2 sequences of actions starting at state A
        Sequence { next: Some(1),     action: &actionAtoA },  // 0
        Sequence { next: None,        action: &actionAtoA },  // 1

        Sequence { next: Some(3),     action: &actionAtoB },  // 2
        Sequence { next: None,        action: &actionBtoA },  // 3

        Sequence { next: Some(5),     action: &actionAtoA },  // 4
        Sequence { next: None,        action: &actionAtoB },  // 5

        Sequence { next: Some(7),     action: &actionAtoB },  // 6
        Sequence { next: None,        action: &actionBtoB },  // 7


        // Length 2 sequences of actions starting at state B
        Sequence { next: Some(9),     action: &actionBtoA },  // 8
        Sequence { next: None,        action: &actionAtoA },  // 9

        Sequence { next: Some(11),    action: &actionBtoB },  // 10
        Sequence { next: None,        action: &actionBtoA },  // 11

        Sequence { next: Some(13),    action: &actionBtoA },  // 12
        Sequence { next: None,        action: &actionAtoB },  // 13

        Sequence { next: Some(15),    action: &actionBtoB },  // 14
        Sequence { next: None,        action: &actionBtoB },  // 15


        // Length 1 sequences of actions (i.e. just single actions)
        Sequence { next: None,        action: &actionAtoA },  // 16
        Sequence { next: None,        action: &actionAtoB },  // 17
        Sequence { next: None,        action: &actionBtoA },  // 18
        Sequence { next: None,        action: &actionBtoB },  // 19

        Sequence { next: None,        action: &identity_action }  // 20
    ];


    // The comonad structures on states A and B determine how sequences of actions fold together.
    let comonad_structure_for_stateA  = Lens {
        source: &stateA,
        target: &stateA,
        data: vec![
            Delegation { from: &sequences[0],   to: &sequences[16] },   // AtoA:AtoA -> AtoA
            Delegation { from: &sequences[2],   to: &sequences[16] },   // AtoB:BtoA -> AtoA
            Delegation { from: &sequences[4],   to: &sequences[17] },   // AtoA:AtoB -> AtoB
            Delegation { from: &sequences[6],   to: &sequences[17] },   // AtoB:BtoB -> AtoB
        ]
    };

    let comonad_structure_for_stateB = Lens {
        source: &stateB,
        target: &stateB,
        data: vec![
            Delegation { from: &sequences[8],   to: &sequences[18] },   // BtoA:AtoA -> BtoA
            Delegation { from: &sequences[10],  to: &sequences[18] },   // BtoB:BtoA -> BtoA
            Delegation { from: &sequences[12],  to: &sequences[19] },   // BtoA:AtoB -> BtoB
            Delegation { from: &sequences[14],  to: &sequences[19] },   // BtoB:BtoB -> BtoB
        ]
    };


    // The counit structures on states A and B imply that each of these states has an identity (i.e. "do nothing") action.
    let counit_structure_for_stateA = Lens {
        source: &stateA,
        target: &unit_state,
        data: vec![
            Delegation { from: &sequences[20],  to: &sequences[16] },
        ]
    };

    let counit_structure_for_stateB = Lens {
        source: &stateA,
        target: &unit_state,
        data: vec![
            Delegation { from: &sequences[20],  to: &sequences[19] },
        ]
    };

}
