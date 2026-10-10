//! The order of values (docs/adr/0217 §6), one for both platforms (the web
//! through `orderBy`): İfadeye göre sırala sorts objects by an expression's
//! values. A value is none, a truth value, a number or a text, as an
//! expression's results are.
//!
//! A value that is none, or a number that is not finite (an infinity or not
//! a number, which JSON writes as null), has no place in the order: such
//! values go where Boşlar puts them, first or last. Other values compare by
//! their kind when their kinds differ (truth values, then numbers, then
//! texts), else false before true, numbers by their value (−0 is 0), texts in
//! the natural order (`text::natural`, docs/adr/0153 §6). Azalan turns the
//! values' order round; equal values and the values without a place keep
//! the input's order. The independent reference is
//! `scripts/fixtures/layer_ops_cases.py`.

use std::cmp::Ordering;

use crate::api::Op;
use crate::api::json::{FromJson, Json};
use crate::op;
use crate::text::natural::{Token, tokens, tokens_cmp};

/// A value that has a place in the order.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Bool(bool),
    Num(f64),
    Text(String),
}

impl FromJson for Key {
    fn from_json(v: &Json) -> Result<Key, String> {
        match v {
            Json::Bool(b) => Ok(Key::Bool(*b)),
            Json::Num(x) => Ok(Key::Num(*x)),
            Json::Str(s) => Ok(Key::Text(s.clone())),
            _ => Err("sıralanacak değer evet/hayır, sayı ya da metin olmalı".into()),
        }
    }
}

/// A value read for the order: its kind's rank and what it compares by (a text's pieces read once).
enum Sorted<'a> {
    Bool(bool),
    Num(f64),
    Text(&'a str, Vec<Token<'a>>),
}

impl Sorted<'_> {
    fn rank(&self) -> u8 {
        match self {
            Sorted::Bool(_) => 0,
            Sorted::Num(_) => 1,
            Sorted::Text(..) => 2,
        }
    }
}

fn sorted(key: Option<&Key>) -> Option<Sorted<'_>> {
    match key? {
        Key::Bool(b) => Some(Sorted::Bool(*b)),
        Key::Num(x) if x.is_finite() => Some(Sorted::Num(*x)),
        Key::Num(_) => None,
        Key::Text(s) => Some(Sorted::Text(s, tokens(s))),
    }
}

fn compare(a: &Sorted<'_>, b: &Sorted<'_>) -> Ordering {
    match (a, b) {
        (Sorted::Bool(x), Sorted::Bool(y)) => x.cmp(y),
        // Finite numbers: −0 and 0 are equal.
        (Sorted::Num(x), Sorted::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (Sorted::Text(x, tx), Sorted::Text(y, ty)) => tokens_cmp((x, tx), (y, ty)),
        _ => a.rank().cmp(&b.rank()),
    }
}

/// The values' indices in their order: `descending` turns the values' order
/// round, `nulls_first` puts the values without a place first.
pub fn order(keys: &[Option<Key>], descending: bool, nulls_first: bool) -> Vec<u32> {
    let read: Vec<Option<Sorted<'_>>> = keys.iter().map(|k| sorted(k.as_ref())).collect();
    let mut at: Vec<u32> = (0..keys.len()).map(|i| i as u32).collect();
    // Stable: equal values keep the input's order.
    at.sort_by(|&i, &j| match (&read[i as usize], &read[j as usize]) {
        (Some(a), Some(b)) => {
            let o = compare(a, b);
            if descending { o.reverse() } else { o }
        }
        (Some(_), None) => {
            if nulls_first {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (None, Some(_)) => {
            if nulls_first {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (None, None) => Ordering::Equal,
    });
    at
}

pub(crate) static OPS: &[Op] =
    &[op!("orderBy", |keys: Vec<Option<Key>>,
                      descending: bool,
                      nulls_first: bool| {
        order(&keys, descending, nulls_first)
    })];

#[cfg(test)]
mod tests {
    use super::*;

    fn num(x: f64) -> Option<Key> {
        Some(Key::Num(x))
    }

    fn text(s: &str) -> Option<Key> {
        Some(Key::Text(s.into()))
    }

    #[test]
    fn numbers_by_value_and_equal_ones_in_the_input_order() {
        let keys = [num(10.0), num(2.0), None, num(-0.0), num(0.0), num(2.0)];
        assert_eq!(order(&keys, false, false), [3, 4, 1, 5, 0, 2]);
        // Azalan turns the values round, not the equal ones; Başta puts the empty first.
        assert_eq!(order(&keys, true, true), [2, 0, 1, 5, 3, 4]);
    }

    #[test]
    fn kinds_then_natural_texts_and_not_finite_numbers_as_none() {
        let keys = [
            text("P10"),
            num(f64::NAN),
            Some(Key::Bool(true)),
            text("P2"),
            num(5.0),
            Some(Key::Bool(false)),
            num(f64::INFINITY),
        ];
        assert_eq!(order(&keys, false, false), [5, 2, 4, 3, 0, 1, 6]);
    }

    #[test]
    fn the_op_reads_nulls_and_values() {
        let keys: Vec<Option<Key>> = FromJson::from_json(&Json::Arr(vec![
            Json::Str("b".into()),
            Json::Null,
            Json::Str("a".into()),
        ]))
        .expect("keys");
        assert_eq!(order(&keys, false, true), [1, 2, 0]);
    }
}
