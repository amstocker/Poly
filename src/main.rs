mod parse;
mod poly;


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
struct Action(usize);


fn main() {
    use crate::poly::{Lens, Delegation};

    let state0 = State(0);
    let state1 = State(1);
    let state2 = State(2);

    let action0 = Action(0);    // base = state0
    let action1 = Action(1);    // base = state1
    let action2 = Action(2);    // base = state2
    let action3 = Action(3);    // base = state2

    let lens1: Lens<State, Vec<Delegation<Action>>> = Lens::new(
        &state0,
        &state1,
        vec![
            Delegation { from: &action1, to: &action0 }
        ]
    );

    let lens2: Lens<State, Vec<Delegation<Action>>> = Lens::new(
        &state1,
        &state2,
        vec![
            Delegation { from: &action2, to: &action1 },
            Delegation { from: &action3, to: &action1 }
        ]
    );
    
    let lens3: Lens<State, Vec<Delegation<Action>>> = lens1.compose(&lens2);
    
    print!("{:?}", lens3);
}
