mod diagram;
mod diagram2;

fn main() {
    use diagram::parse::*;

    let src = std::fs::read_to_string("./test.poly").unwrap();
    println!("{:?}", parse(src));
}
