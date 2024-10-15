use std::ops::{Add, Mul};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Action<T>(pub T);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sequence<T>(pub Vec<Action<T>>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Parallel<T>(pub Vec<Sequence<T>>);

pub struct Unit;


impl<T: Ord> Action<T> {
    pub fn new(t: T) -> Parallel<T> {
        Self::sequence([Action(t)])
    }

    pub fn sequence<I: IntoIterator<Item = Action<T>>>(data: I) -> Parallel<T> {
        Self::parallel([Sequence(data.into_iter().collect())])
    }

    pub fn parallel<I: IntoIterator<Item = Sequence<T>>>(data: I) -> Parallel<T> {
        let mut data: Vec<Sequence<T>> = data.into_iter().collect();
        data.sort();
        Parallel(data)
    }
}

impl<T> Add for Parallel<T> where T: Ord {
    type Output = Parallel<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Action::parallel(self.0.into_iter().chain(rhs.0.into_iter()))
    }
}

impl<T> Mul for Parallel<T> where T: Ord + Clone {
    type Output = Parallel<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = Vec::new();
        for Sequence(first_seq) in &self.0 {
            for Sequence(second_seq) in &rhs.0 {
                let mut seq = first_seq.clone();
                seq.append(&mut second_seq.clone());
                data.push(Sequence(seq));
            }
        }
        Action::parallel(data)
    }
}