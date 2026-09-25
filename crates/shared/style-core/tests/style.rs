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
            },
            clip.as_ref(),
            origin,
            num(&c["plotScale"]),
        )
        .unwrap_or_else(|e| panic!("katman {i}: {e}"));
        let batches: Json =
            serde_json::from_str(&out.json).unwrap_or_else(|e| panic!("katman {i}: {e}"));
        assert_eq!(batches, c["batches"], "katman {i}: topluluklar");
        let want = nums(&c["data"]);
        assert_eq!(out.data.len(), want.len(), "katman {i}: sayılar");
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
}
