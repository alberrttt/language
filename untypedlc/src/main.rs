// ! SIDE TRACK: I should learn lamda calculus firsts
//  YAY. My goal is to do this ENTIRELY without AI to test and train my brain :D
/*
t ::=
    x                       variable
    λx. t                   abstraction
    t t                     application
*/
// August 10th, 2026 10:03pm
// Basically I think It works but its like right associative but application is left associative
// So I will fix this tmr
// August 11th: Yay! Its left associative. This parser is a little messy; I didnt really tokenize. Just string
// manipulations. Thats why I kinda have to adhoc ignore all the whitespace. Also I refactoreed to use a ctxt struct
// OKAY if I make my own language: I am going to fix the shittiness of Box and methods like .as_ref() .as_str() etc etc
// AND also Struct and self and that refactorng
// Like why cant I make a recursive data type? Actually I kinda get it but also CONSTRUCTING it sux!
use core::panic;
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, PartialEq)]
pub enum Term {
    Var(u8),
    Abs(Box<Term>),
    App(Box<Term>, Box<Term>),
    Placehold,
}
static LAMBDA: char = 'λ';
fn main() {
    let src = "(λx. x)(λx. x)";
    let res = parse(src);
    dbg!(res);
    // August 11,2026: 11:15pm. I kinda didnt work on this much today!. I think I do subsitution to evaluate the lambda claculus now
}
#[test]
fn test1() {
    let src = "λx. x (x λx. x) x x";
    let res = parse(src);
    dbg!(res);
}
#[test]
fn test2() {
    let src = "λy. y (λx. y x λa. y a y a)";
    let res = parse(src);
    // WHY does making tests suck so much?
    // assert_eq!(
    //     res,
    //     App(
    //         Var(0,),
    //         Abs(App(
    //             App(Var(1,), Var(0,),),
    //             Abs(App(App(App(Var(2,), Var(0,),), Var(2,),), Var(0,),),),
    //         ),),
    //     ),
    // );
    dbg!(res);
}

#[test]
fn ctx_runs_parser() {
    let mut ctx = Ctx::new("λx. x");
    let term = ctx.parse_app();
    assert!(match term {
        Term::Abs(x) => match x.as_ref() {
            Term::Var(x) if *x == 0 => true,
            _ => false,
        },
        _ => false,
    });
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;
fn parse(src: &str) -> Term {
    let mut ctx = Ctx::new(src);
    ctx.parse_app()
}

pub struct Ctx<'a> {
    chars: Chars<'a>,
    names: Vec<char>,
}

impl<'a> Ctx<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().peekable(),
            names: Default::default(),
        }
    }

    fn take_whitespace(&mut self) -> Option<()> {
        while let Some(c) = self.chars.peek()
            && c.is_whitespace()
        {
            self.chars.next();
            return Some(());
        }
        None
    }

    fn expect(&mut self, c: char) -> Option<()> {
        if let Some(ch) = self.chars.next() {
            if ch.eq(&c) {
                return Some(());
            }
        }
        None
    }

    fn take_letter(&mut self) -> Option<char> {
        self.chars.next_if(|c| c.is_alphabetic())
    }

    fn parse_app(&mut self) -> Term {
        self.parse_app_(None)
    }

    fn parse_app_(&mut self, prev: Option<Term>) -> Term {
        let first = match prev {
            None => self.parse_term_not_app(),
            Some(x) => x,
        };
        if self.take_whitespace().is_some()
            || match self.chars.peek() {
                Some(x) if *x == '(' => true,
                Some(x) if *x == ')' => false,
                None => false,
                x => panic!("{:?}", x),
            }
        {
            let second = self.parse_term_not_app();
            let yay = Term::App(first.into(), second.into());
            return self.parse_app_(Some(yay));
        }

        first
    }
    // maybe theres a more efficient way?
    fn index(&mut self, c: char) -> Option<u8> {
        for (i, name) in self.names.iter().rev().enumerate() {
            if c.eq(name) {
                return Some(if i < 256 { i as u8 } else { panic!() });
            }
        }
        None
    }
    fn parse_term_not_app(&mut self) -> Term {
        self.take_whitespace();
        match self.chars.next().expect("Expected term") {
            'λ' => self.parse_abs(),
            '(' => self.parse_paren(),
            x if x.is_alphabetic() => Term::Var(
                self.index(x)
                    .expect(format!("Could not find variable {}", x).as_str()),
            ),
            x => panic!("Expected a term, got {}", x),
        }
    }

    fn starts_term(c: char) -> bool {
        c == 'λ' || c == '(' || c.is_alphabetic()
    }

    fn parse_paren(&mut self) -> Term {
        self.take_whitespace();
        let first = self.parse_term_not_app();
        let result =
            if self.take_whitespace().is_some() && Self::starts_term(*self.chars.peek().unwrap()) {
                let second = self.parse_term_not_app();
                Term::App(first.into(), second.into())
            } else {
                first
            };
        self.expect(')').expect("Expected a closing parenthesis");
        result
    }

    fn parse_abs(&mut self) -> Term {
        let letter = self.take_letter().expect("Expected a name");
        self.expect('.').expect("Expected a period");
        self.take_whitespace();
        self.names.push(letter.into());
        let body = self.parse_app();
        Term::Abs(Box::new(body))
    }
}
