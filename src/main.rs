mod diagram;
mod diagram2;



fn main() {
    use diagram2::*;

    let x: Object<char> = Atom::Value('X').into();
    let y: Object<char> = Atom::Value('Y').into();
    let z: Object<char> = Atom::Value('Z').into();

    println!("{}", x.clone() * y.clone() * x.clone());
    println!("{}", y.clone() + x.clone() + y.clone() * z.clone());
    println!("{}", (z.clone() + y.clone()) * x.clone());
    println!("{}", x.clone() * (z.clone() + y.clone()));
    println!("{}", Object::unit() * x.clone());
    println!("{}", x.clone() * Object::zero());


    let f: Action<char> = Operation::Value('f').into();
    let g: Action<char> = Operation::Value('g').into();
    let h: Action<char> = Operation::Value('h').into();

    println!("{}", h.clone() + f.clone());
    println!("{}", f.clone() * g.clone());
    println!("{}", g.clone() * (f.clone() + h.clone()));
}
