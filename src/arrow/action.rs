


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation<A> {
    Value(A),
    Identity,
}

impl<A> From<A> for Operation<A> {
    fn from(value: A) -> Self {
        Operation::Value(value)
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Operation<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Value(value) => write!(f, "{}", value),
            Operation::Identity => write!(f, "Id"),
        }
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sequence<A> {
    pub data: Vec<Operation<A>>
}

impl<A> From<Operation<A>> for Sequence<A> {
    fn from(atom: Operation<A>) -> Self {
        Sequence {
            data: [atom].into()
        }
    }
}

impl<A> From<A> for Sequence<A> {
    fn from(value: A) -> Self {
        let atom: Operation<A> = value.into();
        atom.into()
    }
}

impl<A: Ord> Sequence<A> {
    pub fn new<I: IntoIterator<Item = Operation<A>>>(data: I) -> Sequence<A> {
        Sequence {
            data: data.into_iter().collect()
        }
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Sequence<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.iter()
            .map(|atom| atom.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")) 
    }
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Parallel<A> {
    pub data: Vec<Sequence<A>>
}

impl<A> From<Sequence<A>> for Parallel<A> {
    fn from(product: Sequence<A>) -> Self {
        Parallel {
            data: [product].into()
        }
    }
}

impl<A> From<Operation<A>> for Parallel<A> {
    fn from(atom: Operation<A>) -> Self {
        let product: Sequence<A> = atom.into();
        product.into()
    }
}

impl<A> From<A> for Parallel<A> {
    fn from(value: A) -> Self {
        let atom: Operation<A> = value.into();
        atom.into()
    }
}

impl<A: Ord> Parallel<A> {
    pub fn new<I: IntoIterator<Item = Sequence<A>>>(data: I) -> Parallel<A> {
        let mut data: Vec<Sequence<A>> = data.into_iter().collect();
        data.sort();
        Parallel {
            data
        }
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Parallel<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.data.len() == 1 {
            write!(f, "{}", self.data[0])
        } else {
            write!(f, "({})", self.data.iter()
                .map(|prod| prod.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        }
        
    }
}


#[derive(Clone)]
pub enum Action<A> {
    Operation(Operation<A>),
    Sequence(Sequence<A>),
    Parallel(Parallel<A>),
    // Disjoint(???)
}

impl<A> Action<A> {
    pub fn identity() -> Action<A> {
        Self::Operation(Operation::Identity)
    }
}

impl<A> From<A> for Action<A> {
    fn from(value: A) -> Self {
        let atom: Operation<A> = value.into();
        atom.into()
    }
}

impl<A> From<Operation<A>> for Action<A> {
    fn from(atom: Operation<A>) -> Self {
        Action::Operation(atom)
    }
}

impl<A> From<Sequence<A>> for Action<A> {
    fn from(product: Sequence<A>) -> Self {
        Action::Sequence(product)
    }
}

impl<A> From<Parallel<A>> for Action<A> {
    fn from(sum: Parallel<A>) -> Self {
        Action::Parallel(sum)
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Action<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Operation(atom) => atom.fmt(f),
            Action::Sequence(product) => product.fmt(f),
            Action::Parallel(sum) => sum.fmt(f),
        }
    }
}


impl<A: Ord> std::ops::Add for Action<A> {
    type Output = Action<A>;

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

impl<A: Ord + Clone> std::ops::Mul for Action<A> {
    type Output = Action<A>;

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