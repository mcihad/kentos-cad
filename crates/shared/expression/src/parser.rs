//! The expression grammar (the TypeScript's `Parser`): precedence climbing
//! over or, and, comparisons, + − ||, * / %; unary not and minus; the power
//! `^`; literals, fields, variables, calls and parentheses. Errors say what
//! is wrong and where, as the dialog shows them.
//!
//! Since docs/adr/0100 §4, at the comparisons' level: `x içinde (a, b)`
//! (IN), `x arasında a ve b` (BETWEEN), `x gibi 'A%'` (LIKE), `x benzer
//! 'a%'` (ILIKE), each with `değil` (NOT) before it, and `x boş` / `x boş
//! değil` (IS [NOT] NULL); and as a value `durum eğer c ise v … yoksa e son`
//! (CASE WHEN … THEN … ELSE … END). These words are keywords only where
//! they can be one: a field called Durum, Son or Gibi is still read as a
//! field, and an expression that did not compile before gives the same error.

use super::CompileError;
use super::lexer::{Tok, Token};
use super::library::{Family, Func, FuncDef, Var, find_function, find_variable};
use super::value::Value;
use crate::js::text::fold_turkish;
use crate::world::Aggregate;
use crate::world::roles::{Role, is_inner, roles};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// `^`: the power.
    Pow,
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
    /// `durum eğer c ise v … [yoksa e] son`: the conditions in order and the else.
    Case(Vec<(Node, Node)>, Option<Box<Node>>),
    /// `x [değil] içinde (a, b, …)`: negated when the flag is set.
    In(Box<Node>, Vec<Node>, bool),
    /// `x [değil] arasında a ve b`.
    Between(Box<Node>, Box<Node>, Box<Node>, bool),
    /// `x [değil] gibi p` and `benzer` (case ignored): the pattern, ignoring
    /// case, negated.
    Like(Box<Node>, Box<Node>, bool, bool),
    /// `x boş` / `x boş değil` (IS [NOT] NULL): negated.
    IsNull(Box<Node>, bool),
    /// `@name` as written (docs/adr/0214 §2.3); compiling turns it into its
    /// value (`resolve`), the builder's flow keeps it.
    At(String),
    /// A function that looks at other objects, resolved (`resolve`): its
    /// place in `Expr::world` and its arguments that are the object's own.
    World(u32, Vec<Node>),
}

/// The language's words (in either language); the builder colours and completes them too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyword {
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
    keyword_of(w)
}

/// The keyword a word is, if it is one.
pub(crate) fn keyword_of(w: &str) -> Option<Keyword> {
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

/// Words that are keywords only where one can stand (a field may be called so).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Word {
    Case,
    When,
    Then,
    Else,
    End,
    In,
    Between,
    Like,
    Ilike,
    Is,
}

fn word(tok: &Token) -> Option<Word> {
    let Tok::Word(w) = &tok.t else {
        return None;
    };
    Some(match fold_turkish(w).as_str() {
        "DURUM" | "CASE" => Word::Case,
        "EGER" | "WHEN" => Word::When,
        "ISE" | "THEN" => Word::Then,
        "YOKSA" | "ELSE" => Word::Else,
        "SON" | "END" => Word::End,
        "ICINDE" | "IN" => Word::In,
        "ARASINDA" | "BETWEEN" => Word::Between,
        "GIBI" | "LIKE" => Word::Like,
        "BENZER" | "ILIKE" => Word::Ilike,
        "IS" => Word::Is,
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

/// The level of the comparisons, where `içinde`, `arasında`, `gibi`, `benzer` and `boş` stand.
const COMPARISONS: usize = 2;

pub struct Parser {
    toks: Vec<Token>,
    i: usize,
    /// Attribute names in the order they first appear.
    pub fields: Vec<String>,
    /// Arguments of a function that looks at other objects being read
    /// (docs/adr/0214 §3): one of those may not stand inside another.
    inner: usize,
    /// Leaves the arguments' rules to the compile: the builder's flow reads
    /// expressions with empty inputs (`?`) where a constant text must go.
    pub lenient: bool,
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
            inner: 0,
            lenient: false,
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
        loop {
            if let Some(op) = self
                .op_at()
                .and_then(|o| LEVELS[level].iter().find(|(s, _)| *s == o))
            {
                self.next();
                let b = self.binary(level + 1)?;
                a = Node::Bin(op.1, Box::new(a), Box::new(b));
                continue;
            }
            if level == COMPARISONS {
                let (with, rest) = self.predicate(a)?;
                a = with;
                if rest {
                    continue;
                }
            }
            break;
        }
        Ok(a)
    }

    /// The token after the cursor.
    fn peek2(&self) -> &Token {
        &self.toks[(self.i + 1).min(self.toks.len() - 1)]
    }

    /// `içinde`, `arasında`, `gibi`, `benzer` (each after an optional
    /// `değil`) and `boş [değil]` / `IS [NOT] NULL` after `a`, at the
    /// comparisons' level; whether one was there.
    fn predicate(&mut self, a: Node) -> Result<(Node, bool), CompileError> {
        let tok = self.peek().clone();
        // `x boş`, `x boş değil`; `x IS NULL`, `x IS NOT NULL`.
        if keyword(&tok) == Some(Keyword::Null) {
            self.next();
            let negated = keyword(self.peek()) == Some(Keyword::Not);
            if negated {
                self.next();
            }
            return Ok((Node::IsNull(Box::new(a), negated), true));
        }
        if word(&tok) == Some(Word::Is) {
            self.next();
            let negated = keyword(self.peek()) == Some(Keyword::Not);
            if negated {
                self.next();
            }
            if keyword(self.peek()) != Some(Keyword::Null) {
                return Err(err(
                    "“IS” yalnız NULL ya da NOT NULL ile kullanılır: Ad IS NULL (Türkçesi: Ad boş).",
                    self.peek().at,
                ));
            }
            self.next();
            return Ok((Node::IsNull(Box::new(a), negated), true));
        }
        // `değil` counts only before one of the words it can negate: otherwise
        // the tokens stay, and the expression fails where it did before.
        let negated = keyword(&tok) == Some(Keyword::Not)
            && matches!(
                word(self.peek2()),
                Some(Word::In | Word::Between | Word::Like | Word::Ilike)
            );
        let at = if negated { self.peek2().clone() } else { tok };
        let Some(w @ (Word::In | Word::Between | Word::Like | Word::Ilike)) = word(&at) else {
            return Ok((a, false));
        };
        if negated {
            self.next();
        }
        self.next();
        let a = Box::new(a);
        Ok((
            match w {
                Word::In => Node::In(a, self.list(&at)?, negated),
                Word::Between => {
                    let low = self.binary(COMPARISONS + 1)?;
                    if keyword(self.peek()) != Some(Keyword::And) {
                        return Err(err(
                            format!(
                                "{} iki değer bekler, aralarında “ve” ile: Kat arasında 3 ve 5.",
                                at.describe()
                            ),
                            self.peek().at,
                        ));
                    }
                    self.next();
                    let high = self.binary(COMPARISONS + 1)?;
                    Node::Between(a, Box::new(low), Box::new(high), negated)
                }
                _ => {
                    let pattern = self.binary(COMPARISONS + 1)?;
                    Node::Like(a, Box::new(pattern), w == Word::Ilike, negated)
                }
            },
            true,
        ))
    }

    /// `(a, b, …)` after `içinde`.
    fn list(&mut self, at: &Token) -> Result<Vec<Node>, CompileError> {
        let usage = || {
            format!(
                "{} parantez içinde bir liste bekler: Nitelik içinde ('Arsa', 'Tarla').",
                at.describe()
            )
        };
        if self.peek().t != Tok::Op("(") || self.peek2().t == Tok::Op(")") {
            return Err(err(usage(), self.peek().at));
        }
        self.next();
        let mut items = Vec::new();
        loop {
            items.push(self.binary(0)?);
            if self.peek().t == Tok::Op(",") {
                self.next();
                continue;
            }
            break;
        }
        self.expect_close("Liste kapanmamış: “)” bekleniyordu.")?;
        Ok(items)
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
        self.power()
    }

    /// `a ^ b`: tighter than the sign before it (−2 ^ 2 is −4), and from the
    /// right (2 ^ 3 ^ 2 is 2 ^ 9).
    fn power(&mut self) -> Result<Node, CompileError> {
        let base = self.primary()?;
        if self.peek().t != Tok::Op("^") {
            return Ok(base);
        }
        self.next();
        let exponent = self.unary()?;
        Ok(Node::Bin(BinOp::Pow, Box::new(base), Box::new(exponent)))
    }

    /// `durum eğer c ise v … [yoksa e] son` (the cursor on `durum`).
    fn case(&mut self) -> Result<Node, CompileError> {
        self.next();
        let mut whens = Vec::new();
        while word(self.peek()) == Some(Word::When) {
            self.next();
            let condition = self.binary(0)?;
            if word(self.peek()) != Some(Word::Then) {
                return Err(err(
                    "“ise” bekleniyordu: durumda her koşulun ardından “ise” ve değeri gelir (durum eğer $alan > 500 ise 'büyük' yoksa 'küçük' son).",
                    self.peek().at,
                ));
            }
            self.next();
            let value = self.binary(0)?;
            whens.push((condition, value));
        }
        let otherwise = if word(self.peek()) == Some(Word::Else) {
            self.next();
            Some(Box::new(self.binary(0)?))
        } else {
            None
        };
        if word(self.peek()) != Some(Word::End) {
            return Err(err(
                "“durum” kapanmamış: “son” bekleniyordu (ya da bir “eğer … ise …”, “yoksa …”).",
                self.peek().at,
            ));
        }
        self.next();
        Ok(Node::Case(whens, otherwise))
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
            Tok::At(name) => Ok(Node::At(name.clone())),
            Tok::Word(name) => {
                if self.peek().t == Tok::Op("(") {
                    return self.call(name, tok.at);
                }
                // `durum eğer …`: a case, only when a condition follows (a field may be called Durum).
                if word(&tok) == Some(Word::Case) && word(self.peek()) == Some(Word::When) {
                    self.i -= 1;
                    return self.case();
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
        let roles = roles(f.func);
        if roles.is_some() && self.inner > 0 && !self.lenient {
            return Err(err(
                format!(
                    "{}() başka katmanlara bakar; böyle bir işlevin içindeki ifadede kullanılamaz.",
                    f.name
                ),
                at,
            ));
        }
        self.next(); // (
        let mut args = Vec::new();
        // Where each argument starts, for the messages about it.
        let mut starts = Vec::new();
        if self.peek().t != Tok::Op(")") {
            loop {
                let role = roles.and_then(|r| r.get(args.len())).copied();
                let inner = role.is_some_and(is_inner);
                starts.push(self.peek().at);
                self.inner += usize::from(inner);
                let arg = self.binary(0);
                self.inner -= usize::from(inner);
                args.push(arg?);
                if self.peek().t == Tok::Op(",") {
                    self.next();
                    continue;
                }
                break;
            }
        }
        self.expect_close(&format!("{}(…) kapanmamış: “)” bekleniyordu.", f.name))?;
        check_arity(f, args.len(), at)?;
        if !self.lenient {
            check_args(f, roles, &args, &starts, at)?;
        }
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

/// The rules a call's arguments keep beyond their number (docs/adr/0214):
/// a layer's name and the other constant texts, a constant regular
/// expression that reads, a map's keys and values in pairs.
fn check_args(
    f: &FuncDef,
    roles: Option<&[Role]>,
    args: &[Node],
    starts: &[usize],
    at: usize,
) -> Result<(), CompileError> {
    let text = |i: usize| match args.get(i) {
        Some(Node::Lit(Value::Text(t))) => Some(t.as_ref()),
        _ => None,
    };
    for (i, role) in roles.unwrap_or_default().iter().enumerate() {
        if i >= args.len() || !matches!(role, Role::Layer | Role::Text) {
            continue;
        }
        let place = starts.get(i).copied().unwrap_or(at);
        let Some(t) = text(i) else {
            let what = match (role, f.func) {
                (Role::Layer, _) => "Katman adı",
                (_, Func::Aggregate) => "Toplamanın adı",
                (_, Func::FromLayer) => "Anahtar alanın adı",
                _ => "Ayraç",
            };
            return Err(err(
                format!(
                    "{what} sabit bir metin olmalı (tırnak içinde). Kullanım: {}",
                    f.signature
                ),
                place,
            ));
        };
        if f.func == Func::Aggregate && *role == Role::Text && Aggregate::from_name(t).is_none() {
            return Err(err(
                format!("Bilinmeyen toplama: '{t}'. {} olabilir.", Aggregate::NAMES),
                place,
            ));
        }
    }
    if f.func.family() == Family::Patterns
        && let Some(p) = text(1)
        && let Err(e) = crate::patterns::read(p)
    {
        return Err(err(e, starts.get(1).copied().unwrap_or(at)));
    }
    if f.func == Func::Map && !args.len().is_multiple_of(2) {
        return Err(err(
            format!(
                "eşleme() anahtar ve değer çiftleri alır; {} değer verildi. Kullanım: {}",
                args.len(),
                f.signature
            ),
            at,
        ));
    }
    Ok(())
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
