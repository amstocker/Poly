mod diagram_old;
mod diagram;

use chumsky::Parser;
use diagram::{constructor::Constructor, parse::parser, query::Query};


mod test {
    use super::diagram::query2::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct BasicArrow {
        source: usize,
        target: usize
    }

    impl Arrow<usize> for BasicArrow {
        fn source(&self) -> usize {
            self.source
        }
    
        fn target(&self) -> usize {
            self.target
        }
    
        fn append_to(&self, path: &Path<Self>) -> impl IntoIterator<Item = Path<Self>> {
            let mut paths = Vec::new();
            if let Some(arrow) = path.target() {
                if self.source == arrow.target {
                    paths.push(path.push(self.clone()));
                }
            }
            paths
        }
    }

    pub fn test() {
        let arrows = [
            BasicArrow { source: 0, target: 1 },
            BasicArrow { source: 1, target: 2 }
        ];

        let query = Query::new(arrows, 0, 2);

        for path in query {
            println!("{:?}", path);
        }
    }
}

fn main() {
    let src = std::fs::read_to_string("./query.poly").unwrap();
    match parser().parse(src) {
        Ok((arrows, query)) => {
            for arrow in &arrows {
                println!("arrow {}", arrow);
            }
            println!("query: {}", query);

            let query: Vec<_> = query.transforms().collect();

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


    test::test();

}
