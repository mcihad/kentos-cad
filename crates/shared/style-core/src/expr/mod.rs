//! İfadeler: a small, safe expression language for processing tools
//! (select by expression, field calculator, filters) and the style engine
//! (data-defined values, rule and category renderers). No eval: the source
//! is tokenized, parsed (precedence climbing) and evaluated as a tree.
//!
//!   Nitelik = 'Arsa' ve $alan > 500
//!   'P' || doldur($sıra, 5)
//!   yuvarla([Tapu alanı] - $alan, 2)
//!
//! Fields: bare names (Parsel) or in brackets ([Tapu alanı]). Text: '…' or
//! "…" (a doubled quote inside is one quote). Variables start with $ (see
//! `library.rs`). Keywords: ve/and, veya/or, değil/not, doğru/true,
//! yanlış/false, boş/null. Operators: = != <> < <= > >= + - * / % and ||
//! (joins text). A faithful port of the TypeScript it replaces
//! (`apps/web/src/model/expression/`, docs/adr/0008 “İfade dili”): the same
//! values, the same text, the same errors at the same positions.

pub mod lexer;
pub mod library;
pub mod parser;
pub mod rows;
pub mod value;

use std::borrow::Cow;

use library::{Thrown, Var, call};
use parser::{BinOp, Node, Parser};
pub use value::Value;
use value::{compare, equals, into_text, to_number, truthy};

use crate::js::text::{MAX_STRING_UNITS, utf16_len};

/// What an expression that does not compile says: a message for the dialog
/// and the 1-based position (in UTF-16 code units) it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub at: usize,
}

impl CompileError {
    /// "12. karakterde: …" (the dialog's form).
    pub fn text(&self) -> String {
        if self.at > 1 {
            format!("{}. karakterde: {}", self.at, self.message)
        } else {
            self.message.clone()
        }
    }
}

/// The variables an expression reads, so a caller computes only those.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    pub measured: bool,
    pub vertices: bool,
    pub kind: bool,
    pub layer: bool,
    pub label: bool,
    pub index: bool,
    pub id: bool,
    pub scale: bool,
}

/// An object's geometry values: `$uzunluk`, `$alan`, and the anchor behind `$y` and `$x`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measured {
    pub length: Option<f64>,
    pub area: Option<f64>,
    pub anchor: Option<(f64, f64)>,
}

/// What an expression sees of one object.
pub trait Scope {
    /// Attribute `i` of the expression's field list (None: the object has no such attribute).
    fn field(&self, i: usize) -> Option<&str>;
    fn measured(&self) -> Measured;
    fn vertices(&self) -> Option<f64>;
    fn kind(&self) -> &str;
    fn layer(&self) -> &str;
    fn label(&self) -> Option<&str>;
    /// 1-based position of the object in the run.
    fn index(&self) -> f64;
    fn id(&self) -> f64;
    /// Denominator of the plot scale while drawing a symbol; None elsewhere.
    fn scale(&self) -> Option<f64>;
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub source: String,
    /// Attribute names the expression reads, in order of appearance.
    pub fields: Vec<String>,
    pub needs: Needs,
    root: Node,
}

pub fn compile(source: &str) -> Result<Expr, CompileError> {
    let mut parser = Parser::new(lexer::tokenize(source)?);
    let root = parser.parse()?;
    let mut needs = Needs::default();
    uses(&root, &mut needs);
    Ok(Expr {
        source: source.to_string(),
        fields: parser.fields,
        needs,
        root,
    })
}

fn uses(n: &Node, needs: &mut Needs) {
    match n {
        Node::Var(v) => match v {
            Var::Area | Var::Length | Var::Y | Var::X => needs.measured = true,
            Var::Vertices => needs.vertices = true,
            Var::Kind => needs.kind = true,
            Var::Layer => needs.layer = true,
            Var::Label => needs.label = true,
            Var::Index => needs.index = true,
            Var::Id => needs.id = true,
            Var::Scale => needs.scale = true,
        },
        Node::Call(_, args) => args.iter().for_each(|a| uses(a, needs)),
        Node::Not(a) | Node::Neg(a) => uses(a, needs),
        Node::Bin(_, a, b) => {
            uses(a, needs);
            uses(b, needs);
        }
        Node::Lit(_) | Node::Field(_) => {}
    }
}

impl Expr {
    /// The value for one object: empty where JavaScript would have thrown,
    /// and for a number that is not finite. Text borrows from the source
    /// and from the object where it can.
    pub fn evaluate<'a>(&'a self, s: &'a dyn Scope) -> Value<'a> {
        match eval(&self.root, s) {
            Ok(Value::Num(x)) if !x.is_finite() => Value::Null,
            Ok(v) => v,
            Err(Thrown) => Value::Null,
        }
    }
}

fn opt(x: Option<f64>) -> Value<'static> {
    x.map_or(Value::Null, Value::Num)
}

fn borrowed(t: &str) -> Value<'_> {
    Value::Text(Cow::Borrowed(t))
}

fn variable(v: Var, s: &dyn Scope) -> Value<'_> {
    match v {
        Var::Area => opt(s.measured().area),
        Var::Length => opt(s.measured().length),
        Var::Y => opt(s.measured().anchor.map(|a| a.0)),
        Var::X => opt(s.measured().anchor.map(|a| a.1)),
        Var::Vertices => opt(s.vertices()),
        Var::Kind => borrowed(s.kind()),
        Var::Layer => borrowed(s.layer()),
        Var::Label => s.label().map_or(Value::Null, borrowed),
        Var::Index => Value::Num(s.index()),
        Var::Id => Value::Num(s.id()),
        Var::Scale => opt(s.scale()),
    }
}

/// Joined text, where JavaScript throws past its longest string. Text
/// either side made already (a join, a number, a padding) is extended in
/// place rather than copied.
fn joined<'a>(a: Cow<'_, str>, b: Cow<'_, str>) -> Result<Value<'a>, Thrown> {
    if a.len() + b.len() > MAX_STRING_UNITS && utf16_len(&a) + utf16_len(&b) > MAX_STRING_UNITS {
        return Err(Thrown);
    }
    let out = match (a, b) {
        (Cow::Owned(mut a), b) => {
            a.push_str(&b);
            a
        }
        (Cow::Borrowed(a), Cow::Owned(mut b)) => {
            b.insert_str(0, a);
            b
        }
        (Cow::Borrowed(a), Cow::Borrowed(b)) => {
            let mut out = String::with_capacity(a.len() + b.len());
            out.push_str(a);
            out.push_str(b);
            out
        }
    };
    Ok(Value::Text(Cow::Owned(out)))
}

fn binary<'a>(op: BinOp, a: Value<'a>, b: Value<'a>) -> Result<Value<'a>, Thrown> {
    Ok(match op {
        BinOp::Or => Value::Bool(truthy(&a) || truthy(&b)),
        BinOp::And => Value::Bool(truthy(&a) && truthy(&b)),
        BinOp::Eq => Value::Bool(equals(&a, &b)),
        BinOp::Ne => Value::Bool(!equals(&a, &b)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare(&a, &b) {
            None => Value::Bool(false),
            Some(c) => Value::Bool(match op {
                BinOp::Lt => c < 0.0,
                BinOp::Le => c <= 0.0,
                BinOp::Gt => c > 0.0,
                _ => c >= 0.0,
            }),
        },
        BinOp::Join => joined(into_text(a), into_text(b))?,
        BinOp::Add => {
            if a == Value::Null || b == Value::Null {
                return Ok(Value::Null);
            }
            match (to_number(&a), to_number(&b)) {
                (Some(na), Some(nb)) => Value::Num(na + nb),
                _ => joined(into_text(a), into_text(b))?,
            }
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            let (Some(na), Some(nb)) = (to_number(&a), to_number(&b)) else {
                return Ok(Value::Null);
            };
            if matches!(op, BinOp::Div | BinOp::Rem) && nb == 0.0 {
                return Ok(Value::Null);
            }
            Value::Num(match op {
                BinOp::Sub => na - nb,
                BinOp::Mul => na * nb,
                BinOp::Div => na / nb,
                _ => na % nb,
            })
        }
    })
}

fn eval<'a>(n: &'a Node, s: &'a dyn Scope) -> Result<Value<'a>, Thrown> {
    Ok(match n {
        Node::Lit(Value::Text(t)) => borrowed(t),
        Node::Lit(v) => v.clone(),
        Node::Field(i) => s.field(*i).map_or(Value::Null, borrowed),
        Node::Var(v) => variable(*v, s),
        Node::Call(f, args) if args.len() <= 4 => {
            // Most calls take a few arguments: they stay on the stack.
            let mut values: [Value<'a>; 4] = Default::default();
            for (slot, a) in values.iter_mut().zip(args) {
                *slot = eval(a, s)?;
            }
            call(*f, &mut values[..args.len()])?
        }
        Node::Call(f, args) => {
            let mut values = args
                .iter()
                .map(|a| eval(a, s))
                .collect::<Result<Vec<_>, _>>()?;
            call(*f, &mut values)?
        }
        Node::Not(a) => Value::Bool(!truthy(&eval(a, s)?)),
        Node::Neg(a) => match to_number(&eval(a, s)?) {
            Some(v) => Value::Num(-v),
            None => Value::Null,
        },
        Node::Bin(op, a, b) => {
            let a = eval(a, s)?;
            let b = eval(b, s)?;
            binary(*op, a, b)?
        }
    })
}

#[cfg(test)]
mod tests;
