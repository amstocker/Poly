mod diagram;
mod arrow;

use arrow::constructor2::*;


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
    println!("f -> g: {}", f.clone().compose(g.clone()));
    println!("f -> h: {}", f.clone().compose(h.clone()));
    println!("f + g: {}", f.clone().add(h.clone()));
    println!("d: {}", d.clone());
    println!("f * f: {}", f.clone().mult(f.clone()));
    println!("d -> (f * f): {}", d.clone().compose(f.clone().mult(f.clone())));

    let r = &Constructor::atom(f);
    let s = &Constructor::atom(f);
    let c = Constructor::product([r, s]);
    let t: Arrow<_> = c.build();
    println!("{}", t);
}
