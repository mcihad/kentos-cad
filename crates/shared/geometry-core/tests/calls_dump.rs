//! The core's answers to named cases of the frozen call fixtures
//! (`fixtures/geometry/v1/calls-*.json`), for reviewing a deliberate change
//! of behaviour before a fixture is updated: each answer is checked by an
//! independent reference first (docs/adr/0149 §5.3 did so for the curves'
//! outlines). Run by hand; prints one JSON line a case:
//!
//! ```text
//! KENTOS_DUMP_CALLS='entityEdges:rastgele 1|pathOf:rastgele 8' \
//!   cargo test -p kentos-geometry-core --test calls_dump -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

#[test]
#[ignore = "a review tool, run by hand"]
fn dump_named_cases() {
    let wanted: Vec<(String, String)> = std::env::var("KENTOS_DUMP_CALLS")
        .unwrap_or_default()
        .split('|')
        .filter_map(|w| w.split_once(':'))
        .map(|(f, n)| (f.to_owned(), n.to_owned()))
        .collect();
    // `fn:name>other` also runs `other` on the case's first argument (the object's edges, say).
    let also = |f: &str, n: &str| -> Option<String> {
        wanted
            .iter()
            .find(|(wf, wn)| wf == f && wn.split('>').next() == Some(n))
            .and_then(|(_, wn)| wn.split_once('>').map(|(_, o)| o.to_owned()))
    };
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geometry/v1");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixture directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("calls-") && n.ends_with(".json"))
        })
        .collect();
    files.sort();
    for path in files {
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture file"))
                .expect("fixture JSON");
        for c in file["cases"].as_array().expect("cases") {
            let (f, n) = (c["fn"].as_str().unwrap(), c["name"].as_str().unwrap());
            if !wanted
                .iter()
                .any(|(wf, wn)| wf == f && wn.split('>').next() == Some(n))
            {
                continue;
            }
            let args = serde_json::to_string(&c["args"]).unwrap();
            let out = run_named(f, &args).expect("an answer");
            let out: Value = serde_json::from_str(&out).expect("answer JSON");
            let file_name = path.file_name().unwrap().to_string_lossy();
            let other = also(f, n).map(|o| {
                let first = serde_json::to_string(&json!([c["args"][0]])).unwrap();
                let out = run_named(&o, &first).expect("an answer");
                json!({ "fn": o, "got": serde_json::from_str::<Value>(&out).expect("answer JSON") })
            });
            println!(
                "{}",
                json!({ "file": file_name, "fn": f, "name": n, "got": out, "also": other })
            );
        }
    }
}
