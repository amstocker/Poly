use diagram2::{action::Parallel, object::Object};

mod diagram;
mod diagram2;




fn main() {
    use diagram2::*;

    let x = Object::atom('X');
    let y = Object::atom('Y');
    let z = Object::atom('Z');

    println!("{:?}", x.clone() * y.clone() * x.clone());
    println!("{:?}", y.clone() + x.clone() + y.clone() * z.clone());
    println!("{:?}", (z.clone() + y.clone()) * x.clone());
    println!("{:?}", x.clone() * (z.clone() + y.clone()));

    let f = Action::new(Arrow { source: x.clone(), target: y.clone() });
    let g = Action::new(Arrow { source: x.clone(), target: z.clone() });
    let h = Action::new(Arrow { source: y.clone(), target: z.clone() });

    println!("{:?}", h.clone() + f.clone());
    println!("{:?}", f.clone() * g.clone());
    println!("{:?}", g.clone() * (f.clone() + h.clone()));
}
