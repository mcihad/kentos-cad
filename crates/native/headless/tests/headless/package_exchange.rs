//! The delivery package's open files (docs/adr/2780): the exchange sample drawing (fixtures/exchange/v1/devir.json, the
//! one ADR 1660's and 1880's tables are of) packed with all six formats. Every open file is in `devir/` (the GML's
//! schema too) with the manifest's size and SHA-256, its `from` the writer; the matrix the package carries is the one `file.exchange.matrix`
//! gives for the package's own project opened from the folder; the report says each cell in the window's words; what
//! the files lose is warned once. A left-out link's objects stay out of the open files too, and the question's refusals
//! come under `exchange.`.

use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_headless::Session;
use serde_json::{Value, json};

use super::scratch;

fn opened(work: &Path) -> Session {
    let text = include_str!("../../../../../fixtures/exchange/v1/devir.json");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(text).expect("the sample");
    let project = work.join("devir.kcad");
    std::fs::write(&project, kentos_kcad::encode_verified(&snapshot).expect("encoded")).expect("written");
    Session::open(&project).expect("opens")
}

fn call(s: &mut Session, command: &str, op: &str, input: Value) -> Value {
    s.call(json!({ "command": command, "op": op, "input": input })).expect("answered")
}

#[test]
fn the_package_delivers_the_open_files_with_their_matrix() {
    let work = scratch("paket-devir");
    let mut s = opened(&work);
    let out = work.join("teslim");
    let input = json!({ "path": out.display().to_string(), "name": "ada", "exchange": {} });
    let r = call(&mut s, "file.package", "execute", input);
    assert_eq!(r["status"], "completed", "{r}");
    let o = &r["output"];
    // Every format's file, then the report, each in the folder with the manifest's size and digest.
    let made: Vec<(&str, &str)> = o["files"]
        .as_array()
        .expect("files")
        .iter()
        .filter(|f| f["kind"] == "exchange")
        .map(|f| (f["path"].as_str().unwrap_or(""), f["from"].as_str().unwrap_or("")))
        .collect();
    assert_eq!(
        made,
        [
            ("devir/ada.gpkg", "file.export.gpkg"),
            ("devir/ada.parquet", "file.export.parquet"),
            ("devir/ada.fgb", "file.export.fgb"),
            ("devir/ada.gml", "file.export.gml"),
            // The GML's application schema beside it.
            ("devir/ada.xsd", "file.export.gml"),
            ("devir/ada.zip", "file.export.shp"),
            ("devir/ada.dxf", "file.export.dxf"),
            ("devir/devir-raporu.csv", "file.exchange.matrix"),
        ]
    );
    for f in o["files"].as_array().into_iter().flatten().filter(|f| f["kind"] == "exchange") {
        let bytes = std::fs::read(out.join(f["path"].as_str().unwrap_or(""))).expect("in the package");
        assert_eq!(bytes.len() as u64, f["bytes"].as_u64().unwrap_or(0), "{}", f["path"]);
        assert_eq!(sha256_hex(&bytes), f["sha256"].as_str().unwrap_or(""), "{}", f["path"]);
    }
    let manifest: Value = serde_json::from_slice(&std::fs::read(out.join("paket.json")).expect("the manifest")).expect("JSON");
    assert_eq!(manifest["exchange"], o["exchange"], "the manifest carries the answer's matrix");
    // The matrix is the package's own project's: opened from the folder, `file.exchange.matrix` gives the same rows.
    let mut packed = Session::open(&out.join("ada.kcad")).expect("the package opens");
    let m = call(&mut packed, "file.exchange.matrix", "execute", json!({}));
    assert_eq!(m["status"], "completed", "{m}");
    assert_eq!(m["output"]["rows"], o["exchange"]["rows"]);
    // ADR 1660's table: text is lost outside DXF, the circle turns into another kind everywhere.
    let objects = &o["exchange"]["rows"][0];
    assert_eq!(objects["aspect"], "objects");
    assert_eq!(objects["cells"][5]["state"], "kept", "DXF keeps the text");
    assert!(objects["cells"].as_array().into_iter().flatten().take(5).all(|c| c["state"] == "lost"), "{objects}");
    // The report: a head of the formats and a row for each aspect, in the window's words.
    let report = std::fs::read_to_string(out.join("devir/devir-raporu.csv")).expect("the report");
    let lines: Vec<&str> = report.lines().collect();
    assert_eq!(lines[0], "Özellik;GeoPackage;GeoParquet;FlatGeobuf;GML;Shapefile;DXF");
    assert_eq!(lines.len(), 1 + kentos_commands::EXCHANGE_ASPECTS.len());
    assert!(lines[1].starts_with("Nesneler;Kaybolur: yazı"), "{}", lines[1]);
    assert!(lines[1].ends_with(";Korunur"), "{}", lines[1]);
    // What the files lose, said once.
    let losses: Vec<&Value> = r["warnings"].as_array().into_iter().flatten().filter(|w| w["code"] == "export_losses").collect();
    assert_eq!(losses.len(), 1, "{}", r["warnings"]);
    assert!(losses[0]["message"].as_str().unwrap_or("").contains("GeoPackage: nesneler"), "{}", losses[0]);
    // Nothing of the plan's scratch is left in the package's folder.
    assert!(!out.join("devir").join("devir.gpkg").exists());
}

#[test]
fn one_format_asked_gives_its_file_and_a_plan_writes_none() {
    let work = scratch("paket-devir-tek");
    let mut s = opened(&work);
    let out = work.join("teslim");
    let input = json!({ "path": out.display().to_string(), "exchange": { "formats": ["dxf", "dxf"] } });
    let r = call(&mut s, "file.package", "plan", input.clone());
    assert_eq!(r["status"], "completed", "{r}");
    assert!(!out.exists(), "a plan writes no package");
    let rows = r["output"]["exchange"]["rows"].as_array().expect("rows");
    assert!(rows.iter().all(|row| row["cells"].as_array().map(Vec::len) == Some(1)), "DXF once");
    let paths: Vec<&str> = r["output"]["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["kind"] == "exchange")
        .filter_map(|f| f["path"].as_str())
        .collect();
    assert_eq!(paths, ["devir/Aktarım örneği.dxf", "devir/devir-raporu.csv"], "the drawing's own name");
    // Without `exchange` the package has no open file and its manifest no matrix.
    let plain = call(&mut s, "file.package", "plan", json!({ "path": out.display().to_string() }));
    assert!(plain["output"].get("exchange").is_none(), "{plain}");
    assert!(plain["output"]["files"].as_array().into_iter().flatten().all(|f| f["kind"] != "exchange"));
}

#[test]
fn the_open_files_question_is_refused_under_exchange() {
    let work = scratch("paket-devir-ret");
    let mut s = opened(&work);
    let out = work.join("teslim").display().to_string();
    let r = call(&mut s, "file.package", "validate", json!({ "path": out, "exchange": { "tolerance": 0.0 } }));
    assert_eq!((r["error"]["code"].as_str(), r["error"]["path"].as_str()), (Some("invalid_tolerance"), Some("exchange.tolerance")), "{r}");
    let r = call(&mut s, "file.package", "execute", json!({ "path": out, "exchange": { "layers": ["yok"] } }));
    assert_eq!((r["error"]["code"].as_str(), r["error"]["path"].as_str()), (Some("layer_not_found"), Some("exchange.layers[0]")), "{r}");
    assert!(!work.join("teslim").exists(), "nothing written");
}
