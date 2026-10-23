//! Independent complete-delivery reference: actual file bytes, relocation, strict omissions and replay tampering.
use std::{
    fs,
    path::{Path, PathBuf},
};
use kentos_commands::{CallEnvelope, CallOp, CommandResult, SheetsPublished, encoding::sha256_hex};
use kentos_domain::Document;
use serde_json::{Value, json};
const FX: &str = include_str!("../../../../../fixtures/sheet/v1/delivery.json");
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn replace(value: &Value, work: &Path) -> Value {
    match value {
        Value::String(s) => json!(s.replace("$work", &work.display().to_string())),
        Value::Array(a) => Value::Array(a.iter().map(|v| replace(v, work)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), replace(v, work)))
                .collect(),
        ),
        other => other.clone(),
    }
}
fn source(fx: &Value, c: &Value, work: &Path) -> Document {
    fs::create_dir_all(work).unwrap();
    for f in c["files"].as_array().unwrap() {
        let path = work.join(f["to"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = if let Some(p) = f["fixture"].as_str() {
            fs::read(root().join("fixtures").join(p)).unwrap()
        } else if f["sourceModel"] == true {
            serde_json::to_vec(&fx["model"]).unwrap()
        } else {
            f["text"].as_str().unwrap().as_bytes().to_vec()
        };
        fs::write(path, bytes).unwrap();
    }
    let mut model = fx["model"].clone();
    model["entities"]
        .as_array_mut()
        .unwrap()
        .extend(replace(&c["entities"], work).as_array().unwrap().clone());
    if c["service"] == true {
        model["layers"][1]["service"] =
            json!({"kind":"xyz","url":"https://invalid.example/{z}/{x}/{y}.png"});
    }
    if c["fontMissing"] == true {
        model["settings"]["textStyles"] =
            json!([{"id":"font-style","name":"Özgün","font":"barlow","fontFile":"romans.shx"}]);
    }
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    let mut book = fx["book"].clone();
    if let Some(kind) = c["profile"].as_str() {
        let p = fx["profile"].clone();
        let mut captured = p.clone();
        captured["name"] = json!("Önceki baskı");
        captured["entries"][0]["style"]["lineWeight"] = json!(350);
        book["plotProfiles"] = json!([p]);
        if kind == "snapshot" {
            book["sheets"][0]["plot"] =
                json!({"settings":{"profile":p["id"],"profileSnapshot":captured}});
        }
        if kind == "setup" {
            book["pageSetups"] = json!([{"id":"office","name":"Ofis","page":book["sheets"][0]["page"],"settings":{"profile":p["id"],"profileSnapshot":captured}}]);
        }
    }
    let book: kentos_sheet::SheetBook = serde_json::from_value(book).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    for command in c["commands"].as_array().into_iter().flatten() {
        let answer = kentos_native_application::call::call(
            &mut doc,
            None,
            envelope(
                command["command"].as_str().unwrap(),
                replace(&command["input"], work),
                CallOp::Execute,
                None,
            ),
        );
        assert!(
            matches!(answer.result, CommandResult::Completed { .. }),
            "{}: {answer:?}",
            c["name"]
        );
    }
    doc
}
fn envelope(command: &str, input: Value, op: CallOp, id: Option<String>) -> CallEnvelope {
    CallEnvelope {
        command: command.into(),
        version: None,
        op,
        request_id: id,
        expected: None,
        relax_constraints: false,
        input,
    }
}
fn publish(
    doc: &mut Document,
    input: &Value,
    op: CallOp,
    id: Option<String>,
) -> kentos_commands::CallAnswer {
    kentos_headless::file_commands::call(
        doc,
        envelope("file.publish.sheets", input.clone(), op, id),
    )
}
fn report(answer: kentos_commands::CallAnswer) -> SheetsPublished {
    let CommandResult::Completed { output, .. } = answer.result else {
        panic!("{answer:?}")
    };
    serde_json::from_value(output).unwrap()
}
fn inventory(fx: &Value, c: &Value, target: &Path, r: &SheetsPublished) {
    for f in &r.files {
        let bytes = fs::read(target.join(&f.name)).unwrap();
        assert_eq!(bytes.len() as u64, f.bytes);
        assert_eq!(sha256_hex(&bytes), f.sha256);
    }
    if c["resources"] == false {
        assert!(r.resources.is_none());
        return;
    }
    let assets = r.resources.as_ref().unwrap();
    let mut fonts = Vec::new();
    let mut licenses = Vec::new();
    let mut deps = Vec::new();
    let mut plots = 0;
    let mut total = 0;
    for f in &assets.files {
        let bytes = fs::read(target.join(&f.path)).unwrap();
        total += bytes.len() as u64;
        assert_eq!(bytes.len() as u64, f.bytes, "{}", f.path);
        assert_eq!(sha256_hex(&bytes), f.sha256, "{}", f.path);
        use kentos_commands::SheetResourceKind as K;
        match f.kind {
            K::Font => fonts.push(f.sha256.clone()),
            K::License => licenses.push(f.sha256.clone()),
            K::Dependency => deps.push(f.path.strip_prefix("kaynaklar/").unwrap().to_string()),
            K::PlotProfile => {
                plots += 1;
                let v: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(v["schema"], "kentos.plot/1");
            }
            _ => {}
        }
    }
    assert_eq!(total, assets.bytes);
    fonts.sort();
    licenses.sort();
    deps.sort();
    assert_eq!(serde_json::to_value(fonts).unwrap(), fx["fontSha256"]);
    assert_eq!(serde_json::to_value(licenses).unwrap(), fx["licenseSha256"]);
    assert_eq!(
        serde_json::to_value(deps).unwrap(),
        c["deps"],
        "{}",
        c["name"]
    );
    assert_eq!(plots, c["plotFiles"].as_u64().unwrap_or(0));
    let manifest: Value =
        serde_json::from_slice(&fs::read(target.join("yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["resources"], serde_json::to_value(assets).unwrap());
    assert_eq!(manifest["source"], serde_json::to_value(&r.source).unwrap());
    let bytes = fs::read(target.join(&assets.project)).unwrap();
    let doc = kentos_kcad::decode(&bytes).unwrap();
    assert!(doc.sheets.is_some());
}
#[test]
fn sheet_package_independent_complete_deliveries_keep_resources_hashes_and_replay_integrity() {
    let fx: Value = serde_json::from_str(FX).unwrap();
    assert!(fx["cases"].as_array().unwrap().len() >= 20);
    for (n, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let dir =
            std::env::temp_dir().join(format!("kentos-sheet-package-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let work = dir.join("work");
        let target = dir.join("delivery");
        let mut doc = source(&fx, c, &work);
        let before = doc.to_snapshot_v2();
        if matches!(c["target"].as_str(), Some("empty" | "nonempty")) {
            fs::create_dir(&target).unwrap();
            if c["target"] == "nonempty" {
                fs::write(target.join("keep.txt"), b"keep").unwrap();
            }
        }
        let input = json!({"path":target,"drawing":work.join("source.kcad"),"resources":c["resources"]!=false});
        let plan = publish(&mut doc, &input, CallOp::Plan, None);
        if let Some(code) = c["code"].as_str() {
            let CommandResult::Failed { error } = plan.result else {
                panic!("{plan:?}")
            };
            assert_eq!(error.code, code);
            assert_eq!(fs::read(target.join("keep.txt")).unwrap(), b"keep");
            continue;
        }
        let planned = report(plan);
        assert_eq!(
            planned.ready,
            c["ready"] == true,
            "{} {:?}",
            c["name"],
            planned.issues
        );
        assert!(!target.join("yayin.json").exists());
        if !planned.ready {
            assert!(
                planned
                    .issues
                    .iter()
                    .any(|i| i.code == c["issue"].as_str().unwrap() && i.error),
                "{planned:?}"
            );
            let failed = publish(&mut doc, &input, CallOp::Execute, None);
            assert!(matches!(failed.result, CommandResult::Failed { .. }));
            assert!(!target.exists());
        } else {
            let request = Some(format!("sheet-package-{n}"));
            let answer = publish(&mut doc, &input, CallOp::Execute, request.clone());
            let call = answer.call;
            let issued = report(answer);
            inventory(&fx, c, &target, &issued);
            let repeated=publish(&mut doc, &input, CallOp::Execute, request.clone());
            assert!(repeated.replayed);
            assert!(matches!(repeated.result, CommandResult::Completed {..}), "{repeated:?}");
            let row=doc.journal_row(call.unwrap()).unwrap().clone();
            let replay=kentos_headless::file_commands::replay_row(&mut doc,&row).unwrap();
            assert!(replay.error.is_none(),"{replay:?}");
            assert!(matches!(replay.outcome,kentos_commands::ReplayOutcome::Skipped));
            if let Some(kind) = c["tamper"].as_str() {
                use kentos_commands::SheetResourceKind as K;
                let resources = issued.resources.as_ref().unwrap();
                let rel = match kind {
                    "pdf" => issued.files[0].name.as_str(),
                    "manifest" => "yayin.json",
                    "missing" | "symlink" => resources
                        .files
                        .iter()
                        .find(|f| f.kind == K::Font)
                        .unwrap()
                        .path
                        .as_str(),
                    _ => resources
                        .files
                        .iter()
                        .find(|f| match kind {
                            "font" => f.kind == K::Font,
                            "license" => f.kind == K::License,
                            "drawing" => f.kind == K::Drawing,
                            "profile" => f.kind == K::PlotProfile,
                            "pageSetups" => f.kind == K::PageSetups,
                            "dependency" => f.kind == K::Dependency,
                            _ => false,
                        })
                        .unwrap()
                        .path
                        .as_str(),
                };
                let path = target.join(rel);
                if kind == "missing" {
                    fs::remove_file(&path).unwrap();
                } else if kind == "symlink" {
                    #[cfg(unix)]
                    {
                        fs::remove_file(&path).unwrap();
                        fs::write(dir.join("outside.ttf"), b"outside").unwrap();
                        std::os::unix::fs::symlink(dir.join("outside.ttf"), &path).unwrap();
                    }
                } else {
                    fs::write(&path, b"changed").unwrap();
                }
                let replay = publish(&mut doc, &input, CallOp::Execute, request);
                assert!(replay.replayed);
                assert!(
                    matches!(replay.result,CommandResult::Failed{ref error} if error.code=="integrity_failed"),
                    "{} {replay:?}",
                    c["name"]
                );
                let row = doc.journal_row(call.unwrap()).unwrap().clone();
                let recipe = kentos_headless::file_commands::replay_row(&mut doc, &row).unwrap();
                assert_eq!(recipe.error.unwrap().code, "integrity_failed");
            } else if let Some(assets) = &issued.resources {
                let moved = dir.join("moved");
                fs::rename(&target, &moved).unwrap();
                fs::remove_dir_all(&work).unwrap();
                let opened = kentos_headless::Session::open(&moved.join(&assets.project)).unwrap();
                assert_eq!(opened.document().len(), doc.len());
                // Source paths are portable even after both the output folder and original data have moved away.
                let snapshot = serde_json::to_value(opened.document().to_snapshot_v2()).unwrap();
                for e in snapshot["entities"].as_array().unwrap() {
                    if let Some(p) = e["file"].as_str() {
                        assert!(moved.join("kaynaklar").join(p).is_file(), "{p}");
                    }
                }
            }
        }
        let after = doc.to_snapshot_v2();
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.layers, before.layers);
        assert_eq!(after.settings, before.settings);
        assert_eq!(after.sheets, before.sheets);
        assert!(!fs::read_dir(&dir).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".kentos-yayin-")
        }));
        fs::remove_dir_all(dir).unwrap();
    }
}
