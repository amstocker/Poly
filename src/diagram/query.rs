use super::constructor::*;



const PLACEHOLDER: &'static str = "_";

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Placeholder<T: Clone> {
    Blank,
    Constructor(Constructor<T>)
}

impl From<String> for Placeholder<String> {
    fn from(value: String) -> Self {
        if value == PLACEHOLDER {
            Placeholder::Blank
        } else {
            Placeholder::Constructor(value.into())
        }
    }
}

impl std::fmt::Display for Placeholder<String> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Placeholder::Blank => write!(f, "{}", PLACEHOLDER),
            Placeholder::Constructor(constructor) => write!(f, "{}", constructor),
        }
    }
}