mod diagram;


fn main() {
    use diagram::fin::*;

    let f = Arrow([(0, 3), (1, 2)].into());
    let g = Arrow([(1, 5), (3, 2), (2, 2)].into());
    let h = Arrow([(1, 5), (2, 2), (5, 2)].into());

    println!("f: {:?}", f);
    println!("g: {:?}", g);
    println!("{:?}", f.image(0));
    println!("{:?}", g.preimage(2));
    println!("{:?}", f.compose(&g));

    let (p1, p2) = f.product(&g);
    println!("product: {:?}", (&p1, &p2));
    println!("p1 -> f: {:?}", p1.compose(&f));
    println!("{:?}", f.coproduct(&g));

    println!("h: {:?}", h);
    println!("eq: {:?}", g.equalize(&h));


    let s = Arrow([(1, 4), (2, 4), (3, 5)].into());
    let t = Arrow([(1, 4), (2, 5), (3, 5)].into());

    println!("{:?}", s.equalize(&t));
    println!("{:?}", s.coequalize(&t));
    println!("{:?}", t.pullback(&s));  // paths of length 2
}
