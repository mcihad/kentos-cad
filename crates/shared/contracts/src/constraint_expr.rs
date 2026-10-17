//! The expressions of a drawing's parameters (docs/adr/0334 §3; AutoCAD's parameter manager): numbers, units on
//! numbers (`2.5m`, `250cm`, `30deg`), the other parameters by name, `+ - * / ^`, parentheses and a few functions.
//! A length's value is in the drawing's unit, an angle's in the project's angle unit; the functions take and give
//! angles in that unit. The rules are here, beside the constraints, so the file format, the documents, the commands
//! and the Python reader (`tools/kcad/kcad.py`) hold the same grammar.

use std::collections::{BTreeMap, BTreeSet};

/// The longest expression kept.
pub const EXPRESSION_MAX: usize = 1_000;

/// The longest parameter name.
pub const PARAMETER_NAME_MAX: usize = 64;

/// The deepest an expression nests (parentheses, calls, signs and powers).
const DEPTH_MAX: usize = 64;

/// The functions an expression may call, with how many arguments each takes.
pub const PARAMETER_FUNCTIONS: [(&str, usize); 13] = [
    ("sin", 1),
    ("cos", 1),
    ("tan", 1),
    ("asin", 1),
    ("acos", 1),
    ("atan", 1),
    ("sqrt", 1),
    ("abs", 1),
    ("round", 1),
    ("floor", 1),
    ("ceil", 1),
    ("min", 2),
    ("max", 2),
];

/// The names no parameter may take: the functions and `pi`.
pub fn parameter_name_reserved(name: &str) -> bool {
    name == "pi" || PARAMETER_FUNCTIONS.iter().any(|(f, _)| *f == name)
}

/// Whether `name` is a parameter's name: a letter or `_` first, then letters, digits and `_` (Turkish letters too),
/// at most [`PARAMETER_NAME_MAX`] characters, not reserved.
pub fn parameter_name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
        && name.chars().count() <= PARAMETER_NAME_MAX
        && !parameter_name_reserved(name)
}

/// The unit a number is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralUnit {
    Metre,
    Centimetre,
    Millimetre,
    Kilometre,
    Foot,
    Inch,
    Degree,
    Grad,
    Radian,
}

const UNITS: [(&str, LiteralUnit); 9] = [
    ("mm", LiteralUnit::Millimetre),
    ("cm", LiteralUnit::Centimetre),
    ("km", LiteralUnit::Kilometre),
    ("m", LiteralUnit::Metre),
    ("ft", LiteralUnit::Foot),
    ("in", LiteralUnit::Inch),
    ("deg", LiteralUnit::Degree),
    ("grad", LiteralUnit::Grad),
    ("rad", LiteralUnit::Radian),
];

/// A parsed expression.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamExpr {
    Number(f64, Option<LiteralUnit>),
    Pi,
    Name(String),
    Neg(Box<ParamExpr>),
    Add(Box<ParamExpr>, Box<ParamExpr>),
    Sub(Box<ParamExpr>, Box<ParamExpr>),
    Mul(Box<ParamExpr>, Box<ParamExpr>),
    Div(Box<ParamExpr>, Box<ParamExpr>),
    Pow(Box<ParamExpr>, Box<ParamExpr>),
    Call(&'static str, Vec<ParamExpr>),
}

struct Parser<'a> {
    text: &'a [char],
    at: usize,
    depth: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.text.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.space();
        self.text.get(self.at).copied()
    }

    fn fail(&self, words: &str) -> String {
        format!("{}. karakterde: {words}", self.at + 1)
    }

    fn deeper(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > DEPTH_MAX {
            return Err(self.fail("ifade çok derin"));
        }
        Ok(())
    }

    fn sum(&mut self) -> Result<ParamExpr, String> {
        let mut left = self.product()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.at += 1;
                    left = ParamExpr::Add(Box::new(left), Box::new(self.product()?));
                }
                Some('-') => {
                    self.at += 1;
                    left = ParamExpr::Sub(Box::new(left), Box::new(self.product()?));
                }
                _ => return Ok(left),
            }
        }
    }

    fn product(&mut self) -> Result<ParamExpr, String> {
        let mut left = self.power()?;
        loop {
            match self.peek() {
                Some('*') => {
                    self.at += 1;
                    left = ParamExpr::Mul(Box::new(left), Box::new(self.power()?));
                }
                Some('/') => {
                    self.at += 1;
                    left = ParamExpr::Div(Box::new(left), Box::new(self.power()?));
                }
                _ => return Ok(left),
            }
        }
    }

    /// `a ^ b ^ c` is `a ^ (b ^ c)`; `-a ^ b` is `-(a ^ b)`.
    fn power(&mut self) -> Result<ParamExpr, String> {
        self.deeper()?;
        let out = match self.peek() {
            Some('-') => {
                self.at += 1;
                Ok(ParamExpr::Neg(Box::new(self.power()?)))
            }
            Some('+') => {
                self.at += 1;
                self.power()
            }
            _ => {
                let base = self.atom()?;
                if self.peek() == Some('^') {
                    self.at += 1;
                    Ok(ParamExpr::Pow(Box::new(base), Box::new(self.power()?)))
                } else {
                    Ok(base)
                }
            }
        };
        self.depth -= 1;
        out
    }

    fn atom(&mut self) -> Result<ParamExpr, String> {
        match self.peek() {
            Some('(') => {
                self.at += 1;
                self.deeper()?;
                let inner = self.sum()?;
                self.depth -= 1;
                if self.peek() != Some(')') {
                    return Err(self.fail("“)” bekleniyordu"));
                }
                self.at += 1;
                Ok(inner)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() || c == '_' => self.word(),
            Some(c) => Err(self.fail(&format!("“{c}” beklenmiyordu"))),
            None => Err(self.fail("ifade yarım kaldı")),
        }
    }

    fn number(&mut self) -> Result<ParamExpr, String> {
        let start = self.at;
        let digits = |p: &mut Self| {
            let from = p.at;
            while p.text.get(p.at).is_some_and(|c| c.is_ascii_digit()) {
                p.at += 1;
            }
            p.at > from
        };
        let whole = digits(self);
        let mut fraction = false;
        if self.text.get(self.at) == Some(&'.') {
            self.at += 1;
            fraction = digits(self);
        }
        if !whole && !fraction {
            return Err(self.fail("sayı okunamadı"));
        }
        // An exponent only when a digit follows `e` (so `2e` is not a number and a unit).
        if matches!(self.text.get(self.at), Some('e' | 'E')) {
            let back = self.at;
            self.at += 1;
            if matches!(self.text.get(self.at), Some('+' | '-')) {
                self.at += 1;
            }
            if !digits(self) {
                self.at = back;
            }
        }
        let written: String = self.text[start..self.at].iter().collect();
        let value: f64 = written
            .parse()
            .map_err(|_| format!("{}. karakterde: “{written}” sayı değil", start + 1))?;
        // A unit written right after the number.
        let rest: String = self.text[self.at..]
            .iter()
            .take_while(|c| c.is_alphanumeric() || **c == '_')
            .collect();
        let unit = if rest.is_empty() {
            None
        } else {
            let Some((word, unit)) = UNITS.iter().find(|(w, _)| *w == rest) else {
                return Err(format!(
                    "{}. karakterde: “{rest}” bilinen bir birim değil (m, cm, mm, km, ft, in, deg, grad, rad)",
                    self.at + 1
                ));
            };
            self.at += word.chars().count();
            Some(*unit)
        };
        Ok(ParamExpr::Number(value, unit))
    }

    fn word(&mut self) -> Result<ParamExpr, String> {
        let start = self.at;
        while self
            .text
            .get(self.at)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_')
        {
            self.at += 1;
        }
        let name: String = self.text[start..self.at].iter().collect();
        if self.peek() == Some('(') {
            let Some((function, count)) = PARAMETER_FUNCTIONS.iter().find(|(f, _)| *f == name)
            else {
                return Err(format!(
                    "{}. karakterde: “{name}” bilinen bir işlev değil",
                    start + 1
                ));
            };
            self.at += 1;
            self.deeper()?;
            let mut args = Vec::new();
            if self.peek() != Some(')') {
                loop {
                    args.push(self.sum()?);
                    match self.peek() {
                        Some(',') | Some(';') => self.at += 1,
                        _ => break,
                    }
                }
            }
            self.depth -= 1;
            if self.peek() != Some(')') {
                return Err(self.fail("“)” bekleniyordu"));
            }
            self.at += 1;
            if args.len() != *count {
                return Err(format!(
                    "{}. karakterde: “{function}” {count} değer alır, {} verildi",
                    start + 1,
                    args.len()
                ));
            }
            return Ok(ParamExpr::Call(function, args));
        }
        if name == "pi" {
            return Ok(ParamExpr::Pi);
        }
        if PARAMETER_FUNCTIONS.iter().any(|(f, _)| *f == name) {
            return Err(format!(
                "{}. karakterde: “{name}” bir işlevdir, ardından “(” gelir",
                start + 1
            ));
        }
        Ok(ParamExpr::Name(name))
    }
}

/// Parses an expression; the error says where and why, in Turkish.
pub fn parse_param_expr(text: &str) -> Result<ParamExpr, String> {
    if text.chars().count() > EXPRESSION_MAX {
        return Err(format!("ifade {EXPRESSION_MAX} karakterden uzun"));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut p = Parser {
        text: &chars,
        at: 0,
        depth: 0,
    };
    if p.peek().is_none() {
        return Err("ifade boş".into());
    }
    let expr = p.sum()?;
    if let Some(c) = p.peek() {
        return Err(p.fail(&format!("“{c}” beklenmiyordu")));
    }
    Ok(expr)
}

/// The parameter names an expression reads, each once, in sorted order.
pub fn param_expr_names(expr: &ParamExpr) -> BTreeSet<String> {
    fn walk(e: &ParamExpr, out: &mut BTreeSet<String>) {
        match e {
            ParamExpr::Number(..) | ParamExpr::Pi => {}
            ParamExpr::Name(n) => {
                out.insert(n.clone());
            }
            ParamExpr::Neg(a) => walk(a, out),
            ParamExpr::Add(a, b)
            | ParamExpr::Sub(a, b)
            | ParamExpr::Mul(a, b)
            | ParamExpr::Div(a, b)
            | ParamExpr::Pow(a, b) => {
                walk(a, out);
                walk(b, out);
            }
            ParamExpr::Call(_, args) => args.iter().for_each(|a| walk(a, out)),
        }
    }
    let mut out = BTreeSet::new();
    walk(expr, &mut out);
    out
}

/// `text` with every name `from` it reads written `to` (a parameter renamed, docs/adr/0336 §1): only whole names, so
/// numbers, their units, functions and other names stay as written, and so does every space.
pub fn rename_in_expression(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            // A word right after a number is its unit (`2.5m`), not a name.
            let unit = start > 0 && (chars[start - 1].is_ascii_digit() || chars[start - 1] == '.');
            out.push_str(if word == from && !unit { to } else { &word });
        } else if c.is_ascii_digit() || c == '.' {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                out.push(chars[i]);
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// What an expression's units mean in a drawing: a metre in its length unit and a radian in its angle unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamUnits {
    /// Metres per drawing unit (1 for metres, 0.01 for centimetres …).
    pub metres_per_unit: f64,
    /// Radians per angle unit (π/180 for degrees, π/200 for grads).
    pub radians_per_unit: f64,
}

impl ParamUnits {
    /// A project's: its drawing unit and its angle unit.
    pub fn of(settings: &crate::ProjectSettings) -> Self {
        let (mul, div) = settings.unit().metres();
        ParamUnits {
            metres_per_unit: mul / div,
            radians_per_unit: match settings.angle_unit {
                crate::AngleUnit::Deg => std::f64::consts::PI / 180.0,
                crate::AngleUnit::Grad => std::f64::consts::PI / 200.0,
            },
        }
    }

    fn literal(&self, value: f64, unit: Option<LiteralUnit>) -> f64 {
        let Some(unit) = unit else {
            return value;
        };
        let metres = |m: f64| value * m / self.metres_per_unit;
        let radians = |r: f64| value * r / self.radians_per_unit;
        match unit {
            LiteralUnit::Metre => metres(1.0),
            LiteralUnit::Centimetre => metres(0.01),
            LiteralUnit::Millimetre => metres(0.001),
            LiteralUnit::Kilometre => metres(1000.0),
            LiteralUnit::Foot => metres(0.3048),
            LiteralUnit::Inch => metres(0.0254),
            LiteralUnit::Degree => radians(std::f64::consts::PI / 180.0),
            LiteralUnit::Grad => radians(std::f64::consts::PI / 200.0),
            LiteralUnit::Radian => radians(1.0),
        }
    }
}

/// The value of an expression, its names read through `value`; an error when a name has no value, a division by
/// zero, a function outside its domain or a result not finite.
pub fn eval_param_expr(
    expr: &ParamExpr,
    units: ParamUnits,
    value: &dyn Fn(&str) -> Option<f64>,
) -> Result<f64, String> {
    let to_rad = |a: f64| a * units.radians_per_unit;
    let from_rad = |r: f64| r / units.radians_per_unit;
    let v = match expr {
        ParamExpr::Number(n, unit) => units.literal(*n, *unit),
        ParamExpr::Pi => from_rad(std::f64::consts::PI),
        ParamExpr::Name(n) => value(n).ok_or_else(|| format!("“{n}” diye bir parametre yok"))?,
        ParamExpr::Neg(a) => -eval_param_expr(a, units, value)?,
        ParamExpr::Add(a, b) => {
            eval_param_expr(a, units, value)? + eval_param_expr(b, units, value)?
        }
        ParamExpr::Sub(a, b) => {
            eval_param_expr(a, units, value)? - eval_param_expr(b, units, value)?
        }
        ParamExpr::Mul(a, b) => {
            eval_param_expr(a, units, value)? * eval_param_expr(b, units, value)?
        }
        ParamExpr::Div(a, b) => {
            let d = eval_param_expr(b, units, value)?;
            if d == 0.0 {
                return Err("sıfıra bölme".into());
            }
            eval_param_expr(a, units, value)? / d
        }
        ParamExpr::Pow(a, b) => libm::pow(
            eval_param_expr(a, units, value)?,
            eval_param_expr(b, units, value)?,
        ),
        ParamExpr::Call(f, args) => {
            let x = eval_param_expr(&args[0], units, value)?;
            let inverse = |r: f64, name: &str| {
                if (-1.0..=1.0).contains(&x) {
                    Ok(from_rad(r))
                } else {
                    Err(format!("“{name}” yalnız -1 ile 1 arasını alır"))
                }
            };
            match *f {
                "sin" => libm::sin(to_rad(x)),
                "cos" => libm::cos(to_rad(x)),
                "tan" => libm::tan(to_rad(x)),
                "asin" => inverse(libm::asin(x), "asin")?,
                "acos" => inverse(libm::acos(x), "acos")?,
                "atan" => from_rad(libm::atan(x)),
                "sqrt" if x < 0.0 => return Err("“sqrt” eksi sayı almaz".into()),
                "sqrt" => libm::sqrt(x),
                "abs" => x.abs(),
                // Halves away from zero, as the display rule does.
                "round" => libm::round(x),
                "floor" => libm::floor(x),
                "ceil" => libm::ceil(x),
                "min" => x.min(eval_param_expr(&args[1], units, value)?),
                "max" => x.max(eval_param_expr(&args[1], units, value)?),
                other => return Err(format!("“{other}” bilinen bir işlev değil")),
            }
        }
    };
    if !v.is_finite() {
        return Err("sonuç sonlu bir sayı değil".into());
    }
    Ok(v)
}

/// The order to work out parameters in, each after the names it reads; or the names that read each other round in
/// a circle, in sorted order. Names an expression reads that are not in `reads`' keys are left to the caller.
pub fn param_order(reads: &BTreeMap<String, BTreeSet<String>>) -> Result<Vec<String>, Vec<String>> {
    // Kahn's algorithm over the names in sorted order: the same order on every host.
    let mut waiting: BTreeMap<&str, usize> = BTreeMap::new();
    let mut readers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, names) in reads {
        let known = names.iter().filter(|n| reads.contains_key(*n)).count();
        waiting.insert(name, known);
        for n in names.iter().filter(|n| reads.contains_key(*n)) {
            readers.entry(n.as_str()).or_default().push(name);
        }
    }
    let mut ready: BTreeSet<&str> = waiting
        .iter()
        .filter(|(_, c)| **c == 0)
        .map(|(n, _)| *n)
        .collect();
    let mut order = Vec::with_capacity(reads.len());
    while let Some(name) = ready.pop_first() {
        order.push(name.to_string());
        for reader in readers.get(name).into_iter().flatten() {
            if let Some(count) = waiting.get_mut(reader) {
                *count -= 1;
                if *count == 0 {
                    ready.insert(reader);
                }
            }
        }
    }
    if order.len() == reads.len() {
        return Ok(order);
    }
    // What is left reads round; keep only the names on a circle (not those that merely read one).
    let left: BTreeSet<&str> = waiting
        .iter()
        .filter(|(_, c)| **c > 0)
        .map(|(n, _)| *n)
        .collect();
    let on_circle = |start: &str| {
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            for next in reads.get(n).into_iter().flatten() {
                if next == start {
                    return true;
                }
                if left.contains(next.as_str()) && seen.insert(next.as_str()) {
                    stack.push(next);
                }
            }
        }
        false
    };
    Err(left
        .iter()
        .filter(|n| on_circle(n))
        .map(|n| n.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_touches_only_the_name() {
        assert_eq!(
            rename_in_expression("d1 * 2 + d10 / d1", "d1", "genislik"),
            "genislik * 2 + d10 / genislik"
        );
        assert_eq!(rename_in_expression("2.5m + m", "m", "boy"), "2.5m + boy");
        assert_eq!(
            rename_in_expression("max(a, sin(a))", "a", "b"),
            "max(b, sin(b))"
        );
    }

    const METRES: ParamUnits = ParamUnits {
        metres_per_unit: 1.0,
        radians_per_unit: std::f64::consts::PI / 180.0,
    };

    fn value(text: &str) -> Result<f64, String> {
        let e = parse_param_expr(text)?;
        eval_param_expr(&e, METRES, &|n| match n {
            "d1" => Some(4.0),
            "genişlik" => Some(2.5),
            _ => None,
        })
    }

    #[test]
    fn arithmetic_names_units_and_functions_work_out() {
        assert_eq!(value("1 + 2 * 3").unwrap(), 7.0);
        assert_eq!(value("-2 ^ 2").unwrap(), -4.0);
        assert_eq!(value("2 ^ 3 ^ 2").unwrap(), 512.0);
        assert_eq!(value("d1 / 2 + genişlik").unwrap(), 4.5);
        assert_eq!(value("250cm").unwrap(), 2.5);
        assert!((value("sin(30)").unwrap() - 0.5).abs() < 1e-15);
        assert!((value("asin(1)").unwrap() - 90.0).abs() < 1e-12);
        assert!((value("pi").unwrap() - 180.0).abs() < 1e-12);
        assert_eq!(value("max(d1; 3)").unwrap(), 4.0);
        assert_eq!(value("1.5e2").unwrap(), 150.0);
    }

    #[test]
    fn mistakes_are_said_where_they_are() {
        assert!(value("1 +").unwrap_err().contains("yarım"));
        assert!(value("2x").unwrap_err().contains("birim"));
        assert!(value("foo(1)").unwrap_err().contains("işlev"));
        assert!(value("1/0").unwrap_err().contains("sıfıra"));
        assert!(value("nope").unwrap_err().contains("parametre yok"));
        assert!(value("min(1)").unwrap_err().contains("2 değer"));
        assert!(value(&"(".repeat(100)).is_err());
    }

    #[test]
    fn names_are_checked_and_circles_found() {
        assert!(parameter_name_valid("genişlik_2"));
        assert!(!parameter_name_valid("2a"));
        assert!(!parameter_name_valid("sin"));
        let reads: BTreeMap<String, BTreeSet<String>> = [
            ("a", vec!["b"]),
            ("b", vec!["c"]),
            ("c", vec!["a"]),
            ("d", vec!["a"]),
            ("e", vec![]),
        ]
        .into_iter()
        .map(|(n, r)| (n.to_string(), r.into_iter().map(String::from).collect()))
        .collect();
        assert_eq!(param_order(&reads).unwrap_err(), vec!["a", "b", "c"]);
    }
}
