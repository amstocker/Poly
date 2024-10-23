

pub type Index = usize;

pub enum Constructor<A> {
    Atom(A),
    Sum(Vec<Index>),
    Product(Vec<Index>),
    Sequence(Vec<Index>)
}

pub trait Constructible {
    fn add(self, other: Self) -> Self;
    fn mult(self, other: Self) -> Self;
    fn compose(self, other: Self) -> Self;
    fn zero() -> Self;
    fn unit() -> Self;
    fn identity() -> Self;
}

pub struct Constructors<A> {
    elements: Vec<Constructor<A>>
}

impl<A> Constructors<A> {
    pub fn construct(&self, index: Index) -> A where A: Constructible + Clone {
        match &self.elements[index] {
            Constructor::Atom(atom) => atom.clone(),
            Constructor::Sum(elems) => elems.iter().copied()
                .fold(A::zero(), |acc, index| acc.add(self.construct(index))),
            Constructor::Product(elems) => elems.iter().copied()
                .fold(A::unit(), |acc, index| acc.mult(self.construct(index))),
            Constructor::Sequence(elems) => elems.iter().copied()
                .fold(A::identity(), |acc, index| acc.compose(self.construct(index))),
        }
    }
}