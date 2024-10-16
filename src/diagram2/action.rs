


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation<T> {
    Value(T),
    Identity,
}

impl<T> From<T> for Operation<T> {
    fn from(value: T) -> Self {
        Operation::Value(value)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Operation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Value(value) => write!(f, "{}", value),
            Operation::Identity => write!(f, "Id"),
        }
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sequence<T> {
    pub data: Vec<Operation<T>>
}

impl<T> From<Operation<T>> for Sequence<T> {
    fn from(atom: Operation<T>) -> Self {
        Sequence {
            data: [atom].into()
        }
    }
}

impl<T> From<T> for Sequence<T> {
    fn from(value: T) -> Self {
        let atom: Operation<T> = value.into();
        atom.into()
    }
}

impl<T: Ord> Sequence<T> {
    pub fn new<I: IntoIterator<Item = Operation<T>>>(data: I) -> Sequence<T> {
        Sequence {
            data: data.into_iter().collect()
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Sequence<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.iter()
            .map(|atom| atom.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")) 
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Parallel<T> {
    pub data: Vec<Sequence<T>>
}

impl<T> From<Sequence<T>> for Parallel<T> {
    fn from(product: Sequence<T>) -> Self {
        Parallel {
            data: [product].into()
        }
    }
}

impl<T> From<Operation<T>> for Parallel<T> {
    fn from(atom: Operation<T>) -> Self {
        let product: Sequence<T> = atom.into();
        product.into()
    }
}

impl<T> From<T> for Parallel<T> {
    fn from(value: T) -> Self {
        let atom: Operation<T> = value.into();
        atom.into()
    }
}

impl<T: Ord> Parallel<T> {
    pub fn new<I: IntoIterator<Item = Sequence<T>>>(data: I) -> Parallel<T> {
        let mut data: Vec<Sequence<T>> = data.into_iter().collect();
        data.sort();
        Parallel {
            data
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Parallel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.iter()
            .map(|prod| prod.to_string())
            .collect::<Vec<_>>()
            .join(" | "))
    }
}


#[derive(Clone)]
pub enum Action<T> {
    Operation(Operation<T>),
    Sequence(Sequence<T>),
    Parallel(Parallel<T>)
}

impl<T> Action<T> {
    pub fn identity() -> Action<T> {
        Self::Operation(Operation::Identity)
    }
}

impl<T> From<T> for Action<T> {
    fn from(value: T) -> Self {
        let atom: Operation<T> = value.into();
        atom.into()
    }
}

impl<T> From<Operation<T>> for Action<T> {
    fn from(atom: Operation<T>) -> Self {
        Action::Operation(atom)
    }
}

impl<T> From<Sequence<T>> for Action<T> {
    fn from(product: Sequence<T>) -> Self {
        Action::Sequence(product)
    }
}

impl<T> From<Parallel<T>> for Action<T> {
    fn from(sum: Parallel<T>) -> Self {
        Action::Parallel(sum)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Action<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Operation(atom) => atom.fmt(f),
            Action::Sequence(product) => product.fmt(f),
            Action::Parallel(sum) => sum.fmt(f),
        }
    }
}


impl<T> std::ops::Add for Action<T> where T: Ord {
    type Output = Action<T>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Action::Operation(left), Action::Operation(right)) => {
                Parallel::new([left.into(), right.into()]).into()
            },
            (Action::Operation(atom), Action::Sequence(product)) |
            (Action::Sequence(product), Action::Operation(atom)) => {
                Parallel::new([product, atom.into()]).into()
            },
            (Action::Operation(atom), Action::Parallel(sum)) |
            (Action::Parallel(sum), Action::Operation(atom)) => {
                let mut data = sum.data;
                data.push(atom.into());
                Parallel::new(data).into()
            },
            (Action::Sequence(left), Action::Sequence(right)) => {
                Parallel::new([left, right]).into()
            },
            (Action::Sequence(product), Action::Parallel(sum)) |
            (Action::Parallel(sum), Action::Sequence(product)) => {
                let mut data = sum.data;
                data.push(product);
                Parallel::new(data).into()
            },
            (Action::Parallel(left), Action::Parallel(mut right)) => {
                let mut data = left.data;
                data.append(&mut right.data);
                Parallel::new(data).into()
            },
        }
    }
}

impl<T> std::ops::Mul for Action<T> where T: Ord + Clone {
    type Output = Action<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Action::Operation(first), Action::Operation(second)) => match (first, second) {
                (Operation::Identity, op) | (op, Operation::Identity) => op.into(),
                (first, second) => Sequence::new([first, second]).into(),
            },
            (Action::Operation(op), Action::Sequence(sequence)) => {
                let mut data = sequence.data;
                data.insert(0, op);
                Sequence::new(data).into() 
            },
            (Action::Sequence(sequence), Action::Operation(op)) => {
                let mut data = sequence.data;
                data.push(op);
                Sequence::new(data).into() 
            },
            (Action::Operation(op), Action::Parallel(parallel)) => {
                let mut data = parallel.data;
                for sequence in &mut data {
                    let data = &mut sequence.data;
                    data.insert(0, op.clone());
                    *sequence = Sequence::new(data.iter().cloned());
                }
                Parallel::new(data).into()
            },
            (Action::Parallel(parallel), Action::Operation(op)) => {
                let mut data = parallel.data;
                for sequence in &mut data {
                    let data = &mut sequence.data;
                    data.push(op.clone());
                    *sequence = Sequence::new(data.iter().cloned());
                }
                Parallel::new(data).into()
            }
            (Action::Sequence(first), Action::Sequence(mut second)) => {
                let mut data = first.data;
                data.append(&mut second.data);
                Sequence::new(data).into()
            },
            (Action::Sequence(mut first_sequence), Action::Parallel(parallel)) => {
                let mut data = parallel.data;
                for sequence in &mut data {
                    let data = &mut first_sequence.data;
                    data.append(&mut sequence.data.clone());
                }
                Parallel::new(data).into()
            },
            (Action::Parallel(parallel), Action::Sequence(second_sequence)) => {
                let mut data = parallel.data;
                for sequence in &mut data {
                    let data = &mut sequence.data;
                    data.append(&mut second_sequence.data.clone());
                }
                Parallel::new(data).into()
            },
            (Action::Parallel(first), Action::Parallel(second)) => {
                let mut data = Vec::new();
                for first_sequence in &first.data {
                    for second_sequence in &second.data {
                        let mut first_data = first_sequence.data.clone();
                        first_data.append(&mut second_sequence.data.clone());
                        data.push(Sequence::new(first_data));
                    }
                }
                Parallel::new(data).into()
            },
        }
    }
}