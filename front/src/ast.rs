use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Enum(EnumDef),
    Fn(FnDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: Ident,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: Ident,
    // Field types, e.g. `Succ(Nat)` has fields `[Nat]`. Types are terms.
    pub fields: Vec<Term>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<Term>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Term,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(LetStmt),
    Expr(Term),
    Return(ReturnStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: Ident,
    // Dependently typed: the annotation is itself a term, e.g. `Succ(Zero)`.
    pub ty: Option<Term>,
    pub value: Term,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Term>,
    pub span: Span,
}

/// Terms occupy both expression and type position, since types can depend
/// on values (e.g. `let num: Succ(Zero) = 1;`).
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Ident(Ident),
    Int { value: i64, span: Span },
    Float { int_part: i32, frac_part: i32, span: Span },
    Str { value: String, span: Span },
    Bool { value: bool, span: Span },
    Call {
        callee: Box<Term>,
        args: Vec<Term>,
        span: Span,
    },
    Match {
        scrutinee: Box<Term>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    If {
        cond: Box<Term>,
        then_branch: Box<Term>,
        else_branch: Option<Box<Term>>,
        span: Span,
    },
    Binary {
        op: BinOp,
        lhs: Box<Term>,
        rhs: Box<Term>,
        span: Span,
    },
    Unary {
        op: UnOp,
        operand: Box<Term>,
        span: Span,
    },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Term,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard { span: Span },
    Ident(Ident),
    Int { value: i64, span: Span },
    Ctor {
        name: Ident,
        args: Vec<Pattern>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

impl Term {
    pub fn span(&self) -> Span {
        match self {
            Term::Ident(ident) => ident.span,
            Term::Int { span, .. }
            | Term::Float { span, .. }
            | Term::Str { span, .. }
            | Term::Bool { span, .. }
            | Term::Call { span, .. }
            | Term::Match { span, .. }
            | Term::If { span, .. }
            | Term::Binary { span, .. }
            | Term::Unary { span, .. } => *span,
            Term::Block(block) => block.span,
        }
    }
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span } | Pattern::Int { span, .. } | Pattern::Ctor { span, .. } => {
                *span
            }
            Pattern::Ident(ident) => ident.span,
        }
    }
}
