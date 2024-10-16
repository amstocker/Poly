


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom<T> {
    Value(T),
    Unit,
    Zero
}

impl<T> From<T> for Atom<T> {
    fn from(value: T) -> Self {
        Atom::Value(value)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Atom<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Value(value) => write!(f, "{}", value),
            Atom::Unit => write!(f, "Unit"),
            Atom::Zero => write!(f, "Zero"),
        }
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl<T> From<T> for Product<T> {
    fn from(value: T) -> Self {
        let atom: Atom<T> = value.into();
        atom.into()
    }
}

impl<T: Ord> Product<T> {
    pub fn new<I: IntoIterator<Item = Atom<T>>>(data: I) -> Product<T> {
        let mut data: Vec<Atom<T>> = data.into_iter().collect();
        data.sort();
        Product {
            data
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Product<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.data.len() == 1 {
            write!(f, "{}", self.data[0].to_string())
        } else {
            write!(f, "({})", self.data.iter()
                .map(|atom| atom.to_string())
                .collect::<Vec<_>>()
                .join(", ")) 
        }
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl<T> From<T> for Sum<T> {
    fn from(value: T) -> Self {
        let atom: Atom<T> = value.into();
        atom.into()
    }
}

impl<T: Ord> Sum<T> {
    pub fn new<I: IntoIterator<Item = Product<T>>>(data: I) -> Sum<T> {
        let mut data: Vec<Product<T>> = data.into_iter().collect();
        data.sort();
        Sum {
            data
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Sum<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.iter()
            .map(|prod| prod.to_string())
            .collect::<Vec<_>>()
            .join(" + "))
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Object<T> {
    Atom(Atom<T>),
    Product(Product<T>),
    Sum(Sum<T>)
}

impl<T> Object<T> {
    pub fn zero() -> Object<T> {
        Self::Atom(Atom::Zero)
    }

    pub fn unit() -> Object<T> {
        Self::Atom(Atom::Unit)
    }
}

impl<T> From<T> for Object<T> {
    fn from(value: T) -> Self {
        let atom: Atom<T> = value.into();
        atom.into()
    }
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

impl<T: std::fmt::Display> std::fmt::Display for Object<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Atom(atom) => atom.fmt(f),
            Object::Product(product) => product.fmt(f),
            Object::Sum(sum) => sum.fmt(f),
        }
    }
}


impl<T> std::ops::Add for Object<T> where T: Ord {
    type Output = Object<T>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Atom(left), Object::Atom(right)) => match (left, right) {
                (Atom::Zero, atom) | (atom, Atom::Zero) => atom.into(),
                (left, right) => {
                    Sum::new([left.into(), right.into()]).into()
                }
            },
            (Object::Atom(atom), Object::Product(product)) |
            (Object::Product(product), Object::Atom(atom)) => match atom {
                Atom::Zero => product.into(),
                atom => Sum::new([product, atom.into()]).into()
            },
            (Object::Atom(atom), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Atom(atom)) => match atom {
                Atom::Zero => sum.into(),
                atom => {
                    let mut data = sum.data;
                    data.push(atom.into());
                    Sum::new(data).into()
                }
            },
            (Object::Product(left), Object::Product(right)) => {
                Sum::new([left, right]).into()
            },
            (Object::Product(product), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Product(product)) => {
                let mut data = sum.data;
                data.push(product);
                Sum::new(data).into()
            },
            (Object::Sum(left), Object::Sum(mut right)) => {
                let mut data = left.data;
                data.append(&mut right.data);
                Sum::new(data).into()
            },
        }
    }
}

impl<T> std::ops::Mul for Object<T> where T: Ord + Clone {
    type Output = Object<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Object::Atom(left), Object::Atom(right)) => match (left, right) {
                (Atom::Zero, _) | (_, Atom::Zero) => Atom::Zero.into(),
                (Atom::Unit, atom) | (atom, Atom::Unit) => atom.into(),
                (left, right) => Product::new([left, right]).into(),
            },
            (Object::Atom(atom), Object::Product(product)) |
            (Object::Product(product), Object::Atom(atom)) => {
                let mut data = product.data;
                data.push(atom);
                Product::new(data).into() 
            },
            (Object::Atom(atom), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Atom(atom)) => {
                let mut data = sum.data;
                for product in &mut data {
                    let data = &mut product.data;
                    data.push(atom.clone());
                    *product = Product::new(data.iter().cloned());
                }
                Sum::new(data).into()
            }
            (Object::Product(left), Object::Product(mut right)) => {
                let mut data = left.data;
                data.append(&mut right.data);
                Product::new(data).into()
            },
            (Object::Product(other_product), Object::Sum(sum)) |
            (Object::Sum(sum), Object::Product(other_product)) => {
                let mut data = sum.data;
                for product in &mut data {
                    let data = &mut product.data;
                    data.append(&mut other_product.data.clone());
                }
                Sum::new(data).into()
            },
            (Object::Sum(left), Object::Sum(right)) => {
                let mut data = Vec::new();
                for left_product in &left.data {
                    for right_product in &right.data {
                        let mut left_data = left_product.data.clone();
                        left_data.append(&mut right_product.data.clone());
                        data.push(Product::new(left_data));
                    }
                }
                Sum::new(data).into()
            },
        }
    }
}