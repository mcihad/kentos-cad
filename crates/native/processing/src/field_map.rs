//! What an Alan eşleme field holds, shows and does (docs/adr/0216 §2; the
//! web's `fieldPlan.ts` and `parameters.ts`): the rows of a target field, a
//! source column and a default; each row's problem as the field shows it,
//! the check that keeps a run from starting, and the map after a row is
//! typed, removed or added, or after Aynı adlarla doldur. Both platforms play
//! fixtures/processing/v1/field-map.json.

use serde_json::{Value, json};

use crate::text::{js_trim, utf16_len};

pub const TARGET: &str = "Hedef alan";
pub const SOURCE: &str = "Kaynak alan";
pub const DEFAULT: &str = "Varsayılan";
pub const NONE: &str = "(yok)";
pub const ADD: &str = "Satır ekle";
pub const FILL: &str = "Aynı adlarla doldur";
pub const FILL_HINT: &str =
    "Kaynağın anahtar dışındaki, henüz eşlenmemiş her sütunu kendi adıyla bir satır olur.";
pub const REMOVE: &str = "Satırı kaldır";
/// An empty map: what the run does.
pub const EMPTY: &str = "Boş: anahtardan başka bütün sütunlar kendi adlarıyla aktarılır.";
/// The source's columns are not known yet.
pub const NO_SOURCE: &str = "Kaynağın sütunları kaynak seçilince gelir.";
pub const EMPTY_TARGET: &str = "Hedef alan boş.";

fn twice(name: &str) -> String {
    format!("“{name}” iki satırda hedef.")
}

fn no_column(name: &str) -> String {
    format!("Kaynakta “{name}” yok.")
}

/// A row: the target's field, the source's column (empty: none) and the
/// value when the column's cell is blank (empty: none), as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapRow {
    pub target: String,
    pub source: String,
    pub default: String,
}

impl MapRow {
    pub fn new(target: &str, source: &str, default: &str) -> Self {
        Self {
            target: target.into(),
            source: source.into(),
            default: default.into(),
        }
    }
}

/// The part of a row typed or chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Target,
    Source,
    Default,
}

/// A map's rows from its value (`[{ target, source, default }]`); none when it is not one.
pub fn read(v: &Value) -> Option<Vec<MapRow>> {
    v.as_array()?
        .iter()
        .map(|r| {
            let text = |k: &str| r.get(k).and_then(Value::as_str).map(str::to_owned);
            Some(MapRow {
                target: text("target")?,
                source: text("source")?,
                default: text("default")?,
            })
        })
        .collect()
}

/// A map as its value holds it.
pub fn value_of(map: &[MapRow]) -> Value {
    Value::Array(
        map.iter()
            .map(|r| json!({ "target": r.target, "source": r.source, "default": r.default }))
            .collect(),
    )
}

/// A row as the field shows it: what is typed, and why it cannot run (shown on the row).
#[derive(Clone, Debug, PartialEq)]
pub struct RowView {
    pub target: String,
    pub source: String,
    pub default: String,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldMapView {
    pub rows: Vec<RowView>,
    /// The target's fields, offered in a row's target list.
    pub targets: Vec<String>,
    /// The source's columns, offered in a row's source list; empty while they are not known.
    pub columns: Vec<String>,
    /// Under the rows: what an empty map does, or that the columns come with the source.
    pub note: Option<&'static str>,
    /// Aynı adlarla doldur can add a row (the columns are known and one is not mapped yet).
    pub fills: bool,
}

/// The rows with their problems, in order: a target left empty, a target an
/// earlier row has, a source column the source does not have (only when its
/// columns are known: `columns` none while no source is read).
pub fn view(
    map: &[MapRow],
    targets: Option<&[String]>,
    columns: Option<&[String]>,
    key: &str,
) -> FieldMapView {
    let mut seen: Vec<&str> = Vec::new();
    let rows = map
        .iter()
        .map(|r| {
            let target = js_trim(&r.target);
            let source = js_trim(&r.source);
            let problem = if target.is_empty() {
                Some(EMPTY_TARGET.to_owned())
            } else if seen.contains(&target) {
                Some(twice(target))
            } else if columns.is_some_and(|c| !source.is_empty() && !c.iter().any(|n| n == source))
            {
                Some(no_column(source))
            } else {
                None
            };
            if !target.is_empty() {
                seen.push(target);
            }
            RowView {
                target: r.target.clone(),
                source: r.source.clone(),
                default: r.default.clone(),
                problem,
            }
        })
        .collect();
    let note = match columns {
        None => Some(NO_SOURCE),
        Some(_) if map.is_empty() => Some(EMPTY),
        Some(_) => None,
    };
    FieldMapView {
        rows,
        targets: targets.unwrap_or(&[]).to_vec(),
        columns: columns.unwrap_or(&[]).to_vec(),
        note,
        fills: columns.is_some_and(|c| filled(map, c, key).len() > map.len()),
    }
}

/// A row's part typed or chosen.
pub fn with_part(map: &[MapRow], i: usize, part: Part, text: &str) -> Vec<MapRow> {
    let mut out = map.to_vec();
    if let Some(r) = out.get_mut(i) {
        let slot = match part {
            Part::Target => &mut r.target,
            Part::Source => &mut r.source,
            Part::Default => &mut r.default,
        };
        text.clone_into(slot);
    }
    out
}

/// A row taken away (×).
pub fn without_row(map: &[MapRow], i: usize) -> Vec<MapRow> {
    map.iter()
        .enumerate()
        .filter(|(k, _)| *k != i)
        .map(|(_, r)| r.clone())
        .collect()
}

/// Satır ekle: an empty row last.
pub fn with_new_row(map: &[MapRow]) -> Vec<MapRow> {
    let mut out = map.to_vec();
    out.push(MapRow::default());
    out
}

/// Aynı adlarla doldur: the rows as they are, then a row under its own name
/// for each column (in the source's order) but the key, one no row takes as
/// its source and none has as its target yet.
pub fn filled(map: &[MapRow], columns: &[String], key: &str) -> Vec<MapRow> {
    let mut out = map.to_vec();
    let mut taken: Vec<String> = map
        .iter()
        .flat_map(|r| [js_trim(&r.target), js_trim(&r.source)])
        .map(str::to_owned)
        .collect();
    let key = js_trim(key);
    for c in columns {
        let name = js_trim(c);
        if name.is_empty() || name == key || taken.iter().any(|t| t == name) {
            continue;
        }
        taken.push(name.to_owned());
        out.push(MapRow::new(name, name, ""));
    }
    out
}

/// A map's first problem, its rows in order: a target field left empty, one
/// named twice, or a name a field cannot have (as a field parameter's: at
/// most 64 characters, no square brackets). `name` is the parameter's, quoted.
pub fn problem(name: &str, map: &[MapRow]) -> Option<String> {
    let mut seen: Vec<&str> = Vec::new();
    for (i, r) in map.iter().enumerate() {
        let target = js_trim(&r.target);
        if target.is_empty() {
            return Some(format!("{name}: {}. satırın hedef alanı boş.", i + 1));
        }
        if seen.contains(&target) {
            return Some(format!("{name}: “{target}” iki satırda hedef."));
        }
        seen.push(target);
        for f in [target, js_trim(&r.source)] {
            if utf16_len(f) > 64 {
                return Some(format!("{name}: alan adı en çok 64 karakter olabilir."));
            }
            if f.contains(['[', ']']) {
                return Some(format!("{name}: alan adında köşeli parantez kullanılamaz."));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/processing/v1/field-map.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases read"))
            .expect("the cases are JSON")
    }

    fn names(v: &Value) -> Option<Vec<String>> {
        v.as_array().map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
    }

    fn view_value(v: &FieldMapView) -> Value {
        json!({
            "rows": v.rows.iter().map(|r| json!({
                "target": r.target, "source": r.source, "default": r.default, "problem": r.problem,
            })).collect::<Vec<_>>(),
            "targets": v.targets,
            "columns": v.columns,
            "note": v.note,
            "fills": v.fills,
        })
    }

    /// The shared cases (fixtures/processing/v1/field-map.json), written by the
    /// independent reference (scripts/fixtures/field_map_cases.py); the web plays
    /// them too (`ui/processing/fieldMap.test.ts`).
    #[test]
    fn the_field_map_cases_hold() {
        let f = fixture();
        assert_eq!(
            (f["format"].as_str(), f["version"].as_u64()),
            (Some("kentos.field-map-cases"), Some(1))
        );
        let mut count = 0;
        for c in f["views"].as_array().expect("views") {
            let map = read(&c["value"]).expect("a map");
            let got = view(
                &map,
                names(&c["targets"]).as_deref(),
                names(&c["columns"]).as_deref(),
                c["key"].as_str().unwrap_or(""),
            );
            assert_eq!(view_value(&got), c["expect"], "{}", c["title"]);
            count += 1;
        }
        for c in f["fills"].as_array().expect("fills") {
            let map = read(&c["value"]).expect("a map");
            let columns = names(&c["columns"]).expect("columns");
            let got = filled(&map, &columns, c["key"].as_str().unwrap_or(""));
            assert_eq!(value_of(&got), c["expect"], "{}", c["title"]);
            count += 1;
        }
        for c in f["problems"].as_array().expect("problems") {
            let map = read(&c["value"]).expect("a map");
            let got = problem(c["name"].as_str().unwrap_or(""), &map);
            assert_eq!(json!(got), c["expect"], "{}", c["title"]);
            count += 1;
        }
        for (n, e) in f["edits"].as_array().expect("edits").iter().enumerate() {
            let map = read(&e["value"]).expect("a map");
            let i = e["index"].as_u64().unwrap_or(0) as usize;
            let got = match e["op"].as_str() {
                Some("set") => {
                    let patch = e["patch"].as_object().expect("a patch");
                    patch.iter().fold(map, |m, (k, v)| {
                        let part = match k.as_str() {
                            "target" => Part::Target,
                            "source" => Part::Source,
                            _ => Part::Default,
                        };
                        with_part(&m, i, part, v.as_str().unwrap_or(""))
                    })
                }
                Some("remove") => without_row(&map, i),
                _ => with_new_row(&map),
            };
            assert_eq!(value_of(&got), e["expect"], "edit {}", n + 1);
            count += 1;
        }
        assert!(count >= 27, "{count} cases");
    }

    /// The field's words are the web's (`DIALOG_TEXTS.fieldMap`, recorded in
    /// fixtures/processing/v1/dialog.json).
    #[test]
    fn the_texts_are_the_webs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/processing/v1/dialog.json");
        let dialog: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("dialog.json reads"))
                .expect("dialog.json is JSON");
        let t = &dialog["texts"]["fieldMap"];
        for (key, text) in [
            ("target", TARGET),
            ("source", SOURCE),
            ("default", DEFAULT),
            ("none", NONE),
            ("add", ADD),
            ("fill", FILL),
            ("fillHint", FILL_HINT),
            ("remove", REMOVE),
            ("empty", EMPTY),
            ("noSource", NO_SOURCE),
            ("emptyTarget", EMPTY_TARGET),
        ] {
            assert_eq!(t[key], text, "{key}");
        }
        let sample = |key: &str| t[key]["sample"].as_str().unwrap_or("").to_owned();
        assert_eq!(t["twice"]["text"], twice(&sample("twice")));
        assert_eq!(t["noColumn"]["text"], no_column(&sample("noColumn")));
    }

    #[test]
    fn a_value_that_is_not_a_map_reads_as_none() {
        assert_eq!(read(&json!([{ "target": "A", "source": "B" }])), None);
        assert_eq!(read(&json!({ "target": "A" })), None);
        assert_eq!(read(&json!([])), Some(Vec::new()));
    }
}
