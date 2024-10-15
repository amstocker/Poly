use std::ops::{Add, Mul};


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Atom<T> {
    data: T
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Product<T> {
    data: Vec<Atom<T>>
}

#[derive(Debug, PartialEq, Eq)]
pub struct Sum<T> {
    data: Vec<Product<T>>
}


impl<T> Atom<T> {
    pub fn new(data: T) -> Atom<T> {
        Atom { data }
    }
}

impl<T> Product<T> where T: Ord {
    pub fn new<I: IntoIterator<Item = Atom<T>>>(data: I) -> Product<T> {
        let mut data: Vec<Atom<T>> = data.into_iter().collect();
        data.sort();
        Product { data }
    }
}

impl<T> Sum<T> where T: Ord {
    pub fn new<I: IntoIterator<Item = Product<T>>>(data: I) -> Sum<T> {
        let mut data: Vec<Product<T>> = data.into_iter().collect();
        data.sort();
        Sum { data }
    }
}


impl<T> From<Atom<T>> for Product<T> where T: Ord {
    fn from(value: Atom<T>) -> Self {
        Product::new([value])
    }
}

impl<T> From<Atom<T>> for Sum<T> where T: Ord {
    fn from(value: Atom<T>) -> Self {
        Sum::new([value.into()])
    }
}

impl<T> From<Product<T>> for Sum<T> where T: Ord {
    fn from(value: Product<T>) -> Self {
        Sum::new([value])
    }
}


impl<T> Mul for Atom<T> where T: Ord {
    type Output = Product<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        Product::new([self, rhs])
    }
}

impl<T> Add for Atom<T> where T: Ord {
    type Output = Sum<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Sum::new([self.into(), rhs.into()])
    }
}

impl<T, R> Mul<R> for Product<T> where T: Ord, R: Into<Product<T>> {
    type Output = Product<T>;

    fn mul(self, rhs: R) -> Self::Output {
        let mut data = self.data;
        data.append(&mut rhs.into().data);
        Product::new(data)
    }
}

impl<T, R> Add<R> for Sum<T> where T: Ord, R: Into<Sum<T>> {
    type Output = Sum<T>;

    fn add(self, rhs: R) -> Self::Output {
        let mut data = self.data;
        data.append(&mut rhs.into().data);
        Sum::new(data)
    }
}

impl<T> Mul for Sum<T> where T: Ord + Clone {
    type Output = Sum<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = Vec::new();
        for x in &self.data {
            for y in &rhs.data {
                data.push(x.clone() * y.clone());
            }
        }
        Sum::new(data)
    }
}