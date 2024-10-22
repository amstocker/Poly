mod diagram;
mod arrow;




use arrow::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Test {
    source: Object<char>,
    target: Object<char>
}

impl std::fmt::Display for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} -> {}]", self.source, self.target)
    }
}

impl Arrow<char> for Test {
    fn source(&self) -> Object<char> {
        self.source.clone()
    }

    fn target(&self) -> Object<char> {
        self.target.clone()
    }
}





fn main() {

    let x: Object<char> = Atom::Value('X').into();
    let y: Object<char> = Atom::Value('Y').into();
    let z: Object<char> = Atom::Value('Z').into();

    println!("{}", x.clone() * y.clone() * x.clone());
    println!("{}", y.clone() + x.clone() + y.clone() * z.clone());
    println!("{}", (z.clone() + y.clone()) * x.clone());
    println!("{}", x.clone() * (z.clone() + y.clone()));
    println!("{}", Object::unit() * x.clone());
    println!("{}", x.clone() * Object::zero());


    let f: Action<Test> = Operation::Value(Test { source: x.clone(), target: y.clone() }).into();
    let g: Action<Test> = Operation::Value(Test { source: y.clone(), target: z.clone() }).into();
    let h: Action<Test> = Operation::Value(Test { source: z.clone(), target: x.clone() }).into();

    let f1 = h.clone() + f.clone();
    let f2 = f.clone() * g.clone();
    let f3 = g.clone() * (f.clone() + h.clone());
    let f4 = g.clone() * f.clone();

    println!("{} : {} -> {}", f1, f1.source(), f1.target());
    println!("{} : {} -> {}", f2, f2.source(), f2.target());
    println!("{} : {} -> {}", f3, f3.source(), f3.target());
    println!("{} : {} -> {}", f4, f4.source(), f4.target());


}
