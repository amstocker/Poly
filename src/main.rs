mod parse;
mod poly;
mod vecpoly;

use crate::poly::Lens;


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
    use crate::vecpoly::Lens;

    let lens1 = Lens {
        source: 0,
        target: 1,
        data: vec![(6, 5)]
    };

    let lens2 = Lens {
        source: 1,
        target: 2,
        data: vec![(7, 6), (8, 6)]
    };

    print!("{:?}", lens1.compose(lens2));
}
