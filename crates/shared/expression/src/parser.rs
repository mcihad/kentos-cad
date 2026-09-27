//! The expression grammar (the TypeScript's `Parser`): precedence climbing
//! over or, and, comparisons, + − ||, * / %; unary not and minus; literals,
//! fields, variables, calls and parentheses. Errors say what is wrong and
//! where, as the dialog shows them.

use super::CompileError;
use super::lexer::{Tok, Token};
use super::library::{Func, FuncDef, Var, find_function, find_variable};
use super::value::Value;
use crate::js::text::fold_turkish;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Join,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Debug)]
pub enum Node {
    Lit(Value<'static>),
    /// An attribute, by its index in the expression's field list.
    Field(usize),
    Var(Var),
    Call(Func, Vec<Node>),
    Not(Box<Node>),
    Neg(Box<Node>),
    Bin(BinOp, Box<Node>, Box<Node>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keyword {
    And,
    Or,
    Not,
    True,
    False,
    Null,
}

fn keyword(tok: &Token) -> Option<Keyword> {
    let Tok::Word(w) = &tok.t else {
        return None;
    };
    Some(match fold_turkish(w).as_str() {
        "VE" | "AND" => Keyword::And,
        "VEYA" | "OR" => Keyword::Or,
        "DEGIL" | "NOT" => Keyword::Not,
        "DOGRU" | "TRUE" => Keyword::True,
        "YANLIS" | "FALSE" => Keyword::False,
        "BOS" | "NULL" => Keyword::Null,
        _ => return None,
    })
}

/// Binary operators by precedence, loosest first.
const LEVELS: [&[(&str, BinOp)]; 5] = [
    &[("or", BinOp::Or)],
    &[("and", BinOp::And)],
    &[
        ("=", BinOp::Eq),
        ("==", BinOp::Eq),
        ("!=", BinOp::Ne),
        ("<>", BinOp::Ne),
        ("<", BinOp::Lt),
        ("<=", BinOp::Le),
        (">", BinOp::Gt),
        (">=", BinOp::Ge),
    ],
    &[("+", BinOp::Add), ("-", BinOp::Sub), ("||", BinOp::Join)],
    &[("*", BinOp::Mul), ("/", BinOp::Div), ("%", BinOp::Rem)],
];

pub struct Parser {
    toks: Vec<Token>,
    i: usize,
    /// Attribute names in the order they first appear.
    pub fields: Vec<String>,
}

fn err(message: impl Into<String>, at: usize) -> CompileError {
    CompileError {
        message: message.into(),
        at,
    }
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Parser {
        Parser {
            toks,
            i: 0,
            fields: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Node, CompileError> {
        if self.peek().t == Tok::End {
            return Err(err("İfade boş.", 1));
        }
        let n = self.binary(0)?;
        let rest = self.peek();
        if rest.t != Tok::End {
            return Err(err(
                format!(
                    "Beklenmeyen {}; iki değer arasında işleç eksik olabilir.",
                    rest.describe()
                ),
                rest.at,
            ));
        }
        Ok(n)
    }

    fn peek(&self) -> &Token {
        // The token list always ends with End, which is never passed.
        &self.toks[self.i.min(self.toks.len() - 1)]
    }

    fn next(&mut self) -> Token {
        let t = self.peek().clone();
        self.i += 1;
        t
    }

    fn field(&mut self, name: &str) -> usize {
        match self.fields.iter().position(|f| f == name) {
            Some(i) => i,
            None => {
                self.fields.push(name.to_string());
                self.fields.len() - 1
            }
        }
    }

    /// The operator at the cursor, words folded to and/or.
    fn op_at(&self) -> Option<&'static str> {
        let tok = self.peek();
        if let Tok::Op(o) = tok.t {
            return Some(o);
        }
        match keyword(tok) {
            Some(Keyword::And) => Some("and"),
            Some(Keyword::Or) => Some("or"),
            _ => None,
        }
    }

    fn binary(&mut self, level: usize) -> Result<Node, CompileError> {
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut a = self.binary(level + 1)?;
        while let Some(op) = self
            .op_at()
            .and_then(|o| LEVELS[level].iter().find(|(s, _)| *s == o))
        {
            self.next();
            let b = self.binary(level + 1)?;
            a = Node::Bin(op.1, Box::new(a), Box::new(b));
        }
        Ok(a)
    }

    fn unary(&mut self) -> Result<Node, CompileError> {
        let tok = self.peek().clone();
        if keyword(&tok) == Some(Keyword::Not) {
            self.next();
            return Ok(Node::Not(Box::new(self.unary()?)));
        }
        if let Tok::Op(o @ ("-" | "+")) = tok.t {
            self.next();
            let a = self.unary()?;
            return Ok(if o == "-" { Node::Neg(Box::new(a)) } else { a });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Node, CompileError> {
        let tok = self.next();
        match &tok.t {
            Tok::Num(v) => Ok(Node::Lit(Value::Num(*v))),
            Tok::Str(v) => Ok(Node::Lit(Value::text(v.clone()))),
            Tok::Field(name) => Ok(Node::Field(self.field(name))),
            Tok::Var(name) => match find_variable(name) {
                Some(v) => Ok(Node::Var(v.var)),
                None => Err(err(format!("Bilinmeyen değişken: ${name}."), tok.at)),
            },
            Tok::Word(name) => {
                if self.peek().t == Tok::Op("(") {
                    return self.call(name, tok.at);
                }
                match keyword(&tok) {
                    Some(Keyword::True) => Ok(Node::Lit(Value::Bool(true))),
                    Some(Keyword::False) => Ok(Node::Lit(Value::Bool(false))),
                    Some(Keyword::Null) => Ok(Node::Lit(Value::Null)),
                    Some(_) => Err(err(
                        format!(
                            "{} burada kullanılamaz; önünde bir değer olmalı.",
                            tok.describe()
                        ),
                        tok.at,
                    )),
                    None => Ok(Node::Field(self.field(name))),
                }
            }
            Tok::Op("(") => {
                let n = self.binary(0)?;
                self.expect_close("Parantez kapanmamış: “)” bekleniyordu.")?;
                Ok(n)
            }
            Tok::Op(_) => Err(err(
                format!(
                    "Beklenmeyen {}; burada bir değer, alan ya da işlev olmalı.",
                    tok.describe()
                ),
                tok.at,
            )),
            Tok::End => Err(err("İfade yarım kalmış: sonunda bir değer eksik.", tok.at)),
        }
    }

    fn call(&mut self, name: &str, at: usize) -> Result<Node, CompileError> {
        let Some(f) = find_function(name) else {
            return Err(err(format!("Bilinmeyen işlev: {name}()."), at));
        };
        self.next(); // (
        let mut args = Vec::new();
        if self.peek().t != Tok::Op(")") {
            loop {
                args.push(self.binary(0)?);
                if self.peek().t == Tok::Op(",") {
                    self.next();
                    continue;
                }
                break;
            }
        }
        self.expect_close(&format!("{}(…) kapanmamış: “)” bekleniyordu.", f.name))?;
        check_arity(f, args.len(), at)?;
        Ok(Node::Call(f.func, args))
    }

    /// A closing parenthesis, else `message` where the cursor is.
    fn expect_close(&mut self, message: &str) -> Result<(), CompileError> {
        let tok = self.peek();
        if tok.t == Tok::Op(")") {
            self.next();
            Ok(())
        } else {
            Err(err(message, tok.at))
        }
    }
}

fn check_arity(f: &FuncDef, n: usize, at: usize) -> Result<(), CompileError> {
    let (min, max) = f.arity;
    if n >= min && max.is_none_or(|m| n <= m) {
        return Ok(());
    }
    let want = match max {
        Some(m) if m == min => format!("{min}"),
        None => format!("en az {min}"),
        Some(m) => format!("{min} ya da {m}"),
    };
    Err(err(
        format!(
            "{}() {want} değer alır; {n} verildi. Kullanım: {}",
            f.name, f.signature
        ),
        at,
    ))
}
