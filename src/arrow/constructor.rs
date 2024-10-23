


pub trait Constructor<T> {
    fn construct(self) -> T;
}


#[derive(PartialEq, Eq)]
pub enum Object<T> {
    Atom(T),
    Product(Parallel<T>),
    Sum(Disjoint<T>)
}

pub enum Arrow<T> {
    Arrow {
        source: Object<T>,
        target: Object<T>
    },
    Identity,
    Zero
}

impl<T: Eq> Arrow<T> {
    pub fn then(self, other: Self) -> Self {
        match (self, other) {
            (
                Arrow::Arrow { source, target },
                Arrow::Arrow { source: other_source, target: other_target }
            )
                => if target == other_source {
                    Arrow::Arrow { source, target: other_target }
                } else {
                    Arrow::Zero
                },
            (Arrow::Identity, arrow) |
            (arrow, Arrow::Identity)
                => arrow,
            (Arrow::Zero, _) |
            (_, Arrow::Zero)
                => Arrow::Zero
        }
    }

    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Arrow::Arrow { source, target }, Arrow::Arrow { source: other_source, target: other_target }) => todo!(),
            (Arrow::Identity, arrow) | (arrow, Arrow::Identity) => arrow,
            (Arrow::Zero, _) | (_, Arrow::Zero) => Arrow::Zero
        }
    }

    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Arrow::Arrow { source, target }, Arrow::Arrow { source: other_source, target: other_target }) => todo!(),
            (Arrow::Arrow { source, target }, Arrow::Identity) => todo!(),
            (Arrow::Arrow { source, target }, Arrow::Zero) => todo!(),
            (Arrow::Identity, Arrow::Arrow { source, target }) => todo!(),
            (Arrow::Identity, Arrow::Identity) => todo!(),
            (Arrow::Identity, Arrow::Zero) => todo!(),
            (Arrow::Zero, Arrow::Arrow { source, target }) => todo!(),
            (Arrow::Zero, Arrow::Identity) => todo!(),
            (Arrow::Zero, Arrow::Zero) => todo!(),
        }
    }
}

//impl<T: Eq> Constructor<Arrow<T>> for Sequence<Arrow<T>> {
//    fn construct(self) -> Arrow<T> {
//        self.data.into_iter()
//            .fold(Arrow::Identity, |acc, next| acc.then(next))
//    }
//}
//
//impl<T: Eq> Constructor<Arrow<T>> for Parallel<Arrow<T>> {
//    fn construct(self) -> Arrow<T> {
//        self.data.into_iter()
//            .fold(Arrow::Identity, |acc, next| acc.and(next.construct()))
//    }
//}
//
//impl<T: Eq> Constructor<Arrow<T>> for Disjoint<Arrow<T>> {
//    fn construct(self) -> Arrow<T> {
//        self.data.into_iter()
//            .fold(Arrow::Zero, |acc, next| acc.or(next.construct()))
//    }
//}



pub trait ComposeSequence<A, B> {
    fn then(self, other: impl Into<Sequence<A>>) -> B;
}

pub trait ComposeParallel<A, B> {
    fn and(self, other: impl Into<Parallel<A>>) -> B;
}

pub trait ComposeDisjoint<A, B> {
    fn or(self, other: impl Into<Disjoint<A>>) -> B;
}





/*
 * Parallel
 */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Parallel<A> {
    data: Vec<A>
}

impl<A> From<A> for Parallel<A> {
    fn from(value: A) -> Self {
        Parallel {
            data: [value].into()
        }
    }
}

impl<A: Ord> Parallel<A> {
    pub fn new(mut data: Vec<A>) -> Parallel<A> {
        data.sort();
        Parallel {
            data
        }
    }
}

impl<T: Into<Parallel<A>>, A: Ord> ComposeParallel<A, Parallel<A>> for T {
    fn and(self, other: impl Into<Parallel<A>>) -> Parallel<A> {
        let mut data = self.into().data;
        data.append(&mut other.into().data);
        Parallel::new(data)
    }
}

impl<A: std::fmt::Display> std::fmt::Display for Parallel<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.data.len() == 1 {
            write!(f, "{}", self.data[0])
        } else {
            write!(f, "({})", self.data.iter()
                .map(|seq| seq.to_string())
                .collect::<Vec<_>>()
                .join(", "))
        }
        
    }
}


/*
 * Disjoint
 */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Disjoint<A> {
    data: Vec<Parallel<A>>
}

impl<A> From<Parallel<A>> for Disjoint<A> {
    fn from(parallel: Parallel<A>) -> Self {
        Disjoint {
            data: [parallel].into()
        }
    }
}

impl<A> From<A> for Disjoint<A> {
    fn from(value: A) -> Self {
        let parallel: Parallel<A> = value.into();
        parallel.into()
    }
}

impl<A: Ord> Disjoint<A> {
    pub fn new(mut data: Vec<Parallel<A>>) -> Disjoint<A> {
        data.sort();
        Disjoint {
            data
        }
    }
}

//impl<T: Into<Disjoint<A>>, A> ComposeDisjoint<A, Disjoint<A>> for T {
//    fn or(self, mut either: Disjoint<A>) -> Disjoint<A> {
//        let mut data = self.into().data;
//        data.append(&mut either.data);
//        Disjoint {
//            data
//        }
//    }
//}
//
//impl<T: Into<Disjoint<A>>, A> ComposeParallel<A, Disjoint<A>> for T {
//    fn and(self, other_parallel: Parallel<A>) -> Disjoint<A> {
//        let mut either = self.into();
//        for parallel in &mut either.data {
//            *parallel = parallel.and(other_parallel);
//        }
//        either
//    }
//}

impl<A: std::fmt::Display> std::fmt::Display for Disjoint<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.iter()
            .map(|parallel| parallel.to_string())
            .collect::<Vec<_>>()
            .join(" + ")) 
    }
}


/*
 * Sequence
 */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sequence<A> {
    data: Vec<Disjoint<A>>
}

impl<A> From<Disjoint<A>> for Sequence<A> {
    fn from(disjoint: Disjoint<A>) -> Self {
        Sequence {
            data: [disjoint].into()
        }
    }
}

impl<A> From<Parallel<A>> for Sequence<A> {
    fn from(value: Parallel<A>) -> Self {
        let disjoint: Disjoint<A> = value.into();
        disjoint.into()
    }
}

impl<A> From<A> for Sequence<A> {
    fn from(value: A) -> Self {
        let parallel: Parallel<A> = value.into();
        parallel.into()
    }
}

impl<A> Sequence<A> {
    pub fn new(data: Vec<Disjoint<A>>) -> Sequence<A> {
        Sequence {
            data
        }
    }
}

impl<T: Into<Sequence<A>>, A> ComposeSequence<A, Sequence<A>> for T {
    fn then(self, other: impl Into<Sequence<A>>) -> Sequence<A> {
        let mut seq: Sequence<A> = self.into();
        seq.data.append(&mut other.into().data);
        seq
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




// impl<A: Ord> std::ops::Add for Action<A> {
//     type Output = Action<A>;
// 
//     fn add(self, rhs: Self) -> Self::Output {
//         match (self, rhs) {
//             (Action::Operation(left), Action::Operation(right)) => {
//                 Parallel::new([left.into(), right.into()]).into()
//             },
//             (Action::Operation(atom), Action::Sequence(product)) |
//             (Action::Sequence(product), Action::Operation(atom)) => {
//                 Parallel::new([product, atom.into()]).into()
//             },
//             (Action::Operation(atom), Action::Parallel(sum)) |
//             (Action::Parallel(sum), Action::Operation(atom)) => {
//                 let mut data = sum.data;
//                 data.push(atom.into());
//                 Parallel::new(data).into()
//             },
//             (Action::Sequence(left), Action::Sequence(right)) => {
//                 Parallel::new([left, right]).into()
//             },
//             (Action::Sequence(product), Action::Parallel(sum)) |
//             (Action::Parallel(sum), Action::Sequence(product)) => {
//                 let mut data = sum.data;
//                 data.push(product);
//                 Parallel::new(data).into()
//             },
//             (Action::Parallel(left), Action::Parallel(mut right)) => {
//                 let mut data = left.data;
//                 data.append(&mut right.data);
//                 Parallel::new(data).into()
//             },
//         }
//     }
// }
// 
// impl<A: Ord + Clone> std::ops::Mul for Action<A> {
//     type Output = Action<A>;
// 
//     fn mul(self, rhs: Self) -> Self::Output {
//         match (self, rhs) {
//             (Action::Operation(first), Action::Operation(second)) => match (first, second) {
//                 (Atom::Identity, op) | (op, Atom::Identity) => op.into(),
//                 (first, second) => Sequence::new([first, second]).into(),
//             },
//             (Action::Operation(op), Action::Sequence(sequence)) => {
//                 let mut data = sequence.data;
//                 data.insert(0, op);
//                 Sequence::new(data).into() 
//             },
//             (Action::Sequence(sequence), Action::Operation(op)) => {
//                 let mut data = sequence.data;
//                 data.push(op);
//                 Sequence::new(data).into() 
//             },
//             (Action::Operation(op), Action::Parallel(parallel)) => {
//                 let mut data = parallel.data;
//                 for sequence in &mut data {
//                     let data = &mut sequence.data;
//                     data.insert(0, op.clone());
//                     *sequence = Sequence::new(data.iter().cloned());
//                 }
//                 Parallel::new(data).into()
//             },
//             (Action::Parallel(parallel), Action::Operation(op)) => {
//                 let mut data = parallel.data;
//                 for sequence in &mut data {
//                     let data = &mut sequence.data;
//                     data.push(op.clone());
//                     *sequence = Sequence::new(data.iter().cloned());
//                 }
//                 Parallel::new(data).into()
//             }
//             (Action::Sequence(first), Action::Sequence(mut second)) => {
//                 let mut data = first.data;
//                 data.append(&mut second.data);
//                 Sequence::new(data).into()
//             },
//             (Action::Sequence(mut first_sequence), Action::Parallel(parallel)) => {
//                 let mut data = parallel.data;
//                 for sequence in &mut data {
//                     let data = &mut first_sequence.data;
//                     data.append(&mut sequence.data.clone());
//                 }
//                 Parallel::new(data).into()
//             },
//             (Action::Parallel(parallel), Action::Sequence(second_sequence)) => {
//                 let mut data = parallel.data;
//                 for sequence in &mut data {
//                     let data = &mut sequence.data;
//                     data.append(&mut second_sequence.data.clone());
//                 }
//                 Parallel::new(data).into()
//             },
//             (Action::Parallel(first), Action::Parallel(second)) => {
//                 let mut data = Vec::new();
//                 for first_sequence in &first.data {
//                     for second_sequence in &second.data {
//                         let mut first_data = first_sequence.data.clone();
//                         first_data.append(&mut second_sequence.data.clone());
//                         data.push(Sequence::new(first_data));
//                     }
//                 }
//                 Parallel::new(data).into()
//             },
//         }
//     }
// }