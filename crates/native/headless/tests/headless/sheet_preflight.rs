//! Independent preflight matrix through the actual native command, without output directories.
use std::{fs, path::Path};
use kentos_commands::{CallEnvelope, CallOp, CommandResult, SheetsPreflighted};
use kentos_domain::Document;
use serde_json::{json, Value};
const FX: &str = include_str!("../../../../../fixtures/commands/v1/file.preflight.sheets.json");
fn replace(v: &Value, work: &Path) -> Value {
    match v {
        Value::String(s) => json!(s.replace("$work", &work.display().to_string())),
        Value::Array(a) => Value::Array(a.iter().map(|v| replace(v, work)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), replace(v, work)))
                .collect(),
        ),
        v => v.clone(),
    }
}
fn source(fx: &Value, c: &Value, root: &Path) -> Document {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    fs::create_dir_all(root).unwrap();
    if let Some(files) = c["files"].as_array() {
        for f in files {
            let to = root.join(f["to"].as_str().unwrap());
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            if let Some(file)=f["fixture"].as_str(){fs::copy(fixture.join(file),to).unwrap();}else{fs::write(to,f["text"].as_str().unwrap().as_bytes()).unwrap();}
        }
    }
    let mut model = fx["model"].clone();
    if let Some(e) = c.get("entity") {
        model["entities"]
            .as_array_mut()
            .unwrap()
            .push(replace(e, root));
    }
    if let Some(unit) = c.get("unit") {
        model["settings"]["drawingUnit"] = unit.clone();
    }
    if c["resourceLayerVisible"] == true {
        model["layers"][1]["visible"] = json!(true);
    }
    if c["service"] == true {
        model["layers"][1]["service"] =
            json!({"kind":"xyz","url":"https://invalid.example/{z}/{x}/{y}.png"});
    }
    if c["fontMissing"]==true {model["settings"]["textStyles"]=json!([{"id":"font-style","name":"Özgün","font":"barlow","fontFile":"romans.shx"}]);}
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    if c["noBook"] == true {
        return doc;
    }
    let mut book = fx["book"].clone();
    if let Some(patch) = c["patch"].as_object() {
        book["sheets"][0]["items"][0]["kind"]
            .as_object_mut()
            .unwrap()
            .extend(patch.clone());
    }
    if let Some(patch) = c["item"].as_object() {
        book["sheets"][0]["items"][0]
            .as_object_mut()
            .unwrap()
            .extend(patch.clone());
    }
    if let Some(center)=c.get("center"){book["sheets"][0]["items"][0]["kind"]["view"]["center"]=center.clone();}
    if let Some(scale) = c.get("scale") {
        book["sheets"][0]["items"][0]["kind"]["view"]["scale"] = scale.clone();
    }
    if let Some(factor) = c.get("factor") {
        book["sheets"][0]["plot"] = json!({"settings":{"area":{"type":"window","rect":{"left":0,"top":0,"width":420000,"height":297000}},"scale":{"type":"ratio","ratio":{"paper":factor,"model":1}}}});
    }
    if let Some(item) = c.get("extraItem") {
        book["sheets"][0]["items"]
            .as_array_mut()
            .unwrap()
            .push(item.clone());
    }
    if c["png"]==true{book["assets"]=json!([c["assetMeta"]]);book["sheets"][0]["items"].as_array_mut().unwrap().push(json!({"id":"png","name":"PNG","frame":c["pictureFrame"],"kind":{"type":"picture","asset":c["assetMeta"]["sha256"],"fit":c["fit"]}}));}
    if c.get("svg").is_some() {
        book["assets"] = json!([c["svgMeta"]]);
        book["sheets"][0]["items"].as_array_mut().unwrap().push(json!({"id":"svg","name":"SVG","frame":{"left":10000,"top":10000,"width":10000,"height":10000},"kind":{"type":"picture","asset":c["svgMeta"]["sha256"],"fit":"contain"}}));
    }
    let book: kentos_sheet::SheetBook = serde_json::from_value(book).unwrap();
    doc.set_sheets(
        kentos_sheet::project::project_sheets(&book, |sha| {
            if c["assetMeta"]["sha256"].as_str()==Some(sha){Some(fs::read(fixture.join("interaction/v1/logo.png")).unwrap())}else if c["svgMeta"]["sha256"].as_str() == Some(sha) {
                Some(c["svg"].as_str().unwrap().as_bytes().to_vec())
            } else {
                None
            }
        })
        .unwrap(),
    )
    .unwrap();
    doc
}
#[test]
fn sheet_preflight_independent_physics_visibility_resources_and_output_free_protocol() {
    let fx: Value = serde_json::from_str(FX).unwrap();
    assert!(fx["cases"].as_array().unwrap().len() >= 20);
    for (n, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let root =
            std::env::temp_dir().join(format!("kentos-preflight-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut doc = source(&fx, c, &root);
        let before = doc.to_snapshot_v2();
        let file_count = fs::read_dir(&root).unwrap().count();
        let mut input = json!({"drawing":root.join("source.kcad"),"sheets":["ground"]});
        if let Some(p) = c["input"].as_object() {
            input.as_object_mut().unwrap().extend(p.clone());
        }
        let mut first = None;
        for op in [CallOp::Plan, CallOp::Execute] {
            let answer = kentos_headless::file_commands::call(
                &mut doc,
                CallEnvelope {
                    command: kentos_commands::FILE_PREFLIGHT_SHEETS.into(),
                    version: Some(1),
                    op,
                    request_id: None,
                    expected: None,
                    relax_constraints: false,
                    input: input.clone(),
                },
            );
            if let Some(code) = c["code"].as_str() {
                assert!(
                    matches!(&answer.result,CommandResult::Failed{error} if error.code==code),
                    "{} {answer:?}",
                    c["name"]
                );
                continue;
            }
            let CommandResult::Completed { output, .. } = answer.result else {
                panic!("{} {answer:?}", c["name"])
            };
            let report: SheetsPreflighted = serde_json::from_value(output).unwrap();
            for code in c["expect"].as_array().unwrap() {
                assert!(
                    report
                        .issues
                        .iter()
                        .any(|i| i.code == code.as_str().unwrap()),
                    "{} missing {code}: {:?}",
                    c["name"],
                    report.issues
                );
            }
            if let Some(absent) = c["absent"].as_array() {
                for code in absent {
                    assert!(
                        !report
                            .issues
                            .iter()
                            .any(|i| i.code == code.as_str().unwrap()),
                        "{} unexpected {code}: {:?}",
                        c["name"],
                        report.issues
                    );
                }
            }
            assert_eq!(report.ready, !report.issues.iter().any(|i| i.error));
            if let Some(first) = &first {
                assert_eq!(first, &report);
            } else {
                first = Some(report);
            }
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), file_count);
        let after = doc.to_snapshot_v2();
        assert_eq!(
            serde_json::to_value(after).unwrap(),
            serde_json::to_value(before).unwrap(),
            "query never edits the source"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
