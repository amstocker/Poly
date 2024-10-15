mod diagram;
mod diagram2;

fn main() {
    use diagram::parse::*;

    let src = std::fs::read_to_string("./test.poly").unwrap();
    println!("{:?}", parse(src));

    use diagram2::object::*;
    use diagram2::arrow::*;

    let x = Object::atom('X');
    let y = Object::atom('Y');
    let z = Object::atom('Z');

    println!("{:?}", x.clone() * y.clone() * x.clone());
    println!("{:?}", y.clone() + x.clone() + y.clone() * z.clone());
    println!("{:?}", (z.clone() + y.clone()) * x.clone());
    println!("{:?}", x.clone() * (z.clone() + y.clone()));

    let f = Arrow::action('f');
    let g = Arrow::action('g');
    let h = Arrow::action('h');

    println!("{:?}", h.clone() + f.clone());
    println!("{:?}", f.clone() * g.clone());
    println!("{:?}", g.clone() * (f.clone() + h.clone()));
}
