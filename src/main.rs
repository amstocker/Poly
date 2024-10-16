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


    // let f = Action::new(Arrow { source: x.clone(), target: y.clone() });
    // let g = Action::new(Arrow { source: x.clone(), target: z.clone() });
    // let h = Action::new(Arrow { source: y.clone(), target: z.clone() });

    // println!("{:?}", h.clone() + f.clone());
    // println!("{:?}", f.clone() * g.clone());
    // println!("{:?}", g.clone() * (f.clone() + h.clone()));
}
