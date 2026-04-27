pub mod parse;

use im::Vector;



#[derive(Clone, PartialEq, Eq)]
pub struct Term<T: Clone>(Vector<T>);

impl<T: Clone> From<T> for Term<T> {
    fn from(value: T) -> Self {
        Term(Vector::unit(value))
    }
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Term<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.len() {
            0 => write!(f, "()"),
            1 => write!(f, "{}", self.0[0]),
            _ => write!(f, "({})", self.0.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(", "))
        }
    }
}

impl<T: Clone> Term<T> {
    pub fn dim(&self) -> usize {
        self.0.len()
    }

    pub fn mult(&self, other: &Term<T>) -> Term<T> {
        let mut atoms = self.0.clone();
        atoms.extend(other.0.clone());
        Term(atoms)
    }

    pub fn transform<'t>(&self, transforms: &'t Vec<Transform<T>>) -> Transformer<'t, T> {
        Transformer::new(transforms, self.clone())
    }
}


#[derive(Clone)]
pub struct Transform<T: Clone> {
    pub source: Term<T>,
    pub target: Term<T>
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Transform<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} => {}", self.source, self.target)
    }
}


pub struct Transformer<'t, T: Clone> {
    transforms: &'t Vec<Transform<T>>,
    source: Term<T>,
    index: usize,
    stack: Vec<usize>
}

impl<'t, T: Clone> Transformer<'t, T> {
    pub fn new(transforms: &'t Vec<Transform<T>>, source: Term<T>) -> Transformer<'t, T> {
        Transformer {
            transforms,
            source,
            index: 0,
            stack: Vec::new()
        }
    }
}

impl<'t, T: Clone> Iterator for Transformer<'t, T> {
    type Item = Term<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.source.dim() {

        }
        unimplemented!()
    }
}




#[derive(Clone, PartialEq, Eq)]
pub struct Path<T: Clone>(Vector<Term<T>>);

impl<T: Clone + Eq> PartialOrd for Path<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.depth().partial_cmp(&self.depth())
    }
}

impl<T: Clone + Eq> Ord for Path<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.depth().cmp(&self.depth())
    }
}

impl<T: Clone> From<T> for Path<T> {
    fn from(value: T) -> Self {
        Path::from(Term::from(value))
    }
}

impl<T: Clone> From<Term<T>> for Path<T> {
    fn from(term: Term<T>) -> Self {
        Path(Vector::unit(term))
    }
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Path<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().map(|elem| elem.to_string()).collect::<Vec<_>>().join(" => "))
    }
}

impl<T: Clone> Path<T> {
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn target(&self) -> &Term<T> {
        match self.0.last() {
            Some(term) => term,
            None => unreachable!(),
        }
    }

    pub fn push(&self, term: Term<T>) -> Path<T> {
        let mut terms = self.0.clone();
        terms.push_back(term);
        Path(terms)
    }
}
