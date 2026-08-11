// ! SIDE TRACK: I should learn lamda calculus firsts

/*
t ::=
    x                       variable
    λx. t                   abstraction
    t t                     application
*/

use core::panic;
use std::{fmt::Result, u8::MAX};

#[derive(Debug)]
pub enum Term {
    Var(char),
    Abs(char, Box<Term>),
    App(Box<Term>, Box<Term>),
    Placehold,
}
fn take_whitespace(chars: &mut Chars) -> Option<()> {
    while let Some(c) = chars.peek()
        && c.is_whitespace()
    {
        chars.next();
        return Some(());
    }
    return None;
}
fn expect(chars: &mut Chars, c: char) -> Option<()> {
    if let Some(ch) = chars.next() {
        if ch.eq(&c) {
            return Some(());
        }
    }
    None
}
fn take_letter(chars: &mut Chars) -> Option<char> {
    chars.next_if(|c| c.is_alphabetic())
}
enum Mode {
    StartParen,
    CloseParen,
}
static LAMBDA: char = 'λ';
fn main() {
    // let mut src = "λx.x λx.x";
    let src = "(λx. x x)(λx. x x)";
    let res = run(src);
    dbg!(res);
}
#[test]
fn test1() {
    let src = "λx. x x λx. x x";
    let res = run(src);
    // August 10th, 2026 10:03pm
    // Basically I think It works but its like right associative but application is left associative
    // So I will fix this tmr
    dbg!(res);
}
type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;
fn run(src: &str) -> Term {
    let mut chars: Chars = src.chars().peekable();
    let lc = parse_app(&mut chars);
  
    dbg!(chars);
    lc
}
fn parse_app(chars: &mut Chars) -> Term {
    let first = parse_term_not_app(chars);
    if take_whitespace(chars).is_some()
        || match chars.peek() {
            Some(x) if *x == '(' => true,
            None => false,
            _ => panic!(),
        }
    {
        let second = parse_app(chars);
        return Term::App(first.into(), second.into());
    }
    first
}
fn parse_term_not_app(chars: &mut Chars) -> Term {
    take_whitespace(chars);
    match chars.next().expect("Expected term") {
        'λ' => parse_abs(chars),
        '(' => parse_paren(chars),
        x if x.is_alphabetic() => Term::Var(x),
        _ => panic!("Expected a term"),
    }
}
fn starts_term(c: char) -> bool {
    c == 'λ' || c == '(' || c.is_alphabetic()
}
fn parse_paren(chars: &mut Chars) -> Term {
    take_whitespace(chars);
    let first = parse_term_not_app(chars);
    if take_whitespace(chars).is_some() && starts_term(*chars.peek().unwrap()) {
        let second = parse_term_not_app(chars);
        return Term::App(first.into(), second.into());
    }
    expect(chars, ')').expect("Expected a closing parenthesis");
    first
}

fn parse_abs(chars: &mut Chars) -> Term {
    let letter = take_letter(chars).expect("Expected a name");
    expect(chars, '.').expect("Expected a period");
    take_whitespace(chars);
    dbg!(&chars);
    let body = parse_app(chars);

    Term::Abs(letter, Box::new(body))
}
