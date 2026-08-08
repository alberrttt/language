#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Ident(String),
    Int(i64),
    Float(i32, i32),
    Str(String),

    // Keywords
    Let,
    Fn,
    Def,
    Match,
    If,
    Else,
    Return,
    True,
    False,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,

    // Operators
    Arrow,    // ->
    FatArrow, // =>
    Eq,       // =
    EqEq,     // ==
    Bang,     // !
    BangEq,   // !=
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe, // |
    Amp,  // &
    Dot,

    Eof,
}

pub use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}
