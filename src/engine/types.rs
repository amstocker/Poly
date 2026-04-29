// ============================================================================
// Types and parameters
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type<T> {
    Int,
    Str,
    Bool,
    Named(T),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param<T> {
    pub name: T,
    pub ty: Type<T>,
}


// ============================================================================
// Expression AST
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr<T> {
    LitInt(i64),
    LitStr(String),
    LitBool(bool),
    Var(T),
    Field(Box<Expr<T>>, T),
    BinOp(BinOp, Box<Expr<T>>, Box<Expr<T>>),
    UnOp(UnOp, Box<Expr<T>>),
    Construct(T, Vec<Expr<T>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp { Neg, Not }


// ============================================================================
// Schema declarations
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema<T> {
    pub name: T,
    pub body: SchemaBody<T>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaBody<T> {
    Record(Vec<Param<T>>),
    Sum(Vec<Variant<T>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
}


// ============================================================================
// Interface declarations
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub positions: Vec<Position<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub guard: Option<Expr<T>>,
    pub directions: Vec<Direction<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Direction<T> {
    pub name: T,
    pub params: Vec<Param<T>>,
    pub guard: Option<Expr<T>>,
    pub transition: Option<Transition<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition<T> {
    pub target_pos: T,
    pub args: Vec<Expr<T>>,
}

impl<T> Interface<T> {
    pub fn is_parameterized(&self) -> bool {
        !self.params.is_empty()
            || self.positions.iter().any(|p| {
                !p.params.is_empty()
                    || p.guard.is_some()
                    || p.directions.iter().any(|d| !d.params.is_empty() || d.guard.is_some())
            })
    }
}

impl<T: PartialEq> Interface<T> {
    pub fn position(&self, name: &T) -> Option<&Position<T>> {
        self.positions.iter().find(|p| &p.name == name)
    }
}


// ============================================================================
// Defer declarations
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Defer<T> {
    pub name: T,
    pub source: T,
    pub target: T,
    pub entries: Vec<DeferEntry<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeferEntry<T> {
    pub source_pos: T,
    pub source_pattern: Vec<Pattern<T>>,
    pub source_guard: Option<Expr<T>>,
    pub target_pos: T,
    pub target_args: Vec<Expr<T>>,
    pub directions: Vec<DirMapping<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern<T> {
    Wildcard,
    Bind(T),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirMapping<T> {
    pub target_dir: DirRef<T>,
    pub source_dir: DirRef<T>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirRef<T> {
    Named(T),
    Abstract {
        src_pos: T,
        src_pattern: Vec<Pattern<T>>,
        tgt_pos: T,
        tgt_args: Vec<Expr<T>>,
    },
}


// ============================================================================
// Top-level declaration
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decl<T> {
    Interface(Interface<T>),
    Defer(Defer<T>),
    Schema(Schema<T>),
}


