mod parse;
mod poly;
mod vecpoly;

use vecpoly::*;


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

    let lens1 = poly::Lens::new(0, 1, Transformation {
        data: vec![(MutationIndex { base: 1, index: 6 }, MutationIndex { base: 0, index: 5 })]
    });

    let lens2 = poly::Lens::new(1, 2, Transformation {
        data: vec![(MutationIndex { base: 2, index: 7 }, MutationIndex { base: 1, index: 6 })]
    });

    print!("{:?}", lens1.compose(lens2));
}
