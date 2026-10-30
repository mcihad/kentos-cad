//! A bound table's CHECK constraints asked of a record (docs/adr/0755 §3, §4). A constraint is read in its own SQL,
//! SQLite's or PostgreSQL's, and judged over the record's values with SQL's three-valued meaning: it breaks only when
//! its answer is false for certain (SQLite's zero), so KentOS refuses only what the table would refuse; what KentOS
//! cannot follow (a function it does not have, a collation's order) is the table's to judge, and its refusal is said by
//! name when the table gives it. A constraint tying one column to a list of values gives the field that list. The
//! independent reference is `scripts/fixtures/table_checks_cases.py`.

mod eval;
mod lex;
mod parse;
mod said;
mod text;
mod value;

use std::collections::HashMap;

use kentos_contracts::{LayerField, LayerFieldKind, SourceKind, TableCheck};

use crate::record::Attrs;
use eval::Judge;
use parse::{Aff, Bin, Cmp, Expr, Match, Ty, walk};
use value::{T3, V, Val, field_value};

/// The SQL a table's constraints are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// A GeoPackage's (SQLite's).
    Sqlite,
    /// PostgreSQL's.
    Postgres,
}

impl Dialect {
    /// A bound table's, by its source.
    pub fn of(kind: SourceKind) -> Dialect {
        match kind {
            SourceKind::Geopackage => Dialect::Sqlite,
            SourceKind::Postgis => Dialect::Postgres,
        }
    }
}

/// How much of a constraint KentOS judges (docs/adr/0755 §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judged {
    /// The whole of it: KentOS asks it before the table does.
    Whole,
    /// A part of it (a function KentOS has not, an order of texts): what it can.
    Partly,
    /// None of it: only the table asks it.
    TableOnly,
}

/// A constraint a record breaks: the field it is shown on (its first column that is a field) and why.
#[derive(Clone, Debug)]
pub struct BrokenCheck<'a> {
    pub check: &'a TableCheck,
    pub at: usize,
    pub field: Option<&'a LayerField>,
    pub message: String,
}

#[derive(Clone, Debug)]
struct Item {
    check: TableCheck,
    expr: Option<Expr>,
    judged: Judged,
    sentence: Option<String>,
}

/// A table's constraints, read once.
#[derive(Clone, Debug, Default)]
pub struct TableChecks {
    d: Option<Dialect>,
    items: Vec<Item>,
    regexes: HashMap<(String, bool), Option<regex_lite::Regex>>,
}

/// A name of a constraint as the column it is, among `columns`: SQLite's whatever the case of ASCII letters,
/// PostgreSQL's unquoted in lowercase.
fn named<'c>(columns: &'c [String], text: &str, quoted: bool, d: Dialect) -> Option<&'c String> {
    match d {
        Dialect::Sqlite => columns.iter().find(|c| c.eq_ignore_ascii_case(text)),
        Dialect::Postgres if quoted => columns.iter().find(|c| *c == text),
        Dialect::Postgres => {
            let lower = text.to_ascii_lowercase();
            columns.iter().find(|c| **c == lower)
        }
    }
}

impl TableChecks {
    /// The constraints of a table in `d`'s SQL over the layer's `fields`.
    pub fn new(checks: &[TableCheck], fields: &[LayerField], d: Dialect) -> TableChecks {
        let mut regexes = HashMap::new();
        let items = checks
            .iter()
            .map(|c| {
                let expr = (!c.table_only).then(|| parse::parse(&c.sql, d)).flatten();
                let label = |text: &str, quoted: bool| -> Option<String> {
                    let col = named(&c.columns, text, quoted, d)?;
                    Some(
                        fields
                            .iter()
                            .find(|f| f.name == *col)
                            .map_or_else(|| col.clone(), |f| f.label().to_owned()),
                    )
                };
                let sentence = expr.as_ref().and_then(|e| said::sentence(e, &label));
                let judged = match &expr {
                    None => Judged::TableOnly,
                    Some(e) => judged(e, d, &c.columns, fields),
                };
                if let Some(e) = &expr {
                    walk(e, &mut |x| {
                        if let Expr::Like {
                            how: Match::Regex { insensitive },
                            pattern,
                            ..
                        } = x
                            && let Some(p) = constant_text(pattern)
                        {
                            regexes
                                .entry((p.clone(), *insensitive))
                                .or_insert_with(|| text::compile_regex(&p, *insensitive));
                        }
                    });
                }
                Item {
                    check: c.clone(),
                    expr,
                    judged,
                    sentence,
                }
            })
            .collect();
        TableChecks {
            d: Some(d),
            items,
            regexes,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How much of the `i`th constraint KentOS judges.
    pub fn judged(&self, i: usize) -> Judged {
        self.items.get(i).map_or(Judged::TableOnly, |x| x.judged)
    }

    /// The `i`th constraint said: its sentence, else its text.
    pub fn said(&self, i: usize) -> String {
        self.items
            .get(i)
            .map(|x| x.sentence.clone().unwrap_or_else(|| x.check.sql.clone()))
            .unwrap_or_default()
    }

    /// The constraints the record `after` breaks among those a write from `before` asks (none: a new record, every
    /// one): a constraint is asked when a column it reads changed. In the table's order.
    pub fn broken<'a>(
        &'a self,
        fields: &'a [LayerField],
        before: Option<&Attrs>,
        after: &Attrs,
    ) -> Vec<BrokenCheck<'a>> {
        let Some(d) = self.d else {
            return Vec::new();
        };
        let field_of = |col: &str| fields.iter().find(|f| f.name == col);
        let mut out = Vec::new();
        for (at, item) in self.items.iter().enumerate() {
            let Some(expr) = &item.expr else {
                continue;
            };
            if let Some(b) = before
                && !item
                    .check
                    .columns
                    .iter()
                    .filter_map(|c| field_of(c))
                    .any(|f| b.get(&f.name) != after.get(&f.name))
            {
                continue;
            }
            let value = |col: &str| -> Val {
                field_of(col).map_or_else(Val::unknown, |f| {
                    field_value(f, after.get(&f.name).map(String::as_str), d)
                })
            };
            let judge = Judge {
                d,
                columns: &item.check.columns,
                value: &value,
                regexes: &self.regexes,
            };
            if judge.truth(&judge.eval(expr)) != T3::False {
                continue;
            }
            out.push(BrokenCheck {
                check: &item.check,
                at,
                field: item.check.columns.iter().find_map(|c| field_of(c)),
                message: message(&item.check, item.sentence.as_deref()),
            });
        }
        out
    }
}

/// The words a broken constraint is told with.
pub fn message(check: &TableCheck, sentence: Option<&str>) -> String {
    match (&check.name, sentence) {
        (Some(n), Some(s)) => format!("Tablonun “{n}” kısıtı: {s}."),
        (None, Some(s)) => format!("Tablonun kısıtı: {s}."),
        (Some(n), None) => format!("Kayıt tablonun “{n}” kısıtını bozuyor: {}.", check.sql),
        (None, None) => format!("Kayıt tablonun kısıtını bozuyor: {}.", check.sql),
    }
}

/// How much of a constraint KentOS judges: all but what is never known (a function it has not, a value it cannot
/// know, a pattern it does not read) and PostgreSQL's order of texts.
fn judged(e: &Expr, d: Dialect, columns: &[String], fields: &[LayerField]) -> Judged {
    let mut partly = false;
    let text_side = |x: &Expr| -> bool {
        match x {
            Expr::Str(_) | Expr::Cast(_, Ty::Text) => true,
            Expr::Name { text, quoted, .. } => named(columns, text, *quoted, d)
                .and_then(|c| fields.iter().find(|f| f.name == *c))
                .is_some_and(|f| f.kind == LayerFieldKind::Text),
            _ => false,
        }
    };
    walk(e, &mut |x| match x {
        Expr::Opaque => partly = true,
        Expr::Call(name, _) if !known_function(name, d) => partly = true,
        Expr::Like {
            how: Match::Opaque, ..
        } => partly = true,
        Expr::Like {
            how: Match::Regex { insensitive },
            pattern,
            ..
        } => {
            if constant_text(pattern)
                .is_none_or(|p| text::compile_regex(&p, *insensitive).is_none())
            {
                partly = true;
            }
        }
        Expr::Bin(Bin::Cmp(Cmp::Lt | Cmp::Le | Cmp::Gt | Cmp::Ge), a, b)
            if d == Dialect::Postgres && (text_side(a) || text_side(b)) =>
        {
            partly = true;
        }
        Expr::Between { e, lo, hi, .. }
            if d == Dialect::Postgres && (text_side(e) || text_side(lo) || text_side(hi)) =>
        {
            partly = true;
        }
        _ => {}
    });
    if partly {
        Judged::Partly
    } else {
        Judged::Whole
    }
}

/// The functions KentOS judges in a dialect.
fn known_function(name: &str, d: Dialect) -> bool {
    let both = [
        "length",
        "octet_length",
        "lower",
        "upper",
        "trim",
        "ltrim",
        "rtrim",
        "abs",
        "coalesce",
        "nullif",
        "substr",
        "replace",
    ];
    both.contains(&name)
        || match d {
            Dialect::Sqlite => ["ifnull", "instr", "typeof", "date"].contains(&name),
            Dialect::Postgres => [
                "char_length",
                "character_length",
                "btrim",
                "substring",
                "strpos",
                "round",
            ]
            .contains(&name),
        }
}

/// A constant text: a string through its casts.
fn constant_text(e: &Expr) -> Option<String> {
    match e {
        Expr::Str(s) => Some(s.clone()),
        Expr::Cast(e, _) | Expr::Collate(e, _) => constant_text(e),
        _ => None,
    }
}

/// The columns of `table_columns` (in their order) a constraint's SQL reads, its names compared as `d` compares them;
/// a text this does not read gives the columns its name tokens are.
pub fn columns_of(sql: &str, d: Dialect, table_columns: &[String]) -> Vec<String> {
    let names = parse::names(sql, d);
    table_columns
        .iter()
        .filter(|c| {
            names
                .iter()
                .any(|(t, q, _)| named(std::slice::from_ref(*c), t, *q, d).is_some())
        })
        .cloned()
        .collect()
}

/// The values a constraint ties its one column, a field of `kind`, to (docs/adr/0755 §4): `x IN (…)`,
/// `x = ANY (ARRAY[…])`, either with `x IS NULL OR`; the column (as the constraint's columns write it) and the values as
/// texts in their order, none repeated. None for another shape, a list with NULL in it (which holds for every value),
/// and a list with a decimal for a field that is not one (SQLite writes a real's text its own way).
pub fn value_list(
    check: &TableCheck,
    d: Dialect,
    kind: LayerFieldKind,
) -> Option<(String, Vec<String>)> {
    if check.table_only || check.columns.len() != 1 {
        return None;
    }
    let e = parse::parse(&check.sql, d)?;
    let (side, values) = listed(&e)?;
    let column = match side {
        Expr::Name { text, quoted, .. } => named(&check.columns, text, *quoted, d)?,
        Expr::Cast(inner, Ty::Text) => match &**inner {
            Expr::Name { text, quoted, .. } if d == Dialect::Postgres => {
                named(&check.columns, text, *quoted, d)?
            }
            _ => return None,
        },
        _ => return None,
    };
    let none = |_: &str| Val::unknown();
    let regexes = HashMap::new();
    let judge = Judge {
        d,
        columns: &[],
        value: &none,
        regexes: &regexes,
    };
    let mut out: Vec<String> = Vec::new();
    for v in values {
        let v = match v {
            Element::Expr(x) => judge.eval(x).v,
            Element::Value(v) => v,
        };
        // An integer where a text is wanted reads as its digits (SQLite's text affinity).
        let code = match v {
            V::Text(t) => t,
            V::Int(i) => i.to_string(),
            V::Real(r) if kind == LayerFieldKind::Decimal => r.exact?.text(),
            _ => return None,
        };
        if !out.contains(&code) {
            out.push(code);
        }
    }
    (!out.is_empty()).then(|| (column.clone(), out))
}

enum Element<'e> {
    Expr(&'e Expr),
    Value(V),
}

/// A list's column side and its elements.
fn listed(e: &Expr) -> Option<(&Expr, Vec<Element<'_>>)> {
    match e {
        Expr::In {
            e,
            not: false,
            list,
        } => Some((e, list.iter().map(Element::Expr).collect())),
        Expr::Quantified {
            e,
            op: Cmp::Eq,
            all: false,
            array,
        } => Some((e, array_elements(array)?)),
        Expr::Or(a, b) => {
            let (null_side, list_side) = match (&**a, &**b) {
                (Expr::IsNull { e, not: false }, other)
                | (other, Expr::IsNull { e, not: false }) => (e, other),
                _ => return None,
            };
            let (side, items) = listed(list_side)?;
            (strip(null_side) == strip(side)).then_some((side, items))
        }
        _ => None,
    }
}

fn strip(e: &Expr) -> &Expr {
    match e {
        Expr::Cast(e, _) => strip(e),
        other => other,
    }
}

fn array_elements(e: &Expr) -> Option<Vec<Element<'_>>> {
    match e {
        Expr::Array(items) => Some(items.iter().map(Element::Expr).collect()),
        Expr::Cast(inner, Ty::Array(of)) => match &**inner {
            Expr::Array(items) => Some(items.iter().map(Element::Expr).collect()),
            Expr::Str(_) => {
                let none = |_: &str| Val::unknown();
                let regexes = HashMap::new();
                let judge = Judge {
                    d: Dialect::Postgres,
                    columns: &[],
                    value: &none,
                    regexes: &regexes,
                };
                match judge
                    .eval(&Expr::Cast(inner.clone(), Ty::Array(of.clone())))
                    .v
                {
                    V::Array(items) => Some(items.into_iter().map(Element::Value).collect()),
                    _ => None,
                }
            }
            other => array_elements(other),
        },
        _ => None,
    }
}

/// Whether SQLite gives a column of type `decl` the affinity KentOS judges a field of `kind` with (docs/adr/0755 §3);
/// a constraint over a column that does not fit is the table's only.
pub fn sqlite_fits(decl: &str, kind: LayerFieldKind) -> bool {
    Aff::of_type(decl) == value::natural(kind)
}

#[cfg(test)]
mod tests;
