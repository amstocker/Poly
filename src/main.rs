mod diagram;


fn main() {
    use chumsky::prelude::*;
    use diagram::parse::*;

    let src = std::fs::read_to_string("./test.poly").unwrap();
    println!("{:?}", parser().parse(src));
}
