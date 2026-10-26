//! The time filter of export (docs/adr/0646 §2): fixtures/time-export/v1/cases.json, written by
//! scripts/fixtures/time_export_cases.py, runs each export of `drawing.kcad` with its time and compares the answer's
//! status, warnings and refusal. With KENTOS_WRITE_TIME_EXPORT=1 the files go to fixtures/time-export/v1/out for
//! `time_export_cases.py --written`, which reads the objects they hold with OGR and pyarrow.

use std::path::{Path, PathBuf};

use kentos_headless::Session;
use serde_json::Value;

use super::scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/time-export/v1")
}

#[test]
fn time_exports() {
    let cases: Value =
        serde_json::from_slice(&std::fs::read(root().join("cases.json")).expect("the cases"))
            .expect("JSON");
    let out = if std::env::var_os("KENTOS_WRITE_TIME_EXPORT").is_some() {
        let dir = root().join("out");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the folder");
        dir
    } else {
        scratch("time-export")
    };
    let mut s = Session::open(&root().join("drawing.kcad")).expect("opens");
    let before = s.document().revision();
    for case in cases["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let mut input = case["input"].clone();
        let path = input["path"].as_str().expect("path").replace("$out:", "");
        input["path"] = Value::String(out.join(&path).display().to_string());
        let answer = s
            .call(serde_json::json!({ "command": case["command"], "input": input }))
            .expect("answers");
        assert_eq!(answer["status"], case["status"], "{name}: {answer}");
        if case["status"] == "completed" {
            let time_warnings: Vec<&Value> = answer["warnings"]
                .as_array()
                .map(|w| w.iter().filter(|w| w["code"] == "time_left_out").collect())
                .unwrap_or_default();
            let expected: Vec<&Value> = case["warnings"]
                .as_array()
                .map(|w| w.iter().collect())
                .unwrap_or_default();
            assert_eq!(time_warnings, expected, "{name}: {answer}");
            assert!(out.join(&path).exists(), "{name}: not written");
        } else {
            assert_eq!(
                answer["error"]["code"], case["error"]["code"],
                "{name}: {answer}"
            );
            assert_eq!(
                answer["error"]["message"], case["error"]["message"],
                "{name}: {answer}"
            );
            assert_eq!(
                answer["error"]["path"], case["error"]["path"],
                "{name}: {answer}"
            );
        }
    }
    // An export leaves the open drawing as it was.
    assert_eq!(s.document().revision(), before);
}
