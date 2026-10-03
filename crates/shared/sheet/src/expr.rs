//! Data-bound values (design §7) through KentOS's expression language
//! (`kentos-expression`): no language of its own. A sheet adds one thing to
//! it, `@name` values; they are rewritten as bracketed fields (`[@name]`) and
//! answered, in this order, by the atlas object, the sheet's variables, the
//! project's and the built-in ones. Bare names are the object's own fields
//! (an atlas object, a table's row).
//!
//! A value that is not there is never written as empty text: `[% @ada %]`
//! without a value is `‹ada?›` on the paper, and the preflight says so.

use kentos_expression::exec::Slot;
use kentos_expression::host::{FieldDef, FieldSource, FieldType, Objects, Schema};
use kentos_expression::rows::{As, BOOL, EMPTY, NUMBER, TEXT};
use kentos_expression::{Builtin, Geometry, compile, compile_with};

use crate::model::{VarValue, Variable};

/// A name as it is looked up: Turkish letters folded, case ignored (`@Ölçek` is `@olcek`).
pub fn fold(name: &str) -> String {
    kentos_expression::js::text::fold_turkish(name)
}

/// What answers the names of an expression.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Names that come first (a grid label's `@deger`, `@eksen`), folded, without `@`.
    pub extra: Vec<(String, VarValue)>,
    /// The object's fields (bare names, exact) — an atlas object or a table's row.
    pub fields: Vec<(String, VarValue)>,
    /// The atlas object's fields, also answered as `@atlas_<name>`.
    pub atlas: Vec<(String, VarValue)>,
    pub sheet: Vec<Variable>,
    pub project: Vec<Variable>,
    /// Built-in values (`proje_adi`, `sayfa` …), folded, without `@`.
    pub builtins: Vec<(String, VarValue)>,
    /// `$alan` and `$uzunluk` of a table's row.
    pub area: Option<f64>,
    pub length: Option<f64>,
}

impl Scope {
    /// The value of `@name` (folded lookup), or of a bare field name (exact).
    pub fn lookup(&self, field: &str) -> Option<&VarValue> {
        if let Some(name) = field.strip_prefix('@') {
            let f = fold(name);
            if let Some((_, v)) = self.extra.iter().find(|(n, _)| *n == f) {
                return Some(v);
            }
            if let Some(rest) = f.strip_prefix("ATLAS_")
                && let Some((_, v)) = self.atlas.iter().find(|(n, _)| fold(n) == rest)
            {
                return Some(v);
            }
            for vars in [&self.sheet, &self.project] {
                if let Some(v) = vars.iter().find(|v| fold(&v.name) == f) {
                    return Some(&v.value);
                }
            }
            return self.builtins.iter().find(|(n, _)| *n == f).map(|(_, v)| v);
        }
        self.fields.iter().find(|(n, _)| n == field).map(|(_, v)| v)
    }

    /// Adds a built-in value (name without `@`).
    pub fn builtin(&mut self, name: &str, value: VarValue) {
        self.builtins.push((fold(name), value));
    }
}

/// The result of one expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Eval {
    Value(VarValue),
    /// A name with no value: written `‹name?›` (angle quotes every drawing face has, design §7).
    Missing(String),
    /// It does not compile.
    Error(String),
    /// It gave nothing (empty).
    Null,
}

/// `@word` outside quotes and brackets rewritten as `[@word]`.
pub fn rewrite_at_names(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 8);
    let mut chars = src.chars().peekable();
    let mut quote: Option<char> = None;
    let mut bracket = false;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                if chars.peek() == Some(&q) {
                    if let Some(n) = chars.next() {
                        out.push(n);
                    }
                } else {
                    quote = None;
                }
            }
            continue;
        }
        if bracket {
            out.push(c);
            if c == ']' {
                bracket = false;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                out.push(c);
            }
            '[' => {
                bracket = true;
                out.push(c);
            }
            '@' => {
                let mut word = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' {
                        word.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if word.is_empty() {
                    out.push('@');
                } else {
                    out.push_str("[@");
                    out.push_str(&word);
                    out.push(']');
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// One row of values as the column engine reads it.
struct Row<'a> {
    values: Vec<(String, &'a VarValue)>,
    area: Option<f64>,
    length: Option<f64>,
}

impl<'a> Objects<'a> for Row<'a> {
    fn len(&self) -> usize {
        1
    }

    fn field(&self, name: &str, _ty: FieldType, _start: usize, mut slot: Slot<'_, 'a>) {
        let v = self.values.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
        for i in 0..slot.len() {
            match v {
                Some(VarValue::Number(x)) => slot.number(i, Some(*x)),
                Some(VarValue::Bool(b)) => slot.truth(i, Some(*b)),
                Some(VarValue::Text(t)) => slot.text(i, Some(t.as_str())),
                _ => slot.text(i, None),
            }
        }
    }

    fn geometry(&self, what: Geometry, _start: usize, mut slot: Slot<'_, 'a>) {
        let x = match what {
            Geometry::Area => self.area,
            Geometry::Length => self.length,
            _ => None,
        };
        for i in 0..slot.len() {
            slot.number(i, x);
        }
    }

    fn builtin(&self, _what: Builtin, _start: usize, mut slot: Slot<'_, 'a>) {
        for i in 0..slot.len() {
            slot.text(i, None);
        }
    }
}

fn field_type(v: &VarValue) -> FieldType {
    match v {
        VarValue::Number(_) => FieldType::Number,
        VarValue::Bool(_) => FieldType::Bool,
        _ => FieldType::Text,
    }
}

/// Evaluates one expression (`@` names allowed); `want` as the column engine takes it.
fn run(source: &str, scope: &Scope, want: As) -> Eval {
    let src = rewrite_at_names(source);
    let first = match compile(&src) {
        Ok(e) => e,
        Err(e) => return Eval::Error(e.text()),
    };
    let mut values = Vec::with_capacity(first.fields.len());
    let mut schema = Schema::default();
    for f in &first.fields {
        match scope.lookup(f) {
            Some(v) if !v.is_null() => {
                schema.fields.push(FieldDef {
                    name: f.clone(),
                    ty: field_type(v),
                    source: FieldSource::User,
                    description: String::new(),
                });
                values.push((f.clone(), v));
            }
            _ => return Eval::Missing(f.trim_start_matches('@').to_owned()),
        }
    }
    let expr = match compile_with(&src, &schema) {
        Ok(e) => e,
        Err(e) => return Eval::Error(e.text()),
    };
    let row = Row {
        values,
        area: scope.area,
        length: scope.length,
    };
    let col = expr.evaluate_objects(&row, want);
    match col.kinds.first().copied() {
        Some(NUMBER) => col
            .numbers
            .first()
            .map_or(Eval::Null, |x| Eval::Value(VarValue::Number(*x))),
        Some(BOOL) => col
            .numbers
            .first()
            .map_or(Eval::Null, |x| Eval::Value(VarValue::Bool(*x != 0.0))),
        Some(TEXT) => Eval::Value(VarValue::Text(col.texts)),
        Some(EMPTY) | None => Eval::Null,
        Some(_) => Eval::Null,
    }
}

/// The value of an expression: a number, text or true/false.
pub fn evaluate(source: &str, scope: &Scope) -> Eval {
    run(source, scope, As::Value)
}

/// The value of an expression as text (numbers to 12 significant digits, true/false as “doğru”/“yanlış”).
pub fn evaluate_text(source: &str, scope: &Scope) -> Eval {
    run(source, scope, As::Text)
}

/// Whether an expression compiles; the error's text otherwise.
pub fn check(source: &str) -> Result<(), String> {
    compile(&rewrite_at_names(source))
        .map(|_| ())
        .map_err(|e| e.text())
}

/// A `[% … %]` part that could not be written.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub expression: String,
    pub eval: Eval,
}

/// Text with its `[% … %]` parts written, and what could not be.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rendered {
    pub text: String,
    pub problems: Vec<Problem>,
}

/// The mark a value that is not there leaves on the paper.
pub fn missing_mark(name: &str) -> String {
    format!("‹{name}?›")
}

/// Writes `content`'s `[% … %]` parts with `scope`.
pub fn render(content: &str, scope: &Scope) -> Rendered {
    let mut out = Rendered::default();
    let mut rest = content;
    while let Some(start) = rest.find("[%") {
        out.text.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("%]") else {
            out.problems.push(Problem {
                expression: after.to_owned(),
                eval: Eval::Error("“%]” eksik: ifade kapanmamış.".to_owned()),
            });
            out.text.push_str(&rest[start..]);
            return out;
        };
        let expr = after[..end].trim();
        match evaluate_text(expr, scope) {
            Eval::Value(VarValue::Text(t)) => out.text.push_str(&t),
            Eval::Value(v) => out.text.push_str(&value_text(&v)),
            e => {
                let name = match &e {
                    Eval::Missing(n) => n.clone(),
                    Eval::Error(_) => "ifade".to_owned(),
                    _ => "değer".to_owned(),
                };
                out.text.push_str(&missing_mark(&name));
                out.problems.push(Problem {
                    expression: expr.to_owned(),
                    eval: e,
                });
            }
        }
        rest = &after[end + 2..];
    }
    out.text.push_str(rest);
    out
}

/// A value as text, the expression language's way.
pub fn value_text(v: &VarValue) -> String {
    match v {
        VarValue::Null => String::new(),
        VarValue::Bool(b) => if *b { "doğru" } else { "yanlış" }.to_owned(),
        VarValue::Number(x) => kentos_expression::value::number_text(*x),
        VarValue::Text(t) => t.clone(),
    }
}

/// Whether a text has `[% … %]` parts.
pub fn has_parts(content: &str) -> bool {
    content.contains("[%")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::VarKind;

    fn scope() -> Scope {
        let mut s = Scope::default();
        s.sheet.push(Variable {
            name: "ada".into(),
            label: "Ada".into(),
            kind: VarKind::Text,
            value: VarValue::Text("101".into()),
        });
        s.sheet.push(Variable {
            name: "parsel".into(),
            label: "Parsel".into(),
            kind: VarKind::Text,
            value: VarValue::Null,
        });
        s.builtin("sayfa", VarValue::Number(2.0));
        s.builtin("olcek", VarValue::Text("1/1000".into()));
        s.atlas
            .push(("mahalle".into(), VarValue::Text("Kızılay".into())));
        s.fields
            .push(("Nitelik".into(), VarValue::Text("Arsa".into())));
        s
    }

    #[test]
    fn at_names_become_fields_outside_quotes() {
        assert_eq!(
            rewrite_at_names("@ada || '@x' || [a@b]"),
            "[@ada] || '@x' || [a@b]"
        );
        assert_eq!(rewrite_at_names("@ölçek"), "[@ölçek]");
        assert_eq!(rewrite_at_names("a @ b"), "a @ b");
    }

    #[test]
    fn values_come_from_the_scopes_in_order() {
        let s = scope();
        assert_eq!(
            evaluate("@sayfa + 1", &s),
            Eval::Value(VarValue::Number(3.0))
        );
        assert_eq!(
            evaluate("@Sayfa * 2", &s),
            Eval::Value(VarValue::Number(4.0))
        );
        assert_eq!(
            evaluate("@ölçek", &s),
            Eval::Value(VarValue::Text("1/1000".into()))
        );
        assert_eq!(
            evaluate("@atlas_mahalle", &s),
            Eval::Value(VarValue::Text("Kızılay".into()))
        );
        assert_eq!(
            evaluate("Nitelik = 'Arsa'", &s),
            Eval::Value(VarValue::Bool(true))
        );
        assert_eq!(evaluate("@parsel", &s), Eval::Missing("parsel".into()));
        assert_eq!(evaluate("@yok", &s), Eval::Missing("yok".into()));
        assert!(matches!(evaluate("1 +", &s), Eval::Error(_)));
    }

    #[test]
    fn text_parts_are_written_and_gaps_marked() {
        let s = scope();
        let r = render(
            "Ada [% @ada %], parsel [% @parsel %], sayfa [% @sayfa %]",
            &s,
        );
        assert_eq!(r.text, "Ada 101, parsel ‹parsel?›, sayfa 2");
        assert_eq!(r.problems.len(), 1);
        assert_eq!(render("düz", &s).text, "düz");
        let open = render("a [% @ada", &s);
        assert_eq!(open.text, "a [% @ada");
        assert_eq!(open.problems.len(), 1);
        assert_eq!(render("[% 1/3 %]", &s).text, "0.333333333333");
    }
}
