//! The part of an İfadeyle seç expression a data source answers itself, as the data drivers' condition tree
//! (docs/adr/9109): the language's own rules, once (`kentos_expression::pushdown::sent`, docs/adr/0581 §3), written as a
//! `kentos_data::Condition`. What is sent keeps every row the expression keeps; unless it is exact, the rows it keeps
//! are asked the expression again here, so the answer is the expression's whatever the source's own rules are.

use kentos_data::{Condition, FieldKind, Op, Schema, Value};
use kentos_expression::Expr;
use kentos_expression::parser::BinOp;
use kentos_expression::pushdown::{ColumnSort, Sent, sent};

/// What a source is sent of an expression.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pushed {
    /// The condition the source checks; none: nothing of the expression could be sent.
    pub condition: Option<Condition>,
    /// Whether it keeps exactly the expression's rows: then the expression is not asked again here.
    pub exact: bool,
}

/// Whether a driver's columns hold only their declared type, as a database's do; SQLite's (a GeoPackage's) keep any
/// value in any column.
pub fn typed(driver: &str) -> bool {
    driver != "gpkg"
}

/// What a field holds, as far as sending a condition goes.
fn sort_of(kind: &FieldKind) -> ColumnSort {
    match kind {
        k if k.is_number() => ColumnSort::Number,
        FieldKind::Text { .. } => ColumnSort::Text,
        _ => ColumnSort::Other,
    }
}

/// The part of `e` the table of `schema` answers, in a source whose columns are `typed` or not.
pub fn pushed(e: &Expr, schema: &Schema, typed: bool) -> Pushed {
    let columns = |name: &str| -> Option<(String, ColumnSort)> {
        if name == schema.key {
            return Some((name.to_owned(), ColumnSort::Number));
        }
        schema.field(name).map(|f| (f.name.clone(), sort_of(&f.kind)))
    };
    let Some((tree, exact)) = sent(e, &columns) else {
        return Pushed::default();
    };
    Pushed {
        exact: exact && (typed || !tree.bounds_numbers()),
        condition: Some(condition_of(&tree, typed)),
    }
}

fn op_of(op: BinOp) -> Op {
    match op {
        BinOp::Lt => Op::Lt,
        BinOp::Le => Op::Le,
        BinOp::Gt => Op::Gt,
        _ => Op::Ge,
    }
}

/// The tree in the drivers' words.
fn condition_of(s: &Sent, typed: bool) -> Condition {
    // A number column of SQLite may hold a text, which the language orders as a text: such rows go through too.
    let texts_too = |c: Condition, column: &str| {
        if typed {
            c
        } else {
            Condition::Or(vec![c, Condition::Text(column.to_owned())])
        }
    };
    let empty = || Value::Text(String::new());
    match s {
        Sent::And(a, b) => Condition::And(vec![condition_of(a, typed), condition_of(b, typed)]),
        Sent::Or(a, b) => Condition::Or(vec![condition_of(a, typed), condition_of(b, typed)]),
        Sent::Order { column, op, number } => texts_too(Condition::cmp(column, op_of(*op), Value::Real(*number)), column),
        Sent::Between { column, low, high } => texts_too(
            Condition::Between {
                field: column.clone(),
                low: Value::Real(*low),
                high: Value::Real(*high),
            },
            column,
        ),
        Sent::Equals { column, text } => Condition::eq(column, Value::Text(text.clone())),
        Sent::Any(parts) => Condition::Or(parts.iter().map(|p| condition_of(p, typed)).collect()),
        // An empty text is empty too, except in a typed number column, which holds none.
        Sent::Empty { column, sort, negated } => match (typed && *sort == ColumnSort::Number, negated) {
            (true, false) => Condition::IsNull(column.clone()),
            (true, true) => Condition::NotNull(column.clone()),
            (false, false) => Condition::Or(vec![Condition::IsNull(column.clone()), Condition::eq(column, empty())]),
            (false, true) => {
                Condition::And(vec![Condition::NotNull(column.clone()), Condition::cmp(column, Op::Ne, empty())])
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_data::Field;

    fn schema() -> Schema {
        Schema {
            key: "fid".into(),
            fields: vec![
                Field::new("alan", FieldKind::Real),
                Field::new("ada", FieldKind::INT32),
                Field::new("kullanim", FieldKind::Text { max: None }),
                Field::new("tarih", FieldKind::Date),
            ],
            key_made: true,
            geometry: None,
        }
    }

    fn push(text: &str, typed: bool) -> Pushed {
        pushed(&kentos_expression::compile(text).expect("compiles"), &schema(), typed)
    }

    #[test]
    fn a_text_equality_is_sent_exactly_everywhere() {
        let p = push("kullanim = 'Konut'", false);
        assert_eq!(p.condition, Some(Condition::eq("kullanim", Value::Text("Konut".into()))));
        assert!(p.exact);
        let p = push("kullanim içinde ('Konut', 'Park')", true);
        assert!(p.exact);
        assert!(matches!(p.condition, Some(Condition::Or(ref v)) if v.len() == 2));
    }

    #[test]
    fn a_number_order_keeps_sqlite_texts_and_is_exact_only_where_typed() {
        let typed = push("alan > 500", true);
        assert_eq!(typed.condition, Some(Condition::cmp("alan", Op::Gt, Value::Real(500.0))));
        assert!(typed.exact);
        let untyped = push("alan > 500", false);
        assert_eq!(
            untyped.condition,
            Some(Condition::Or(vec![
                Condition::cmp("alan", Op::Gt, Value::Real(500.0)),
                Condition::Text("alan".into())
            ]))
        );
        assert!(!untyped.exact);
        // An equality on numbers is a window, never exact; `arasında` is.
        assert!(!push("ada = 101", true).exact);
        assert!(push("ada arasında 100 ve 110", true).exact);
        assert!(push("fid > 10", true).exact, "the key is a number column");
    }

    #[test]
    fn what_cannot_be_sent_narrows_and_drops_or() {
        // A measure is the language's: the rest is sent, not exact.
        let p = push("ada > 0 ve $alan > 900", true);
        assert_eq!(p.condition, Some(Condition::cmp("ada", Op::Gt, Value::Real(0.0))));
        assert!(!p.exact);
        assert_eq!(push("ada > 0 veya $alan > 900", true), Pushed::default());
        assert_eq!(push("tarih boş", true), Pushed::default(), "a date is not sent");
        assert_eq!(push("yok = 1", true), Pushed::default(), "not a column");
    }

    #[test]
    fn empty_is_a_null_or_an_empty_text() {
        assert_eq!(push("ada boş", true).condition, Some(Condition::IsNull("ada".into())));
        let p = push("kullanim boş değil", true);
        assert_eq!(
            p.condition,
            Some(Condition::And(vec![
                Condition::NotNull("kullanim".into()),
                Condition::cmp("kullanim", Op::Ne, Value::Text(String::new()))
            ]))
        );
        assert!(p.exact);
        assert!(matches!(push("ada boş", false).condition, Some(Condition::Or(_))));
    }
}
