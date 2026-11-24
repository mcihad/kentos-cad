//! The frozen answers of the style compiler (fixtures/style/v1/cases.json,
//! recorded by apps/web/scripts/fixtures/record-style.test.ts after the core
//! agreed with the TypeScript it replaced), natively: one symbol on one
//! object through the preview entry, and whole layers from what the page
//! gave the core (the program, the objects' numbers, the expression table)
//! with a geometry store of the case's objects. Same bits as through WASM.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json as CoreJson;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_style_core::style::build::{LayerObjects, Program, build_layer, compile_one};
use serde_json::{Value as Json, json};

fn fixture() -> Json {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/style/v1/cases.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A number as the fixture writes it ("NaN", "Infinity", "-Infinity", "-0" as text).
fn num(v: &Json) -> f64 {
    match v {
        Json::String(s) if s == "-0" => -0.0,
        Json::String(s) if s == "NaN" => f64::NAN,
        Json::String(s) if s == "Infinity" => f64::INFINITY,
        Json::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        v => v.as_f64().unwrap_or_else(|| panic!("sayı değil: {v}")),
    }
}

fn nums(v: &Json) -> Vec<f64> {
    v.as_array().into_iter().flatten().map(num).collect()
}

#[test]
fn one_symbol_on_one_object_as_frozen() {
    let file = fixture();
    let cases = file["symbols"]
        .as_array()
        .unwrap_or_else(|| panic!("symbols"));
    for (i, c) in cases.iter().enumerate() {
        let input = json!({
            "symbol": c["symbol"],
            "entity": c["entity"],
            "layerName": c["layerName"],
            "kindLabel": c["kindLabel"],
            "vertices": c["vertices"],
            "plotScale": c["plotScale"],
            "assets": file["assets"],
        });
        let input =
            CoreJson::parse(&input.to_string()).unwrap_or_else(|e| panic!("sembol {i}: {e}"));
        let got = compile_one(&input).unwrap_or_else(|e| panic!("sembol {i}: {e}"));
        let got: Json = serde_json::from_str(&got).unwrap_or_else(|e| panic!("sembol {i}: {e}"));
        assert_eq!(got, c["primitives"], "sembol {i}");
    }
    assert!(cases.len() >= 100, "{}", cases.len());
}

#[test]
fn whole_layers_as_frozen() {
    let file = fixture();
    let cases = file["layers"]
        .as_array()
        .unwrap_or_else(|| panic!("layers"));
    let mut numbers_checked = 0;
    // `KENTOS_WRITE_STYLE_CASES=1`: a change meant to change the layers' numbers writes them anew (the web that
    // recorded them is paused, ADR 0228); the batches must still be the frozen ones.
    let writing = std::env::var_os("KENTOS_WRITE_STYLE_CASES").is_some();
    let mut rewritten: Vec<Vec<f32>> = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let mut store = Store::new();
        store
            .put_json(&c["entities"].to_string())
            .unwrap_or_else(|e| panic!("katman {i}: {e}"));
        let program = Program::read(c["program"].as_str().unwrap_or_default())
            .unwrap_or_else(|e| panic!("katman {i}: {e}"));
        let ids: Vec<f64> = c["entities"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| num(&e["id"]))
            .collect();
        let objects: Vec<i32> = nums(&c["objects"]).iter().map(|&x| x as i32).collect();
        let lens: Vec<i32> = nums(&c["table"]["lens"])
            .iter()
            .map(|&x| x as i32)
            .collect();
        let numbers = nums(&c["table"]["numbers"]);
        let clip = match &c["clip"] {
            Json::Null => None,
            b => Some(Bounds {
                min_x: num(&b["minX"]),
                min_y: num(&b["minY"]),
                max_x: num(&b["maxX"]),
                max_y: num(&b["maxY"]),
            }),
        };
        let origin = Vec2::new(num(&c["origin"]["x"]), num(&c["origin"]["y"]));
        let out = build_layer(
            &store,
            &program,
            &LayerObjects {
                ids: &ids,
                objects: &objects,
                texts: c["table"]["texts"].as_str().unwrap_or_default(),
                text_lens: &lens,
                numbers: &numbers,
                pieces: &[],
                bands: &[],
            },
            clip.as_ref(),
            origin,
            num(&c["plotScale"]),
            false,
        )
        .unwrap_or_else(|e| panic!("katman {i}: {e}"));
        let batches: Json =
            serde_json::from_str(&out.json).unwrap_or_else(|e| panic!("katman {i}: {e}"));
        assert_eq!(batches, c["batches"], "katman {i}: topluluklar");
        let want = nums(&c["data"]);
        assert_eq!(out.data.len(), want.len(), "katman {i}: sayılar");
        if writing {
            rewritten.push(out.data.clone());
            numbers_checked += want.len();
            continue;
        }
        for (k, (got, want)) in out.data.iter().zip(&want).enumerate() {
            // The fixture's numbers are float32 values in their fewest digits, read through a double.
            assert_eq!(
                got.to_bits(),
                (*want as f32).to_bits(),
                "katman {i}: sayı {k}"
            );
        }
        numbers_checked += want.len();
    }
    assert!(numbers_checked > 10_000, "{numbers_checked}");
    if writing {
        write_layer_numbers(&rewritten);
    }
}

/// The layers' `data` written anew in fixtures/style/v1/cases.json, each number that changed as the fewest digits of
/// its float32 (the recorder's form), every other byte of the file as it was: a case is one line of the `layers` list.
fn write_layer_numbers(layers: &[Vec<f32>]) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/style/v1/cases.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let start = text.find("\n \"layers\": [\n").expect("the layers' list");
    let (head, rest) = text.split_at(start);
    let mut out = String::with_capacity(text.len());
    out.push_str(head);
    let mut case = 0;
    let mut changed = 0;
    for line in rest.split_inclusive('\n') {
        let Some(at) = line.find("\"data\":[").filter(|_| line.trim_start().starts_with('{') && case < layers.len()) else {
            out.push_str(line);
            continue;
        };
        let open = at + "\"data\":[".len();
        let close = open + line[open..].find(']').expect("the data's end");
        let old: Vec<&str> = if close > open { line[open..close].split(',').collect() } else { Vec::new() };
        assert_eq!(old.len(), layers[case].len(), "katman {case}: sayılar");
        let new: Vec<String> = old
            .iter()
            .zip(&layers[case])
            .map(|(was, now)| {
                let same = num(&serde_json::from_str::<Json>(was).expect("a number")) as f32;
                if same.to_bits() == now.to_bits() {
                    (*was).to_owned()
                } else {
                    changed += 1;
                    if now.is_nan() { "\"NaN\"".to_owned() } else { format!("{now}") }
                }
            })
            .collect();
        out.push_str(&line[..open]);
        out.push_str(&new.join(","));
        out.push_str(&line[close..]);
        case += 1;
    }
    assert_eq!(case, layers.len(), "every case's line");
    std::fs::write(path, out).unwrap_or_else(|e| panic!("{path}: {e}"));
    println!("{changed} sayı yeniden yazıldı");
}
