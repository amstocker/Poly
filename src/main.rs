mod diagram_old;
mod diagram;

use chumsky::Parser;
use diagram::{constructor::Constructor, parse::parser, query::Query};


fn main() {
    // TODO: Parse any constructor... binding is (recursively) product > sum > sequence

    let src = std::fs::read_to_string("./query.poly").unwrap();
    match parser().parse(src) {
        Ok((arrows, query)) => {
            for arrow in &arrows {
                println!("arrow {}", arrow);
            }
            println!("query: {}", query);

            let query: Vec<_> = query.pairs().collect();

            if let (
                Constructor::Atom(source),
                Constructor::Atom(target)
            ) = (
                query[0].source.clone(),
                query[0].target.clone()
            ) {
                let query = Query::new(arrows, source, target);

                for path in query {
                    println!("{}", path);
                }
            }
        },
        Err(err) => println!("{:?}", err),
    }
}
