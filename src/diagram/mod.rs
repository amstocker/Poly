pub mod parse;

use im::Vector;



#[derive(Debug, Clone)]
pub struct Term<T: Clone>(Vector<T>);

impl<T: Clone> From<T> for Term<T> {
    fn from(value: T) -> Self {
        Term([value].into_iter().collect())
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


pub struct Transform<T: Clone> {
    pub source: Term<T>,
    pub target: Term<T>
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for Transform<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} => {}", self.source, self.target)
    }
}
