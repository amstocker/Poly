

pub enum Object<T> {
    Raw(RawObject<T>),
    Simplified(SimplifiedObject<T>)
}

pub enum RawObject<T> {
    Atom(T),
    Sum(Vec<Object<T>>),
    Product(Vec<Object<T>>)
}

pub struct SimplifiedObject<T> {
    // Ordered sum of products?
    data: Vec<Vec<T>>
}

impl<T> RawObject<T> {
    pub fn simplified(self) -> SimplifiedObject<T> {
        unimplemented!()
    }
}