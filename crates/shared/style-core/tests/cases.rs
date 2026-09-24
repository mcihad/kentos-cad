//! The frozen answers of the expression language (fixtures/expression/v1/cases.json,
//! recorded by apps/web/scripts/fixtures/record-expression.test.ts after the core
//! agreed with the TypeScript it replaced), natively: each case's objects become
//! the table the browser sends (`expr::rows`), evaluated in every result mode.

use kentos_style_core::expr::rows::{As, BOOL, EMPTY, NUMBER, RowsInput, TEXT, evaluate_rows};
use kentos_style_core::expr::{self, Expr};
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/expression/v1/cases.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A number as the fixture writes it ("-0", "Infinity", "-Infinity" as text).
fn num(v: &Json) -> f64 {
    match v {
        Json::String(s) if s == "-0" => -0.0,
        Json::String(s) if s == "Infinity" => f64::INFINITY,
        Json::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        v => v.as_f64().unwrap_or_else(|| panic!("sayı değil: {v}")),
    }
}

/// The table for `e` (the layout `expr::rows` documents), from the fixture's objects.
fn table(e: &Expr, objects: &[Json]) -> (String, Vec<i32>, Vec<f64>, Vec<f64>) {
    let mut texts = String::new();
    let mut lens = Vec::new();
    let mut numbers = Vec::new();
    let mut measures = Vec::new();
    let mut put = |v: Option<&str>| match v {
        None => lens.push(-1),
        Some(s) => {
            texts.push_str(s);
            lens.push(s.encode_utf16().count() as i32);
        }
    };
    for o in objects {
        for f in &e.fields {
            put(o["attrs"].get(f).and_then(Json::as_str));
        }
        if e.needs.label {
            put(o["label"].as_str());
        }
        if e.needs.layer {
            put(o["layer"].as_str());
        }
        if e.needs.kind {
            put(o["kind"].as_str());
        }
        if e.needs.id {
            numbers.push(num(&o["id"]));
        }
        if e.needs.vertices {
            numbers.push(o["vertices"].as_f64().unwrap_or(f64::NAN));
        }
        if e.needs.measured {
            measures.extend(o["measures"].as_array().into_iter().flatten().map(num));
        }
    }
    (texts, lens, numbers, measures)
}

#[test]
fn the_core_gives_the_frozen_answers() {
    let file = fixture();
    let cases = file["cases"].as_array().unwrap_or_else(|| panic!("cases"));
    let mut checked = 0;
    for c in cases {
        let source = c["source"].as_str().unwrap_or_default();
        match (expr::compile(source), c.get("error")) {
            (Err(got), Some(want)) => {
                assert_eq!(
                    got.message,
                    want["message"].as_str().unwrap_or_default(),
                    "{source:?}"
                );
                assert_eq!(got.at as f64, num(&want["at"]), "{source:?}");
            }
            (Ok(e), None) => {
                let fields: Vec<&str> = c["fields"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Json::as_str)
                    .collect();
                assert_eq!(e.fields, fields, "{source:?}");
                let objects = c["objects"].as_array().cloned().unwrap_or_default();
                let (texts, lens, numbers, measures) = table(&e, &objects);
                let scale = c["plotScale"].as_f64().unwrap_or(f64::NAN);
                let input = RowsInput {
                    n: objects.len(),
                    texts: &texts,
                    text_lens: &lens,
                    numbers: &numbers,
                    measures: &measures,
                    scale,
                };
                for (mode, want) in c["results"].as_object().into_iter().flatten() {
                    let as_ = match mode.as_str() {
                        "number" => As::Number,
                        "text" => As::Text,
                        "bool" => As::Bool,
                        "textNumber" => As::TextNumber,
                        _ => As::Value,
                    };
                    let got = evaluate_rows(&e, &input, as_)
                        .unwrap_or_else(|m| panic!("{source:?}: {m}"));
                    let mut text_at = 0;
                    let mut t = 0;
                    for (i, w) in want.as_array().into_iter().flatten().enumerate() {
                        let kind = got.kinds[i];
                        let what = format!("{source:?} [{mode}] nesne {}", i + 1);
                        match w
                            .as_array()
                            .map(|p| (p[0].as_str().unwrap_or_default(), &p[1]))
                        {
                            None => assert_eq!(kind, EMPTY, "{what}"),
                            Some(("n", v)) => {
                                assert_eq!(kind, NUMBER, "{what}");
                                assert_eq!(got.numbers[i].to_bits(), num(v).to_bits(), "{what}");
                            }
                            Some(("b", v)) => {
                                assert_eq!(kind, BOOL, "{what}");
                                assert_eq!(
                                    got.numbers[i] == 1.0,
                                    v.as_bool().unwrap_or_default(),
                                    "{what}"
                                );
                            }
                            Some((_, v)) => {
                                assert_eq!(kind, TEXT, "{what}");
                                // The value's UTF-16 length, in bytes of the joined texts.
                                let len = got.text_lens[t] as usize;
                                let rest = &got.texts[text_at..];
                                let mut units = 0;
                                let bytes = rest
                                    .char_indices()
                                    .find(|&(_, ch)| {
                                        let done = units >= len;
                                        units += ch.len_utf16();
                                        done
                                    })
                                    .map_or(rest.len(), |(b, _)| b);
                                assert_eq!(
                                    &rest[..bytes],
                                    v.as_str().unwrap_or_default(),
                                    "{what}"
                                );
                                text_at += bytes;
                                t += 1;
                            }
                        }
                        checked += 1;
                    }
                }
            }
            (got, want) => panic!("{source:?}: derleme {got:?}, beklenen hata {want:?}"),
        }
    }
    assert!(checked > 1000, "{checked}");
}
