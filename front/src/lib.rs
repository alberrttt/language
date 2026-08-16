pub mod lexer;
pub mod token;

pub use lexer::Lexer;
pub use token::{Span, SpannedToken, Token};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_fn_signature() {
        let src = "fn add(a: i64, b: i64) -> i64 { a + b }";
        let tokens: Vec<Token> = Lexer::new(src)
            .tokenize()
            .into_iter()
            .map(|t| t.token)
            .collect();

        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Ident("add".into()),
                Token::LParen,
                Token::Ident("a".into()),
                Token::Colon,
                Token::Ident("i64".into()),
                Token::Comma,
                Token::Ident("b".into()),
                Token::Colon,
                Token::Ident("i64".into()),
                Token::RParen,
                Token::Arrow,
                Token::Ident("i64".into()),
                Token::LBrace,
                Token::Ident("a".into()),
                Token::Plus,
                Token::Ident("b".into()),
                Token::RBrace,
                Token::Eof,
            ]
        );
    }
    #[test]
    fn floats() {
        let src = "3.14 0.5 10.0";
        let tokens: Vec<Token> = Lexer::new(src)
            .tokenize()
            .into_iter()
            .map(|t| t.token)
            .collect();

        assert_eq!(
            tokens,
            vec![
                Token::Float(3, 14),
                Token::Float(0, 5),
                Token::Float(10, 0),
                Token::Eof,
            ]
        );
    }
    #[test]
    fn lexes_let_and_match() {
        let src = r#"let x = 5; match x { 1 => "one", _ => "other" }"#;
        let tokens: Vec<Token> = Lexer::new(src)
            .tokenize()
            .into_iter()
            .map(|t| t.token)
            .collect();

        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Ident("x".into()),
                Token::Eq,
                Token::Int(5),
                Token::Semicolon,
                Token::Match,
                Token::Ident("x".into()),
                Token::LBrace,
                Token::Int(1),
                Token::FatArrow,
                Token::Str("one".into()),
                Token::Comma,
                Token::Ident("_".into()),
                Token::FatArrow,
                Token::Str("other".into()),
                Token::RBrace,
                Token::Eof,
            ]
        );
    }
}
