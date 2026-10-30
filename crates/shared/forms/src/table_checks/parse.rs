//! A table constraint's SQL read into an expression (docs/adr/0755 §3) with each dialect's precedence: SQLite's
//! (`=` below `<`, `||` above `*`) and PostgreSQL's (comparisons below `BETWEEN`, `IN` and `LIKE`, `::` above all).
//! What a constraint cannot hold or this reader does not know (a subquery, a row, an array's element, a function
//! called with keywords) gives none: the constraint is the table's to judge.

use super::Dialect;
use super::lex::{Tok, tokens};

/// How deep an expression may nest before it is left to the table.
const MAX_DEPTH: u32 = 128;

/// A comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// An operator between two values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bin {
    Cmp(Cmp),
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Concat,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

/// A pattern match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Match {
    Like,
    ILike,
    Glob,
    Regex {
        insensitive: bool,
    },
    /// SQLite's `REGEXP` and `MATCH`, PostgreSQL's `SIMILAR TO`: never judged.
    Opaque,
}

/// SQLite's type affinities (docs/adr/0755 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Aff {
    None,
    Text,
    Numeric,
    Integer,
    Real,
    Blob,
}

impl Aff {
    /// The affinity of a declared type, by SQLite's five rules in their order.
    pub(crate) fn of_type(decl: &str) -> Aff {
        let up = decl.to_ascii_uppercase();
        if up.contains("INT") {
            Aff::Integer
        } else if up.contains("CHAR") || up.contains("CLOB") || up.contains("TEXT") {
            Aff::Text
        } else if up.contains("BLOB") || up.trim().is_empty() {
            Aff::Blob
        } else if up.contains("REAL") || up.contains("FLOA") || up.contains("DOUB") {
            Aff::Real
        } else {
            Aff::Numeric
        }
    }

    pub(super) fn numeric(self) -> bool {
        matches!(self, Aff::Numeric | Aff::Integer | Aff::Real)
    }
}

/// A type a value is cast to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Ty {
    Text,
    /// PostgreSQL's `character(n)`, whose trailing blanks do not count.
    BpChar,
    Int,
    Numeric,
    Float,
    Bool,
    Date,
    Time,
    Stamp,
    StampTz,
    Uuid,
    Json,
    Bytes,
    Other,
    Array(Box<Ty>),
    /// SQLite's: a type is its affinity.
    Affinity(Aff),
}

/// A constraint's expression.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Expr {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    /// A name: a column of the table, or in SQLite a string in double quotes no column has.
    Name {
        text: String,
        quoted: bool,
        double: bool,
    },
    /// What is never known before the table answers (`CURRENT_DATE`, a blob).
    Opaque,
    Cast(Box<Expr>, Ty),
    Collate(Box<Expr>, String),
    Neg(Box<Expr>),
    Pos(Box<Expr>),
    BitNot(Box<Expr>),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Bin(Bin, Box<Expr>, Box<Expr>),
    /// `IS NOT DISTINCT FROM` (SQLite's `IS`), `negate`: `IS DISTINCT FROM` (SQLite's `IS NOT`).
    Same {
        a: Box<Expr>,
        b: Box<Expr>,
        negate: bool,
    },
    IsNull {
        e: Box<Expr>,
        not: bool,
    },
    /// `IS [NOT] TRUE | FALSE | UNKNOWN` (`value` none).
    IsBool {
        e: Box<Expr>,
        not: bool,
        value: Option<bool>,
    },
    In {
        e: Box<Expr>,
        not: bool,
        list: Vec<Expr>,
    },
    /// `e op ANY (array)` or `e op ALL (array)`.
    Quantified {
        e: Box<Expr>,
        op: Cmp,
        all: bool,
        array: Box<Expr>,
    },
    Between {
        e: Box<Expr>,
        not: bool,
        symmetric: bool,
        lo: Box<Expr>,
        hi: Box<Expr>,
    },
    Like {
        e: Box<Expr>,
        not: bool,
        how: Match,
        pattern: Box<Expr>,
        escape: Option<Box<Expr>>,
    },
    Case {
        operand: Option<Box<Expr>>,
        whens: Vec<(Expr, Expr)>,
        otherwise: Option<Box<Expr>>,
    },
    /// A function by its name (unquoted ones in lowercase).
    Call(String, Vec<Expr>),
    Array(Vec<Expr>),
}

/// An infix or postfix operator as the parser meets it.
#[derive(Clone, Copy, Debug)]
enum Infix {
    Or,
    And,
    Cmp(Cmp),
    Is,
    IsNull(bool),
    In(bool),
    Between(bool),
    /// A keyword match (`LIKE`, `GLOB` …), `not`.
    Like(Match, bool),
    /// PostgreSQL's operator spelling (`~~`, `~*` …), which binds as its other operators do.
    Sym(Match, bool),
    Bin(Bin),
    Collate,
    Cast,
}

/// How tightly an operator binds, by the dialect's precedence.
fn power(d: Dialect, op: Infix) -> u8 {
    use Dialect::{Postgres, Sqlite};
    match (d, op) {
        (_, Infix::Or) => 10,
        (_, Infix::And) => 20,
        (Postgres, Infix::Is | Infix::IsNull(_)) => 40,
        (Postgres, Infix::Cmp(_)) => 50,
        (Postgres, Infix::In(_) | Infix::Between(_) | Infix::Like(..)) => 60,
        (Postgres, Infix::Sym(..)) => 70,
        (Postgres, Infix::Bin(Bin::Add | Bin::Sub)) => 80,
        (Postgres, Infix::Bin(Bin::Mul | Bin::Div | Bin::Rem)) => 90,
        (Postgres, Infix::Bin(Bin::Pow)) => 100,
        (Postgres, Infix::Bin(_)) => 70,
        (Postgres, Infix::Collate) => 120,
        (Postgres, Infix::Cast) => 130,
        (Sqlite, Infix::Cmp(Cmp::Eq | Cmp::Ne))
        | (Sqlite, Infix::Is | Infix::IsNull(_) | Infix::In(_) | Infix::Between(_))
        | (Sqlite, Infix::Like(..) | Infix::Sym(..)) => 40,
        (Sqlite, Infix::Cmp(_)) => 50,
        (Sqlite, Infix::Bin(Bin::BitAnd | Bin::BitOr | Bin::Shl | Bin::Shr)) => 60,
        (Sqlite, Infix::Bin(Bin::Add | Bin::Sub)) => 70,
        (Sqlite, Infix::Bin(Bin::Mul | Bin::Div | Bin::Rem)) => 80,
        (Sqlite, Infix::Bin(_)) => 90,
        (Sqlite, Infix::Collate | Infix::Cast) => 110,
    }
}

/// How tightly a sign binds its operand.
fn unary_power(d: Dialect) -> u8 {
    match d {
        Dialect::Postgres => 110,
        Dialect::Sqlite => 100,
    }
}

/// `NOT`'s.
const NOT_POWER: u8 = 30;

/// The expression of `sql`; none for a text this does not read whole.
pub(super) fn parse(sql: &str, d: Dialect) -> Option<Expr> {
    let toks = tokens(sql, d)?;
    let mut p = Parser {
        t: &toks,
        i: 0,
        d,
        depth: 0,
    };
    let e = p.expr(0)?;
    (p.i == toks.len()).then_some(e)
}

/// The names `sql` reads, as written, in their first order: from its expression, else (a text this does not read)
/// every name token of it.
pub(super) fn names(sql: &str, d: Dialect) -> Vec<(String, bool, bool)> {
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    match parse(sql, d) {
        Some(e) => walk(&e, &mut |e| {
            if let Expr::Name {
                text,
                quoted,
                double,
            } = e
                && !out.iter().any(|(t, q, _)| t == text && q == quoted)
            {
                out.push((text.clone(), *quoted, *double));
            }
        }),
        None => {
            for t in tokens(sql, d).unwrap_or_default() {
                if let Tok::Name {
                    text,
                    quoted,
                    double,
                } = t
                    && !out.iter().any(|(x, q, _)| *x == text && *q == quoted)
                {
                    out.push((text, quoted, double));
                }
            }
        }
    }
    out
}

/// Every node of `e`, parents first.
pub(super) fn walk(e: &Expr, f: &mut impl FnMut(&Expr)) {
    f(e);
    match e {
        Expr::Null
        | Expr::Bool(_)
        | Expr::Num(_)
        | Expr::Str(_)
        | Expr::Name { .. }
        | Expr::Opaque => {}
        Expr::Cast(a, _)
        | Expr::Collate(a, _)
        | Expr::Neg(a)
        | Expr::Pos(a)
        | Expr::BitNot(a)
        | Expr::Not(a)
        | Expr::IsNull { e: a, .. }
        | Expr::IsBool { e: a, .. } => walk(a, f),
        Expr::And(a, b) | Expr::Or(a, b) | Expr::Bin(_, a, b) | Expr::Same { a, b, .. } => {
            walk(a, f);
            walk(b, f);
        }
        Expr::Quantified { e, array, .. } => {
            walk(e, f);
            walk(array, f);
        }
        Expr::In { e, list, .. } => {
            walk(e, f);
            list.iter().for_each(|x| walk(x, f));
        }
        Expr::Between { e, lo, hi, .. } => {
            walk(e, f);
            walk(lo, f);
            walk(hi, f);
        }
        Expr::Like {
            e, pattern, escape, ..
        } => {
            walk(e, f);
            walk(pattern, f);
            if let Some(x) = escape {
                walk(x, f);
            }
        }
        Expr::Case {
            operand,
            whens,
            otherwise,
        } => {
            if let Some(x) = operand {
                walk(x, f);
            }
            for (a, b) in whens {
                walk(a, f);
                walk(b, f);
            }
            if let Some(x) = otherwise {
                walk(x, f);
            }
        }
        Expr::Call(_, args) | Expr::Array(args) => args.iter().for_each(|x| walk(x, f)),
    }
}

struct Parser<'t> {
    t: &'t [Tok],
    i: usize,
    d: Dialect,
    depth: u32,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.t.get(self.i + k)
    }

    /// Whether the token `k` ahead is the keyword `kw` (an unquoted name, in any case).
    fn is_kw(&self, k: usize, kw: &str) -> bool {
        matches!(self.peek_at(k), Some(Tok::Name { text, quoted: false, .. }) if text.eq_ignore_ascii_case(kw))
    }

    fn is_op(&self, k: usize, op: &str) -> bool {
        matches!(self.peek_at(k), Some(Tok::Op(o)) if *o == op)
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        let yes = self.is_kw(0, kw);
        if yes {
            self.i += 1;
        }
        yes
    }

    fn eat_op(&mut self, op: &str) -> bool {
        let yes = self.is_op(0, op);
        if yes {
            self.i += 1;
        }
        yes
    }

    fn expect_op(&mut self, op: &str) -> Option<()> {
        self.eat_op(op).then_some(())
    }

    fn expr(&mut self, min: u8) -> Option<Expr> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut lhs = self.prefix()?;
        while let Some((op, len)) = self.infix() {
            let power = power(self.d, op);
            if power < min {
                break;
            }
            self.i += len;
            lhs = self.rest(lhs, op, power)?;
        }
        self.depth -= 1;
        Some(lhs)
    }

    /// The operator ahead and how many tokens it takes; none where an expression ends.
    fn infix(&self) -> Option<(Infix, usize)> {
        let pg = self.d == Dialect::Postgres;
        if let Some(Tok::Op(o)) = self.peek() {
            let op = match *o {
                "::" if pg => Infix::Cast,
                "=" => Infix::Cmp(Cmp::Eq),
                "==" if !pg => Infix::Cmp(Cmp::Eq),
                "<>" | "!=" => Infix::Cmp(Cmp::Ne),
                "<" => Infix::Cmp(Cmp::Lt),
                "<=" => Infix::Cmp(Cmp::Le),
                ">" => Infix::Cmp(Cmp::Gt),
                ">=" => Infix::Cmp(Cmp::Ge),
                "||" => Infix::Bin(Bin::Concat),
                "+" => Infix::Bin(Bin::Add),
                "-" => Infix::Bin(Bin::Sub),
                "*" => Infix::Bin(Bin::Mul),
                "/" => Infix::Bin(Bin::Div),
                "%" => Infix::Bin(Bin::Rem),
                "^" if pg => Infix::Bin(Bin::Pow),
                "&" => Infix::Bin(Bin::BitAnd),
                "|" => Infix::Bin(Bin::BitOr),
                "<<" => Infix::Bin(Bin::Shl),
                ">>" => Infix::Bin(Bin::Shr),
                "~~" if pg => Infix::Sym(Match::Like, false),
                "!~~" if pg => Infix::Sym(Match::Like, true),
                "~~*" if pg => Infix::Sym(Match::ILike, false),
                "!~~*" if pg => Infix::Sym(Match::ILike, true),
                "~" if pg => Infix::Sym(Match::Regex { insensitive: false }, false),
                "~*" if pg => Infix::Sym(Match::Regex { insensitive: true }, false),
                "!~" if pg => Infix::Sym(Match::Regex { insensitive: false }, true),
                "!~*" if pg => Infix::Sym(Match::Regex { insensitive: true }, true),
                _ => return None,
            };
            return Some((op, 1));
        }
        let kw = |k: &str| self.is_kw(0, k);
        if kw("OR") {
            return Some((Infix::Or, 1));
        }
        if kw("AND") {
            return Some((Infix::And, 1));
        }
        if kw("COLLATE") {
            return Some((Infix::Collate, 1));
        }
        if kw("IS") {
            return Some((Infix::Is, 1));
        }
        if kw("ISNULL") {
            return Some((Infix::IsNull(false), 1));
        }
        if kw("NOTNULL") {
            return Some((Infix::IsNull(true), 1));
        }
        let (not, at) = if kw("NOT") { (true, 1) } else { (false, 0) };
        let next = |k: &str| self.is_kw(at, k);
        let found = if not && !pg && next("NULL") {
            Infix::IsNull(true)
        } else if next("IN") {
            Infix::In(not)
        } else if next("BETWEEN") {
            Infix::Between(not)
        } else if next("LIKE") {
            Infix::Like(Match::Like, not)
        } else if pg && next("ILIKE") {
            Infix::Like(Match::ILike, not)
        } else if !pg && next("GLOB") {
            Infix::Like(Match::Glob, not)
        } else if !pg && (next("REGEXP") || next("MATCH")) {
            Infix::Like(Match::Opaque, not)
        } else if pg && next("SIMILAR") && self.is_kw(at + 1, "TO") {
            return Some((Infix::Like(Match::Opaque, not), at + 2));
        } else {
            return None;
        };
        Some((found, at + 1))
    }

    /// The rest of an expression after its operator.
    fn rest(&mut self, lhs: Expr, op: Infix, power: u8) -> Option<Expr> {
        let b = Box::new;
        Some(match op {
            Infix::Cast => Expr::Cast(b(lhs), self.ty()?),
            Infix::Collate => {
                let name = match self.peek()? {
                    Tok::Name { text, .. } | Tok::Str(text) => text.clone(),
                    _ => return None,
                };
                self.i += 1;
                Expr::Collate(b(lhs), name)
            }
            Infix::Or => Expr::Or(b(lhs), b(self.expr(power + 1)?)),
            Infix::And => Expr::And(b(lhs), b(self.expr(power + 1)?)),
            Infix::Cmp(c) => {
                let all = self.is_kw(0, "ALL");
                if self.d == Dialect::Postgres
                    && (all || self.is_kw(0, "ANY") || self.is_kw(0, "SOME"))
                    && self.is_op(1, "(")
                {
                    self.i += 2;
                    let array = self.expr(0)?;
                    self.expect_op(")")?;
                    Expr::Quantified {
                        e: b(lhs),
                        op: c,
                        all,
                        array: b(array),
                    }
                } else {
                    Expr::Bin(Bin::Cmp(c), b(lhs), b(self.expr(power + 1)?))
                }
            }
            Infix::Is => {
                let not = self.eat_kw("NOT");
                if self.eat_kw("NULL") {
                    Expr::IsNull { e: b(lhs), not }
                } else if self.eat_kw("TRUE") {
                    Expr::IsBool {
                        e: b(lhs),
                        not,
                        value: Some(true),
                    }
                } else if self.eat_kw("FALSE") {
                    Expr::IsBool {
                        e: b(lhs),
                        not,
                        value: Some(false),
                    }
                } else if self.eat_kw("UNKNOWN") {
                    Expr::IsBool {
                        e: b(lhs),
                        not,
                        value: None,
                    }
                } else if self.eat_kw("DISTINCT") {
                    if !self.eat_kw("FROM") {
                        return None;
                    }
                    Expr::Same {
                        a: b(lhs),
                        b: b(self.expr(power + 1)?),
                        negate: !not,
                    }
                } else if self.d == Dialect::Sqlite {
                    Expr::Same {
                        a: b(lhs),
                        b: b(self.expr(power + 1)?),
                        negate: not,
                    }
                } else {
                    return None;
                }
            }
            Infix::IsNull(not) => Expr::IsNull { e: b(lhs), not },
            Infix::In(not) => {
                self.expect_op("(")?;
                if ["SELECT", "VALUES", "WITH"]
                    .iter()
                    .any(|k| self.is_kw(0, k))
                {
                    return None;
                }
                let list = self.list(")")?;
                if list.is_empty() && self.d == Dialect::Postgres {
                    return None;
                }
                Expr::In {
                    e: b(lhs),
                    not,
                    list,
                }
            }
            Infix::Between(not) => {
                let symmetric = self.d == Dialect::Postgres && self.eat_kw("SYMMETRIC");
                if self.d == Dialect::Postgres && !symmetric {
                    self.eat_kw("ASYMMETRIC");
                }
                let lo = self.expr(power + 1)?;
                if !self.eat_kw("AND") {
                    return None;
                }
                let hi = self.expr(power + 1)?;
                Expr::Between {
                    e: b(lhs),
                    not,
                    symmetric,
                    lo: b(lo),
                    hi: b(hi),
                }
            }
            Infix::Like(how, not) | Infix::Sym(how, not) => {
                let pattern = self.expr(power + 1)?;
                let escape = if self.eat_kw("ESCAPE") {
                    Some(b(self.expr(power + 1)?))
                } else {
                    None
                };
                Expr::Like {
                    e: b(lhs),
                    not,
                    how,
                    pattern: b(pattern),
                    escape,
                }
            }
            Infix::Bin(op) => Expr::Bin(op, b(lhs), b(self.expr(power + 1)?)),
        })
    }

    /// Expressions separated by commas up to `close` (consumed); none may be empty between commas.
    fn list(&mut self, close: &str) -> Option<Vec<Expr>> {
        let mut out = Vec::new();
        if self.eat_op(close) {
            return Some(out);
        }
        loop {
            out.push(self.expr(0)?);
            if self.eat_op(close) {
                return Some(out);
            }
            self.expect_op(",")?;
        }
    }

    fn prefix(&mut self) -> Option<Expr> {
        let tok = self.peek()?.clone();
        self.i += 1;
        let b = Box::new;
        Some(match tok {
            Tok::Op("(") => {
                if ["SELECT", "VALUES", "WITH"]
                    .iter()
                    .any(|k| self.is_kw(0, k))
                {
                    return None;
                }
                let e = self.expr(0)?;
                self.expect_op(")")?;
                e
            }
            Tok::Op("-") => Expr::Neg(b(self.expr(unary_power(self.d))?)),
            Tok::Op("+") => Expr::Pos(b(self.expr(unary_power(self.d))?)),
            Tok::Op("~") => Expr::BitNot(b(self.expr(unary_power(self.d))?)),
            Tok::Op(_) => return None,
            Tok::Num(n) => Expr::Num(n),
            Tok::Str(s) => Expr::Str(s),
            Tok::Blob => Expr::Opaque,
            Tok::Name {
                text,
                quoted,
                double,
            } => {
                if !quoted && let Some(e) = self.keyword(&text)? {
                    return Some(e);
                }
                self.named(text, quoted, double)?
            }
        })
    }

    /// An expression a keyword begins (`Some(Some(e))`); `Some(None)`: the word is a name; none: a construct this does
    /// not read.
    fn keyword(&mut self, word: &str) -> Option<Option<Expr>> {
        let up = word.to_ascii_uppercase();
        let b = Box::new;
        Some(Some(match up.as_str() {
            "NULL" => Expr::Null,
            "TRUE" => Expr::Bool(true),
            "FALSE" => Expr::Bool(false),
            "NOT" => Expr::Not(b(self.expr(NOT_POWER)?)),
            "CASE" => self.case()?,
            "CAST" => {
                self.expect_op("(")?;
                let e = self.expr(0)?;
                if !self.eat_kw("AS") {
                    return None;
                }
                let ty = self.ty()?;
                self.expect_op(")")?;
                Expr::Cast(b(e), ty)
            }
            "ARRAY" if self.d == Dialect::Postgres => {
                self.expect_op("[")?;
                Expr::Array(self.list("]")?)
            }
            "EXISTS" | "SELECT" | "ARRAY" | "ROW" | "INTERVAL" => return None,
            "CURRENT_DATE" | "CURRENT_TIME" | "CURRENT_TIMESTAMP" | "LOCALTIME"
            | "LOCALTIMESTAMP" | "CURRENT_USER" | "SESSION_USER" | "USER" | "CURRENT_ROLE"
            | "CURRENT_CATALOG" | "CURRENT_SCHEMA" => {
                if self.is_op(0, "(") && self.d == Dialect::Postgres {
                    self.i += 1;
                    let _ = self.list(")")?;
                }
                Expr::Opaque
            }
            "DATE" | "TIME" | "TIMESTAMP"
                if self.d == Dialect::Postgres && matches!(self.peek(), Some(Tok::Str(_))) =>
            {
                let Some(Tok::Str(s)) = self.peek().cloned() else {
                    return None;
                };
                self.i += 1;
                let ty = match up.as_str() {
                    "DATE" => Ty::Date,
                    "TIME" => Ty::Time,
                    _ => Ty::Stamp,
                };
                Expr::Cast(b(Expr::Str(s)), ty)
            }
            _ => return Some(None),
        }))
    }

    /// A name: a column (`a`, `t.a`), or a function called (`lower(a)`, `pg_catalog.lower(a)`).
    fn named(&mut self, mut text: String, mut quoted: bool, mut double: bool) -> Option<Expr> {
        while self.is_op(0, ".") {
            let Some(Tok::Name {
                text: t,
                quoted: q,
                double: d,
            }) = self.peek_at(1).cloned()
            else {
                return None;
            };
            self.i += 2;
            (text, quoted, double) = (t, q, d);
        }
        if !self.eat_op("(") {
            return Some(Expr::Name {
                text,
                quoted,
                double,
            });
        }
        if self.is_op(0, "*") || self.is_kw(0, "DISTINCT") || self.is_kw(0, "ALL") {
            return None;
        }
        let args = self.list(")")?;
        if ["OVER", "FILTER", "WITHIN"]
            .iter()
            .any(|k| self.is_kw(0, k))
        {
            return None;
        }
        let name = if quoted {
            text
        } else {
            text.to_ascii_lowercase()
        };
        Some(Expr::Call(name, args))
    }

    fn case(&mut self) -> Option<Expr> {
        let operand = if self.is_kw(0, "WHEN") {
            None
        } else {
            Some(Box::new(self.expr(0)?))
        };
        let mut whens = Vec::new();
        while self.eat_kw("WHEN") {
            let w = self.expr(0)?;
            if !self.eat_kw("THEN") {
                return None;
            }
            whens.push((w, self.expr(0)?));
        }
        if whens.is_empty() {
            return None;
        }
        let otherwise = if self.eat_kw("ELSE") {
            Some(Box::new(self.expr(0)?))
        } else {
            None
        };
        if !self.eat_kw("END") {
            return None;
        }
        Some(Expr::Case {
            operand,
            whens,
            otherwise,
        })
    }

    /// A type's name: PostgreSQL's after `::` or `AS` (`character varying`, `timestamp without time zone`,
    /// `numeric(10,2)`, `text[]`), SQLite's in `CAST` (its affinity).
    fn ty(&mut self) -> Option<Ty> {
        let mut words: Vec<String> = Vec::new();
        let word = |p: &Self, k: usize| match p.peek_at(k) {
            Some(Tok::Name { text, .. }) => Some(text.to_ascii_lowercase()),
            _ => None,
        };
        if self.d == Dialect::Sqlite {
            while let Some(w) = word(self, 0) {
                words.push(w);
                self.i += 1;
            }
            if words.is_empty() {
                return None;
            }
            self.size()?;
            return Some(Ty::Affinity(Aff::of_type(&words.join(" "))));
        }
        let mut first = word(self, 0)?;
        self.i += 1;
        while self.is_op(0, ".") {
            first = word(self, 1)?;
            self.i += 2;
        }
        words.push(first.clone());
        let follow: &[&str] = match first.as_str() {
            "character" | "char" | "bit" | "national" => &["varying"],
            "double" => &["precision"],
            _ => &[],
        };
        if let Some(w) = word(self, 0)
            && follow.contains(&w.as_str())
        {
            words.push(w);
            self.i += 1;
        }
        if matches!(first.as_str(), "time" | "timestamp") {
            self.size()?;
            if self.is_kw(0, "WITH") || self.is_kw(0, "WITHOUT") {
                if !(self.is_kw(1, "TIME") && self.is_kw(2, "ZONE")) {
                    return None;
                }
                words.push(word(self, 0)?);
                self.i += 3;
            }
        }
        self.size()?;
        let mut ty = match words.join(" ").as_str() {
            "text" | "varchar" | "character varying" | "char varying" | "name" => Ty::Text,
            "character" | "char" | "bpchar" | "nchar" => Ty::BpChar,
            "integer" | "int" | "int4" | "smallint" | "int2" | "bigint" | "int8" => Ty::Int,
            "numeric" | "decimal" => Ty::Numeric,
            "real" | "float4" | "double precision" | "float8" | "float" => Ty::Float,
            "boolean" | "bool" => Ty::Bool,
            "date" => Ty::Date,
            "time" | "time without" => Ty::Time,
            "timestamp" | "timestamp without" => Ty::Stamp,
            "timestamp with" | "timestamptz" => Ty::StampTz,
            "uuid" => Ty::Uuid,
            "json" | "jsonb" => Ty::Json,
            "bytea" => Ty::Bytes,
            _ => Ty::Other,
        };
        while self.eat_op("[") {
            if let Some(Tok::Num(_)) = self.peek() {
                self.i += 1;
            }
            self.expect_op("]")?;
            ty = Ty::Array(Box::new(ty));
        }
        Some(ty)
    }

    /// A type's size in parentheses (`(10)`, `(10,2)`), when it has one.
    fn size(&mut self) -> Option<()> {
        if !self.eat_op("(") {
            return Some(());
        }
        loop {
            match self.peek()? {
                Tok::Num(_) => self.i += 1,
                Tok::Op("-" | "+") => self.i += 1,
                _ => return None,
            }
            if self.eat_op(")") {
                return Some(());
            }
            self.expect_op(",")?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pg(sql: &str) -> Expr {
        parse(sql, Dialect::Postgres).unwrap_or_else(|| panic!("{sql}"))
    }

    fn lite(sql: &str) -> Expr {
        parse(sql, Dialect::Sqlite).unwrap_or_else(|| panic!("{sql}"))
    }

    fn name(t: &str) -> Box<Expr> {
        Box::new(Expr::Name {
            text: t.into(),
            quoted: false,
            double: false,
        })
    }

    #[test]
    fn postgresql_writes_its_constraints_so() {
        let e = pg(
            "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying])::text[]))",
        );
        let Expr::Quantified { e, op, all, array } = e else {
            panic!("{e:?}");
        };
        assert_eq!((op, all), (Cmp::Eq, false));
        assert_eq!(*e, Expr::Cast(name("tur"), Ty::Text));
        let Expr::Cast(inner, Ty::Array(of)) = *array else {
            panic!("{array:?}");
        };
        assert_eq!(*of, Ty::Text);
        assert!(matches!(*inner, Expr::Array(ref v) if v.len() == 2));
        assert!(matches!(
            pg("((oncelik >= 1) AND (oncelik <= 3))"),
            Expr::And(..)
        ));
        assert!(matches!(
            pg("((bitis IS NULL) OR (bitis > baslangic))"),
            Expr::Or(..)
        ));
        assert!(matches!(
            pg("((kod)::text ~ '^[0-9]{2}$'::text)"),
            Expr::Like {
                how: Match::Regex { insensitive: false },
                not: false,
                ..
            }
        ));
        assert!(matches!(
            pg("(length(btrim(gerekce)) >= 3)"),
            Expr::Bin(Bin::Cmp(Cmp::Ge), ..)
        ));
        assert!(matches!(
            pg("(d > '2020-01-01'::date)"),
            Expr::Bin(Bin::Cmp(Cmp::Gt), _, ref r) if **r == Expr::Cast(Box::new(Expr::Str("2020-01-01".into())), Ty::Date)
        ));
        assert!(matches!(
            pg("(t <= '2026-01-01 00:00:00'::timestamp without time zone)"),
            Expr::Bin(_, _, ref r) if matches!(**r, Expr::Cast(_, Ty::Stamp))
        ));
        assert!(matches!(
            pg("(x <> ALL ('{a,b}'::text[]))"),
            Expr::Quantified {
                all: true,
                op: Cmp::Ne,
                ..
            }
        ));
        assert!(matches!(pg("(jsonb_array_length(x) > 0)"), Expr::Bin(..)));
        assert!(matches!(pg("(a::numeric(10,2) > 0)"), Expr::Bin(..)));
    }

    #[test]
    fn sqlite_s_precedence() {
        // `=` binds below `<` in SQLite: a = (b < c).
        let Expr::Bin(Bin::Cmp(Cmp::Eq), a, r) = lite("a = b < c") else {
            panic!();
        };
        assert_eq!(a, name("a"));
        assert!(matches!(*r, Expr::Bin(Bin::Cmp(Cmp::Lt), ..)));
        // `||` binds above `*`.
        assert!(
            matches!(lite("a * b || c"), Expr::Bin(Bin::Mul, _, ref r) if matches!(**r, Expr::Bin(Bin::Concat, ..)))
        );
        assert!(matches!(lite("x NOT NULL"), Expr::IsNull { not: true, .. }));
        assert!(matches!(
            lite("x IS NOT ''"),
            Expr::Same { negate: true, .. }
        ));
        assert!(matches!(
            lite("x BETWEEN 1 AND 3 AND y > 0"),
            Expr::And(ref l, _) if matches!(**l, Expr::Between { .. })
        ));
        assert!(
            matches!(lite("tur NOT IN ('a', \"b\")"), Expr::In { not: true, ref list, .. } if list.len() == 2)
        );
        assert!(matches!(
            lite("kod GLOB '[0-9]*'"),
            Expr::Like {
                how: Match::Glob,
                ..
            }
        ));
        assert!(
            matches!(lite("CAST(x AS VARCHAR(10)) = 'a'"), Expr::Bin(_, ref l, _) if matches!(**l, Expr::Cast(_, Ty::Affinity(Aff::Text))))
        );
        assert!(
            matches!(lite("typeof(x) = 'integer'"), Expr::Bin(_, ref l, _) if matches!(**l, Expr::Call(ref n, _) if n == "typeof"))
        );
        assert!(matches!(lite("NOT x = 1"), Expr::Not(_)));
        assert!(matches!(
            lite("CASE WHEN a THEN 1 ELSE 0 END = 1"),
            Expr::Bin(..)
        ));
        assert!(matches!(
            lite("CASE a WHEN 1 THEN 1 END"),
            Expr::Case {
                operand: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn what_a_check_does_not_hold() {
        for sql in [
            "x IN (SELECT a FROM t)",
            "EXISTS (SELECT 1)",
            "count(*) > 0",
            "(a, b) = (1, 2)",
            "x = ",
            "a ++",
            "x IN t",
        ] {
            assert_eq!(parse(sql, Dialect::Sqlite), None, "{sql}");
        }
        assert_eq!(parse("arr[1] = 0", Dialect::Postgres), None);
        assert_eq!(parse("x IS JSON", Dialect::Postgres), None);
        let deep = format!("{}1{}", "(".repeat(200), ")".repeat(200));
        assert_eq!(parse(&deep, Dialect::Postgres), None);
    }

    #[test]
    fn the_names_it_reads() {
        let n = names(
            "bitis > baslangic AND \"bitis\" IS NOT NULL",
            Dialect::Postgres,
        );
        assert_eq!(
            n.iter()
                .map(|(t, q, _)| (t.as_str(), *q))
                .collect::<Vec<_>>(),
            vec![("bitis", false), ("baslangic", false), ("bitis", true)]
        );
        // A text it does not read: every name token.
        let n = names("x IN (SELECT a FROM t)", Dialect::Sqlite);
        assert!(n.iter().any(|(t, _, _)| t == "x") && n.iter().any(|(t, _, _)| t == "a"));
    }

    #[test]
    fn sqlite_s_affinities() {
        for (decl, aff) in [
            ("INTEGER", Aff::Integer),
            ("MEDIUMINT", Aff::Integer),
            ("POINT", Aff::Integer),
            ("VARCHAR(10)", Aff::Text),
            ("TEXT", Aff::Text),
            ("BLOB", Aff::Blob),
            ("", Aff::Blob),
            ("DOUBLE", Aff::Real),
            ("FLOAT", Aff::Real),
            ("REAL", Aff::Real),
            ("BOOLEAN", Aff::Numeric),
            ("DATE", Aff::Numeric),
            ("DATETIME", Aff::Numeric),
            ("STRING", Aff::Numeric),
            ("NUMERIC(10,2)", Aff::Numeric),
        ] {
            assert_eq!(Aff::of_type(decl), aff, "{decl}");
        }
    }
}
