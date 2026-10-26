//! `cad.rasters.reproject`'s files (docs/adr/0644 §3): every completed case of fixtures/commands/v1/cad.rasters.reproject.json
//! run again with its GeoTIFF written. `KENTOS_WRITE_REPROJECT=1` writes them to fixtures/raster/v1/reproject/out,
//! which `scripts/fixtures/raster_reproject_cases.py --written` compares with the rule's values from PROJ (and
//! gdalwarp's); without it they go to a folder of the test's and are read back.

use std::path::PathBuf;

use kentos_commands::{CommandResult, RastersReproject};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, rasters_reproject};
use serde_json::Value;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

fn placed(v: &Value, out: &std::path::Path) -> Value {
    match v {
        Value::String(t) => Value::String(
            t.replace("$fixture:", &format!("{}/", fixtures().display()))
                .replace("$tmp:", &format!("{}/", out.display())),
        ),
        Value::Object(m) => {
            Value::Object(m.iter().map(|(k, v)| (k.clone(), placed(v, out))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(|v| placed(v, out)).collect()),
        other => other.clone(),
    }
}

#[test]
fn reproject_files() {
    let write = std::env::var_os("KENTOS_WRITE_REPROJECT").is_some();
    let out = if write {
        fixtures().join("raster/v1/reproject/out")
    } else {
        std::env::temp_dir().join(format!("kentos-reproject-{}", std::process::id()))
    };
    std::fs::create_dir_all(&out).unwrap();
    let text =
        std::fs::read_to_string(fixtures().join("commands/v1/cad.rasters.reproject.json")).unwrap();
    let doc_: Value = serde_json::from_str(&text).unwrap();
    let mut written = 0;
    for case in doc_["cases"].as_array().unwrap() {
        let step = &case["steps"][0];
        let output = step["input"]["output"].as_str().unwrap_or("");
        if step["op"] != "execute"
            || step["result"]["status"] != "completed"
            || !output.starts_with("$tmp:")
        {
            continue;
        }
        let setup = serde_json::to_string(&case["setup"]).unwrap();
        let mut doc = Document::from_snapshot(
            kentos_commands::DocumentSnapshotV1::from_json(&setup).unwrap(),
        )
        .unwrap();
        let mut input: RastersReproject =
            serde_json::from_value(placed(&step["input"], &out)).unwrap();
        input.overwrite = Some(true);
        let name = case["name"].as_str().unwrap_or("?");
        match rasters_reproject::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { output, .. } => {
                let bytes = std::fs::read(&output.file).unwrap();
                assert!(bytes.len() > 8, "{name}");
                written += 1;
            }
            other => panic!("{name}: {other:?}"),
        }
    }
    assert!(written >= 10, "{written}");
    if !write {
        let _ = std::fs::remove_dir_all(&out);
    }
}
