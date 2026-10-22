//! A condition's part a data source can check itself (docs/adr/0581 §3): the
//! SQL a source's query carries so that it hands over fewer rows. The rule is
//! a superset: the SQL keeps every row the expression keeps, and the rows the
//! source hands over are checked again with the expression, so the answer is
//! the expression's whatever the source's own rules are (its empty values,
//! its collation, a text stored in a number column). What is sent:
//!
//! - an order (`<`, `<=`, `>`, `>=`) between a number column and a number;
//! - equality between a number column and a number, as a window wider than the
//!   language's own tolerance;
//! - equality between a text column and a text that does not read as a number;
//! - `içinde` of such equalities, `arasında` of two numbers on a number column;
//! - `boş` and `boş değil` on a number or text column;
//! - `ve` of what can be sent (a part that cannot is left out), `veya` when
//!   both sides can be.
//!
//! A number is sent only below 2⁵³, where the language's reading of a column's
//! integer keeps its order with it. SQLite (GeoPackage) may hold text in a
//! number column: there the order and the window also keep text values.

use crate::Expr;
use kentos_geometry_core::jsmath::js_max;
use crate::parser::{BinOp, Node};
use crate::scalar;
use crate::value::Value;

/// The SQL a source speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// SQLite (GeoPackage): `?` placeholders, any value in any column.
    Sqlite,
    /// PostgreSQL (PostGIS): `$n` placeholders, typed columns.
    Postgres,
}

/// What a column holds, as far as sending a condition goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnSort {
    /// Integers or doubles.
    Number,
    /// Text.
    Text,
    /// Anything else (dates, true/false, bytes): nothing on it is sent.
    Other,
}

/// A value the SQL binds.
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Number(f64),
    Text(String),
}

/// What a source is sent of a condition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pushed {
    /// The SQL condition (without `WHERE`); empty: nothing is sent.
    pub sql: String,
    /// Its values in placeholder order.
    pub params: Vec<Param>,
    /// Whether the SQL is the whole condition and keeps exactly what it keeps (the rows are still checked again).
    pub exact: bool,
}

impl Pushed {
    /// The SQL with its values written in, as a plan shows it.
    pub fn shown(&self) -> String {
        let mut out = String::new();
        let mut params = self.params.iter();
        let mut chars = self.sql.chars().peekable();
        while let Some(c) = chars.next() {
            let placeholder = match c {
                '?' => true,
                '$' if chars.peek().is_some_and(char::is_ascii_digit) => {
                    while chars.peek().is_some_and(char::is_ascii_digit) {
                        chars.next();
                    }
                    true
                }
                _ => false,
            };
            if !placeholder {
                out.push(c);
                continue;
            }
            // A `::float8` cast after the placeholder goes with it.
            match params.next() {
                Some(Param::Number(x)) => out.push_str(&number_text(*x)),
                Some(Param::Text(t)) => {
                    out.push('\'');
                    out.push_str(&t.replace('\'', "''"));
                    out.push('\'');
                }
                None => out.push('?'),
            }
        }
        out.replace("::float8", "")
    }
}

fn number_text(x: f64) -> String {
    let mut s = String::new();
    crate::read::push_number_text(&mut s, x);
    s
}

/// The largest number sent: past it a column's integer is read rounded.
const LIMIT: f64 = 9_007_199_254_740_992.0;

/// The column a field names: its SQL name (quoted) and what it holds; none: not a column of the source.
pub type Columns<'a> = dyn Fn(&str) -> Option<(String, ColumnSort)> + 'a;

/// The part of `e` a source can check (docs/adr/0581 §3).
pub fn push(e: &Expr, columns: &Columns<'_>, dialect: Dialect) -> Pushed {
    let mut w = Writer {
        e,
        columns,
        dialect,
        params: Vec::new(),
    };
    match w.node(&e.root) {
        Some((sql, exact)) => Pushed {
            sql,
            params: w.params,
            exact,
        },
        None => Pushed::default(),
    }
}

struct Writer<'a, 'c> {
    e: &'a Expr,
    columns: &'a Columns<'c>,
    dialect: Dialect,
    params: Vec<Param>,
}

/// A literal as a number to send: finite and below 2⁵³ (a number, or a text that reads as one).
fn number_of(v: &Value) -> Option<f64> {
    let x = match v {
        Value::Num(x) => *x,
        Value::Text(t) => scalar::to_number(scalar::V::Text(t))?,
        _ => return None,
    };
    (x.is_finite() && x.abs() < LIMIT).then_some(x)
}

/// A literal as a text to send: not empty, not a number, not an array or a map.
fn text_of(v: &Value) -> Option<String> {
    let Value::Text(t) = v else { return None };
    if t.is_empty()
        || scalar::to_number(scalar::V::Text(t)).is_some()
        || crate::compound::is_compound(t)
    {
        return None;
    }
    Some(t.to_string())
}

/// A literal node's value (a minus before a number folded in).
fn literal(n: &Node) -> Option<Value<'static>> {
    match n {
        Node::Lit(v) => Some(v.clone()),
        Node::Neg(inner) => match inner.as_ref() {
            Node::Lit(Value::Num(x)) => Some(Value::Num(-x)),
            _ => None,
        },
        _ => None,
    }
}

impl Writer<'_, '_> {
    fn column(&self, n: &Node) -> Option<(String, ColumnSort)> {
        let Node::Field(i) = n else { return None };
        let name = self.e.fields.get(*i)?;
        (self.columns)(name)
    }

    fn param(&mut self, p: Param) -> String {
        let number = matches!(p, Param::Number(_));
        self.params.push(p);
        match self.dialect {
            Dialect::Sqlite => "?".into(),
            Dialect::Postgres if number => format!("${}::float8", self.params.len()),
            Dialect::Postgres => format!("${}", self.params.len()),
        }
    }

    /// The SQL of a node and whether it is exact; none: the node cannot be sent.
    fn node(&mut self, n: &Node) -> Option<(String, bool)> {
        match n {
            Node::Bin(BinOp::And, a, b) => {
                let a = self.node(a);
                let b = self.node(b);
                match (a, b) {
                    (Some((a, ea)), Some((b, eb))) => Some((format!("({a} AND {b})"), ea && eb)),
                    // What cannot be sent is left out: the rest keeps more, not fewer.
                    (Some((s, _)), None) | (None, Some((s, _))) => Some((s, false)),
                    (None, None) => None,
                }
            }
            Node::Bin(BinOp::Or, a, b) => {
                let mark = self.params.len();
                let a = self.node(a);
                let b = a.as_ref().and_then(|_| self.node(b));
                match (a, b) {
                    (Some((a, ea)), Some((b, eb))) => Some((format!("({a} OR {b})"), ea && eb)),
                    _ => {
                        self.params.truncate(mark);
                        None
                    }
                }
            }
            Node::Bin(op @ (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq), a, b) => {
                // The field on the left: `100 < alan` is `alan > 100`.
                let (col, lit, op) = match (self.column(a), literal(b)) {
                    (Some(c), Some(l)) => (c, l, *op),
                    _ => match (literal(a), self.column(b)) {
                        (Some(l), Some(c)) => (c, l, flip(*op)),
                        _ => return None,
                    },
                };
                self.compare(col, &lit, op)
            }
            Node::In(x, items, false) => {
                let col = self.column(x)?;
                let lits: Vec<Value<'static>> = items.iter().map(literal).collect::<Option<_>>()?;
                let mark = self.params.len();
                let mut parts = Vec::with_capacity(lits.len());
                let mut exact = true;
                for l in &lits {
                    match self.compare(col.clone(), l, BinOp::Eq) {
                        Some((s, e)) => {
                            parts.push(s);
                            exact &= e;
                        }
                        None => {
                            self.params.truncate(mark);
                            return None;
                        }
                    }
                }
                (!parts.is_empty()).then(|| (format!("({})", parts.join(" OR ")), exact))
            }
            Node::Between(x, low, high, false) => {
                let (name, sort) = self.column(x)?;
                if sort != ColumnSort::Number {
                    return None;
                }
                let lo = number_of(&literal(low)?)?;
                let hi = number_of(&literal(high)?)?;
                let a = self.param(Param::Number(lo));
                let b = self.param(Param::Number(hi));
                let exact = self.exact_number();
                Some((self.keep_text(&name, format!("{name} BETWEEN {a} AND {b}")), exact))
            }
            Node::IsNull(x, negated) => {
                let (name, sort) = self.column(x)?;
                let typed_number = self.dialect == Dialect::Postgres && sort == ColumnSort::Number;
                match sort {
                    ColumnSort::Other => None,
                    _ if typed_number && !negated => Some((format!("{name} IS NULL"), true)),
                    _ if typed_number => Some((format!("{name} IS NOT NULL"), true)),
                    _ if !negated => Some((format!("({name} IS NULL OR {name} = '')"), true)),
                    _ => Some((format!("({name} IS NOT NULL AND {name} <> '')"), true)),
                }
            }
            _ => None,
        }
    }

    /// `name op lit` on a column of `sort`.
    fn compare(&mut self, (name, sort): (String, ColumnSort), lit: &Value, op: BinOp) -> Option<(String, bool)> {
        match sort {
            ColumnSort::Number => {
                let x = number_of(lit)?;
                if op == BinOp::Eq {
                    // Wider than the language's 1e-9 · max(1, |a|, |b|): what it calls equal is inside.
                    let d = 1e-8 * js_max(x.abs(), 1.0);
                    let a = self.param(Param::Number(x - d));
                    let b = self.param(Param::Number(x + d));
                    return Some((self.keep_text(&name, format!("{name} BETWEEN {a} AND {b}")), false));
                }
                let p = self.param(Param::Number(x));
                let sym = match op {
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    _ => ">=",
                };
                let exact = self.exact_number();
                Some((self.keep_text(&name, format!("{name} {sym} {p}")), exact))
            }
            ColumnSort::Text if op == BinOp::Eq => {
                let t = text_of(lit)?;
                let p = self.param(Param::Text(t));
                Some((format!("{name} = {p}"), true))
            }
            _ => None,
        }
    }

    /// Whether a number condition keeps exactly the language's rows: in PostgreSQL a number column holds numbers;
    /// in SQLite its texts go through as well (`keep_text`), more than the language may keep.
    fn exact_number(&self) -> bool {
        self.dialect == Dialect::Postgres
    }

    /// In SQLite a number column may hold text, which the language may keep: such rows go through too.
    fn keep_text(&self, name: &str, sql: String) -> String {
        match self.dialect {
            Dialect::Sqlite => format!("({sql} OR typeof({name}) = 'text')"),
            Dialect::Postgres => sql,
        }
    }
}

fn flip(op: BinOp) -> BinOp {
    match op {
        BinOp::Lt => BinOp::Gt,
        BinOp::Le => BinOp::Ge,
        BinOp::Gt => BinOp::Lt,
        BinOp::Ge => BinOp::Le,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    fn cols(name: &str) -> Option<(String, ColumnSort)> {
        let sort = match name {
            "alan" | "no" => ColumnSort::Number,
            "ada" | "mahalle" => ColumnSort::Text,
            "tarih" => ColumnSort::Other,
            _ => return None,
        };
        Some((format!("\"{name}\""), sort))
    }

    fn sql(source: &str, d: Dialect) -> Pushed {
        push(&compile(source).unwrap(), &cols, d)
    }

    #[test]
    fn orders_and_equalities_go_to_the_source() {
        let p = sql("alan > 100", Dialect::Sqlite);
        assert_eq!(p.sql, "(\"alan\" > ? OR typeof(\"alan\") = 'text')");
        assert_eq!(p.params, vec![Param::Number(100.0)]);
        // A text in the column goes through too: more than the language keeps.
        assert!(!p.exact);
        let p = sql("100 >= alan and mahalle = 'Çankaya'", Dialect::Postgres);
        assert_eq!(p.sql, "(\"alan\" <= $1::float8 AND \"mahalle\" = $2)");
        assert_eq!(p.shown(), "(\"alan\" <= 100 AND \"mahalle\" = 'Çankaya')");
        assert!(p.exact);
    }

    #[test]
    fn what_cannot_be_sent_is_left_out_of_and_but_drops_or() {
        let p = sql("alan > 5 and ada gibi 'A%'", Dialect::Postgres);
        assert_eq!(p.sql, "\"alan\" > $1::float8");
        assert!(!p.exact);
        let p = sql("alan > 5 veya ada gibi 'A%'", Dialect::Postgres);
        assert_eq!(p, Pushed::default());
        // A number on a text column, a number-like text: never sent.
        assert_eq!(sql("ada = 101", Dialect::Sqlite), Pushed::default());
        assert_eq!(sql("ada = '101'", Dialect::Sqlite), Pushed::default());
        assert_eq!(sql("tarih boş", Dialect::Sqlite), Pushed::default());
        assert_eq!(sql("alan <> 5", Dialect::Sqlite), Pushed::default());
        assert_eq!(sql("alan > 9007199254740993", Dialect::Sqlite), Pushed::default());
    }

    #[test]
    fn equality_on_numbers_is_a_window() {
        let p = sql("alan = -2.5", Dialect::Postgres);
        assert_eq!(p.sql, "\"alan\" BETWEEN $1::float8 AND $2::float8");
        assert_eq!(
            p.params,
            vec![Param::Number(-2.5 - 2.5e-8), Param::Number(-2.5 + 2.5e-8)]
        );
        assert!(!p.exact);
    }

    #[test]
    fn in_between_and_empty() {
        let p = sql("mahalle içinde ('A', 'B') and no arasında 1 ve 9", Dialect::Postgres);
        assert_eq!(
            p.sql,
            "((\"mahalle\" = $1 OR \"mahalle\" = $2) AND \"no\" BETWEEN $3::float8 AND $4::float8)"
        );
        let p = sql("alan boş veya mahalle boş değil", Dialect::Postgres);
        assert_eq!(
            p.sql,
            "(\"alan\" IS NULL OR (\"mahalle\" IS NOT NULL AND \"mahalle\" <> ''))"
        );
        let p = sql("alan boş", Dialect::Sqlite);
        assert_eq!(p.sql, "(\"alan\" IS NULL OR \"alan\" = '')");
    }
}
