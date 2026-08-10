// ! SIDE TRACK: I should learn lamda calculus firsts

/*
t ::=
    x                       variable
    λx. t                   abstraction
    t t                     application
*/

#[derive(Debug)]
pub enum Term {
    Var(u8),
    Abs(String, Box<Term>),
    App(Box<Term>, Box<Term>),
}

fn main() {
    
}
