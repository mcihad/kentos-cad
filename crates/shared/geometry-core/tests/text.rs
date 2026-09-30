//! A text's editing rules (docs/adr/0145 §3, §6) against the shared cases
//! (fixtures/text/v1, written from the rules by scripts/fixtures/text_cases.py):
//! Artır's next number, Bul ve değiştir's matching, Okunur yap's turn and
//! the point a new alignment gives,
//! through the core's call table, as the web calls them through WASM
//! (apps/web/src/model/textEdit.test.ts).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/text/v1")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(file["format"], "kentos.text-cases");
    assert_eq!(file["version"], 1);
    file["cases"].as_array().expect("cases").clone()
}

fn call(name: &str, args: Value) -> Value {
    let out = run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_str(&out).expect("the core's JSON")
}

#[test]
fn artır_gives_the_next_number() {
    let cases = cases("increment.json");
    assert!(cases.len() >= 10);
    for c in cases {
        let got = call("textIncrement", json!([c["text"]]));
        assert_eq!(got, c["next"], "{}", c["name"]);
    }
}

#[test]
fn bul_ve_değiştir_matches_as_the_rules_say() {
    let cases = cases("pattern.json");
    assert!(cases.len() >= 15);
    for c in cases {
        let how = json!({
            "wildcard": c["wildcard"],
            "caseless": c["caseless"],
            "wholeWord": c["wholeWord"],
        });
        let got = call(
            "textReplace",
            json!([[c["text"]], c["find"], c["replace"], how]),
        );
        assert_eq!(got, json!([c["expect"]]), "{}", c["name"]);
    }
}

#[test]
fn okunur_yap_turns_what_reads_upside_down_about_its_box() {
    let cases = cases("readable.json");
    assert!(cases.len() >= 10);
    for c in cases {
        let got = call("textReadable", json!([c]));
        let want = &c["expect"];
        if want.is_null() {
            assert!(got.is_null(), "{}: {got}", c["name"]);
            continue;
        }
        let num = |v: &Value| v.as_f64().expect("a number");
        // The point to a nanometre (float64 at 10⁶ m is a tenth of that); the turn exactly.
        for k in ["x", "y"] {
            let (g, w) = (num(&got["p"][k]), num(&want["p"][k]));
            assert!((g - w).abs() <= 1e-9, "{}: {k} {g} ≠ {w}", c["name"]);
        }
        assert_eq!(
            num(&got["rotation"]),
            num(&want["rotation"]),
            "{}",
            c["name"]
        );
    }
}

#[test]
fn hizayı_değiştir_keeps_the_text_where_it_is() {
    let cases = cases("realign.json");
    assert!(cases.len() >= 5);
    for c in cases {
        let got = call("textRealign", json!([c, c["to"]]));
        let num = |v: &Value| v.as_f64().expect("a number");
        for k in ["x", "y"] {
            let (g, w) = (num(&got[k]), num(&c["expect"][k]));
            assert!((g - w).abs() <= 1e-9, "{}: {k} {g} ≠ {w}", c["name"]);
        }
    }
}
