mod diagram;


fn main() {
    use diagram::fin::*;

    let s = Arrow([(1, 4), (2, 4), (3, 5)].into());
    let t = Arrow([(1, 4), (2, 5), (3, 5)].into());

    println!("{:?}", s.equalize(&t));  // self-loops
    println!("{:?}", s.coequalize(&t));  // connected components
    println!("{:?}", t.pullback(&s));  // paths of length 2
}
