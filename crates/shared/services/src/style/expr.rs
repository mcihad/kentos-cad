//! MapLibre's expressions, the subset docs/adr/0208 §9 names, and the
//! older forms they replaced: legacy filters (`["==", "class", "park"]`,
//! `["in", "class", …]`, `$type`, `$id`) and legacy functions (`{"stops":
//! …}` by zoom or by a property; exponential, interval, categorical,
//! identity). An operator KentOS does not know becomes `Unsupported`: its
//! value is null and the style's note names it.

use serde_json::Value as Json;

use super::color::{self, Rgba};
use kentos_style_core::js::number;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Color(Rgba),
    Arr(Vec<Val>),
}

impl Val {
    pub fn num(&self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Val::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A colour: one already, or a text that reads as one.
    pub fn color(&self) -> Option<Rgba> {
        match self {
            Val::Color(c) => Some(*c),
            Val::Str(s) => color::parse(s),
            _ => None,
        }
    }

    /// `to-string`'s text.
    pub fn text(&self) -> String {
        match self {
            Val::Null => String::new(),
            Val::Bool(b) => b.to_string(),
            Val::Num(n) => number::to_string(*n),
            Val::Str(s) => s.clone(),
            Val::Color(c) => format!(
                "rgba({},{},{},{})",
                (c[0] * 255.0).round(),
                (c[1] * 255.0).round(),
                (c[2] * 255.0).round(),
                number::to_string(c[3])
            ),
            Val::Arr(a) => format!(
                "[{}]",
                a.iter().map(Val::text).collect::<Vec<_>>().join(",")
            ),
        }
    }

    fn from_json(v: &Json) -> Val {
        match v {
            Json::Null => Val::Null,
            Json::Bool(b) => Val::Bool(*b),
            Json::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
            Json::String(s) => Val::Str(s.clone()),
            Json::Array(a) => Val::Arr(a.iter().map(Val::from_json).collect()),
            Json::Object(_) => Val::Null,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interp {
    Linear,
    Exponential(f64),
    /// A cubic Bézier's control points x₁, y₁, x₂, y₂.
    Bezier([f64; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Min,
    Max,
    Abs,
    Floor,
    Ceil,
    Round,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Ln,
    Log10,
    Log2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(Val),
    Get(Box<Expr>),
    Has(Box<Expr>),
    Zoom,
    /// `geometry-type`: `Point`, `LineString`, `Polygon` or their `Multi` forms.
    GeometryType,
    /// A legacy filter's `$type`: the type without `Multi`.
    BaseType,
    Id,
    Not(Box<Expr>),
    Cmp(Cmp, Box<Expr>, Box<Expr>),
    All(Vec<Expr>),
    Any(Vec<Expr>),
    In(Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Vec<(Vec<Val>, Expr)>, Box<Expr>),
    Case(Vec<(Expr, Expr)>, Box<Expr>),
    Coalesce(Vec<Expr>),
    Step(Box<Expr>, Box<Expr>, Vec<(f64, Expr)>),
    Interpolate(Interp, Box<Expr>, Vec<(f64, Expr)>),
    Concat(Vec<Expr>),
    ToString(Box<Expr>),
    ToNumber(Vec<Expr>),
    ToBoolean(Box<Expr>),
    ToColor(Vec<Expr>),
    Upcase(Box<Expr>),
    Downcase(Box<Expr>),
    Length(Box<Expr>),
    At(Box<Expr>, Box<Expr>),
    Rgba(Vec<Expr>),
    Math(MathOp, Vec<Expr>),
    /// A type assertion (`number`, `string`, `boolean`, `array`, `object`):
    /// the first argument of the type.
    Assert(&'static str, Vec<Expr>),
    TypeOf(Box<Expr>),
    Let(Vec<(String, Expr)>, Box<Expr>),
    Var(String),
    IndexOf(Box<Expr>, Box<Expr>),
    Slice(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    ToRgba(Box<Expr>),
    Unsupported(String),
}

/// What a feature tells the expressions.
pub trait Feature {
    fn property(&self, key: &str) -> Option<Val>;
    /// `Point`, `LineString`, `Polygon`, or their `Multi` forms.
    fn geometry_type(&self) -> &str;
    fn id(&self) -> Option<f64>;
}

/// What an expression is evaluated against: the view's zoom and, for a
/// data-driven value, the feature.
pub struct Ctx<'a> {
    pub zoom: f64,
    pub feature: Option<&'a dyn Feature>,
    /// `let`'s names in scope, innermost last.
    pub vars: &'a [(String, Val)],
}

impl<'a> Ctx<'a> {
    pub fn new(zoom: f64, feature: Option<&'a dyn Feature>) -> Ctx<'a> {
        Ctx {
            zoom,
            feature,
            vars: &[],
        }
    }
}

/// The operators the parser knows: an array starting with one is an
/// expression, any other array a literal.
const OPS: [&str; 47] = [
    "get",
    "has",
    "!has",
    "zoom",
    "geometry-type",
    "id",
    "!",
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "all",
    "any",
    "none",
    "in",
    "!in",
    "match",
    "case",
    "coalesce",
    "step",
    "interpolate",
    "interpolate-hcl",
    "interpolate-lab",
    "concat",
    "to-string",
    "to-number",
    "to-boolean",
    "to-color",
    "upcase",
    "downcase",
    "length",
    "at",
    "rgb",
    "rgba",
    "literal",
    "+",
    "-",
    "*",
    "/",
    "%",
    "^",
    "min",
    "max",
    "format",
    "number-format",
];
const MATH_ONE: [(&str, MathOp); 5] = [
    ("abs", MathOp::Abs),
    ("floor", MathOp::Floor),
    ("ceil", MathOp::Ceil),
    ("round", MathOp::Round),
    ("sqrt", MathOp::Sqrt),
];
/// More operators read: assertions, variables, constants, the trigonometry.
const MORE: [&str; 24] = [
    "number",
    "string",
    "boolean",
    "array",
    "object",
    "typeof",
    "let",
    "var",
    "pi",
    "e",
    "ln2",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "ln",
    "log10",
    "log2",
    "index-of",
    "slice",
    "to-rgba",
    "properties",
];
/// MapLibre's operators KentOS does not draw: noted, their value null.
const UNSUPPORTED: [&str; 12] = [
    "within",
    "distance",
    "image",
    "collator",
    "resolved-locale",
    "is-supported-script",
    "heatmap-density",
    "line-progress",
    "accumulated",
    "feature-state",
    "global-state",
    "elevation",
];

/// Whether a JSON value is an expression (an array whose first item names an operator).
pub fn is_expression(v: &Json) -> bool {
    v.as_array()
        .and_then(|a| a.first())
        .and_then(Json::as_str)
        .is_some_and(|op| {
            OPS.contains(&op)
                || MATH_ONE.iter().any(|(n, _)| *n == op)
                || MORE.contains(&op)
                || UNSUPPORTED.contains(&op)
        })
}

/// What a style uses that KentOS does not draw (names, once each).
#[derive(Default, Debug)]
pub struct Notes(pub Vec<String>);

impl Notes {
    pub fn add(&mut self, what: impl Into<String>) {
        let w = what.into();
        if !self.0.contains(&w) {
            self.0.push(w);
        }
    }
}

/// A property's value: a literal, a legacy function or an expression.
pub fn parse(v: &Json, notes: &mut Notes) -> Expr {
    if v.is_object() && v.get("stops").is_some() {
        return function(v, notes);
    }
    if is_expression(v) {
        return expression(v, notes);
    }
    Expr::Lit(Val::from_json(v))
}

fn boxed(v: &Json, notes: &mut Notes) -> Box<Expr> {
    Box::new(parse_arg(v, notes))
}

/// An argument inside an expression: a nested expression or a literal.
fn parse_arg(v: &Json, notes: &mut Notes) -> Expr {
    if is_expression(v) {
        expression(v, notes)
    } else {
        Expr::Lit(Val::from_json(v))
    }
}

fn stops(list: &[Json], notes: &mut Notes) -> Vec<(f64, Expr)> {
    list.chunks_exact(2)
        .filter_map(|p| Some((p[0].as_f64()?, parse_arg(&p[1], notes))))
        .collect()
}

fn expression(v: &Json, notes: &mut Notes) -> Expr {
    let a = v.as_array().map(Vec::as_slice).unwrap_or_default();
    let op = a.first().and_then(Json::as_str).unwrap_or("");
    let args = &a[1.min(a.len())..];
    let all = |notes: &mut Notes| args.iter().map(|x| parse_arg(x, notes)).collect::<Vec<_>>();
    match op {
        "literal" => Expr::Lit(Val::from_json(args.first().unwrap_or(&Json::Null))),
        "get" if args.len() == 1 => Expr::Get(boxed(&args[0], notes)),
        "has" if args.len() == 1 => Expr::Has(boxed(&args[0], notes)),
        "!has" if args.len() == 1 => Expr::Not(Box::new(Expr::Has(boxed(&args[0], notes)))),
        "zoom" => Expr::Zoom,
        "geometry-type" => Expr::GeometryType,
        "id" => Expr::Id,
        "!" if args.len() == 1 => Expr::Not(boxed(&args[0], notes)),
        "==" | "!=" | "<" | "<=" | ">" | ">=" if args.len() >= 2 => {
            let c = match op {
                "==" => Cmp::Eq,
                "!=" => Cmp::Ne,
                "<" => Cmp::Lt,
                "<=" => Cmp::Le,
                ">" => Cmp::Gt,
                _ => Cmp::Ge,
            };
            Expr::Cmp(c, boxed(&args[0], notes), boxed(&args[1], notes))
        }
        "all" => Expr::All(all(notes)),
        "any" => Expr::Any(all(notes)),
        "none" => Expr::Not(Box::new(Expr::Any(all(notes)))),
        "in" if args.len() == 2 => Expr::In(boxed(&args[0], notes), boxed(&args[1], notes)),
        "!in" if args.len() == 2 => Expr::Not(Box::new(Expr::In(
            boxed(&args[0], notes),
            boxed(&args[1], notes),
        ))),
        "match" if args.len() >= 2 => {
            let input = boxed(&args[0], notes);
            let rest = &args[1..];
            let (pairs, fallback) = rest.split_at(rest.len() - 1);
            let cases = pairs
                .chunks_exact(2)
                .map(|p| {
                    let labels = match &p[0] {
                        Json::Array(l) => l.iter().map(Val::from_json).collect(),
                        other => vec![Val::from_json(other)],
                    };
                    (labels, parse_arg(&p[1], notes))
                })
                .collect();
            Expr::Match(input, cases, boxed(&fallback[0], notes))
        }
        "case" if !args.is_empty() => {
            let (pairs, fallback) = args.split_at(args.len() - 1);
            let branches = pairs
                .chunks_exact(2)
                .map(|p| (parse_arg(&p[0], notes), parse_arg(&p[1], notes)))
                .collect();
            Expr::Case(branches, boxed(&fallback[0], notes))
        }
        "coalesce" => Expr::Coalesce(all(notes)),
        "step" if args.len() >= 2 => Expr::Step(
            boxed(&args[0], notes),
            boxed(&args[1], notes),
            stops(&args[2..], notes),
        ),
        "interpolate" | "interpolate-hcl" | "interpolate-lab" if args.len() >= 2 => {
            if op != "interpolate" {
                notes.add(format!("{op} (renkler RGB'de ara değerlenir)"));
            }
            let kind = args[0].as_array().map(Vec::as_slice).unwrap_or_default();
            let interp = match kind.first().and_then(Json::as_str) {
                Some("exponential") => match kind.get(1).and_then(Json::as_f64) {
                    Some(b) if b != 1.0 => Interp::Exponential(b),
                    _ => Interp::Linear,
                },
                Some("cubic-bezier") => {
                    let n: Vec<f64> = kind[1..].iter().filter_map(Json::as_f64).collect();
                    if n.len() == 4 {
                        Interp::Bezier([n[0], n[1], n[2], n[3]])
                    } else {
                        Interp::Linear
                    }
                }
                _ => Interp::Linear,
            };
            Expr::Interpolate(interp, boxed(&args[1], notes), stops(&args[2..], notes))
        }
        "concat" => Expr::Concat(all(notes)),
        "format" => {
            // Its texts in order; their formatting options are left out.
            Expr::Concat(
                args.iter()
                    .filter(|x| !x.is_object())
                    .map(|x| parse_arg(x, notes))
                    .collect(),
            )
        }
        "number-format" if !args.is_empty() => Expr::ToString(boxed(&args[0], notes)),
        "to-string" if args.len() == 1 => Expr::ToString(boxed(&args[0], notes)),
        "to-number" => Expr::ToNumber(all(notes)),
        "to-boolean" if args.len() == 1 => Expr::ToBoolean(boxed(&args[0], notes)),
        "to-color" => Expr::ToColor(all(notes)),
        "upcase" if args.len() == 1 => Expr::Upcase(boxed(&args[0], notes)),
        "downcase" if args.len() == 1 => Expr::Downcase(boxed(&args[0], notes)),
        "length" if args.len() == 1 => Expr::Length(boxed(&args[0], notes)),
        "at" if args.len() == 2 => Expr::At(boxed(&args[0], notes), boxed(&args[1], notes)),
        "rgb" | "rgba" => Expr::Rgba(all(notes)),
        "properties" => Expr::Unsupported("properties".into()),
        "number" | "string" | "boolean" | "array" | "object" => {
            let kind = match op {
                "number" => "number",
                "string" => "string",
                "boolean" => "boolean",
                "array" => "array",
                _ => "object",
            };
            // `array`'s item type and length come first when given.
            let skip = if op == "array" {
                args.iter()
                    .take_while(|x| x.is_string() || x.is_number())
                    .count()
                    .min(args.len().saturating_sub(1))
            } else {
                0
            };
            Expr::Assert(
                kind,
                args[skip..].iter().map(|x| parse_arg(x, notes)).collect(),
            )
        }
        "typeof" if args.len() == 1 => Expr::TypeOf(boxed(&args[0], notes)),
        "let" if args.len() >= 3 && args.len() % 2 == 1 => {
            let (pairs, body) = args.split_at(args.len() - 1);
            let binds = pairs
                .chunks_exact(2)
                .filter_map(|p| Some((p[0].as_str()?.to_owned(), parse_arg(&p[1], notes))))
                .collect();
            Expr::Let(binds, boxed(&body[0], notes))
        }
        "var" if args.len() == 1 => Expr::Var(args[0].as_str().unwrap_or("").to_owned()),
        "pi" => Expr::Lit(Val::Num(std::f64::consts::PI)),
        "e" => Expr::Lit(Val::Num(std::f64::consts::E)),
        "ln2" => Expr::Lit(Val::Num(std::f64::consts::LN_2)),
        "index-of" if args.len() >= 2 => {
            Expr::IndexOf(boxed(&args[0], notes), boxed(&args[1], notes))
        }
        "slice" if args.len() >= 2 => Expr::Slice(
            boxed(&args[0], notes),
            boxed(&args[1], notes),
            args.get(2).map(|x| boxed(x, notes)),
        ),
        "to-rgba" if args.len() == 1 => Expr::ToRgba(boxed(&args[0], notes)),
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "ln" | "log10" | "log2" => {
            let m = match op {
                "sin" => MathOp::Sin,
                "cos" => MathOp::Cos,
                "tan" => MathOp::Tan,
                "asin" => MathOp::Asin,
                "acos" => MathOp::Acos,
                "atan" => MathOp::Atan,
                "ln" => MathOp::Ln,
                "log10" => MathOp::Log10,
                _ => MathOp::Log2,
            };
            Expr::Math(m, all(notes))
        }
        "+" => Expr::Math(MathOp::Add, all(notes)),
        "-" => Expr::Math(MathOp::Sub, all(notes)),
        "*" => Expr::Math(MathOp::Mul, all(notes)),
        "/" => Expr::Math(MathOp::Div, all(notes)),
        "%" => Expr::Math(MathOp::Rem, all(notes)),
        "^" => Expr::Math(MathOp::Pow, all(notes)),
        "min" => Expr::Math(MathOp::Min, all(notes)),
        "max" => Expr::Math(MathOp::Max, all(notes)),
        other => match MATH_ONE.iter().find(|(n, _)| *n == other) {
            Some((_, m)) => Expr::Math(*m, all(notes)),
            None => {
                notes.add(format!("“{other}” ifadesi"));
                Expr::Unsupported(other.to_owned())
            }
        },
    }
}

/// A legacy function: by zoom, or by a property; its type's default
/// exponential for numbers, colours and number arrays, interval otherwise.
fn function(v: &Json, notes: &mut Notes) -> Expr {
    let list = v["stops"].as_array().map(Vec::as_slice).unwrap_or_default();
    if list.iter().any(|s| s[0].is_object()) {
        notes.add("yakınlığa ve özniteliğe birlikte bağlı işlev");
        return Expr::Unsupported("zoom-and-property function".into());
    }
    let input = match v["property"].as_str() {
        Some(p) => Box::new(Expr::Get(Box::new(Expr::Lit(Val::Str(p.to_owned()))))),
        None => Box::new(Expr::Zoom),
    };
    let outputs: Vec<Val> = list.iter().map(|s| Val::from_json(&s[1])).collect();
    let interpolatable = outputs.iter().all(|o| match o {
        Val::Num(_) => true,
        Val::Str(s) => color::parse(s).is_some(),
        Val::Arr(a) => a.iter().all(|x| matches!(x, Val::Num(_))),
        _ => false,
    });
    let kind = v["type"].as_str().unwrap_or(
        if v["property"].is_string() && list.iter().any(|s| !s[0].is_number()) {
            "categorical"
        } else if interpolatable {
            "exponential"
        } else {
            "interval"
        },
    );
    let pairs = |out: &[Val]| -> Vec<(f64, Expr)> {
        list.iter()
            .zip(out)
            .filter_map(|(s, o)| Some((s[0].as_f64()?, Expr::Lit(o.clone()))))
            .collect()
    };
    match kind {
        "identity" => *input,
        "categorical" => Expr::Match(
            input,
            list.iter()
                .zip(&outputs)
                .map(|(s, o)| (vec![Val::from_json(&s[0])], Expr::Lit(o.clone())))
                .collect(),
            Box::new(Expr::Lit(Val::from_json(
                v.get("default").unwrap_or(&Json::Null),
            ))),
        ),
        "interval" => {
            let first = outputs.first().cloned().unwrap_or(Val::Null);
            Expr::Step(input, Box::new(Expr::Lit(first)), pairs(&outputs))
        }
        _ => {
            let base = v["base"].as_f64().unwrap_or(1.0);
            let interp = if base == 1.0 {
                Interp::Linear
            } else {
                Interp::Exponential(base)
            };
            Expr::Interpolate(interp, input, pairs(&outputs))
        }
    }
}

/// Whether a filter is an expression (MapLibre's `isExpressionFilter`) rather than a legacy filter.
fn is_expression_filter(f: &Json) -> bool {
    let Some(a) = f.as_array() else {
        return f.is_boolean();
    };
    let Some(op) = a.first().and_then(Json::as_str) else {
        return false;
    };
    match op {
        "has" => a.len() >= 2 && a[1] != "$id" && a[1] != "$type",
        "in" => a.len() >= 3 && (!a[1].is_string() || a[2].is_array()),
        "!in" | "!has" | "none" => false,
        "==" | "!=" | ">" | ">=" | "<" | "<=" => a.len() != 3 || a[1].is_array() || a[2].is_array(),
        "any" | "all" => a[1..]
            .iter()
            .all(|x| x.is_boolean() || is_expression_filter(x)),
        _ => true,
    }
}

/// A layer's filter, in either form.
pub fn filter(f: &Json, notes: &mut Notes) -> Expr {
    if is_expression_filter(f) {
        return parse_arg(f, notes);
    }
    legacy(f, notes)
}

fn legacy_key(k: &str) -> Expr {
    match k {
        "$type" => Expr::BaseType,
        "$id" => Expr::Id,
        other => Expr::Get(Box::new(Expr::Lit(Val::Str(other.to_owned())))),
    }
}

fn legacy(f: &Json, notes: &mut Notes) -> Expr {
    let a = f.as_array().map(Vec::as_slice).unwrap_or_default();
    let op = a.first().and_then(Json::as_str).unwrap_or("");
    let key = a.get(1).and_then(Json::as_str).unwrap_or("");
    let lit = |v: &Json| Box::new(Expr::Lit(Val::from_json(v)));
    match op {
        "all" => Expr::All(a[1..].iter().map(|x| legacy(x, notes)).collect()),
        "any" => Expr::Any(a[1..].iter().map(|x| legacy(x, notes)).collect()),
        "none" => Expr::Not(Box::new(Expr::Any(
            a[1..].iter().map(|x| legacy(x, notes)).collect(),
        ))),
        "has" => Expr::Has(Box::new(Expr::Lit(Val::Str(key.to_owned())))),
        "!has" => Expr::Not(Box::new(Expr::Has(Box::new(Expr::Lit(Val::Str(
            key.to_owned(),
        )))))),
        "in" | "!in" => {
            let list = Expr::Lit(Val::Arr(a[2..].iter().map(Val::from_json).collect()));
            let e = Expr::In(Box::new(legacy_key(key)), Box::new(list));
            if op == "in" {
                e
            } else {
                Expr::Not(Box::new(e))
            }
        }
        "==" | "!=" | "<" | "<=" | ">" | ">=" if a.len() == 3 => {
            let c = match op {
                "==" => Cmp::Eq,
                "!=" => Cmp::Ne,
                "<" => Cmp::Lt,
                "<=" => Cmp::Le,
                ">" => Cmp::Gt,
                _ => Cmp::Ge,
            };
            Expr::Cmp(c, Box::new(legacy_key(key)), lit(&a[2]))
        }
        _ => {
            notes.add(format!("“{op}” süzgeci"));
            Expr::Lit(Val::Bool(true))
        }
    }
}

fn equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => x == y,
        (Val::Str(x), Val::Str(y)) => x == y,
        (Val::Bool(x), Val::Bool(y)) => x == y,
        (Val::Null, Val::Null) => true,
        _ => false,
    }
}

fn compare(c: Cmp, a: &Val, b: &Val) -> bool {
    let ord = match (a, b) {
        (Val::Num(x), Val::Num(y)) => x.partial_cmp(y),
        (Val::Str(x), Val::Str(y)) => Some(x.cmp(y)),
        _ => None,
    };
    match c {
        Cmp::Eq => equal(a, b),
        Cmp::Ne => !equal(a, b),
        Cmp::Lt => ord.is_some_and(std::cmp::Ordering::is_lt),
        Cmp::Le => ord.is_some_and(std::cmp::Ordering::is_le),
        Cmp::Gt => ord.is_some_and(std::cmp::Ordering::is_gt),
        Cmp::Ge => ord.is_some_and(std::cmp::Ordering::is_ge),
    }
}

/// A cubic Bézier's y at x (Newton's steps on its x), as MapLibre's `UnitBezier`.
fn bezier(p: [f64; 4], x: f64) -> f64 {
    let [x1, y1, x2, y2] = p;
    let (cx, bx) = (3.0 * x1, 3.0 * (x2 - x1) - 3.0 * x1);
    let ax = 1.0 - cx - bx;
    let (cy, by) = (3.0 * y1, 3.0 * (y2 - y1) - 3.0 * y1);
    let ay = 1.0 - cy - by;
    let fx = |t: f64| ((ax * t + bx) * t + cx) * t;
    let dx = |t: f64| (3.0 * ax * t + 2.0 * bx) * t + cx;
    let mut t = x;
    for _ in 0..8 {
        let e = fx(t) - x;
        if e.abs() < 1e-7 {
            break;
        }
        let d = dx(t);
        if d.abs() < 1e-7 {
            break;
        }
        t -= e / d;
    }
    let t = t.clamp(0.0, 1.0);
    ((ay * t + by) * t + cy) * t
}

fn lerp(a: &Val, b: &Val, t: f64) -> Val {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => Val::Num(x + (y - x) * t),
        (Val::Arr(x), Val::Arr(y)) if x.len() == y.len() => {
            Val::Arr(x.iter().zip(y).map(|(p, q)| lerp(p, q, t)).collect())
        }
        _ => match (a.color(), b.color()) {
            (Some(p), Some(q)) => Val::Color([
                p[0] + (q[0] - p[0]) * t,
                p[1] + (q[1] - p[1]) * t,
                p[2] + (q[2] - p[2]) * t,
                p[3] + (q[3] - p[3]) * t,
            ]),
            _ => {
                if t < 1.0 {
                    a.clone()
                } else {
                    b.clone()
                }
            }
        },
    }
}

/// An expression's value.
pub fn eval(e: &Expr, cx: &Ctx<'_>) -> Val {
    match e {
        Expr::Lit(v) => v.clone(),
        Expr::Get(k) => match (eval(k, cx), cx.feature) {
            (Val::Str(key), Some(f)) => f.property(&key).unwrap_or(Val::Null),
            _ => Val::Null,
        },
        Expr::Has(k) => match (eval(k, cx), cx.feature) {
            (Val::Str(key), Some(f)) => Val::Bool(f.property(&key).is_some()),
            _ => Val::Bool(false),
        },
        Expr::Zoom => Val::Num(cx.zoom),
        Expr::GeometryType => cx
            .feature
            .map_or(Val::Null, |f| Val::Str(f.geometry_type().to_owned())),
        Expr::BaseType => cx.feature.map_or(Val::Null, |f| {
            Val::Str(f.geometry_type().trim_start_matches("Multi").to_owned())
        }),
        Expr::Id => cx.feature.and_then(|f| f.id()).map_or(Val::Null, Val::Num),
        Expr::Not(x) => Val::Bool(!matches!(eval(x, cx), Val::Bool(true))),
        Expr::Cmp(c, a, b) => Val::Bool(compare(*c, &eval(a, cx), &eval(b, cx))),
        Expr::All(list) => Val::Bool(list.iter().all(|x| matches!(eval(x, cx), Val::Bool(true)))),
        Expr::Any(list) => Val::Bool(list.iter().any(|x| matches!(eval(x, cx), Val::Bool(true)))),
        Expr::In(n, h) => {
            let needle = eval(n, cx);
            Val::Bool(match eval(h, cx) {
                Val::Arr(list) => list.iter().any(|x| equal(x, &needle)),
                Val::Str(s) => needle.str().is_some_and(|t| s.contains(t)),
                _ => false,
            })
        }
        Expr::Match(input, cases, fallback) => {
            let v = eval(input, cx);
            cases
                .iter()
                .find(|(labels, _)| labels.iter().any(|l| equal(l, &v)))
                .map_or_else(|| eval(fallback, cx), |(_, out)| eval(out, cx))
        }
        Expr::Case(branches, fallback) => branches
            .iter()
            .find(|(c, _)| matches!(eval(c, cx), Val::Bool(true)))
            .map_or_else(|| eval(fallback, cx), |(_, out)| eval(out, cx)),
        Expr::Coalesce(list) => list
            .iter()
            .map(|x| eval(x, cx))
            .find(|v| *v != Val::Null)
            .unwrap_or(Val::Null),
        Expr::Step(input, first, stops) => {
            let Some(x) = eval(input, cx).num() else {
                return eval(first, cx);
            };
            match stops.iter().rposition(|(z, _)| x >= *z) {
                Some(i) => eval(&stops[i].1, cx),
                None => eval(first, cx),
            }
        }
        Expr::Interpolate(kind, input, stops) => {
            let (Some(x), Some(first), Some(last)) =
                (eval(input, cx).num(), stops.first(), stops.last())
            else {
                return Val::Null;
            };
            if x <= first.0 {
                return eval(&first.1, cx);
            }
            if x >= last.0 {
                return eval(&last.1, cx);
            }
            let i = stops.iter().rposition(|(z, _)| x >= *z).unwrap_or(0);
            let (z0, z1) = (stops[i].0, stops[i + 1].0);
            let span = z1 - z0;
            let p = if span > 0.0 { (x - z0) / span } else { 0.0 };
            let t = match kind {
                Interp::Linear => p,
                Interp::Exponential(base) => {
                    (libm::pow(*base, x - z0) - 1.0) / (libm::pow(*base, span) - 1.0)
                }
                Interp::Bezier(b) => bezier(*b, p),
            };
            lerp(&eval(&stops[i].1, cx), &eval(&stops[i + 1].1, cx), t)
        }
        Expr::Concat(list) => Val::Str(list.iter().map(|x| eval(x, cx).text()).collect()),
        Expr::ToString(x) => Val::Str(eval(x, cx).text()),
        Expr::ToNumber(list) => list
            .iter()
            .find_map(|x| match eval(x, cx) {
                Val::Num(n) => Some(n),
                Val::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
                Val::Null => Some(0.0),
                Val::Str(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            })
            .map_or(Val::Null, Val::Num),
        Expr::ToBoolean(x) => Val::Bool(match eval(x, cx) {
            Val::Bool(b) => b,
            Val::Num(n) => n != 0.0 && !n.is_nan(),
            Val::Str(s) => !s.is_empty(),
            Val::Null => false,
            _ => true,
        }),
        Expr::ToColor(list) => list
            .iter()
            .find_map(|x| eval(x, cx).color())
            .map_or(Val::Null, Val::Color),
        Expr::Upcase(x) => Val::Str(eval(x, cx).text().to_uppercase()),
        Expr::Downcase(x) => Val::Str(eval(x, cx).text().to_lowercase()),
        Expr::Length(x) => match eval(x, cx) {
            Val::Str(s) => Val::Num(s.chars().count() as f64),
            Val::Arr(a) => Val::Num(a.len() as f64),
            _ => Val::Null,
        },
        Expr::At(i, a) => match (eval(i, cx).num(), eval(a, cx)) {
            (Some(i), Val::Arr(list)) if i >= 0.0 => {
                list.get(i as usize).cloned().unwrap_or(Val::Null)
            }
            _ => Val::Null,
        },
        Expr::Rgba(list) => {
            let n: Vec<f64> = list.iter().filter_map(|x| eval(x, cx).num()).collect();
            if n.len() < 3 {
                return Val::Null;
            }
            Val::Color([
                n[0] / 255.0,
                n[1] / 255.0,
                n[2] / 255.0,
                n.get(3).copied().unwrap_or(1.0),
            ])
        }
        Expr::Math(op, list) => {
            let n: Vec<f64> = list.iter().filter_map(|x| eval(x, cx).num()).collect();
            if n.len() != list.len() || n.is_empty() {
                return Val::Null;
            }
            Val::Num(match op {
                MathOp::Add => n.iter().sum(),
                MathOp::Mul => n.iter().product(),
                MathOp::Sub if n.len() == 1 => -n[0],
                MathOp::Sub => n[0] - n[1],
                MathOp::Div => n[0] / n.get(1).copied().unwrap_or(1.0),
                MathOp::Rem => n[0] % n.get(1).copied().unwrap_or(1.0),
                MathOp::Pow => libm::pow(n[0], n.get(1).copied().unwrap_or(1.0)),
                MathOp::Min => n.iter().copied().fold(f64::INFINITY, f64::min),
                MathOp::Max => n.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                MathOp::Abs => n[0].abs(),
                MathOp::Floor => n[0].floor(),
                MathOp::Ceil => n[0].ceil(),
                MathOp::Round => n[0].round(),
                MathOp::Sqrt => n[0].sqrt(),
                MathOp::Sin => libm::sin(n[0]),
                MathOp::Cos => libm::cos(n[0]),
                MathOp::Tan => libm::tan(n[0]),
                MathOp::Asin => libm::asin(n[0]),
                MathOp::Acos => libm::acos(n[0]),
                MathOp::Atan => libm::atan(n[0]),
                MathOp::Ln => libm::log(n[0]),
                MathOp::Log10 => libm::log10(n[0]),
                MathOp::Log2 => libm::log2(n[0]),
            })
        }
        Expr::Assert(kind, list) => list
            .iter()
            .map(|x| eval(x, cx))
            .find(|v| match (*kind, v) {
                ("number", Val::Num(_))
                | ("string", Val::Str(_))
                | ("boolean", Val::Bool(_))
                | ("array", Val::Arr(_)) => true,
                ("object", _) => false,
                _ => false,
            })
            .unwrap_or(Val::Null),
        Expr::TypeOf(x) => Val::Str(
            match eval(x, cx) {
                Val::Null => "null",
                Val::Bool(_) => "boolean",
                Val::Num(_) => "number",
                Val::Str(_) => "string",
                Val::Color(_) => "color",
                Val::Arr(_) => "array",
            }
            .to_owned(),
        ),
        Expr::Let(binds, body) => {
            let mut vars: Vec<(String, Val)> = cx.vars.to_vec();
            for (name, e) in binds {
                let v = eval(
                    e,
                    &Ctx {
                        zoom: cx.zoom,
                        feature: cx.feature,
                        vars: &vars,
                    },
                );
                vars.push((name.clone(), v));
            }
            eval(
                body,
                &Ctx {
                    zoom: cx.zoom,
                    feature: cx.feature,
                    vars: &vars,
                },
            )
        }
        Expr::Var(name) => cx
            .vars
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map_or(Val::Null, |(_, v)| v.clone()),
        Expr::IndexOf(needle, haystack) => {
            let n = eval(needle, cx);
            match eval(haystack, cx) {
                Val::Arr(list) => Val::Num(
                    list.iter()
                        .position(|x| equal(x, &n))
                        .map_or(-1.0, |i| i as f64),
                ),
                Val::Str(s) => Val::Num(
                    n.str()
                        .and_then(|t| s.find(t))
                        .map_or(-1.0, |i| s[..i].chars().count() as f64),
                ),
                _ => Val::Null,
            }
        }
        Expr::Slice(input, start, end) => {
            let from = eval(start, cx).num().unwrap_or(0.0).max(0.0) as usize;
            let to = end
                .as_ref()
                .and_then(|e| eval(e, cx).num())
                .map(|t| t.max(0.0) as usize);
            match eval(input, cx) {
                Val::Arr(list) => {
                    let to = to.unwrap_or(list.len()).min(list.len());
                    Val::Arr(
                        list.get(from.min(to)..to)
                            .map(<[Val]>::to_vec)
                            .unwrap_or_default(),
                    )
                }
                Val::Str(s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let to = to.unwrap_or(chars.len()).min(chars.len());
                    Val::Str(chars[from.min(to)..to].iter().collect())
                }
                _ => Val::Null,
            }
        }
        Expr::ToRgba(x) => eval(x, cx).color().map_or(Val::Null, |c| {
            Val::Arr(vec![
                Val::Num(c[0] * 255.0),
                Val::Num(c[1] * 255.0),
                Val::Num(c[2] * 255.0),
                Val::Num(c[3]),
            ])
        }),
        Expr::Unsupported(_) => Val::Null,
    }
}

/// Whether an expression reads the feature (it is evaluated per feature)
/// rather than the zoom alone.
pub fn reads_feature(e: &Expr) -> bool {
    match e {
        Expr::Get(_)
        | Expr::Has(_)
        | Expr::GeometryType
        | Expr::BaseType
        | Expr::Id
        | Expr::Var(_) => true,
        Expr::Lit(_) | Expr::Zoom | Expr::Unsupported(_) => false,
        Expr::Assert(_, l) => l.iter().any(reads_feature),
        Expr::TypeOf(x) | Expr::ToRgba(x) => reads_feature(x),
        Expr::Let(b, body) => reads_feature(body) || b.iter().any(|(_, x)| reads_feature(x)),
        Expr::IndexOf(a, b) => reads_feature(a) || reads_feature(b),
        Expr::Slice(a, b, c) => {
            reads_feature(a) || reads_feature(b) || c.as_deref().is_some_and(reads_feature)
        }
        Expr::Not(x)
        | Expr::ToString(x)
        | Expr::ToBoolean(x)
        | Expr::Upcase(x)
        | Expr::Downcase(x)
        | Expr::Length(x) => reads_feature(x),
        Expr::Cmp(_, a, b) | Expr::In(a, b) | Expr::At(a, b) => {
            reads_feature(a) || reads_feature(b)
        }
        Expr::All(l)
        | Expr::Any(l)
        | Expr::Coalesce(l)
        | Expr::Concat(l)
        | Expr::ToNumber(l)
        | Expr::ToColor(l)
        | Expr::Rgba(l)
        | Expr::Math(_, l) => l.iter().any(reads_feature),
        Expr::Match(i, c, f) => {
            reads_feature(i) || reads_feature(f) || c.iter().any(|(_, x)| reads_feature(x))
        }
        Expr::Case(b, f) => {
            reads_feature(f) || b.iter().any(|(c, x)| reads_feature(c) || reads_feature(x))
        }
        Expr::Step(i, f, s) => {
            reads_feature(i) || reads_feature(f) || s.iter().any(|(_, x)| reads_feature(x))
        }
        Expr::Interpolate(_, i, s) => reads_feature(i) || s.iter().any(|(_, x)| reads_feature(x)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct F(Vec<(&'static str, Val)>, &'static str);

    impl Feature for F {
        fn property(&self, key: &str) -> Option<Val> {
            self.0
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        }
        fn geometry_type(&self) -> &str {
            self.1
        }
        fn id(&self) -> Option<f64> {
            Some(42.0)
        }
    }

    fn at(e: &Expr, zoom: f64, f: &F) -> Val {
        eval(e, &Ctx::new(zoom, Some(f)))
    }

    #[test]
    fn zoom_functions_old_and_new() {
        let mut n = Notes::default();
        let f = F(vec![], "LineString");
        // Liberty's road width: exponential base 1.2 from 6.5 px at 13 to 20 px at 20.
        let e = parse(
            &json!([
                "interpolate",
                ["exponential", 1.2],
                ["zoom"],
                13,
                6.5,
                20,
                20
            ]),
            &mut n,
        );
        assert_eq!(at(&e, 13.0, &f), Val::Num(6.5));
        let Val::Num(w) = at(&e, 16.5, &f) else {
            panic!()
        };
        let t = (1.2f64.powf(3.5) - 1.0) / (1.2f64.powf(7.0) - 1.0);
        assert!((w - (6.5 + 13.5 * t)).abs() < 1e-12);
        let old = parse(
            &json!({"base": 1.2, "stops": [[13, 6.5], [20, 20]]}),
            &mut n,
        );
        assert_eq!(at(&old, 16.5, &f), Val::Num(w));
        let step = parse(&json!({"stops": [[8, "a"], [12, "b"]]}), &mut n);
        assert_eq!(at(&step, 5.0, &f), Val::Str("a".into()));
        assert_eq!(at(&step, 12.0, &f), Val::Str("b".into()));
        let c = parse(
            &json!([
                "interpolate",
                ["linear"],
                ["zoom"],
                0,
                "#000000",
                10,
                "#ffffff"
            ]),
            &mut n,
        );
        assert_eq!(at(&c, 5.0, &f), Val::Color([0.5, 0.5, 0.5, 1.0]));
        assert!(n.0.is_empty());
    }

    #[test]
    fn data_expressions_and_filters() {
        let mut n = Notes::default();
        let f = F(
            vec![
                ("class", Val::Str("primary".into())),
                ("rank", Val::Num(3.0)),
                ("name", Val::Str("Ankara".into())),
            ],
            "MultiPolygon",
        );
        let m = parse(
            &json!([
                "match",
                ["get", "class"],
                ["motorway", "trunk"],
                1,
                "primary",
                2,
                0
            ]),
            &mut n,
        );
        assert_eq!(at(&m, 10.0, &f), Val::Num(2.0));
        let c = parse(
            &json!(["case", ["<", ["get", "rank"], 2], "big", "small"]),
            &mut n,
        );
        assert_eq!(at(&c, 10.0, &f), Val::Str("small".into()));
        let t = parse(
            &json!([
                "concat",
                ["get", "name"],
                " (",
                ["to-string", ["get", "rank"]],
                ")"
            ]),
            &mut n,
        );
        assert_eq!(at(&t, 10.0, &f), Val::Str("Ankara (3)".into()));
        assert_eq!(
            at(
                &parse(
                    &json!(["coalesce", ["get", "name:tr"], ["get", "name"]]),
                    &mut n
                ),
                0.0,
                &f
            ),
            Val::Str("Ankara".into())
        );
        // A legacy filter: $type without Multi, in, has.
        let legacy = filter(
            &json!([
                "all",
                ["==", "$type", "Polygon"],
                ["in", "class", "primary", "secondary"],
                ["!has", "brunnel"]
            ]),
            &mut n,
        );
        assert_eq!(at(&legacy, 0.0, &f), Val::Bool(true));
        let expr = filter(
            &json!([
                "match",
                ["geometry-type"],
                ["MultiPolygon", "Polygon"],
                true,
                false
            ]),
            &mut n,
        );
        assert_eq!(at(&expr, 0.0, &f), Val::Bool(true));
        let id = filter(&json!(["==", "$id", 42]), &mut n);
        assert_eq!(at(&id, 0.0, &f), Val::Bool(true));
        let _ = parse(&json!(["within", {}]), &mut n);
        assert_eq!(n.0, vec!["“within” ifadesi"]);
        let l = parse(
            &json!(["let", "r", ["get", "rank"], ["*", ["var", "r"], 2]]),
            &mut n,
        );
        assert_eq!(at(&l, 0.0, &f), Val::Num(6.0));
        assert_eq!(
            at(
                &parse(&json!(["number", ["get", "name"], ["get", "rank"]]), &mut n),
                0.0,
                &f
            ),
            Val::Num(3.0)
        );
        assert_eq!(
            at(
                &parse(&json!(["slice", ["get", "name"], 1, 3]), &mut n),
                0.0,
                &f
            ),
            Val::Str("nk".into())
        );
        // A literal array is no expression.
        assert_eq!(
            parse(&json!(["Noto Sans Regular"]), &mut n),
            Expr::Lit(Val::Arr(vec![Val::Str("Noto Sans Regular".into())]))
        );
        assert_eq!(
            parse(&json!([2, 1]), &mut n),
            Expr::Lit(Val::Arr(vec![Val::Num(2.0), Val::Num(1.0)]))
        );
    }
}
