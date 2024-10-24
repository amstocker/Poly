mod diagram;
mod arrow;




use arrow::constructor2::*;




fn main() {


    let x = Constructor::Atom('x');
    let y = Constructor::Atom('y');
    let z = Constructor::Atom('z');

    let a = Constructor::Atom('a');
    let b = Constructor::Atom('b');

    let f = Arrow::arrow([(x.clone(), a.clone()), (y.clone(), a.clone()), (z.clone(), b.clone())]);
    let g = Arrow::arrow([(b.clone(), a.clone()), (a.clone(), b.clone())]);
    let h = Arrow::arrow([(x.clone(), b.clone()), (a.clone(), a.clone())]);

    println!("f: {f}");
    println!("g: {g}");
    println!("h: {h}");
    println!("f -> g: {}", f.clone().compose(g.clone()));
    println!("f -> h: {}", f.clone().compose(h.clone()));
    println!("f + g: {}", f.add(h));
}
