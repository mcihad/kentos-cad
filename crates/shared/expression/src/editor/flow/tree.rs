//! The flow's tree (docs/adr/0101): the parser's tree with empty inputs
//! (`?` in the text), read from text and written back as text. Writing is
//! canonical: single spaces around operators, the Turkish words, single
//! quotes, the parentheses the grammar needs (and around a comparison
//! tested by `boş`, `içinde` …, for the eye); reading what was written
//! gives the same tree.

use crate::js::number;
use crate::js::text::{fold_turkish, utf16_len};
use crate::lexer::tokenize;
use crate::library::{FUNCTIONS, Func, FuncDef, VARIABLES, Var, VarDef};
use crate::parser::{BinOp, Node, Parser, keyword_of};
use crate::value::Value;
use crate::{CompileError, editor};

/// A field name no one writes: an empty input (`?`) while the text is read.
const HOLE: &str = "\u{e000}";

/// An expression of the flow: the parser's tree, with `Hole` where an
/// input has nothing connected.
#[derive(Clone, Debug, PartialEq)]
pub enum T {
    Hole,
    /// A number as written: negative with a minus before it.
    Num(f64),
    Text(String),
    Bool(bool),
    Null,
    Field(String),
    Var(Var),
    Call(Func, Vec<T>),
    Not(Box<T>),
    Neg(Box<T>),
    Bin(BinOp, Box<T>, Box<T>),
    Case(Vec<(T, T)>, Option<Box<T>>),
    In(Box<T>, Vec<T>, bool),
    Between(Box<T>, Box<T>, Box<T>, bool),
    Like(Box<T>, Box<T>, bool, bool),
    IsNull(Box<T>, bool),
}

pub(crate) fn func_def(f: Func) -> &'static FuncDef {
    FUNCTIONS
        .iter()
        .find(|d| d.func == f)
        .unwrap_or(&FUNCTIONS[0])
}

pub(crate) fn var_def(v: Var) -> &'static VarDef {
    VARIABLES
        .iter()
        .find(|d| d.var == v)
        .unwrap_or(&VARIABLES[0])
}

/// The symbol an operator is written with.
pub(crate) fn symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Or => "veya",
        BinOp::And => "ve",
        BinOp::Eq => "=",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Join => "||",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Pow => "^",
    }
}

/// The operator a symbol (or word) writes, as the flow's palette names them.
pub(crate) fn binop(s: &str) -> Option<BinOp> {
    const ALL: [BinOp; 15] = [
        BinOp::Or,
        BinOp::And,
        BinOp::Eq,
        BinOp::Ne,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
        BinOp::Add,
        BinOp::Sub,
        BinOp::Join,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Rem,
        BinOp::Pow,
    ];
    let f = fold_turkish(s);
    ALL.into_iter()
        .find(|&op| fold_turkish(symbol(op)) == f)
        .or(match f.as_str() {
            "==" => Some(BinOp::Eq),
            "<>" => Some(BinOp::Ne),
            "AND" => Some(BinOp::And),
            "OR" => Some(BinOp::Or),
            _ => None,
        })
}

/// Reads a tree from text: `?` is an empty input. The language's error
/// (its position in `src`) when the text is not an expression.
pub fn read(src: &str) -> Result<T, CompileError> {
    // `?` outside text and brackets becomes a field no one writes; the rest
    // is the language's own reading.
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = editor::lex::lex(&u);
    let mut with = String::with_capacity(src.len());
    let mut last = 0;
    // Where each `?` was, in UTF-16 units.
    let mut holes: Vec<usize> = Vec::new();
    for p in &pieces {
        if p.lex == editor::lex::Lex::Bad && p.end == p.start + 1 && u[p.start] == u16::from(b'?') {
            with.push_str(&String::from_utf16_lossy(&u[last..p.start]));
            with.push('[');
            with.push_str(HOLE);
            with.push(']');
            holes.push(p.start);
            last = p.end;
        }
    }
    with.push_str(&String::from_utf16_lossy(&u[last..]));
    let parsed = tokenize(&with).and_then(|toks| {
        let mut parser = Parser::new(toks);
        let root = parser.parse()?;
        Ok((root, parser.fields))
    });
    match parsed {
        Ok((root, fields)) => Ok(from_node(&root, &fields)),
        Err(mut e) => {
            // Back to the position in `src`: each `?` before the error was
            // written two units longer (`[`, the mark, `]`).
            let before = holes
                .iter()
                .enumerate()
                .filter(|&(k, &at)| at + 2 * k + 1 < e.at)
                .count();
            e.at = e.at.saturating_sub(2 * before);
            Err(e)
        }
    }
}

fn from_node(n: &Node, fields: &[String]) -> T {
    let b = |n: &Node| Box::new(from_node(n, fields));
    match n {
        Node::Lit(Value::Num(x)) => T::Num(*x),
        Node::Lit(Value::Text(s)) => T::Text(s.to_string()),
        Node::Lit(Value::Bool(v)) => T::Bool(*v),
        Node::Lit(Value::Null) => T::Null,
        Node::Field(i) => match fields.get(*i) {
            Some(name) if name == HOLE => T::Hole,
            Some(name) => T::Field(name.clone()),
            None => T::Hole,
        },
        Node::Var(v) => T::Var(*v),
        Node::Call(f, args) => T::Call(*f, args.iter().map(|a| from_node(a, fields)).collect()),
        Node::Not(a) => T::Not(b(a)),
        // A number with a minus before it is one number (`-5`), as it reads.
        Node::Neg(a) => match from_node(a, fields) {
            T::Num(x) if !x.is_sign_negative() => T::Num(-x),
            a => T::Neg(Box::new(a)),
        },
        Node::Bin(op, x, y) => T::Bin(*op, b(x), b(y)),
        Node::Case(whens, otherwise) => T::Case(
            whens
                .iter()
                .map(|(c, v)| (from_node(c, fields), from_node(v, fields)))
                .collect(),
            otherwise.as_ref().map(|e| b(e)),
        ),
        Node::In(x, items, negated) => T::In(
            b(x),
            items.iter().map(|i| from_node(i, fields)).collect(),
            *negated,
        ),
        Node::Between(x, low, high, negated) => T::Between(b(x), b(low), b(high), *negated),
        Node::Like(x, p, fold, negated) => T::Like(b(x), b(p), *fold, *negated),
        Node::IsNull(x, negated) => T::IsNull(b(x), *negated),
    }
}

impl T {
    /// How tightly it binds, loosest first: veya 0, ve 1, comparisons and
    /// the tests 2, + − || 3, * / % 4, değil, the sign and a negative
    /// number 5, ^ 6, the rest 7.
    fn level(&self) -> u8 {
        match self {
            T::Bin(BinOp::Or, ..) => 0,
            T::Bin(BinOp::And, ..) => 1,
            T::Bin(BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge, ..)
            | T::In(..)
            | T::Between(..)
            | T::Like(..)
            | T::IsNull(..) => 2,
            T::Bin(BinOp::Add | BinOp::Sub | BinOp::Join, ..) => 3,
            T::Bin(BinOp::Mul | BinOp::Div | BinOp::Rem, ..) => 4,
            T::Not(_) | T::Neg(_) => 5,
            T::Num(x) if x.is_sign_negative() => 5,
            T::Bin(BinOp::Pow, ..) => 6,
            _ => 7,
        }
    }

    /// The inputs in port order, `Hole` where nothing is connected.
    pub fn inputs(&self) -> Vec<&T> {
        match self {
            T::Call(_, args) => args.iter().collect(),
            T::Not(a) | T::Neg(a) | T::IsNull(a, _) => vec![a],
            T::Bin(_, a, b) | T::Like(a, b, ..) => vec![a, b],
            T::Between(a, b, c, _) => vec![a, b, c],
            T::In(x, items, _) => std::iter::once(&**x).chain(items.iter()).collect(),
            T::Case(whens, otherwise) => whens
                .iter()
                .flat_map(|(c, v)| [c, v])
                .chain(otherwise.as_deref())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Input `port`, if the node has it (for `durum`: 2k condition k, 2k + 1
    /// its value, the else last).
    pub fn input_mut(&mut self, port: usize) -> Option<&mut T> {
        match self {
            T::Call(_, args) => args.get_mut(port),
            T::Not(a) | T::Neg(a) | T::IsNull(a, _) => (port == 0).then_some(&mut **a),
            T::Bin(_, a, b) | T::Like(a, b, ..) => match port {
                0 => Some(a),
                1 => Some(b),
                _ => None,
            },
            T::Between(a, b, c, _) => match port {
                0 => Some(a),
                1 => Some(b),
                2 => Some(c),
                _ => None,
            },
            T::In(x, items, _) => match port {
                0 => Some(x),
                k => items.get_mut(k - 1),
            },
            T::Case(whens, otherwise) => {
                let n = 2 * whens.len();
                if port < n {
                    let (c, v) = &mut whens[port / 2];
                    Some(if port.is_multiple_of(2) { c } else { v })
                } else if port == n {
                    otherwise.as_deref_mut()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// The node at `path` (ports from here).
    pub fn at(&self, path: &[usize]) -> Option<&T> {
        match path.split_first() {
            None => Some(self),
            Some((&port, rest)) => self.inputs().get(port).and_then(|c| c.at(rest)),
        }
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut T> {
        match path.split_first() {
            None => Some(self),
            Some((&port, rest)) => self.input_mut(port)?.at_mut(rest),
        }
    }

    /// Writes the tree as the language reads it, and each node's span in the
    /// text (UTF-16 units) by its path.
    pub fn write(&self) -> (String, Vec<(Vec<usize>, usize, usize)>) {
        let mut w = Writer {
            out: String::new(),
            units: 0,
            spans: Vec::new(),
            path: Vec::new(),
        };
        w.node(self, 0);
        (w.out, w.spans)
    }

    /// The tree's text.
    pub fn text(&self) -> String {
        self.write().0
    }
}

struct Writer {
    out: String,
    /// `out`'s length in UTF-16 units.
    units: usize,
    spans: Vec<(Vec<usize>, usize, usize)>,
    path: Vec<usize>,
}

impl Writer {
    fn put(&mut self, s: &str) {
        self.out.push_str(s);
        self.units += utf16_len(s);
    }

    /// Input `port` of the node being written, in parentheses when it binds
    /// looser than `least`.
    fn input(&mut self, port: usize, t: &T, least: u8) {
        self.path.push(port);
        let paren = t.level() < least;
        if paren {
            self.put("(");
        }
        self.node(t, least);
        if paren {
            self.put(")");
        }
        self.path.pop();
    }

    fn node(&mut self, t: &T, _least: u8) {
        let start = self.units;
        match t {
            T::Hole => self.put("?"),
            T::Num(x) => {
                if x.is_sign_negative() {
                    self.put("-");
                }
                let x = x.abs();
                if x.is_finite() {
                    self.put(&number::to_string(x));
                } else if x.is_nan() {
                    self.put("(0 / 0)");
                } else {
                    // Past the largest double: as the lexer reads it back.
                    self.put("1e999");
                }
            }
            T::Text(s) => {
                self.put("'");
                self.put(&s.replace('\'', "''"));
                self.put("'");
            }
            T::Bool(true) => self.put("doğru"),
            T::Bool(false) => self.put("yanlış"),
            T::Null => self.put("boş"),
            T::Field(name) => {
                if bare(name) {
                    self.put(name);
                } else {
                    self.put("[");
                    self.put(name);
                    self.put("]");
                }
            }
            T::Var(v) => {
                self.put("$");
                self.put(var_def(*v).name);
            }
            T::Call(f, args) => {
                self.put(func_def(*f).name);
                self.put("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.put(", ");
                    }
                    self.input(i, a, 0);
                }
                self.put(")");
            }
            T::Not(a) => {
                self.put("değil ");
                self.input(0, a, 5);
            }
            T::Neg(a) => {
                self.put("-");
                // `- -5`, not `--5`: the eye reads two signs.
                if matches!(**a, T::Neg(_)) || matches!(**a, T::Num(x) if x.is_sign_negative()) {
                    self.put(" ");
                }
                self.input(0, a, 5);
            }
            T::Bin(BinOp::Pow, a, b) => {
                self.input(0, a, 7);
                self.put(" ^ ");
                self.input(1, b, 5);
            }
            T::Bin(op, a, b) => {
                let level = t.level();
                self.input(0, a, level);
                self.put(" ");
                self.put(symbol(*op));
                self.put(" ");
                self.input(1, b, level + 1);
            }
            T::Case(whens, otherwise) => {
                self.put("durum");
                for (k, (c, v)) in whens.iter().enumerate() {
                    self.put(" eğer ");
                    self.input(2 * k, c, 0);
                    self.put(" ise ");
                    self.input(2 * k + 1, v, 0);
                }
                if let Some(e) = otherwise {
                    self.put(" yoksa ");
                    self.input(2 * whens.len(), e, 0);
                }
                self.put(" son");
            }
            T::In(x, items, negated) => {
                self.input(0, x, 3);
                self.put(if *negated {
                    " değil içinde ("
                } else {
                    " içinde ("
                });
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.put(", ");
                    }
                    self.input(i + 1, item, 0);
                }
                self.put(")");
            }
            T::Between(x, low, high, negated) => {
                self.input(0, x, 3);
                self.put(if *negated {
                    " değil arasında "
                } else {
                    " arasında "
                });
                self.input(1, low, 3);
                self.put(" ve ");
                self.input(2, high, 3);
            }
            T::Like(x, p, fold, negated) => {
                self.input(0, x, 3);
                self.put(match (*negated, *fold) {
                    (false, false) => " gibi ",
                    (false, true) => " benzer ",
                    (true, false) => " değil gibi ",
                    (true, true) => " değil benzer ",
                });
                self.input(1, p, 3);
            }
            T::IsNull(x, negated) => {
                self.input(0, x, 3);
                self.put(if *negated { " boş değil" } else { " boş" });
            }
        }
        self.spans.push((self.path.clone(), start, self.units));
    }
}

/// The words the grammar reads as its own somewhere (docs/adr/0100 §4): a
/// field so named is written in brackets, so it never reads as the word.
const WORDS: [&str; 19] = [
    "DURUM", "CASE", "EGER", "WHEN", "ISE", "THEN", "YOKSA", "ELSE", "SON", "END", "ICINDE", "IN",
    "ARASINDA", "BETWEEN", "GIBI", "LIKE", "BENZER", "ILIKE", "IS",
];

/// Whether a field's name can stand bare: one word of the language that is
/// none of its words.
fn bare(name: &str) -> bool {
    let u: Vec<u16> = name.encode_utf16().collect();
    let n = crate::lexer::word(&u, 0);
    n == u.len()
        && n > 0
        && keyword_of(name).is_none()
        && !WORDS.contains(&fold_turkish(name).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn again(src: &str) -> String {
        read(src).map(|t| t.text()).unwrap_or_else(|e| e.text())
    }

    #[test]
    fn text_reads_as_a_tree_and_writes_back_canonical() {
        assert_eq!(again("ROUND( $ALAN ,2 )"), "yuvarla($alan, 2)");
        assert_eq!(again("a AND b OR NOT c"), "a ve b veya değil c");
        assert_eq!(again("(a = b) = c"), "a = b = c");
        assert_eq!(again("a = (b = c)"), "a = (b = c)");
        assert_eq!(again("değil (a = b)"), "değil (a = b)");
        assert_eq!(again("(değil a) = b"), "değil a = b");
        assert_eq!(again("-2 ^ 2"), "-2 ^ 2");
        assert_eq!(again("(-2) ^ 2"), "(-2) ^ 2");
        assert_eq!(again("2 ^ (1 / 3)"), "2 ^ (1 / 3)");
        assert_eq!(again("2 ^ 3 ^ 2"), "2 ^ 3 ^ 2");
        assert_eq!(again("(2 ^ 3) ^ 2"), "(2 ^ 3) ^ 2");
        assert_eq!(again("a - (b - c)"), "a - (b - c)");
        assert_eq!(again("- -5"), "- -5");
        assert_eq!(again("\"it's\""), "'it''s'");
        assert_eq!(
            again("[Tapu alanı] + [ve] + [Durum] + Ada"),
            "[Tapu alanı] + [ve] + [Durum] + Ada"
        );
        assert_eq!(
            again("CASE WHEN x IS NOT NULL THEN 1 ELSE 0 END"),
            "durum eğer x boş değil ise 1 yoksa 0 son"
        );
        assert_eq!(again("x = 1 boş"), "(x = 1) boş");
        assert_eq!(again("x NOT IN (1, 2)"), "x değil içinde (1, 2)");
        assert_eq!(again("x not ilike 'a%'"), "x değil benzer 'a%'");
        assert_eq!(again("1e21 + 0.1 + 1e400"), "1e+21 + 0.1 + 1e999");
    }

    #[test]
    fn a_question_mark_is_an_empty_input() {
        let t = read("yuvarla(?, 2) + ?").expect("reads");
        assert_eq!(t.text(), "yuvarla(?, 2) + ?");
        assert_eq!(t.at(&[0, 0]), Some(&T::Hole));
        assert_eq!(t.at(&[1]), Some(&T::Hole));
        // Errors stay where the text has them.
        let e = read("? + ) ").expect_err("an error");
        assert_eq!(e.at, 5);
        // In text and brackets a question mark is itself.
        assert_eq!(again("'?' || [a?]"), "'?' || [a?]");
    }

    #[test]
    fn written_text_reads_back_as_the_same_tree() {
        for src in [
            "durum eğer Kat > 4 ise 'yüksek' eğer Kat > 2 ise 'orta' yoksa 'alçak' son",
            "Nitelik değil içinde ('Arsa', 'Tarla') ve Kat arasında 3 ve 5",
            "birleştir(sol(Ada, 2), '/', sağdoldur(Parsel, 4, '0'))",
            "eğer(boş(Ada), 'yok', Ada || '/' || Parsel)",
            "-(-2) ^ -2 * -$alan",
            "(a veya b) ve (c veya değil d)",
        ] {
            let t = read(src).expect(src);
            let again = read(&t.text()).expect(src);
            assert_eq!(t, again, "{src}");
        }
    }

    #[test]
    fn spans_cover_each_node() {
        let (text, spans) = read("yuvarla($alan * 2, 1)").expect("reads").write();
        let span = |path: &[usize]| {
            let &(_, a, b) = spans.iter().find(|(p, ..)| p == path).expect("a span");
            text[a..b].to_string()
        };
        assert_eq!(span(&[]), "yuvarla($alan * 2, 1)");
        assert_eq!(span(&[0]), "$alan * 2");
        assert_eq!(span(&[0, 1]), "2");
        assert_eq!(span(&[1]), "1");
    }
}
