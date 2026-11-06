//! Paketle's resource report (docs/adr/1111): fixtures/commands/v1/file.package.resources.json, written by
//! scripts/fixtures/package_resource_cases.py from the ADR's rules without KentOS code, played here. Each case's drawing
//! is written as a project file in its work folder, opened and its package planned; the answer's `resources` must be
//! the reference's row for row and its warnings the reference's (the resources' warning word for word). A plan writes
//! no package.

use std::path::{Path, PathBuf};

use kentos_headless::Session;
use serde_json::{Value, json};

use super::scratch;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

/// `$fixture:` paths made the fixtures folder's, anywhere in a value.
fn placed(v: &Value) -> Value {
    match v {
        Value::String(s) => match s.strip_prefix("$fixture:") {
            Some(rest) => Value::String(fixtures().join(rest).display().to_string()),
            None => v.clone(),
        },
        Value::Array(list) => Value::Array(list.iter().map(placed).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(k, x)| (k.clone(), placed(x))).collect()),
        other => other.clone(),
    }
}

/// The warnings as the case says them: each code, and the resources' warning's words.
fn told(warnings: &Value) -> Value {
    Value::Array(
        warnings
            .as_array()
            .into_iter()
            .flatten()
            .map(|w| {
                if w["code"] == "package_resources" {
                    json!({ "code": w["code"], "message": w["message"] })
                } else {
                    json!({ "code": w["code"] })
                }
            })
            .collect(),
    )
}

fn opened(setup: &Value, work: &Path) -> Session {
    std::fs::create_dir_all(work).expect("a work folder");
    let typed: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(placed(setup)).expect("a snapshot");
    let project = work.join("proje.kcad");
    std::fs::write(&project, kentos_kcad::encode_verified(&typed).expect("encoded")).expect("written");
    Session::open(&project).expect("opens")
}

#[test]
fn every_package_resource_case_is_reported_as_the_reference_says() {
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("commands/v1/file.package.resources.json")).expect("the cases"),
    )
    .expect("JSON");
    assert_eq!(fixture["command"], "file.package");
    assert_eq!(fixture["host"], "headless-file");
    let list = fixture["cases"].as_array().expect("cases");
    assert!(list.len() >= 20, "{} cases", list.len());
    let dir = scratch("package-resources");
    let mut problems = Vec::new();
    for (k, case) in list.iter().enumerate() {
        let name = case["name"].as_str().unwrap_or("?");
        let work = dir.join(format!("durum-{k}"));
        let mut s = opened(&case["setup"], &work);
        let target = work.join("paket");
        let answer = s
            .call(json!({ "command": "file.package", "op": case["op"],
                "input": { "path": target.display().to_string(), "from": work.display().to_string() } }))
            .expect("answers");
        let want = &case["expect"];
        if answer["status"] != want["status"] {
            problems.push(format!("{name}: {answer}"));
            continue;
        }
        let got = answer["output"]["resources"].clone();
        let got = if got.is_null() { json!([]) } else { got };
        if got != want["resources"] {
            problems.push(format!("{name}:\n  want {}\n  got  {got}", want["resources"]));
        }
        let warned = told(&answer["warnings"]);
        if warned != want["warnings"] {
            problems.push(format!("{name}: warnings\n  want {}\n  got  {warned}", want["warnings"]));
        }
        if target.exists() {
            problems.push(format!("{name}: the plan wrote the package"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
