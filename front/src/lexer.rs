use crate::token::{Span, SpannedToken, Token};

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            chars: src.char_indices().peekable(),
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let eof = tok.token == Token::Eof;
            tokens.push(tok);
            if eof {
                break;
            }
        }
        tokens
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let next = self.chars.next();
        if let Some((i, c)) = next {
            self.pos = i + c.len_utf8();
        }
        next
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') => {
                    let mut clone = self.chars.clone();
                    clone.next();
                    if let Some((_, '/')) = clone.next() {
                        while let Some(c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.bump();
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> SpannedToken {
        self.skip_whitespace_and_comments();

        let start = self.pos;
        let (idx, c) = match self.bump() {
            Some(pair) => pair,
            None => {
                return SpannedToken {
                    token: Token::Eof,
                    span: Span { start, end: start },
                };
            }
        };
        let _ = idx;

        let token = match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            '.' => Token::Dot,
            '+' => Token::Plus,
            '-' => {
                if self.peek() == Some('>') {
                    self.bump();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '|' => Token::Pipe,
            '&' => Token::Amp,
            '=' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::EqEq
                } else if self.peek() == Some('>') {
                    self.bump();
                    Token::FatArrow
                } else {
                    Token::Eq
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::BangEq
                } else {
                    Token::Bang
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '"' => self.lex_string(),
            c if c.is_ascii_digit() => self.lex_number(c),
            c if is_ident_start(c) => self.lex_ident(c),
            c => panic!("unexpected character '{c}' at byte {start}"),
        };

        SpannedToken {
            token,
            span: Span {
                start,
                end: self.pos,
            },
        }
    }

    fn lex_string(&mut self) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.bump();
            if c == '"' {
                return Token::Str(s);
            }
            if c == '\\' {
                if let Some((_, escaped)) = self.chars.next() {
                    self.pos += escaped.len_utf8();
                    let unescaped = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    };
                    s.push(unescaped);
                }
                continue;
            }
            s.push(c);
        }
        panic!("unterminated string literal");
    }

    fn lex_number(&mut self, first: char) -> Token {
        let mut buf = String::new();
        let mut secondary = String::new();
        buf.push(first);
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                if is_float {
                    secondary.push(c);
                } else {
                    buf.push(c);
                }
                self.bump();
            } else if c == '.' && !is_float {
                is_float = true;
                self.bump();
            } else {
                break;
            }
        }
        if is_float {
            let a = buf.parse().expect("Unable to parse float");
            let b = secondary.parse().expect("Unable to parse float");
            Token::Float(a, b)
        } else {
            Token::Int(buf.parse().expect("invalid int literal"))
        }
    }

    fn lex_ident(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        keyword_or_ident(s)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn keyword_or_ident(s: String) -> Token {
    match s.as_str() {
        "let" => Token::Let,
        "fn" => Token::Fn,
        "def" => Token::Def,
        "match" => Token::Match,
        "if" => Token::If,
        "else" => Token::Else,
        "return" => Token::Return,
        "true" => Token::True,
        "false" => Token::False,
        "does" => Token::Does,
        "enum" => Token::Enum,
        "pub" => Token::Pub,
        _ => Token::Ident(s),
    }
}
