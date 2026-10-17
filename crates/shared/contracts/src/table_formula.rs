//! A table's formulas (docs/adr/0327 §2): a cell whose words begin with `=` is worked out from the table's other cells,
//! as a spreadsheet's: numbers, references (`A1`, `$B$2`) and ranges (`A1:C3`), `+ - * / ^`, parentheses and seven
//! functions, each by its English and Turkish name (SUM TOPLA, AVERAGE ORTALAMA, MIN, MAX MAK, COUNT SAY, ROUND
//! YUVARLA, ABS MUTLAK; arguments apart by `,` or `;`). Its result is the cell's words (the formula's cache); a formula
//! that cannot be worked out writes its error: `#AD?` (not read), `#BAŞV!` (a reference outside the table or deleted),
//! `#DEĞER!` (words in arithmetic), `#SAYI/0!` (division by zero, an average of nothing), `#SAYI!` (no finite result),
//! `#DÖNGÜ!` (formulas that read each other in a ring).
//!
//! A cell's number is its words read as one (`-12.5`, `3e4`; a point before the decimals); empty is nothing, words
//! that are no number are words. Arithmetic takes nothing as 0; the functions skip nothing and words in a range or a
//! reference, as a spreadsheet's do. A result is written in its shortest form (no exponent), or with the cell
//! style's decimals, rounded half away from zero from that shortest form (ROUND alike). The rows and columns
//! inserted or deleted move a formula's references with them (`shift`).

use std::collections::HashMap;

use crate::table::TableEntity;

/// The errors a formula writes.
pub const NOT_READ: &str = "#AD?";
pub const BAD_REF: &str = "#BAŞV!";
pub const NOT_NUMBER: &str = "#DEĞER!";
pub const DIV_ZERO: &str = "#SAYI/0!";
pub const NOT_FINITE: &str = "#SAYI!";
pub const RING: &str = "#DÖNGÜ!";

/// The most decimals ROUND and a cell style take.
pub const MAX_DECIMALS: i32 = 10;

/// A cell's value as formulas read it.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Number(f64),
    Text(String),
    Error(&'static str),
}

/// A column's letters: 0 is `A`, 25 `Z`, 26 `AA`.
pub fn column_letters(mut j: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (j % 26) as u8);
        if j < 26 {
            break;
        }
        j = j / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A column's index from its letters (`A` 0); none past `ZZ` or for anything else.
fn column_index(letters: &str) -> Option<usize> {
    if letters.is_empty() || letters.len() > 2 {
        return None;
    }
    let mut j = 0usize;
    for b in letters.bytes() {
        let d = (b.to_ascii_uppercase()).checked_sub(b'A')?;
        if d >= 26 {
            return None;
        }
        j = j * 26 + usize::from(d) + 1;
    }
    Some(j - 1)
}

/// A cell's number: its words, trimmed, as `[+-]digits[.digits][e[+-]digits]` (or `.digits`).
pub fn number_of(words: &str) -> Option<f64> {
    let s = words.trim();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits = |i: &mut usize| {
        let from = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - from
    };
    let whole = digits(&mut i);
    let mut frac = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        frac = digits(&mut i);
    }
    if whole == 0 && frac == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    s.parse::<f64>().ok().filter(|x| x.is_finite())
}

/// `x` in its shortest decimal form, without an exponent (`0.1`, `1200`, `-3.5`); 0 is `0`.
pub fn shortest(x: f64) -> String {
    if x == 0.0 {
        return "0".to_owned();
    }
    format!("{x}")
}

/// The significant digits a result is written with, as spreadsheets show a number: 0.1 + 0.2 is `0.3`, not the
/// double's `0.30000000000000004`.
pub const SIGNIFICANT: i32 = 15;

/// `x` as a result is written: its shortest decimal form rounded half away from zero to [`SIGNIFICANT`] digits, no
/// trailing zeros, no exponent.
pub fn general(x: f64) -> String {
    let s = shortest(x);
    let body = s.strip_prefix('-').unwrap_or(&s);
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let digits: Vec<u8> = int.bytes().chain(frac.bytes()).collect();
    let Some(first) = digits.iter().position(|&c| c != b'0') else {
        return s;
    };
    let last = digits.iter().rposition(|&c| c != b'0').unwrap_or(first);
    if ((last - first) as i32) < SIGNIFICANT {
        return s;
    }
    // Decimals that keep SIGNIFICANT digits from the first non-zero one.
    let d = SIGNIFICANT - (int.len() as i32 - first as i32);
    let out = round_words(&s, d);
    if out.contains('.') {
        out.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        out
    }
}

/// `x` with `d` decimals (a negative `d` rounds to tens, hundreds …), rounded half away from zero from its written
/// form ([`general`]).
pub fn rounded_text(x: f64, d: i32) -> String {
    round_words(&general(x), d)
}

/// A number's plain decimal words `s` with `d` decimals, rounded half away from zero.
fn round_words(s: &str, d: i32) -> String {
    let s = s.to_owned();
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest.to_owned()),
        None => (false, s),
    };
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_owned(), f.to_owned()),
        None => (s.clone(), String::new()),
    };
    // All its digits and where the point stands among them.
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|c| c - b'0').collect();
    let point = int.len() as i64;
    // Keep `point + d` digits; the next decides.
    let keep = point + i64::from(d);
    if keep < 0 {
        return "0".to_owned() + &zeros_after(d);
    }
    let keep = keep as usize;
    if digits.len() > keep {
        let up = digits[keep] >= 5;
        digits.truncate(keep);
        if up {
            let mut k = digits.len();
            loop {
                if k == 0 {
                    digits.insert(0, 1);
                    break;
                }
                k -= 1;
                if digits[k] == 9 {
                    digits[k] = 0;
                } else {
                    digits[k] += 1;
                    break;
                }
            }
        }
    }
    let grown = digits.len() as i64 - keep as i64; // 1 when a carry put a digit in front
    let point = (point + grown.max(0)) as usize;
    while digits.len() < point {
        digits.push(0);
    }
    let whole: String = if point == 0 {
        "0".to_owned()
    } else {
        digits[..point].iter().map(|d| char::from(b'0' + d)).collect()
    };
    let whole = {
        let t = whole.trim_start_matches('0');
        if t.is_empty() { "0".to_owned() } else { t.to_owned() }
    };
    let mut out = whole;
    if d > 0 {
        let mut f: String = digits
            .get(point..)
            .unwrap_or_default()
            .iter()
            .map(|d| char::from(b'0' + d))
            .collect();
        while f.len() < d as usize {
            f.push('0');
        }
        out.push('.');
        out.push_str(&f);
    }
    let zero = out.bytes().all(|c| c == b'0' || c == b'.');
    if neg && !zero {
        out.insert(0, '-');
    }
    out
}

fn zeros_after(d: i32) -> String {
    if d > 0 {
        format!(".{}", "0".repeat(d as usize))
    } else {
        String::new()
    }
}

/// A value as a cell's words: `decimals` for a number when given.
pub fn written(v: &Value, decimals: Option<u32>) -> String {
    match v {
        Value::Empty => match decimals {
            Some(d) => rounded_text(0.0, d as i32),
            None => "0".to_owned(),
        },
        Value::Number(x) if !x.is_finite() => NOT_FINITE.to_owned(),
        Value::Number(x) => match decimals {
            Some(d) => rounded_text(*x, d as i32),
            None => general(*x),
        },
        Value::Text(t) => t.clone(),
        Value::Error(e) => (*e).to_owned(),
    }
}

// ── Reading ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    /// A cell: row, column.
    Ref(usize, usize),
    /// A range: top left and bottom right.
    Range(usize, usize, usize, usize),
    /// A deleted reference, written `#BAŞV!`.
    Bad,
    Name(String),
    Op(char),
    Open,
    Close,
    Sep,
}

/// A reference's letters and row (`$` signs aside) at the start of `s`: its length in bytes, column and row.
fn reference_at(s: &str) -> Option<(usize, usize, usize)> {
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i] == b'$' {
        i += 1;
    }
    let from = i;
    while i < b.len() && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    let letters = &s[from..i];
    if i < b.len() && b[i] == b'$' {
        i += 1;
    }
    let digits_from = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if digits_from == i || b.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
        return None;
    }
    let col = column_index(letters)?;
    let row: usize = s[digits_from..i].parse().ok()?;
    if row == 0 || row > 1_000_000 {
        return None;
    }
    Some((i, col, row - 1))
}

fn lex(src: &str) -> Option<Vec<Tok>> {
    let mut out = Vec::new();
    let mut rest = src;
    loop {
        rest = rest.trim_start();
        let Some(c) = rest.chars().next() else {
            return Some(out);
        };
        if let Some(after) = rest.strip_prefix(BAD_REF) {
            out.push(Tok::Bad);
            rest = after;
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' | '^' => {
                out.push(Tok::Op(c));
                rest = &rest[1..];
            }
            '(' => {
                out.push(Tok::Open);
                rest = &rest[1..];
            }
            ')' => {
                out.push(Tok::Close);
                rest = &rest[1..];
            }
            ',' | ';' => {
                out.push(Tok::Sep);
                rest = &rest[1..];
            }
            '0'..='9' | '.' => {
                let b = rest.as_bytes();
                let mut i = 0;
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                    let mut k = i + 1;
                    if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
                        k += 1;
                    }
                    let from = k;
                    while k < b.len() && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > from {
                        i = k;
                    }
                }
                out.push(Tok::Num(number_of(&rest[..i])?));
                rest = &rest[i..];
            }
            _ if c == '$' || c.is_ascii_alphabetic() => {
                if let Some((n, col, row)) = reference_at(rest) {
                    rest = &rest[n..];
                    let after = rest.trim_start();
                    if let Some(next) = after.strip_prefix(':') {
                        let next = next.trim_start();
                        let (m, col2, row2) = reference_at(next)?;
                        out.push(Tok::Range(
                            row.min(row2),
                            col.min(col2),
                            row.max(row2),
                            col.max(col2),
                        ));
                        rest = &next[m..];
                    } else {
                        out.push(Tok::Ref(row, col));
                    }
                } else {
                    let n = rest
                        .char_indices()
                        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                        .map_or(rest.len(), |(i, _)| i);
                    if n == 0 {
                        return None;
                    }
                    out.push(Tok::Name(fold(&rest[..n])));
                    rest = &rest[n..];
                }
            }
            _ if c.is_alphabetic() => {
                let n = rest
                    .char_indices()
                    .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                    .map_or(rest.len(), |(i, _)| i);
                out.push(Tok::Name(fold(&rest[..n])));
                rest = &rest[n..];
            }
            _ => return None,
        }
    }
}

/// A function's name in capitals, Turkish dotted and dotless i alike (`mİn`, `min` → `MIN`).
fn fold(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'i' | 'ı' | 'İ' => 'I',
            _ => c.to_ascii_uppercase(),
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Func {
    Sum,
    Average,
    Min,
    Max,
    Count,
    Round,
    Abs,
}

fn func(name: &str) -> Option<Func> {
    Some(match name {
        "SUM" | "TOPLA" => Func::Sum,
        "AVERAGE" | "ORTALAMA" => Func::Average,
        "MIN" => Func::Min,
        "MAX" | "MAK" => Func::Max,
        "COUNT" | "SAY" => Func::Count,
        "ROUND" | "YUVARLA" => Func::Round,
        "ABS" | "MUTLAK" => Func::Abs,
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Num(f64),
    Ref(usize, usize),
    Range(usize, usize, usize, usize),
    Bad,
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        self.at += 1;
        t
    }

    fn sum(&mut self) -> Option<Expr> {
        let mut e = self.product()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek().cloned() {
            self.at += 1;
            e = Expr::Bin(c, Box::new(e), Box::new(self.product()?));
        }
        Some(e)
    }

    fn product(&mut self) -> Option<Expr> {
        let mut e = self.power()?;
        while let Some(Tok::Op(c @ ('*' | '/'))) = self.peek().cloned() {
            self.at += 1;
            e = Expr::Bin(c, Box::new(e), Box::new(self.power()?));
        }
        Some(e)
    }

    // `^` from the left, a sign before it (`-2^2` is 4), as a spreadsheet's.
    fn power(&mut self) -> Option<Expr> {
        let mut e = self.unary()?;
        while let Some(Tok::Op('^')) = self.peek() {
            self.at += 1;
            e = Expr::Bin('^', Box::new(e), Box::new(self.unary()?));
        }
        Some(e)
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.at += 1;
                Some(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op('+')) => {
                self.at += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.next()? {
            Tok::Num(x) => Some(Expr::Num(x)),
            Tok::Ref(r, c) => Some(Expr::Ref(r, c)),
            Tok::Range(..) => None,
            Tok::Bad => Some(Expr::Bad),
            Tok::Open => {
                let e = self.sum()?;
                (self.next()? == Tok::Close).then_some(e)
            }
            Tok::Name(n) => {
                let f = func(&n)?;
                if self.next()? != Tok::Open {
                    return None;
                }
                let mut args = Vec::new();
                if self.peek() == Some(&Tok::Close) {
                    self.at += 1;
                } else {
                    loop {
                        let arg = match self.peek().cloned() {
                            Some(Tok::Range(a, b, c, d))
                                if matches!(
                                    self.toks.get(self.at + 1),
                                    Some(Tok::Sep | Tok::Close)
                                ) =>
                            {
                                self.at += 1;
                                Expr::Range(a, b, c, d)
                            }
                            _ => self.sum()?,
                        };
                        args.push(arg);
                        match self.next()? {
                            Tok::Sep => continue,
                            Tok::Close => break,
                            _ => return None,
                        }
                    }
                }
                let n = args.len();
                let fits = match f {
                    Func::Round => n == 2,
                    Func::Abs => n == 1,
                    _ => n >= 1,
                };
                fits.then_some(Expr::Call(f, args))
            }
            _ => None,
        }
    }
}

/// A formula's words (after its `=`) read, or none when they cannot be.
fn parse(src: &str) -> Option<Expr> {
    let mut p = Parser {
        toks: lex(src)?,
        at: 0,
    };
    let e = p.sum()?;
    (p.at == p.toks.len()).then_some(e)
}

/// Whether a cell's words are a formula: they begin with `=` and say more.
pub fn is_formula(words: &str) -> bool {
    words.starts_with('=') && words.len() > 1
}

// ── Working out ───────────────────────────────────────────────────────────

struct Sheet<'a> {
    cells: &'a [Vec<String>],
    /// The formulas' cells and their read expressions.
    exprs: HashMap<(usize, usize), Option<Expr>>,
    done: HashMap<(usize, usize), Value>,
}

impl Sheet<'_> {
    fn inside(&self, r: usize, c: usize) -> bool {
        self.cells.get(r).is_some_and(|row| c < row.len())
    }

    fn plain(&self, r: usize, c: usize) -> Value {
        let words = &self.cells[r][c];
        if words.trim().is_empty() {
            Value::Empty
        } else if let Some(x) = number_of(words) {
            Value::Number(x)
        } else {
            Value::Text(words.clone())
        }
    }

    /// A cell's value; a formula's is worked out by then (its order is its dependencies').
    fn value(&self, r: usize, c: usize) -> Value {
        if !self.inside(r, c) {
            return Value::Error(BAD_REF);
        }
        match self.done.get(&(r, c)) {
            Some(v) => v.clone(),
            None if self.exprs.contains_key(&(r, c)) => Value::Error(RING),
            None => self.plain(r, c),
        }
    }

    fn eval(&self, e: &Expr) -> Value {
        match e {
            Expr::Num(x) => Value::Number(*x),
            Expr::Bad => Value::Error(BAD_REF),
            Expr::Ref(r, c) => self.value(*r, *c),
            Expr::Range(..) => Value::Error(NOT_NUMBER),
            Expr::Neg(a) => match arithmetic(self.eval(a)) {
                Ok(x) => Value::Number(-x),
                Err(e) => Value::Error(e),
            },
            Expr::Bin(op, a, b) => {
                let (x, y) = match (arithmetic(self.eval(a)), arithmetic(self.eval(b))) {
                    (Err(e), _) | (_, Err(e)) => return Value::Error(e),
                    (Ok(x), Ok(y)) => (x, y),
                };
                let z = match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    '/' if y == 0.0 => return Value::Error(DIV_ZERO),
                    '/' => x / y,
                    _ => x.powf(y),
                };
                if z.is_finite() {
                    Value::Number(z)
                } else {
                    Value::Error(NOT_FINITE)
                }
            }
            Expr::Call(f, args) => self.call(*f, args),
        }
    }

    /// The numbers a function's arguments give: a range's and a reference's cells skip nothing and words,
    /// another argument counts as arithmetic does.
    fn numbers(&self, args: &[Expr]) -> Result<Vec<f64>, &'static str> {
        let mut out = Vec::new();
        for a in args {
            match a {
                Expr::Range(r0, c0, r1, c1) => {
                    for r in *r0..=*r1 {
                        for c in *c0..=*c1 {
                            match self.value(r, c) {
                                Value::Number(x) => out.push(x),
                                Value::Error(e) => return Err(e),
                                _ => {}
                            }
                        }
                    }
                }
                Expr::Ref(r, c) => match self.value(*r, *c) {
                    Value::Number(x) => out.push(x),
                    Value::Error(e) => return Err(e),
                    _ => {}
                },
                other => out.push(arithmetic(self.eval(other))?),
            }
        }
        Ok(out)
    }

    fn call(&self, f: Func, args: &[Expr]) -> Value {
        let finite = |x: f64| {
            if x.is_finite() {
                Value::Number(x)
            } else {
                Value::Error(NOT_FINITE)
            }
        };
        match f {
            Func::Round => {
                let (x, n) = match (
                    arithmetic(self.eval(&args[0])),
                    arithmetic(self.eval(&args[1])),
                ) {
                    (Err(e), _) | (_, Err(e)) => return Value::Error(e),
                    (Ok(x), Ok(n)) => (x, n.trunc()),
                };
                if n.abs() > f64::from(MAX_DECIMALS) {
                    return Value::Error(NOT_FINITE);
                }
                match rounded_text(x, n as i32).parse::<f64>() {
                    Ok(v) => finite(v),
                    Err(_) => Value::Error(NOT_FINITE),
                }
            }
            Func::Abs => match arithmetic(self.eval(&args[0])) {
                Ok(x) => Value::Number(x.abs()),
                Err(e) => Value::Error(e),
            },
            _ => {
                let xs = match self.numbers(args) {
                    Ok(xs) => xs,
                    Err(e) => return Value::Error(e),
                };
                match f {
                    Func::Sum => finite(xs.iter().sum()),
                    Func::Count => Value::Number(xs.len() as f64),
                    Func::Average if xs.is_empty() => Value::Error(DIV_ZERO),
                    Func::Average => finite(xs.iter().sum::<f64>() / xs.len() as f64),
                    Func::Min => Value::Number(xs.iter().copied().fold(f64::INFINITY, f64::min))
                        .clamp_empty(xs.is_empty()),
                    _ => Value::Number(xs.iter().copied().fold(f64::NEG_INFINITY, f64::max))
                        .clamp_empty(xs.is_empty()),
                }
            }
        }
    }
}

impl Value {
    /// MIN and MAX of nothing are 0, as a spreadsheet's.
    fn clamp_empty(self, empty: bool) -> Value {
        if empty { Value::Number(0.0) } else { self }
    }
}

/// A value in arithmetic: nothing is 0, words are refused.
fn arithmetic(v: Value) -> Result<f64, &'static str> {
    match v {
        Value::Empty => Ok(0.0),
        Value::Number(x) => Ok(x),
        Value::Text(_) => Err(NOT_NUMBER),
        Value::Error(e) => Err(e),
    }
}

/// The cells an expression reads that are formulas.
fn reads(e: &Expr, formulas: &HashMap<(usize, usize), Option<Expr>>, out: &mut Vec<(usize, usize)>) {
    match e {
        Expr::Ref(r, c) => {
            if formulas.contains_key(&(*r, *c)) {
                out.push((*r, *c));
            }
        }
        Expr::Range(r0, c0, r1, c1) => {
            let area = (r1 - r0 + 1).saturating_mul(c1 - c0 + 1);
            if area <= formulas.len() {
                for r in *r0..=*r1 {
                    for c in *c0..=*c1 {
                        if formulas.contains_key(&(r, c)) {
                            out.push((r, c));
                        }
                    }
                }
            } else {
                let mut found: Vec<(usize, usize)> = formulas
                    .keys()
                    .filter(|(r, c)| r0 <= r && r <= r1 && c0 <= c && c <= c1)
                    .copied()
                    .collect();
                found.sort_unstable();
                out.extend(found);
            }
        }
        Expr::Neg(a) => reads(a, formulas, out),
        Expr::Bin(_, a, b) => {
            reads(a, formulas, out);
            reads(b, formulas, out);
        }
        Expr::Call(_, args) => args.iter().for_each(|a| reads(a, formulas, out)),
        Expr::Num(_) | Expr::Bad => {}
    }
}

/// A cell's row and column.
type Cell = (usize, usize);

/// Every formula cell's words as its value writes them (`decimals` the cell's, from its style), the table's
/// formulas being the cells whose words in `formulas` (row, column, formula with its `=`) say so; the others' cells
/// as they are. The formulas are worked out after those they read; those reading each other in a ring are `#DÖNGÜ!`.
pub fn work_out(
    cells: &[Vec<String>],
    formulas: &[(usize, usize, String)],
    decimals: &dyn Fn(usize, usize) -> Option<u32>,
) -> Vec<(usize, usize, String)> {
    let mut sheet = Sheet {
        cells,
        exprs: HashMap::new(),
        done: HashMap::new(),
    };
    for (r, c, f) in formulas {
        sheet
            .exprs
            .insert((*r, *c), f.strip_prefix('=').and_then(parse));
    }
    // Their order: depth first along what each reads, iteratively; a cell met again on the path is a ring.
    let mut order: Vec<(usize, usize)> = Vec::new();
    let mut state: HashMap<(usize, usize), u8> = HashMap::new(); // 1 on the path, 2 done
    let mut ringed: Vec<(usize, usize)> = Vec::new();
    let mut keys: Vec<(usize, usize)> = sheet.exprs.keys().copied().collect();
    keys.sort_unstable();
    for start in keys {
        if state.contains_key(&start) {
            continue;
        }
        // Each cell on the path with the cells it reads still to visit.
        let mut stack: Vec<(Cell, Vec<Cell>)> = Vec::new();
        let deps = |k: (usize, usize)| {
            let mut out = Vec::new();
            if let Some(Some(e)) = sheet.exprs.get(&k) {
                reads(e, &sheet.exprs, &mut out);
            }
            out.reverse();
            out
        };
        state.insert(start, 1);
        stack.push((start, deps(start)));
        while let Some((k, next)) = stack.last_mut() {
            match next.pop() {
                Some(n) => match state.get(&n) {
                    None => {
                        state.insert(n, 1);
                        let d = deps(n);
                        stack.push((n, d));
                    }
                    Some(1) => {
                        // A ring: every cell from `n` to the top of the path.
                        let from = stack.iter().position(|(c, _)| *c == n).unwrap_or(0);
                        ringed.extend(stack[from..].iter().map(|(c, _)| *c));
                    }
                    _ => {}
                },
                None => {
                    let k = *k;
                    state.insert(k, 2);
                    order.push(k);
                    stack.pop();
                }
            }
        }
    }
    let ringed: std::collections::HashSet<(usize, usize)> = ringed.into_iter().collect();
    let mut out = Vec::with_capacity(order.len());
    for k in order {
        let v = if ringed.contains(&k) {
            Value::Error(RING)
        } else {
            match sheet.exprs.get(&k) {
                Some(Some(e)) => sheet.eval(e),
                _ => Value::Error(NOT_READ),
            }
        };
        // A formula reading a ring's cell gets its error through `value`, as the ring's are set first below.
        sheet.done.insert(k, v.clone());
        out.push((k.0, k.1, written(&v, decimals(k.0, k.1))));
    }
    out.sort_unstable_by_key(|(r, c, _)| (*r, *c));
    out
}

// ── Moving with rows and columns ──────────────────────────────────────────────

/// Which way rows or columns are inserted or deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Rows,
    Columns,
}

/// An index after `count` rows or columns are inserted before `at` (positive) or deleted from `at` (negative):
/// none for one deleted.
pub fn moved(i: usize, at: usize, count: isize) -> Option<usize> {
    if count >= 0 {
        Some(if i >= at { i + count as usize } else { i })
    } else {
        let n = count.unsigned_abs();
        if i < at {
            Some(i)
        } else if i < at + n {
            None
        } else {
            Some(i - n)
        }
    }
}

/// A range's first and last after the move; none when all of it is deleted.
fn range_moved(a: usize, b: usize, at: usize, count: isize) -> Option<(usize, usize)> {
    if count >= 0 {
        return Some((moved(a, at, count)?, moved(b, at, count)?));
    }
    let n = count.unsigned_abs();
    let a2 = if a < at { a } else if a < at + n { at } else { a - n };
    let b2 = if b < at {
        b
    } else if b < at + n {
        at.checked_sub(1)?
    } else {
        b - n
    };
    (a2 <= b2 && !(a >= at && b < at + n)).then_some((a2, b2))
}

/// A reference's words with its `$` signs, at its new row and column.
fn reference_words(dollar_col: bool, col: usize, dollar_row: bool, row: usize) -> String {
    format!(
        "{}{}{}{}",
        if dollar_col { "$" } else { "" },
        column_letters(col),
        if dollar_row { "$" } else { "" },
        row + 1
    )
}

/// A formula's words with its references moved as `count` rows or columns inserted before `at` (positive) or deleted
/// from `at` (negative) move them; a reference to a deleted cell, or a range all deleted, is `#BAŞV!`. The words
/// that are no reference stay as they are.
pub fn shift(formula: &str, axis: Axis, at: usize, count: isize) -> String {
    let mut out = String::with_capacity(formula.len());
    let mut rest = formula;
    // A name before `(` is a function, never a reference.
    while let Some(c) = rest.chars().next() {
        if c == '$' || c.is_ascii_alphabetic() {
            let word_len = rest
                .char_indices()
                .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '$' || *ch == '_'))
                .map_or(rest.len(), |(i, _)| i);
            if let Some((n, col, row)) = reference_at(rest).filter(|(n, ..)| *n == word_len) {
                let word = &rest[..n];
                let (dc, dr) = dollars(word);
                let after = &rest[n..];
                let trimmed = after.trim_start();
                if let Some(next) = trimmed.strip_prefix(':')
                    && let Some((m, col2, row2)) = reference_at(next.trim_start())
                {
                    let next = next.trim_start();
                    let (dc2, dr2) = dollars(&next[..m]);
                    let (r0, c0, r1, c1) = (row.min(row2), col.min(col2), row.max(row2), col.max(col2));
                    let moved_range = match axis {
                        Axis::Rows => range_moved(r0, r1, at, count).map(|(a, b)| (a, c0, b, c1)),
                        Axis::Columns => range_moved(c0, c1, at, count).map(|(a, b)| (r0, a, r1, b)),
                    };
                    match moved_range {
                        Some((a, b, c, d)) => {
                            out.push_str(&reference_words(dc, b, dr, a));
                            out.push(':');
                            out.push_str(&reference_words(dc2, d, dr2, c));
                        }
                        None => out.push_str(BAD_REF),
                    }
                    rest = &next[m..];
                    continue;
                }
                let place = match axis {
                    Axis::Rows => moved(row, at, count).map(|r| (r, col)),
                    Axis::Columns => moved(col, at, count).map(|c| (row, c)),
                };
                match place {
                    Some((r, c)) => out.push_str(&reference_words(dc, c, dr, r)),
                    None => out.push_str(BAD_REF),
                }
                rest = after;
                continue;
            }
            out.push_str(&rest[..word_len.max(c.len_utf8())]);
            rest = &rest[word_len.max(c.len_utf8())..];
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// Whether a reference's column and row are marked `$`.
fn dollars(word: &str) -> (bool, bool) {
    let col = word.starts_with('$');
    let body = word.trim_start_matches('$');
    let row = body.contains('$');
    (col, row)
}

/// The table with its formulas' cells written again (their words their results).
pub fn with_formulas_worked_out(t: &TableEntity) -> TableEntity {
    let mut out = t.clone();
    if t.formulas.is_empty() {
        return out;
    }
    let formulas: Vec<(usize, usize, String)> = t
        .formulas
        .iter()
        .map(|f| (f.row as usize, f.col as usize, f.formula.clone()))
        .collect();
    let decimals = |r: usize, c: usize| t.cell_style(r, c).decimals;
    for (r, c, words) in work_out(&t.cells, &formulas, &decimals) {
        if let Some(cell) = out.cells.get_mut(r).and_then(|row| row.get_mut(c)) {
            *cell = words;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|s| (*s).to_owned()).collect())
            .collect()
    }

    fn one(cells: &[Vec<String>], f: &str) -> String {
        work_out(cells, &[(9, 9, f.to_owned())], &|_, _| None)
            .pop()
            .map(|(_, _, w)| w)
            .unwrap_or_default()
    }

    #[test]
    fn letters_and_columns_go_both_ways() {
        assert_eq!(column_letters(0), "A");
        assert_eq!(column_letters(25), "Z");
        assert_eq!(column_letters(26), "AA");
        assert_eq!(column_letters(99), "CV");
        assert_eq!(column_index("cv"), Some(99));
        assert_eq!(column_index("AAA"), None);
    }

    #[test]
    fn sums_and_errors_are_a_spreadsheet_s() {
        let mut cells = grid(&[&["1", "2", "x"], &["3", "", "4"]]);
        for r in &mut cells {
            r.resize(10, String::new());
        }
        cells.resize(10, vec![String::new(); 10]);
        assert_eq!(one(&cells, "=SUM(A1:C2)"), "10");
        assert_eq!(one(&cells, "=topla(a1;b1)"), "3");
        assert_eq!(one(&cells, "=A1+C1"), NOT_NUMBER);
        assert_eq!(one(&cells, "=A1/B2"), DIV_ZERO);
        assert_eq!(one(&cells, "=-2^2"), "4");
        assert_eq!(one(&cells, "=ORTALAMA(A1:C2)"), "2.5");
        assert_eq!(one(&cells, "=YUVARLA(2.675;2)"), "2.68");
        assert_eq!(one(&cells, "=MİN(A1:B2)"), "1");
        assert_eq!(one(&cells, "=Z99"), BAD_REF);
        assert_eq!(one(&cells, "=SUM("), NOT_READ);
        assert_eq!(one(&cells, "=0.1+0.2"), "0.3");
        assert_eq!(one(&cells, "=0.028+0.111+0.241+0.273+0.282+0.238+0.153+0.056+0.034"), "1.416");
    }

    #[test]
    fn a_result_has_fifteen_significant_digits() {
        assert_eq!(general(0.1 + 0.2), "0.3");
        assert_eq!(general(1.4160000000000001), "1.416");
        assert_eq!(general(-0.30000000000000004), "-0.3");
        assert_eq!(general(1.0 / 3.0), "0.333333333333333");
        assert_eq!(general(2.0 / 3.0), "0.666666666666667");
        assert_eq!(general(12345678901234567.0), "12345678901234600");
        assert_eq!(general(0.000_123_456_789_012_345_67), "0.000123456789012346");
        assert_eq!(general(1e-20), "0.00000000000000000001");
        assert_eq!(general(1234.5), "1234.5");
        assert_eq!(general(0.0), "0");
        assert_eq!(rounded_text(0.1 + 0.2, 16), "0.3000000000000000");
    }

    #[test]
    fn rounding_is_half_away_from_zero_on_the_shortest_form() {
        assert_eq!(rounded_text(2.675, 2), "2.68");
        assert_eq!(rounded_text(-2.5, 0), "-3");
        assert_eq!(rounded_text(9.995, 2), "10.00");
        assert_eq!(rounded_text(1234.0, -2), "1200");
        assert_eq!(rounded_text(-0.004, 2), "0.00");
        assert_eq!(rounded_text(0.5, 0), "1");
    }

    #[test]
    fn references_move_with_rows_and_columns() {
        assert_eq!(shift("=SUM(A1:A3)+B2", Axis::Rows, 1, 2), "=SUM(A1:A5)+B4");
        assert_eq!(shift("=A2+A1", Axis::Rows, 1, -1), "=#BAŞV!+A1");
        assert_eq!(shift("=SUM(A1:A3)", Axis::Rows, 0, -3), "=SUM(#BAŞV!)");
        assert_eq!(shift("=$B$1*TOPLA(C1:D1)", Axis::Columns, 1, 1), "=$C$1*TOPLA(D1:E1)");
    }

    #[test]
    fn a_ring_is_said() {
        let cells = grid(&[&["", ""]]);
        let out = work_out(
            &cells,
            &[(0, 0, "=B1+1".to_owned()), (0, 1, "=A1".to_owned())],
            &|_, _| None,
        );
        assert_eq!(out[0].2, RING);
        assert_eq!(out[1].2, RING);
    }
}
