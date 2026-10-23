//! Paketle's ways for bound sources and what is left out (docs/adr/0596): fixtures/package/v1/cases.json, written by
//! scripts/fixtures/package_options_cases.py without KentOS code, played here. Each case's drawing is made (tables
//! bound from a copy of qgis.gpkg, a link made a database's, linked pictures), saved in its work folder, opened and
//! packed; the answer is compared field by field, and a package made is opened to count its links and layers.

use std::path::{Path, PathBuf};

use kentos_headless::Session;
use serde_json::{Value, json};

use super::{new_drawing, scratch};

const ADDRESS: &str = "postgresql://cbs@127.0.0.1:1/kent";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

/// `$work:` paths made the work folder's, anywhere in a value.
fn placed(v: &Value, work: &Path) -> Value {
    match v {
        Value::String(s) => match s.strip_prefix("$work:") {
            Some(rest) => Value::String(work.join(rest).display().to_string()),
            None => v.clone(),
        },
        Value::Array(list) => Value::Array(list.iter().map(|x| placed(x, work)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, x)| (k.clone(), placed(x, work)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The case's drawing saved in `work` and opened.
fn drawing(setup: &Value, work: &Path) -> Session {
    std::fs::create_dir_all(work.join("foto")).expect("folders");
    std::fs::copy(
        fixtures().join("interaction/v1/logo.png"),
        work.join("foto/logo.png"),
    )
    .expect("copied");
    let mut s = new_drawing();
    let links = setup["links"].as_array().cloned().unwrap_or_default();
    for l in &links {
        let file = work.join(l["file"].as_str().expect("a file"));
        if !file.exists() {
            std::fs::copy(fixtures().join("geopackage/v1/qgis.gpkg"), &file).expect("copied");
        }
        let linked = s
            .call(json!({ "command": "cad.sources.edit", "input": { "edit": {
                "kind": "link", "path": file.display().to_string(), "table": l["table"] } } }))
            .expect("answers");
        assert_eq!(linked["status"], "completed", "{linked}");
    }
    let mut snapshot = serde_json::to_value(s.document().to_snapshot_v2()).expect("json");
    let layer = snapshot["activeLayer"].clone();
    let next = snapshot["entities"].as_array().map_or(0, Vec::len) as u64 + 100;
    for (k, p) in setup["pictures"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        snapshot["entities"]
            .as_array_mut()
            .expect("entities")
            .push(json!({ "kind": "image", "id": next + k as u64, "layerId": layer, "attrs": {},
                "p": { "x": 0.0, "y": 0.0 }, "width": 10.0, "height": 10.0, "rotation": 0.0, "file": p["file"] }));
        snapshot["uids"]
            .as_array_mut()
            .expect("uids")
            .push(p["uid"].clone());
    }
    for i in setup["databases"].as_array().into_iter().flatten() {
        let i = i.as_u64().expect("an index") as usize;
        let link = &mut snapshot["settings"]["sources"][i];
        let table = format!("public.{}", link["table"].as_str().unwrap_or_default());
        link["kind"] = json!("postgis");
        link["path"] = json!(ADDRESS);
        link["table"] = json!(table);
        if let Some(map) = link.as_object_mut() {
            map.remove("relative");
        }
    }
    let typed: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(snapshot).expect("a snapshot");
    let project = work.join("proje.kcad");
    std::fs::write(
        &project,
        kentos_kcad::encode_verified(&typed).expect("encoded"),
    )
    .expect("written");
    // Opened and saved again: the save writes each GeoPackage link's relative path (docs/adr/0506 §1).
    let mut s = Session::open(&project).expect("opens");
    s.save(None).expect("saved");
    for l in &links {
        if l["gone"].as_bool() == Some(true) {
            let _ = std::fs::remove_file(work.join(l["file"].as_str().unwrap_or_default()));
        }
    }
    Session::open(&project).expect("opens")
}

/// What the answer says, in the case's terms.
fn told(answer: &Value) -> Value {
    if answer["status"] != "completed" {
        return json!({ "status": answer["status"], "error": {
            "code": answer["error"]["code"], "path": answer["error"]["path"] } });
    }
    let out = &answer["output"];
    if out.is_null() {
        return json!({ "status": "completed" });
    }
    let files: Vec<Value> = out["files"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            json!({ "path": f["path"], "kind": f["kind"], "links": f["links"],
            "bytes": f["bytes"], "sha256": f["sha256"] })
        })
        .collect();
    let left: Vec<Value> = out["left"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|l| {
            let mut v =
                json!({ "kind": l["kind"], "path": l["path"], "why": l["why"], "at": l["at"] });
            if !l["objects"].is_null() {
                v["objects"] = l["objects"].clone();
            }
            v
        })
        .collect();
    let embedded: Vec<Value> = out["embedded"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| {
            json!({ "at": e["at"], "layer": e["layer"], "path": e["path"], "table": e["table"],
            "objects": e["objects"] })
        })
        .collect();
    let warnings: Vec<Value> = answer["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|w| w["code"].clone())
        .collect();
    json!({ "status": "completed", "files": files, "left": left, "embedded": embedded, "warnings": warnings })
}

/// The package's project: its links and the bound layers' object counts.
fn packed(project: &Path, layers: &[String]) -> Value {
    let s = Session::open(project).expect("the package opens");
    let doc = s.document();
    let sources: Vec<Value> = doc
        .settings()
        .sources
        .iter()
        .map(|l| json!([l.id, l.kind, l.relative, l.table]))
        .collect();
    let counts: serde_json::Map<String, Value> = layers
        .iter()
        .map(|l| (l.clone(), json!(doc.by_layer_with_uids(l).count())))
        .collect();
    json!({ "sources": sources, "layers": counts })
}

#[test]
fn every_package_case_is_answered_as_the_reference_says() {
    let cases: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("package/v1/cases.json")).expect("the cases"),
    )
    .expect("JSON");
    assert_eq!(cases["format"], "kentos.package-cases");
    let list = cases["cases"].as_array().expect("cases");
    assert!(list.len() >= 20, "{} cases", list.len());
    let dir = scratch("package-cases");
    let mut problems = Vec::new();
    for (k, case) in list.iter().enumerate() {
        let name = case["name"].as_str().unwrap_or("?");
        let work = dir.join(format!("durum-{k}"));
        let mut s = drawing(&case["setup"], &work);
        let mut input = placed(&case["input"], &work);
        input["path"] = json!(work.join("paket").display().to_string());
        input["from"] = json!(work.display().to_string());
        let op = case["op"].as_str().unwrap_or("execute");
        let answer = s
            .call(json!({ "command": "file.package", "op": op, "input": input }))
            .expect("answers");
        let mut got = told(&answer);
        let want = placed(&case["expect"], &work);
        if op == "execute" && answer["status"] == "completed" {
            let layers: Vec<String> = want["project"]["layers"]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            let project = answer["output"]["project"].as_str().expect("a project");
            got["project"] = packed(Path::new(project), &layers);
        }
        if op == "plan" && work.join("paket").exists() {
            problems.push(format!("{name}: the plan wrote the package"));
        }
        if got != want {
            problems.push(format!("{name}:\n  want {want}\n  got  {got}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
