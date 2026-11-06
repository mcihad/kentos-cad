//! Kurtar (docs/adr/1013): fixtures/commands/v1/file.recover.json, written by scripts/fixtures/recover_cases.py from
//! damages made byte by byte to a valid drawing (no KentOS code), played through the host's own `call`. Each case's
//! file is copied to its work folder as `cizim.kcad`; the answer must be the reference's, a written copy must open
//! with the reference's object ids, an unwritten one must not be there, and the source must be byte for byte as it was.

use std::path::{Path, PathBuf};

use kentos_headless::Session;
use serde_json::{Value, json};

use super::new_drawing;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

fn placed(v: &Value, work: &Path) -> Value {
    match v {
        Value::String(s) => match s.strip_prefix("$work:") {
            Some(rest) => Value::String(work.join(rest).display().to_string()),
            None => v.clone(),
        },
        Value::Object(map) => Value::Object(map.iter().map(|(k, x)| (k.clone(), placed(x, work))).collect()),
        other => other.clone(),
    }
}

fn hyphenated(id: &kentos_commands::EntityId) -> String {
    let h: String = id.0.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

#[test]
fn every_recovery_case_answers_as_the_reference_says() {
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures().join("commands/v1/file.recover.json")).expect("the cases"))
            .expect("json");
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    let mut s: Session = new_drawing();
    for (n, c) in cases.iter().enumerate() {
        let name = c["name"].as_str().unwrap_or("");
        let work = super::scratch(&format!("kurtar-{n}"));
        let source = work.join("cizim.kcad");
        std::fs::copy(fixtures().join("kcad/recover/v1").join(c["file"].as_str().unwrap()), &source).expect("copied");
        let before = std::fs::read(&source).expect("the source");
        for made in c["make"].as_array().into_iter().flatten() {
            std::fs::write(work.join(made.as_str().unwrap()), b"var olan").expect("made");
        }
        let mut input = json!({ "path": source.display().to_string() });
        for (k, v) in c["input"].as_object().into_iter().flatten() {
            input[k] = placed(v, &work);
        }
        let answer = s.call(json!({ "command": "file.recover", "op": c["op"], "input": input })).expect("answers");
        let result = if answer.get("status").is_some() { answer.clone() } else { answer["result"].clone() };
        let want = &c["expect"];
        assert_eq!(std::fs::read(&source).expect("the source"), before, "{name}: the source is untouched");
        if want["status"] == "failed" {
            assert_eq!(result["status"], "failed", "{name}: {result}");
            assert_eq!(result["error"]["code"], want["error"]["code"], "{name}: {result}");
            assert_eq!(result["error"]["path"], want["error"]["path"], "{name}: {result}");
            continue;
        }
        assert_eq!(result["status"], "completed", "{name}: {result}");
        if c["op"] == "validate" {
            assert_eq!(result["output"], Value::Null, "{name}");
            continue;
        }
        let out = &result["output"];
        let target = placed(&want["target"], &work);
        assert_eq!(out["target"], target, "{name}");
        for key in ["whole", "written", "objectsRead", "objectsKept"] {
            assert_eq!(out[key], want[key], "{name}: {key}: {out}");
        }
        for part in want["containerHas"].as_array().into_iter().flatten() {
            let words = part.as_str().unwrap();
            assert!(
                out["container"].as_array().into_iter().flatten().any(|c| c.as_str().unwrap_or("").contains(words)),
                "{name}: the container says “{words}”: {}",
                out["container"]
            );
        }
        let lost = out["lost"].as_array().expect("lost");
        let wanted = want["lost"].as_array().expect("lost");
        assert_eq!(lost.len(), wanted.len(), "{name}: {}", out["lost"]);
        for (got, w) in lost.iter().zip(wanted) {
            assert_eq!(got["part"], w["part"], "{name}");
            if let Some(uid) = w.get("uid") {
                assert_eq!(&got["uid"], uid, "{name}");
            }
            if let Some(code) = w["code"].as_str() {
                assert!(got["reason"].as_str().unwrap_or("").starts_with(&format!("{code}:")), "{name}: {}", got["reason"]);
            }
        }
        let path = PathBuf::from(target.as_str().unwrap());
        if want["written"] == true {
            let copy = kentos_kcad::decode(&std::fs::read(&path).expect("the copy")).expect("the copy opens");
            let ids: Vec<String> = copy.uids.iter().map(hyphenated).collect();
            assert_eq!(json!(ids), want["uids"], "{name}");
        } else if c["make"].as_array().is_none_or(|m| m.is_empty()) {
            assert!(!path.exists(), "{name}: nothing is written");
        }
    }
}
