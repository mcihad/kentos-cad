//! Coordinate systems read from WKT and PROJ strings and written as them
//! (docs/adr/0168 §5) against `fixtures/geodesy/v1/text.json`
//! (`scripts/fixtures/crs_text_cases.py`: PROJ reads every text there, its
//! numbers and the ADR's rules give the systems; the written texts are the
//! rules', and PROJ reads them back). Everything exactly; the ops the web
//! calls are run on the same cases through the call table.

use kentos_geometry_core::api::json::{FromJson, Json, to_string};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::System;
use kentos_geometry_core::crs::text::{read_text, write_proj, write_wkt};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/text.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.crs-text");
    assert_eq!(file["version"], 1);
    file
}

/// The value with every object's fields in name order (serde_json sorts
/// its own; numbers compared as the core reads them).
fn sorted(v: Json) -> Json {
    match v {
        Json::Obj(mut fields) => {
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            Json::Obj(fields.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        Json::Arr(items) => Json::Arr(items.into_iter().map(sorted).collect()),
        v => v,
    }
}

fn core_json(v: &Value) -> Json {
    sorted(Json::parse(&v.to_string()).expect("JSON"))
}

#[test]
fn texts_read_as_proj_and_the_rules_read_them() {
    let file = fixture();
    let cases = file["reads"].as_array().expect("reads");
    assert!(cases.len() >= 25);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let text = case["text"].as_str().expect("text");
        let got = read_text(text);
        let out = sorted(
            Json::parse(
                &run_named("crsReadText", &json!([text]).to_string()).expect("the op runs"),
            )
            .expect("op JSON"),
        );
        if let Some(error) = case.get("error") {
            let why = got.expect_err(name);
            assert_eq!(
                (why.kind(), why.detail()),
                (
                    error["kind"].as_str().expect("kind"),
                    error["detail"].as_str().expect("detail")
                ),
                "{name}"
            );
            assert_eq!(out, core_json(&json!({ "error": error })), "{name}: the op");
            continue;
        }
        let want = &case["expect"];
        let read = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(read.name, want["name"].as_str().expect("name"), "{name}");
        let system = System::from_json(&Json::parse(&want["system"].to_string()).expect("JSON"))
            .expect("a system");
        assert_eq!(
            sorted(Json::parse(&to_string(&read.system)).expect("JSON")),
            core_json(&want["system"]),
            "{name}: the system"
        );
        assert_eq!(read.system, system, "{name}: the system's numbers");
        assert_eq!(
            read.authority,
            want["authority"].as_u64().map(|a| a as u32),
            "{name}: authority"
        );
        assert_eq!(
            read.registry,
            want["registry"].as_object().map(|r| (
                r["srid"].as_u64().expect("srid") as u32,
                r["exact"].as_bool().expect("exact")
            )),
            "{name}: registry"
        );
        assert_eq!(out, core_json(want), "{name}: the op");
    }
}

#[test]
fn systems_write_as_the_rules_write_them() {
    let file = fixture();
    let cases = file["writes"].as_array().expect("writes");
    assert!(cases.len() >= 8);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let def = &case["definition"];
        let def_name = def["name"].as_str().expect("the definition's name");
        let system = System::from_json(&Json::parse(&def["system"].to_string()).expect("JSON"))
            .expect("a system");
        // A local system's base is named as the reference names it.
        let base_name = match &system {
            System::Local { .. } => case["wkt"]
                .as_str()
                .and_then(|w| w.split("BASEPROJCRS[\"").nth(1))
                .and_then(|w| w.split('"').next()),
            _ => None,
        };
        assert_eq!(
            write_wkt(def_name, &system, base_name).as_deref(),
            case["wkt"].as_str(),
            "{name}: WKT"
        );
        assert_eq!(
            write_proj(&system).as_deref(),
            case["proj"].as_str(),
            "{name}: PROJ"
        );
        let wkt: Value = serde_json::from_str(
            &run_named(
                "crsWriteWkt",
                &json!([def_name, def["system"], base_name]).to_string(),
            )
            .expect("the op runs"),
        )
        .expect("op JSON");
        assert_eq!(wkt, case["wkt"], "{name}: the WKT op");
        let proj: Value = serde_json::from_str(
            &run_named("crsWriteProj", &json!([def["system"]]).to_string()).expect("the op runs"),
        )
        .expect("op JSON");
        assert_eq!(proj, case["proj"], "{name}: the PROJ op");
    }
}
