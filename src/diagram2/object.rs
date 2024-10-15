use std::ops::{Add, Mul};



#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atom<T>(T);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Product<T>(Vec<Atom<T>>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Object<T>(Vec<Product<T>>);

pub struct Zero;

pub struct Unit;


impl<T: Ord> Object<T> {
    pub fn zero() -> Object<T> {
        Object(Vec::new())
    }

    pub fn atom(t: T) -> Object<T> {
        Self::product([Atom(t)])
    }

    pub fn product<I: IntoIterator<Item = Atom<T>>>(data: I) -> Object<T> {
        let mut data: Vec<Atom<T>> = data.into_iter().collect();
        data.sort();
        Self::sum([Product(data)])
    }

    pub fn sum<I: IntoIterator<Item = Product<T>>>(data: I) -> Object<T> {
        let mut data: Vec<Product<T>> = data.into_iter().collect();
        data.sort();
        Object(data)
    }
}

impl<T> Add for Object<T> where T: Ord {
    type Output = Object<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Object::sum(self.0.into_iter().chain(rhs.0.into_iter()))
    }
}

impl<T> Mul for Object<T> where T: Ord + Clone {
    type Output = Object<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = Vec::new();
        for Product(first_seq) in &self.0 {
            for Product(second_seq) in &rhs.0 {
                let mut seq = first_seq.clone();
                seq.append(&mut second_seq.clone());
                seq.sort();
                data.push(Product(seq));
            }
        }
        Object::sum(data)
    }
}