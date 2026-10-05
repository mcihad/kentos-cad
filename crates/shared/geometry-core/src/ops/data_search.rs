//! Veride ara (docs/adr/0178 §1, §2, §4), one for both platforms (the web
//! through WASM): which objects of a drawing answer a word, by which field,
//! in what order. An object arrives as a [`Record`], the words it holds
//! (its label, a text's words or a leader's note, an insert's block name,
//! its attributes), as each platform reads them from its own objects; the
//! drawing's order is the records' order. The independent reference is
//! `scripts/fixtures/data_search_cases.py`.
//!
//! A record answers when one of the fields asked for does (docs/adr/0178 §2,
//! `text::edit::matches`). Its row shows the first that does, looked at in
//! this order: the label, the text, the block's name, then the attributes by
//! their names (code point order); `more` counts the others that do. Rows
//! come in the records' order, or by a column in the natural order
//! (`text::natural`), equal values in the records' order.

use std::cmp::Ordering;

use crate::api::Op;
use crate::op;
use crate::text::edit::matches;
use crate::text::natural::natural_cmp;
use crate::tools::point_text::js_trim;

/// An object as the search reads it: its kind and layer's path as the rows
/// show them, and the words it holds (none: the field is absent).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Record {
    pub kind: String,
    pub layer: String,
    /// The object's label: a point's name, a parcel's number.
    pub label: Option<String>,
    /// A text's words, a leader's note, a dimension's own text.
    pub text: Option<String>,
    /// The name of the block an insert places.
    pub block: Option<String>,
    /// Attribute name and value, in any order.
    pub attrs: Vec<[String; 2]>,
}

crate::json_struct!(Record {
    kind,
    layer,
    label,
    text,
    block,
    attrs
});

/// The fields to look in (docs/adr/0178 §1): attributes by their values, all
/// of them or the one named.
#[derive(Clone, Debug, PartialEq)]
pub struct Fields {
    pub label: bool,
    pub text: bool,
    pub block: bool,
    pub attrs: bool,
    pub attr_name: Option<String>,
}

crate::json_struct!(Fields {
    label,
    text,
    block,
    attrs,
    attr_name => "attrName"
});

/// What is asked: the word, how it is matched, the fields, the column the
/// rows are sorted by (`layer`, `kind`, `field`, `value`; none: the records'
/// order; any other word the same) and which way, and how many rows at most
/// (0: all).
#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub pattern: String,
    pub match_case: bool,
    pub whole_word: bool,
    pub fields: Fields,
    pub sort: Option<String>,
    pub descending: bool,
    pub limit: usize,
}

crate::json_struct!(Query {
    pattern,
    match_case => "matchCase",
    whole_word => "wholeWord",
    fields,
    sort,
    descending,
    limit
});

/// The field a row shows: `label`, `text`, `block` or `attr` (with its name).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The record's index.
    pub record: u32,
    pub field: &'static str,
    /// The attribute's name, for `attr`.
    pub name: Option<String>,
    /// The field's words, trimmed.
    pub value: String,
    /// How many other fields of the record answer too.
    pub more: u32,
}

crate::json_struct!(out Row {
    record,
    field,
    name,
    value,
    more
});

/// The rows (at most the limit) and how many records answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    pub rows: Vec<Row>,
    pub total: u32,
}

crate::json_struct!(out Found { rows, total });

/// A field's place in the order fields are looked at and sorted.
fn rank(field: &str) -> u8 {
    match field {
        "label" => 0,
        "text" => 1,
        "block" => 2,
        _ => 3,
    }
}

/// A value as the search reads it: trimmed, none when empty.
fn value(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(js_trim).filter(|v| !v.is_empty())
}

/// The fields of `r` that answer, in the order they are looked at.
fn hits<'a>(
    r: &'a Record,
    q: &Query,
    wanted: &str,
) -> Vec<(&'static str, Option<&'a str>, &'a str)> {
    let answers = |v: &str| matches(v, wanted, !q.match_case, q.whole_word);
    let mut out = Vec::new();
    let mut plain = |on: bool, field: &'static str, v: &'a Option<String>| {
        if on && let Some(v) = value(v).filter(|v| answers(v)) {
            out.push((field, None, v));
        }
    };
    plain(q.fields.label, "label", &r.label);
    plain(q.fields.text, "text", &r.text);
    plain(q.fields.block, "block", &r.block);
    if q.fields.attrs {
        let mut named: Vec<&[String; 2]> = r
            .attrs
            .iter()
            .filter(|[name, _]| q.fields.attr_name.as_ref().is_none_or(|n| n == name))
            .collect();
        // By name, as the web's object and the desktop's map each hold them.
        named.sort_by(|a, b| a[0].cmp(&b[0]));
        for [name, v] in named {
            let v = js_trim(v);
            if !v.is_empty() && answers(v) {
                out.push(("attr", Some(name.as_str()), v));
            }
        }
    }
    out
}

/// The records that answer the query (docs/adr/0178): their rows, sorted, the
/// first `limit` of them, and how many answer in all. A word that is empty
/// once trimmed asks nothing.
pub fn data_search(records: &[Record], q: &Query) -> Found {
    let wanted = js_trim(&q.pattern);
    if wanted.is_empty() {
        return Found {
            rows: Vec::new(),
            total: 0,
        };
    }
    let mut rows: Vec<Row> = records
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let found = hits(r, q, wanted);
            let (field, name, v) = *found.first()?;
            Some(Row {
                record: i as u32,
                field,
                name: name.map(str::to_owned),
                value: v.to_owned(),
                more: found.len() as u32 - 1,
            })
        })
        .collect();
    let total = rows.len() as u32;
    let by = |a: &Row, b: &Row| -> Ordering {
        let (ra, rb) = (&records[a.record as usize], &records[b.record as usize]);
        match q.sort.as_deref() {
            Some("layer") => natural_cmp(&ra.layer, &rb.layer),
            Some("kind") => natural_cmp(&ra.kind, &rb.kind),
            Some("field") => rank(a.field).cmp(&rank(b.field)).then_with(|| {
                natural_cmp(
                    a.name.as_deref().unwrap_or(""),
                    b.name.as_deref().unwrap_or(""),
                )
            }),
            Some("value") => natural_cmp(&a.value, &b.value),
            _ => Ordering::Equal,
        }
    };
    if matches!(
        q.sort.as_deref(),
        Some("layer" | "kind" | "field" | "value")
    ) {
        let d = q.descending;
        rows.sort_by(|a, b| {
            let o = by(a, b);
            (if d { o.reverse() } else { o }).then(a.record.cmp(&b.record))
        });
    }
    if q.limit > 0 {
        rows.truncate(q.limit);
    }
    Found { rows, total }
}

pub(crate) static OPS: &[Op] = &[op!("dataSearch", |records: Vec<Record>, query: Query| {
    data_search(&records, &query)
})];
