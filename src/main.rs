mod diagram;
mod diagram2;

fn main() {
    use diagram::parse::*;

    let src = std::fs::read_to_string("./test.poly").unwrap();
    println!("{:?}", parse(src));

    use diagram2::constructor2::*;

    let x = Atom::new('X');
    let y = Atom::new('Y');
    let z = Atom::new('Z');

    println!("{:?}", x * y * x);
    println!("{:?}", y + x + z);
}
