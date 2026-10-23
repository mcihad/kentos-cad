//! The filesystem command has its own host runner; document-only fixtures never pretend to perform file I/O.
use kentos_commands::{
    CallEnvelope, CallOp, CommandResult, PublishSheets, FILE_PUBLISH_SHEETS, encoding::sha256_hex,
};
use kentos_domain::Document;
use kentos_sheet::{SheetBook, project::project_sheets};
use serde_json::{Value, json};
#[path = "../../../plot/tests/support/files.rs"]
mod files;
const CASES: &str = include_str!("../../../../../fixtures/commands/v1/file.publish.sheets.json");
fn fixture() -> Value {
    serde_json::from_str(CASES).unwrap()
}
fn source(fx: &Value, case: &Value, root: &std::path::Path) -> Document {
    let mut model = fx["setup"].clone();
    if case["prepare"]["imageMissing"] == true {
        model["entities"].as_array_mut().unwrap().push(json!({"kind":"image","id":90000,"layerId":"cizim","attrs":{},"file":root.join("missing.png"),"p":{"x":0.,"y":1.},"width":1.,"height":1.,"rotation":0}));
    }
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    if case["prepare"]["noBook"] == true {
        return doc;
    }
    let mut book = fx["book"].clone();
    for sheet in book["sheets"].as_array_mut().unwrap() {
        let id = sheet["id"].as_str().unwrap().to_owned();
        if let Some(name) = case["prepare"]["names"][&id].as_str() {
            sheet["name"] = json!(name);
        }
        if case["prepare"]["missing"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == &id))
        {
            sheet["items"][0]["kind"]["source"] = json!({"type":"viewLink","view":{"id":"gone","name":"Silinmiş","center":{"x":2.5,"y":0.},"width":5.,"height":3.,"twist":0.}});
        }
        if case["prepare"]["unplaced"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == &id))
        {
            sheet["items"][0]["kind"]["view"]["center"] = Value::Null;
        }
    }
    let book: SheetBook = serde_json::from_value(book).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    doc
}
fn subset(got: &Value, want: &Value) {
    match want {
        Value::Object(w) => {
            for (k, v) in w {
                subset(&got[k], v)
            }
        }
        Value::Array(w) => {
            let g = got.as_array().unwrap();
            assert_eq!(g.len(), w.len());
            for (g, w) in g.iter().zip(w) {
                subset(g, w);
            }
        }
        _ => assert_eq!(got, want),
    }
}
#[test]
fn independent_publication_cases_keep_order_names_physics_and_atomic_delivery() {
    let fx = fixture();
    assert!(fx["cases"].as_array().unwrap().len() >= 20);
    for (n, case) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let dir =
            std::env::temp_dir().join(format!("kentos-publish-cases-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let target = dir.join("delivery");
        match case["prepare"]["target"].as_str() {
            Some("empty") => std::fs::create_dir(&target).unwrap(),
            Some("nonempty") => {
                std::fs::create_dir(&target).unwrap();
                std::fs::write(target.join("keep.txt"), b"keep").unwrap();
            }
            _ => {}
        }
        let mut doc = source(&fx, case, &dir);
        for step in case["steps"].as_array().unwrap() {
            let before = doc.to_snapshot_v2();
            let can_undo = doc.can_undo();
            let mut input = step["input"].clone();
            if input["path"] == "$out" {
                input["path"] = json!(target);
            }
            let op: CallOp = serde_json::from_value(step["op"].clone()).unwrap();
            let answer = kentos_headless::file_commands::call(
                &mut doc,
                CallEnvelope {
                    command: FILE_PUBLISH_SHEETS.into(),
                    version: Some(1),
                    op,
                    request_id: None,
                    expected: None,
                    relax_constraints: false,
                    input,
                },
            );
            let got = serde_json::to_value(&answer.result).unwrap();
            eprintln!("{} {}: {}", case["name"], step["op"], got["status"]);
            subset(&got, &step["result"]);
            if let Some(code) = step["issue"].as_str() {
                assert!(
                    got["output"]["issues"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|i| i["code"] == code && i["error"] == true),
                    "{got}"
                );
            }
            assert_eq!(
                target.join("yayin.json").exists(),
                step["published"] == true,
                "{}",
                case["name"]
            );
            if step["published"] == true {
                let manifest: Value =
                    serde_json::from_slice(&std::fs::read(target.join("yayin.json")).unwrap())
                        .unwrap();
                assert_eq!(manifest["format"], "kentos.publish/1");
                assert_eq!(manifest["sheets"], got["output"]["sheets"]);
                for file in got["output"]["files"].as_array().unwrap() {
                    let name = file["name"].as_str().unwrap();
                    let bytes = std::fs::read(target.join(name)).unwrap();
                    assert_eq!(json!(sha256_hex(&bytes)), file["sha256"]);
                    assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
                    let pages = files::pdf_pages(&bytes);
                    assert_eq!(pages.len(), file["sheets"].as_array().unwrap().len());
                    for (page, id) in pages.iter().zip(file["sheets"].as_array().unwrap()) {
                        assert!(
                            (page.width - 420.).abs() < 0.002 && (page.height - 297.).abs() < 0.002
                        );
                        for length in [100., fx["markers"][id.as_str().unwrap()].as_f64().unwrap()]
                        {
                            assert!(
                                page.segments
                                    .iter()
                                    .any(|p| ((p[2] - p[0]).hypot(p[3] - p[1]) - length).abs()
                                        < 0.002)
                            );
                        }
                    }
                }
            }
            if case["prepare"]["target"] == "nonempty" {
                assert_eq!(std::fs::read(target.join("keep.txt")).unwrap(), b"keep");
            }
            assert!(!std::fs::read_dir(&dir).unwrap().any(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".kentos-yayin-")
            }));
            let after = doc.to_snapshot_v2();
            assert_eq!(after.entities, before.entities);
            assert_eq!(after.layers, before.layers);
            assert_eq!(after.settings, before.settings);
            assert_eq!(after.sheets, before.sheets);
            assert_eq!(doc.can_undo(), can_undo);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
#[test]
fn a_file_plan_job_never_executes_publication() {
    let fx = fixture();
    let mut doc = source(&fx, &fx["cases"][0], &std::env::temp_dir());
    let dir = std::env::temp_dir().join(format!("kentos-publish-planjob-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let input: PublishSheets =
        serde_json::from_value(json!({"path":dir,"mode":"separate"})).unwrap();
    let env = CallEnvelope {
        command: FILE_PUBLISH_SHEETS.into(),
        version: Some(1),
        op: CallOp::Plan,
        request_id: None,
        expected: None,
        relax_constraints: false,
        input: serde_json::to_value(input).unwrap(),
    };
    let mut job = kentos_native_application::jobs::Job::start_with(
        "publication-plan".into(),
        FILE_PUBLISH_SHEETS.into(),
        None,
        &doc,
        kentos_headless::file_commands::job_work(env),
    );
    let status = job.wait(&mut doc);
    assert!(matches!(
        status.answer.unwrap().result,
        CommandResult::Completed { .. }
    ));
    assert_eq!(status.progress, Some(1.));
    assert!(!dir.exists());
}

#[test]
fn drawing_set_publication_names_and_manifest_use_literal_captured_metadata() {
    let fx=fixture();let root=std::env::temp_dir().join(format!("kentos-publish-drawing-set-{}",std::process::id()));
    let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();
    let mut doc=source(&fx,&json!({}),&root);
    let (mut book,_)=kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    book.sheets[0].drawing_info=Some(kentos_sheet::drawing_set::DrawingInfo{number:"A-101".into(),title:"Başlık {id}".into(),revision:"R02".into(),status:"Yayın".into(),subset:vec!["Mimari".into(),"Katlar".into()]});
    doc.set_sheets(project_sheets(&book, |_|None).unwrap()).unwrap();
    let answer=kentos_headless::file_commands::call(&mut doc,CallEnvelope{command:FILE_PUBLISH_SHEETS.into(),version:None,op:CallOp::Execute,request_id:None,expected:None,relax_constraints:false,input:json!({"path":root.join("delivery"),"sheets":["ground"],"mode":"separate","naming":"{number}-{title}-{revision}-{status}-{subset}"})});
    let CommandResult::Completed{output,..}=answer.result else{panic!("{answer:?}")};
    assert_eq!(output["files"][0]["name"],"A-101-Başlık {id}-R02-Yayın-Mimari-Katlar.pdf");
    let manifest:Value=serde_json::from_slice(&std::fs::read(root.join("delivery/yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["drawingSet"][0]["info"]["number"],"A-101");
    assert_eq!(manifest["drawingSet"][0]["info"]["revision"],"R02");
    std::fs::remove_dir_all(root).unwrap();
}
