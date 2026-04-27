use logru::{RuleSet, SymbolStore, SymbolStorage};
use logru::ast::{self, Sym};


enum Symbol {
    State,
    Action
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        match self {
            Symbol::State  => "state",
            Symbol::Action => "action",
        }
    }
}

struct Context {
    symbols: SymbolStore,
    rules: RuleSet
}

impl Context {
    pub fn new() -> Self {
        let mut symbols = SymbolStore::new();
        let mut rules = RuleSet::new();

        let state_symbol = symbols.get_or_insert_named(Symbol::State.as_ref());
        let action_symbol = symbols.get_or_insert_named(Symbol::Action.as_ref());

        Context {
            symbols,
            rules
        }
    }

    fn symbol(&mut self, symbol: Symbol) -> Sym {
        self.symbols.get_or_insert_named(symbol.as_ref())
    }

}