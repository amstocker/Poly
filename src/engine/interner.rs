use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(pub(super) u32);

#[derive(Clone, Debug, Default)]
pub struct Interner {
    forward: HashMap<String, Sym>,
    backward: Vec<String>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> Sym {
        if let Some(&sym) = self.forward.get(s) {
            return sym;
        }
        let sym = Sym(self.backward.len() as u32);
        self.backward.push(s.to_string());
        self.forward.insert(s.to_string(), sym);
        sym
    }

    pub fn find(&self, s: &str) -> Option<Sym> {
        self.forward.get(s).copied()
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        &self.backward[sym.0 as usize]
    }
}
