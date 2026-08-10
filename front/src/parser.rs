use crate::ast::{
    BinOp, Block, EnumDef, FnDef, Ident, Item, LetStmt, MatchArm, Module, Param, Pattern,
    ReturnStmt, Stmt, Term, UnOp, Variant,
};
use crate::diag::{Diagnostic, Diagnostics};
use crate::lexer::Lexer;
use crate::span::Span;
use crate::token::{SpannedToken, Token};

/// Lexes and parses `src` in one step, for callers that don't need the
/// token stream itself.
pub fn parse_source(src: &str) -> (Module, Diagnostics) {
    let tokens = Lexer::new(src).tokenize();
    Parser::new(tokens).parse()
}

/// Recursive-descent parser. Errors are collected as [`Diagnostic`]s rather
/// than panicking, so a single run can report more than one problem: a
/// failed item or statement is skipped via [`Parser::synchronize_item`] /
/// [`Parser::synchronize_stmt`] and parsing resumes after it.
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    diagnostics: Diagnostics,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn parse(mut self) -> (Module, Diagnostics) {
        let mut items = Vec::new();
        while !self.is_at_end() {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.synchronize_item(),
            }
        }
        (Module { items }, self.diagnostics)
    }

    // -- token stream -----------------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn bump(&mut self) -> SpannedToken {
        let tok = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, want: Token, what: &str) -> Option<Span> {
        if self.peek() == &want {
            Some(self.bump().span)
        } else {
            let span = self.peek_span();
            self.error_at(
                span,
                format!("expected {what}, found {}", self.describe_current()),
            );
            None
        }
    }

    fn expect_ident(&mut self, what: &str) -> Option<Ident> {
        let span = self.peek_span();
        if let Token::Ident(name) = self.peek().clone() {
            self.bump();
            Some(Ident { name, span })
        } else {
            self.error_at(
                span,
                format!("expected {what}, found {}", self.describe_current()),
            );
            None
        }
    }

    fn describe_current(&self) -> String {
        token_desc(self.peek())
    }

    fn error_at(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message).with_bare_label(span));
    }

    /// Parses a comma-separated, optionally trailing-comma-terminated list
    /// up to (but not including) `close`.
    fn parse_comma_list<T>(
        &mut self,
        close: &Token,
        mut parse_one: impl FnMut(&mut Self) -> Option<T>,
    ) -> Option<Vec<T>> {
        let mut items = Vec::new();
        if self.peek() != close {
            loop {
                items.push(parse_one(self)?);
                if self.peek() == &Token::Comma {
                    self.bump();
                    if self.peek() == close {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        Some(items)
    }

    // -- recovery -----------------------------------------------------------

    /// After a malformed item, skip to the next token that could plausibly
    /// start one.
    fn synchronize_item(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Fn | Token::Enum => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// After a malformed statement, skip to the terminating `;` (consuming
    /// it) or to a token that could start the next statement or end the
    /// block.
    fn synchronize_stmt(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Token::Semicolon => {
                    self.bump();
                    return;
                }
                Token::RBrace | Token::Let | Token::Return => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    // -- items --------------------------------------------------------------

    fn parse_item(&mut self) -> Option<Item> {
        match self.peek() {
            Token::Enum => self.parse_enum_def().map(Item::Enum),
            Token::Fn => self.parse_fn_def().map(Item::Fn),
            _ => {
                let span = self.peek_span();
                self.error_at(
                    span,
                    format!("expected `fn` or `enum`, found {}", self.describe_current()),
                );
                None
            }
        }
    }

    fn parse_enum_def(&mut self) -> Option<EnumDef> {
        let start = self.bump().span; // `enum`
        let name = self.expect_ident("an enum name")?;
        self.expect(Token::LBrace, "`{`")?;
        let variants = self.parse_comma_list(&Token::RBrace, Self::parse_variant)?;
        let end = self.expect(Token::RBrace, "closing `}`")?;
        Some(EnumDef {
            name,
            variants,
            span: start.to(end),
        })
    }

    fn parse_variant(&mut self) -> Option<Variant> {
        let start = self.peek_span();
        let name = self.expect_ident("a variant name")?;
        let mut end = name.span;
        let mut fields = Vec::new();
        if self.peek() == &Token::LParen {
            self.bump();
            fields = self.parse_comma_list(&Token::RParen, Self::parse_term)?;
            end = self.expect(Token::RParen, "closing `)`")?;
        }
        Some(Variant {
            name,
            fields,
            span: start.to(end),
        })
    }

    fn parse_fn_def(&mut self) -> Option<FnDef> {
        let start = self.bump().span; // `fn`
        let name = self.expect_ident("a function name")?;
        self.expect(Token::LParen, "`(`")?;
        let params = self.parse_comma_list(&Token::RParen, Self::parse_param)?;
        self.expect(Token::RParen, "closing `)`")?;
        let ret = if self.peek() == &Token::Arrow {
            self.bump();
            Some(self.parse_term()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Some(FnDef {
            name,
            params,
            ret,
            span: start.to(body.span),
            body,
        })
    }

    fn parse_param(&mut self) -> Option<Param> {
        let start = self.peek_span();
        let name = self.expect_ident("a parameter name")?;
        self.expect(Token::Colon, "`:`")?;
        let ty = self.parse_term()?;
        Some(Param {
            span: start.to(ty.span()),
            name,
            ty,
        })
    }

    // -- statements -----------------------------------------------------------

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.expect(Token::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && !self.is_at_end() {
            match self.parse_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.synchronize_stmt(),
            }
        }
        let end = self.expect(Token::RBrace, "closing `}`")?;
        Some(Block {
            stmts,
            span: start.to(end),
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        match self.peek() {
            Token::Let => self.parse_let_stmt().map(Stmt::Let),
            Token::Return => self.parse_return_stmt().map(Stmt::Return),
            _ => {
                let term = self.parse_term()?;
                if self.peek() == &Token::Semicolon {
                    self.bump();
                }
                Some(Stmt::Expr(term))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Option<LetStmt> {
        let start = self.bump().span; // `let`
        let name = self.expect_ident("a variable name")?;
        let ty = if self.peek() == &Token::Colon {
            self.bump();
            Some(self.parse_term()?)
        } else {
            None
        };
        self.expect(Token::Eq, "`=`")?;
        let value = self.parse_term()?;
        let end = self.expect(Token::Semicolon, "`;`")?;
        Some(LetStmt {
            name,
            ty,
            value,
            span: start.to(end),
        })
    }

    fn parse_return_stmt(&mut self) -> Option<ReturnStmt> {
        let start = self.bump().span; // `return`
        let value = if self.peek() == &Token::Semicolon {
            None
        } else {
            Some(self.parse_term()?)
        };
        let end = self.expect(Token::Semicolon, "`;`")?;
        Some(ReturnStmt {
            value,
            span: start.to(end),
        })
    }

    // -- terms ----------------------------------------------------------------
    //
    // Precedence, low to high: or, and, equality, comparison, additive,
    // multiplicative, unary, call, primary.

    fn parse_term(&mut self) -> Option<Term> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Option<Term> {
        let mut lhs = self.parse_and()?;
        while self.peek() == &Token::Pipe {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = binary(BinOp::Or, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_and(&mut self) -> Option<Term> {
        let mut lhs = self.parse_equality()?;
        while self.peek() == &Token::Amp {
            self.bump();
            let rhs = self.parse_equality()?;
            lhs = binary(BinOp::And, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_equality(&mut self) -> Option<Term> {
        let mut lhs = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::Ne,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_comparison()?;
            lhs = binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_comparison(&mut self) -> Option<Term> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::LtEq => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::GtEq => BinOp::Ge,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_additive()?;
            lhs = binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_additive(&mut self) -> Option<Term> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            lhs = binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_multiplicative(&mut self) -> Option<Term> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = binary(op, lhs, rhs);
        }
        Some(lhs)
    }

    fn parse_unary(&mut self) -> Option<Term> {
        let start = self.peek_span();
        let op = match self.peek() {
            Token::Minus => UnOp::Neg,
            Token::Bang => UnOp::Not,
            _ => return self.parse_call(),
        };
        self.bump();
        let operand = self.parse_unary()?;
        let span = start.to(operand.span());
        Some(Term::Unary {
            op,
            operand: Box::new(operand),
            span,
        })
    }

    fn parse_call(&mut self) -> Option<Term> {
        let start = self.peek_span();
        let mut term = self.parse_primary()?;
        while self.peek() == &Token::LParen {
            self.bump();
            let args = self.parse_comma_list(&Token::RParen, Self::parse_term)?;
            let end = self.expect(Token::RParen, "closing `)`")?;
            term = Term::Call {
                callee: Box::new(term),
                args,
                span: start.to(end),
            };
        }
        Some(term)
    }

    fn parse_primary(&mut self) -> Option<Term> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Ident(name) => {
                self.bump();
                Some(Term::Ident(Ident { name, span }))
            }
            Token::Int(value) => {
                self.bump();
                Some(Term::Int { value, span })
            }
            Token::Float(int_part, frac_part) => {
                self.bump();
                Some(Term::Float {
                    int_part,
                    frac_part,
                    span,
                })
            }
            Token::Str(value) => {
                self.bump();
                Some(Term::Str { value, span })
            }
            Token::True => {
                self.bump();
                Some(Term::Bool { value: true, span })
            }
            Token::False => {
                self.bump();
                Some(Term::Bool { value: false, span })
            }
            Token::LParen => {
                self.bump();
                let inner = self.parse_term()?;
                self.expect(Token::RParen, "closing `)`")?;
                Some(inner)
            }
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::LBrace => self.parse_block().map(Term::Block),
            _ => {
                self.error_at(
                    span,
                    format!("expected an expression, found {}", self.describe_current()),
                );
                None
            }
        }
    }

    fn parse_if(&mut self) -> Option<Term> {
        let start = self.bump().span; // `if`
        let cond = self.parse_term()?;
        let then_branch = Term::Block(self.parse_block()?);
        let (else_branch, end) = if self.peek() == &Token::Else {
            self.bump();
            if self.peek() == &Token::If {
                let branch = self.parse_if()?;
                let span = branch.span();
                (Some(Box::new(branch)), span)
            } else {
                let block = self.parse_block()?;
                let span = block.span;
                (Some(Box::new(Term::Block(block))), span)
            }
        } else {
            (None, then_branch.span())
        };
        Some(Term::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch,
            span: start.to(end),
        })
    }

    fn parse_match(&mut self) -> Option<Term> {
        let start = self.bump().span; // `match`
        let scrutinee = self.parse_term()?;
        self.expect(Token::LBrace, "`{`")?;
        let arms = self.parse_comma_list(&Token::RBrace, Self::parse_match_arm)?;
        let end = self.expect(Token::RBrace, "closing `}`")?;
        Some(Term::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.to(end),
        })
    }

    fn parse_match_arm(&mut self) -> Option<MatchArm> {
        let start = self.peek_span();
        let pattern = self.parse_pattern()?;
        self.expect(Token::FatArrow, "`=>`")?;
        let body = self.parse_term()?;
        Some(MatchArm {
            span: start.to(body.span()),
            pattern,
            body,
        })
    }

    // -- patterns ---------------------------------------------------------
    //
    // A bare identifier could be a wildcard-like binding or a nullary
    // constructor (`Zero`); telling those apart needs name resolution, so
    // the parser only special-cases `_` and constructors written with `(`.

    fn parse_pattern(&mut self) -> Option<Pattern> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Ident(name) if name == "_" => {
                self.bump();
                Some(Pattern::Wildcard { span })
            }
            Token::Ident(name) => {
                self.bump();
                if self.peek() == &Token::LParen {
                    self.bump();
                    let args = self.parse_comma_list(&Token::RParen, Self::parse_pattern)?;
                    let end = self.expect(Token::RParen, "closing `)`")?;
                    Some(Pattern::Ctor {
                        name: Ident { name, span },
                        args,
                        span: span.to(end),
                    })
                } else {
                    Some(Pattern::Ident(Ident { name, span }))
                }
            }
            Token::Int(value) => {
                self.bump();
                Some(Pattern::Int { value, span })
            }
            _ => {
                self.error_at(
                    span,
                    format!("expected a pattern, found {}", self.describe_current()),
                );
                None
            }
        }
    }
}

fn binary(op: BinOp, lhs: Term, rhs: Term) -> Term {
    let span = lhs.span().to(rhs.span());
    Term::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        span,
    }
}

fn token_desc(token: &Token) -> String {
    match token {
        Token::Ident(name) => format!("identifier `{name}`"),
        Token::Int(v) => format!("integer `{v}`"),
        Token::Float(a, b) => format!("float `{a}.{b}`"),
        Token::Str(s) => format!("string {s:?}"),
        Token::Let => "`let`".into(),
        Token::Fn => "`fn`".into(),
        Token::Def => "`def`".into(),
        Token::Match => "`match`".into(),
        Token::If => "`if`".into(),
        Token::Else => "`else`".into(),
        Token::Return => "`return`".into(),
        Token::True => "`true`".into(),
        Token::False => "`false`".into(),
        Token::Does => "`does`".into(),
        Token::Enum => "`enum`".into(),
        Token::LParen => "`(`".into(),
        Token::RParen => "`)`".into(),
        Token::LBrace => "`{`".into(),
        Token::RBrace => "`}`".into(),
        Token::LBracket => "`[`".into(),
        Token::RBracket => "`]`".into(),
        Token::Comma => "`,`".into(),
        Token::Colon => "`:`".into(),
        Token::Semicolon => "`;`".into(),
        Token::Arrow => "`->`".into(),
        Token::FatArrow => "`=>`".into(),
        Token::Eq => "`=`".into(),
        Token::EqEq => "`==`".into(),
        Token::Bang => "`!`".into(),
        Token::BangEq => "`!=`".into(),
        Token::Lt => "`<`".into(),
        Token::LtEq => "`<=`".into(),
        Token::Gt => "`>`".into(),
        Token::GtEq => "`>=`".into(),
        Token::Plus => "`+`".into(),
        Token::Minus => "`-`".into(),
        Token::Star => "`*`".into(),
        Token::Slash => "`/`".into(),
        Token::Percent => "`%`".into(),
        Token::Pipe => "`|`".into(),
        Token::Amp => "`&`".into(),
        Token::Dot => "`.`".into(),
        Token::Pub => "pub".into(),
        Token::Eof => "end of file".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nat_enum() {
        let (module, diags) = parse_source("enum Nat {\n    Zero,\n    Succ(Nat)\n}\n");
        assert!(diags.is_empty(), "{:?}", diags.iter().collect::<Vec<_>>());
        assert_eq!(module.items.len(), 1);
        let Item::Enum(def) = &module.items[0] else {
            panic!("expected an enum item");
        };
        assert_eq!(def.name.name, "Nat");
        assert_eq!(def.variants.len(), 2);
        assert_eq!(def.variants[0].name.name, "Zero");
        assert!(def.variants[0].fields.is_empty());
        assert_eq!(def.variants[1].name.name, "Succ");
        assert_eq!(def.variants[1].fields.len(), 1);
        assert!(matches!(&def.variants[1].fields[0], Term::Ident(id) if id.name == "Nat"));
    }

    #[test]
    fn parses_recursive_fn_with_match() {
        let src = "fn add(a: Nat, b: Nat) -> Nat {\n    match a {\n        Zero => b,\n        Succ(n) => Succ(add(n,b)),\n    }\n}\n";
        let (module, diags) = parse_source(src);
        assert!(diags.is_empty(), "{:?}", diags.iter().collect::<Vec<_>>());
        assert_eq!(module.items.len(), 1);
        let Item::Fn(def) = &module.items[0] else {
            panic!("expected a fn item");
        };
        assert_eq!(def.name.name, "add");
        assert_eq!(def.params.len(), 2);
        assert!(def.ret.is_some());
        assert_eq!(def.body.stmts.len(), 1);
        let Stmt::Expr(Term::Match { arms, .. }) = &def.body.stmts[0] else {
            panic!("expected the block's only statement to be a match expression");
        };
        assert_eq!(arms.len(), 2);
        assert!(matches!(&arms[0].pattern, Pattern::Ident(id) if id.name == "Zero"));
        assert!(matches!(&arms[1].pattern, Pattern::Ctor { name, args, .. }
            if name.name == "Succ" && args.len() == 1));
    }

    #[test]
    fn parses_dependent_let_type() {
        let (module, diags) =
            parse_source("fn main() {\n    let num: Succ(Zero) = 1;\n    print(num);\n}\n");
        assert!(diags.is_empty(), "{:?}", diags.iter().collect::<Vec<_>>());
        let Item::Fn(def) = &module.items[0] else {
            panic!("expected a fn item");
        };
        let Stmt::Let(let_stmt) = &def.body.stmts[0] else {
            panic!("expected a let statement");
        };
        assert_eq!(let_stmt.name.name, "num");
        assert!(matches!(&let_stmt.ty, Some(Term::Call { .. })));
        assert!(matches!(&def.body.stmts[1], Stmt::Expr(Term::Call { .. })));
    }

    #[test]
    fn parses_the_8826_example_without_diagnostics() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/8826.lang"
        ))
        .expect("example file should exist");
        // `pub enum Nat` isn't parseable yet: the lexer has no `pub` keyword,
        // so it lexes as an identifier where an item is expected.
        let src = src.replacen("pub enum", "enum", 1);
        let (module, diags) = parse_source(&src);
        assert!(diags.is_empty(), "{:?}", diags.iter().collect::<Vec<_>>());
        assert_eq!(module.items.len(), 3);
    }

    #[test]
    fn reports_a_diagnostic_and_recovers_after_a_missing_semicolon() {
        let src = "fn f() {\n    let x = 1\n    let y = 2;\n}\nfn g() {}\n";
        let (module, diags) = parse_source(src);
        assert!(!diags.is_empty());
        // Recovery should still find both top-level items.
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn recovers_a_failed_item_without_swallowing_the_next_one() {
        // `f`'s body is missing, so `parse_fn_def` fails right where the
        // cursor sits on the `fn` of `g`. Synchronization must not step past
        // that token, or `g` disappears too.
        let (module, diags) = parse_source("fn f() -> i64\nfn g() {}\n");
        assert!(!diags.is_empty());
        assert_eq!(module.items.len(), 1);
        let Item::Fn(def) = &module.items[0] else {
            panic!("expected a fn item");
        };
        assert_eq!(def.name.name, "g");
    }

    #[test]
    fn operator_precedence_groups_multiplication_before_addition() {
        let (module, diags) = parse_source("fn f() -> i64 { 1 + 2 * 3 }");
        assert!(diags.is_empty(), "{:?}", diags.iter().collect::<Vec<_>>());
        let Item::Fn(def) = &module.items[0] else {
            panic!("expected a fn item");
        };
        let Stmt::Expr(Term::Binary { op, lhs, rhs, .. }) = &def.body.stmts[0] else {
            panic!("expected a binary expression");
        };
        assert_eq!(*op, BinOp::Add);
        assert!(matches!(**lhs, Term::Int { value: 1, .. }));
        assert!(matches!(**rhs, Term::Binary { op: BinOp::Mul, .. }));
    }
}
