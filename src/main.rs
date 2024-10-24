mod diagram_old;
mod diagram;

use diagram::arrow::*;
use diagram::constructor::*;


fn main() {
    let x = &Constructor::Atom('x');
    let y = &Constructor::Atom('y');
    let z = &Constructor::Atom('z');

    let a = &Constructor::Atom('a');
    let b = &Constructor::Atom('b');

    let f = &Arrow::new([(x, a), (y, a), (z, b)]);
    let g = &Arrow::new([(b, a), (a, b)]);
    let h = &Arrow::new([(x, b), (a, a)]);

    let d = &Arrow::dup([x, y]);

    println!("f: {f}");
    println!("g: {g}");
    println!("h: {h}");
    println!("f -> g: {}", f.then(g));
    println!("f -> h: {}", f.then(h));
    println!("f + g: {}", f.add(h));
    println!("d: {}", d.clone());
    println!("f * f: {}", f.mult(f));
    println!("d -> (f * f): {}", d.then(&f.mult(f)));

    let r = &Constructor::atom(f);
    let s = &Constructor::atom(g);
    let c = Constructor::sequence([r, s]);
    let t: Arrow<_> = c.build();
    
    println!("seq: {}", c);
    println!("seq build: {}", t);
}
