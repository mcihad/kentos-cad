//! What a Rasterlere değer and a Raster çiftleri field shows and does
//! (docs/adr/0237 §9; the web's `fieldPlan.ts`): the rows, a row's value
//! as its field shows it, and the value after a row is typed, removed or
//! added, or a pair's comparison chosen.

use serde_json::{Map, Value, json};

use crate::text::js_number;

/// No rasters chosen.
pub const NONE: &str = "Önce rasterleri seçin.";
/// A model's step: its rasters are known when it runs.
pub const LATER: &str = "Rasterler çalışınca belli olur; adlarıyla ekleyin.";
pub const MISSING: &str = "Girdide yok";
pub const REMOVE: &str = "Kaldır";
pub const ADD: &str = "Raster adı";
pub const ADD_BUTTON: &str = "Ekle";
pub const EVEN: &str = "Eşit";

/// A raster's row: its name, its value as the field shows it, and whether
/// the input holds it (else faint, with ×).
#[derive(Clone, Debug, PartialEq)]
pub struct RasterRow {
    pub name: String,
    pub text: String,
    pub listed: bool,
}

/// The rows of a value field; the note when there are none to fill; whether
/// a name can be added (a model's step).
#[derive(Clone, Debug, PartialEq)]
pub struct RasterValuesView {
    pub rows: Vec<RasterRow>,
    pub note: Option<&'static str>,
    pub adds: bool,
}

/// A row's value as its field shows it: a number as JavaScript writes it, a text (or an unread number's text) as it is.
fn cell_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.as_f64().map(js_number).unwrap_or_default(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// The rows: the input's rasters in the run's order, then the names the
/// value holds that the input does not (in their names' order); `rasters`
/// none when they are not known (a model's step), when the value's names
/// are the rows.
pub fn values_view(value: &Value, rasters: Option<&[String]>) -> RasterValuesView {
    let empty = Map::new();
    let o = value.as_object().unwrap_or(&empty);
    let listed = rasters.unwrap_or(&[]);
    let mut rows: Vec<RasterRow> = listed
        .iter()
        .map(|name| RasterRow {
            name: name.clone(),
            text: cell_text(o.get(name)),
            listed: true,
        })
        .collect();
    // The map holds its names in order.
    for (name, v) in o {
        if !listed.contains(name) {
            rows.push(RasterRow {
                name: name.clone(),
                text: cell_text(Some(v)),
                listed: rasters.is_none(),
            });
        }
    }
    let note = match (rows.is_empty(), rasters.is_none()) {
        (false, _) => None,
        (true, true) => Some(LATER),
        (true, false) => Some(NONE),
    };
    RasterValuesView {
        rows,
        note,
        adds: rasters.is_none(),
    }
}

/// A row's field typed: a number cell reads the text as a number field does
/// (a decimal comma too; a text that is no number is kept as it is, which the
/// check calls invalid), a text cell keeps it; an empty field takes the
/// raster's value away.
pub fn with_value(value: &Value, number: bool, name: &str, text: &str) -> Value {
    let mut o = value.as_object().cloned().unwrap_or_default();
    o.remove(name);
    if !text.trim().is_empty() {
        let v = if number {
            let read = text.trim().replace(',', ".");
            read.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map_or_else(|| Value::String(text.to_owned()), |n| json!(n))
        } else {
            Value::String(text.to_owned())
        };
        o.insert(name.to_owned(), v);
    }
    Value::Object(o)
}

/// A name's row taken away (×).
pub fn without(value: &Value, name: &str) -> Value {
    let mut o = value.as_object().cloned().unwrap_or_default();
    o.remove(name);
    Value::Object(o)
}

/// A new name's row added (a model's step): 1 in a number cell, empty text in a text cell; a name held already stays.
pub fn with_name(value: &Value, number: bool, name: &str) -> Value {
    let mut o = value.as_object().cloned().unwrap_or_default();
    let name = name.trim();
    if !name.is_empty() && !o.contains_key(name) {
        o.insert(name.to_owned(), if number { json!(1) } else { json!("") });
    }
    Value::Object(o)
}

/// The comparisons offered for a pair, the first raster's strongest first: 9 … 2, 1 (even), −2 … −9.
pub const PAIR_VALUES: [i32; 17] = [9, 8, 7, 6, 5, 4, 3, 2, 1, -2, -3, -4, -5, -6, -7, -8, -9];

/// A comparison as its list writes it: “Eğim 3 kat”, “Eşit”, “Yol 5 kat”.
pub fn pair_label(a: &str, b: &str, v: i32) -> String {
    match v {
        1 => EVEN.to_owned(),
        v if v > 0 => format!("{a} {v} kat"),
        v => format!("{b} {} kat", -v),
    }
}

/// The pairs of a value as triples (others left out).
fn triples(value: &Value) -> Vec<(String, String, f64)> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let t = r.as_array()?;
            Some((
                t.first()?.as_str()?.to_owned(),
                t.get(1)?.as_str()?.to_owned(),
                t.get(2)?.as_f64()?,
            ))
        })
        .collect()
}

/// A pair's comparison in the value: (a, b, v) as given, (b, a, v) turned round; the first given counts; none: even.
pub fn pair_value(value: &Value, a: &str, b: &str) -> f64 {
    for (x, y, v) in triples(value) {
        if x == a && y == b {
            return v;
        }
        if x == b && y == a {
            return if v == 1.0 { 1.0 } else { -v };
        }
    }
    1.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairRow {
    pub a: String,
    pub b: String,
    pub value: f64,
    pub text: String,
    pub listed: bool,
}

/// The rows: each pair of the input's rasters (the earlier first), then the
/// pairs the value holds that are not among them; the note when there are none.
pub fn pairs_view(
    value: &Value,
    rasters: Option<&[String]>,
) -> (Vec<PairRow>, Option<&'static str>) {
    let listed = rasters.unwrap_or(&[]);
    let mut rows = Vec::new();
    let label = |a: &str, b: &str, v: f64| pair_label(a, b, v as i32);
    for (i, a) in listed.iter().enumerate() {
        for b in &listed[i + 1..] {
            let v = pair_value(value, a, b);
            rows.push(PairRow {
                a: a.clone(),
                b: b.clone(),
                value: v,
                text: label(a, b, v),
                listed: true,
            });
        }
    }
    for (a, b, v) in triples(value) {
        if rows
            .iter()
            .any(|r| (r.a == a && r.b == b) || (r.a == b && r.b == a))
        {
            continue;
        }
        rows.push(PairRow {
            text: label(&a, &b, v),
            a,
            b,
            value: v,
            listed: rasters.is_none(),
        });
    }
    let note = match (rows.is_empty(), rasters.is_none()) {
        (false, _) => None,
        (true, true) => Some(LATER),
        (true, false) => Some(NONE),
    };
    (rows, note)
}

/// A pair's comparison chosen: its earlier entries go, an even one is not kept (a pair not given is even).
pub fn with_pair(value: &Value, a: &str, b: &str, v: i32) -> Value {
    let mut rest: Vec<Value> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| {
            let x = r.get(0).and_then(Value::as_str);
            let y = r.get(1).and_then(Value::as_str);
            !((x == Some(a) && y == Some(b)) || (x == Some(b) && y == Some(a)))
        })
        .cloned()
        .collect();
    if v != 1 {
        rest.push(json!([a, b, v]));
    }
    Value::Array(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn value_rows_are_the_rasters_then_the_others() {
        let v = json!({ "Yol": 0.5, "Eski": 2, "Eğim": "1a" });
        let r = values_view(&v, Some(&names(&["Eğim", "Yol", "Toprak"])));
        let got: Vec<(&str, &str, bool)> = r
            .rows
            .iter()
            .map(|x| (x.name.as_str(), x.text.as_str(), x.listed))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Eğim", "1a", true),
                ("Yol", "0.5", true),
                ("Toprak", "", true),
                ("Eski", "2", false)
            ]
        );
        assert_eq!((r.note, r.adds), (None, false));
        assert_eq!(values_view(&json!({}), Some(&[])).note, Some(NONE));
        let m = values_view(&json!({}), None);
        assert_eq!((m.note, m.adds), (Some(LATER), true));
        let typed = with_value(&v, true, "Toprak", "1,25");
        assert_eq!(typed["Toprak"], json!(1.25));
        assert_eq!(with_value(&v, true, "Yol", " ")["Yol"], Value::Null);
        assert_eq!(with_value(&v, true, "Yol", "x")["Yol"], json!("x"));
        assert_eq!(without(&v, "Eski").get("Eski"), None);
        assert_eq!(with_name(&json!({}), true, " Bant ")["Bant"], json!(1));
    }

    #[test]
    fn pairs_read_both_ways_and_keep_only_the_chosen() {
        let v = json!([["Yol", "Eğim", 3], ["Eğim", "Yol", 7], ["A", "B", -2]]);
        assert_eq!(pair_value(&v, "Eğim", "Yol"), -3.0);
        let (rows, note) = pairs_view(&v, Some(&names(&["Eğim", "Yol", "Toprak"])));
        let got: Vec<(&str, &str, &str, bool)> = rows
            .iter()
            .map(|r| (r.a.as_str(), r.b.as_str(), r.text.as_str(), r.listed))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Eğim", "Yol", "Yol 3 kat", true),
                ("Eğim", "Toprak", "Eşit", true),
                ("Yol", "Toprak", "Eşit", true),
                ("A", "B", "B 2 kat", false)
            ]
        );
        assert_eq!(note, None);
        let chosen = with_pair(&v, "Eğim", "Yol", 5);
        assert_eq!(chosen, json!([["A", "B", -2], ["Eğim", "Yol", 5]]));
        assert_eq!(
            with_pair(&chosen, "Yol", "Eğim", 1),
            json!([["A", "B", -2]])
        );
    }
}
