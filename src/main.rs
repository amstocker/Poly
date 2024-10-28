mod diagram_old;
mod diagram;

use chumsky::Parser;
use diagram::parse::parser;


fn main() {
    // TODO: Parse any constructor... binding is (recursively) product > sum > sequence

    let src = std::fs::read_to_string("./query.poly").unwrap();
    match parser().parse(src) {
        Ok((arrows, query)) => {
            for arrow in arrows {
                println!("arrow {}", arrow);
            }
            println!("query: {}", query);
        },
        Err(err) => println!("{:?}", err),
    }
}
