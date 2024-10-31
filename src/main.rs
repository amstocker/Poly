mod diagram_old;
mod diagram;

use chumsky::Parser;
use diagram::{arrow::Arrow, constructor::Constructor, parse::parser, query::Query};


fn main() {
    let src = std::fs::read_to_string("./query.poly").unwrap();
    match parser().parse(src) {
        Ok((arrows, query)) => {
            for arrow in &arrows {
                println!("arrow {}", arrow);
            }
            println!("query: {}", query);

            // Don't really want to do this, since arrows should be labeled.
            let atoms = arrows.iter().map(Constructor::atom).collect::<Vec<_>>();
            let total = Constructor::sum(atoms.iter()).build::<Arrow<_>>();
            println!("{}", total);

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

    match diagram::parse::constructor::<String>().parse("(X, Y) + Z") {
        Ok(elem) => println!("{}", elem),
        Err(err) => println!("{:?}", err),
    }
}
