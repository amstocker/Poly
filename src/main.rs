mod parse;
mod poly;
mod vecpoly;


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


fn main() {
    use crate::poly::{Lens, Delegation};

    let lens1 = Lens::new(0, 1, vec![Delegation { from: 6, to: 5 }]);

    let lens2 = Lens::new(1, 2, vec![Delegation { from: 7, to: 6 }, Delegation { from: 8, to: 6 }]);

    print!("{:?}", lens1.compose(&lens2));
}
