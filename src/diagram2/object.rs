use std::ops::{Add, Mul};



#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom<T> {
    Value(T),
    Unit,
    Zero
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Product<T> {
    pub data: Vec<Atom<T>>
}

impl<T> From<Atom<T>> for Product<T> {
    fn from(atom: Atom<T>) -> Self {
        Product {
            data: [atom].into()
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sum<T> {
    pub data: Vec<Product<T>>
}

impl<T> From<Product<T>> for Sum<T> {
    fn from(product: Product<T>) -> Self {
        Sum {
            data: [product].into()
        }
    }
}

impl<T> From<Atom<T>> for Sum<T> {
    fn from(atom: Atom<T>) -> Self {
        let product: Product<T> = atom.into();
        product.into()
    }
}


#[derive(Debug)]
pub enum Object<T> {
    Atom(Atom<T>),
    Product(Product<T>),
    Sum(Sum<T>)
}

impl<T> From<Atom<T>> for Object<T> {
    fn from(atom: Atom<T>) -> Self {
        Object::Atom(atom)
    }
}

impl<T> From<Product<T>> for Object<T> {
    fn from(product: Product<T>) -> Self {
        Object::Product(product)
    }
}

impl<T> From<Sum<T>> for Object<T> {
    fn from(sum: Sum<T>) -> Self {
        Object::Sum(sum)
    }
}

impl<T: Ord> Object<T> {
    pub fn unit() -> Object<T> {
        Atom::Unit.into()
    }

    pub fn zero() -> Object<T> {
        Atom::Zero.into()
    }

    pub fn atom(t: T) -> Object<T> {
        Atom::Value(t).into()
    }

    pub fn product<I: IntoIterator<Item = Atom<T>>>(data: I) -> Object<T> {
        let mut data: Vec<Atom<T>> = data.into_iter().collect();
        data.sort();
        Product {
            data
        }.into()
    }

    pub fn sum<I: IntoIterator<Item = Product<T>>>(data: I) -> Object<T> {
        let mut data: Vec<Product<T>> = data.into_iter().collect();
        data.sort();
        Sum {
            data
        }.into()
    }
}


impl<T> Add for Object<T> where T: Ord {
    type Output = Object<T>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Atom(left), Object::Atom(right)) => match (left, right) {
                (Atom::Zero, atom) | (atom, Atom::Zero) => {
                    Object::Atom(atom)
                },
                (left, right) => {
                    Object::sum([left.into(), right.into()])
                }
            },
            (Object::Atom(atom), Object::Product(product)) |
            (Object::Product(product), Object::Atom(atom)) => match atom {
                Atom::Zero => product.into(),
                atom => Object::sum([product, atom.into()])
            },
            (Object::Atom(atom), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Atom(atom)) => match atom {
                Atom::Zero => sum.into(),
                atom => {
                    let mut data = sum.data;
                    data.push(atom.into());
                    Object::sum(data)
                }
            },
            (Object::Product(product), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Product(product)) => {
                let mut data = sum.data;
                data.push(product);
                Object::sum(data)
            },
            (Object::Product(left), Object::Product(right)) => {
                Object::sum([left, right])
            },
            (Object::Sum(left), Object::Sum(mut right)) => {
                let mut data = left.data;
                data.append(&mut right.data);
                Object::sum(data)
            },
        }
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