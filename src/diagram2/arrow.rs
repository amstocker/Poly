use std::ops::{Add, Mul};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Action<T>(T);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sequence<T>(Vec<Action<T>>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arrow<T>(Vec<Sequence<T>>);


impl<T: Ord> Arrow<T> {
    pub fn action(t: T) -> Arrow<T> {
        Self::sequence([Action(t)])
    }

    pub fn sequence<I: IntoIterator<Item = Action<T>>>(data: I) -> Arrow<T> {
        Self::parallel([Sequence(data.into_iter().collect())])
    }

    pub fn parallel<I: IntoIterator<Item = Sequence<T>>>(data: I) -> Arrow<T> {
        let mut data: Vec<Sequence<T>> = data.into_iter().collect();
        data.sort();
        Arrow(data)
    }
}

impl<T> Add for Arrow<T> where T: Ord {
    type Output = Arrow<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Arrow::parallel(self.0.into_iter().chain(rhs.0.into_iter()))
    }
}

impl<T> Mul for Arrow<T> where T: Ord + Clone {
    type Output = Arrow<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = Vec::new();
        for Sequence(first_seq) in &self.0 {
            for Sequence(second_seq) in &rhs.0 {
                let mut seq = first_seq.clone();
                seq.append(&mut second_seq.clone());
                data.push(Sequence(seq));
            }
        }
        Arrow::parallel(data)
    }
}