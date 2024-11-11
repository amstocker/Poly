mod diagram;
mod diagram_old;
mod diagram_old2;

use chumsky::Parser;


fn main() {
    let src = std::fs::read_to_string("./query.poly").unwrap();
    match diagram::parse::parser::<String>().parse(src) {
        Ok(result) => {
            for transform in result {
                println!("{}", transform);
            }
        },
        Err(err) => println!("{:?}", err),
    }
}
