use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(pub(super) u32);

/// String → Sym interner. Each string is allocated exactly once: the same
/// `Rc<str>` lives in `backward` (for `resolve`) and as the key in `forward`
/// (for `intern` / `find`).
#[derive(Clone, Debug, Default)]
pub struct Interner {
    forward: HashMap<Rc<str>, Sym>,
    backward: Vec<Rc<str>>,
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
        let shared: Rc<str> = Rc::from(s);
        self.forward.insert(Rc::clone(&shared), sym);
        self.backward.push(shared);
        sym
    }

    pub fn find(&self, s: &str) -> Option<Sym> {
        self.forward.get(s).copied()
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        &self.backward[sym.0 as usize]
    }
}
