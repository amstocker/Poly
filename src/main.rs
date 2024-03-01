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


type ActionSequenceIndex = usize;

// Might want to add field for sequence length?
#[derive(PartialEq, Eq, Debug)]
struct ActionSequence<A> {
    next: Option<ActionSequenceIndex>,
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
        ActionSequence { next: Some(1),     action: &actionAtoA },  // 0
        ActionSequence { next: None,        action: &actionAtoA },  // 1

        ActionSequence { next: Some(3),     action: &actionAtoA },  // 2
        ActionSequence { next: None,        action: &actionAtoB },  // 3

        ActionSequence { next: Some(5),     action: &actionAtoB },  // 4
        ActionSequence { next: None,        action: &actionBtoA },  // 5

        ActionSequence { next: Some(7),     action: &actionAtoB },  // 6
        ActionSequence { next: None,        action: &actionBtoB },  // 7


        // Length 2 sequences of actions starting at state B
        ActionSequence { next: Some(9),     action: &actionBtoA },  // 8
        ActionSequence { next: None,        action: &actionAtoA },  // 9

        ActionSequence { next: Some(11),    action: &actionBtoB },  // 10
        ActionSequence { next: None,        action: &actionBtoA },  // 11

        ActionSequence { next: Some(13),    action: &actionBtoA },  // 12
        ActionSequence { next: None,        action: &actionAtoB },  // 13

        ActionSequence { next: Some(15),    action: &actionBtoB },  // 14
        ActionSequence { next: None,        action: &actionBtoB },  // 15


        // Length 1 sequences of actions (i.e. just single actions)
        ActionSequence { next: None,        action: &actionAtoA },  // 16
        ActionSequence { next: None,        action: &actionAtoB },  // 17
        ActionSequence { next: None,        action: &actionBtoA },  // 18
        ActionSequence { next: None,        action: &actionBtoB },  // 19

        ActionSequence { next: None,        action: &identity_action }  // 20
    ];


    //  -   The comonad structures on states A and B determine how sequences of actions fold.
    //  -   This data also tells us how the actions correspond to state transitions.  The engine should infer from
    //      the data of lenses what state an action can lead to.  For example, in this lens we declare that
    //      AtoB then BtoB is a composable sequence of actions, and since the action BtoB is based at state B,
    //      we can infer that AtoB points to state B.
    //  -   Note that the above only makes sense if we have the structure of a comonad present in the program,
    //      but any poly program should be built around a central comonad.
    let comonad_structure_for_stateA  = Lens {
        source: &stateA,
        target: &stateA,
        data: vec![
            Delegation { from: &sequences[0],   to: &sequences[16] },   // AtoA:AtoA -> AtoA
            Delegation { from: &sequences[2],   to: &sequences[17] },   // AtoA:AtoB -> AtoB
            Delegation { from: &sequences[4],   to: &sequences[16] },   // AtoB:BtoA -> AtoA
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
