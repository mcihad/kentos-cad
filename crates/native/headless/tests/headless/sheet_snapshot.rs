//! Protocol pins ignore replay bookkeeping, bind file/context versions, and reject stale source deliveries.
use std::{fs, path::Path};
use serde_json::{Value, json};
use kentos_commands::{CallEnvelope, CallOp, CommandResult};
use kentos_domain::Document;
fn call(
    doc: &mut Document,
    input: Value,
    op: CallOp,
    request: Option<&str>,
) -> kentos_commands::CallAnswer {
    kentos_headless::file_commands::call(
        doc,
        CallEnvelope {
            command: kentos_commands::FILE_PUBLISH_SHEETS.into(),
            version: Some(1),
            op,
            request_id: request.map(str::to_owned),
            expected: None,
            relax_constraints: false,
            input,
        },
    )
}
fn done(answer: kentos_commands::CallAnswer) -> Value {
    let CommandResult::Completed { output, .. } = answer.result else {
        panic!("{answer:?}")
    };
    output
}
#[test]
fn sheet_snapshot_request_bookkeeping_does_not_invalidate_a_full_plan_but_a_changed_file_does() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/source-snapshot.json"
    ))
    .unwrap();
    let root = std::env::temp_dir().join(format!("kentos-publish-snapshot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    fs::copy(
        fixture.join(fx["imageA"].as_str().unwrap()),
        root.join("logo.png"),
    )
    .unwrap();
    let mut model = fx["model"].clone();
    model["entities"].as_array_mut().unwrap().push(json!({"kind":"image","id":20,"layerId":"cizim","attrs":{},"file":root.join("logo.png"),"p":{"x":2.4,"y":-0.1},"width":0.2,"height":0.2,"rotation":0}));
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let a = json!({"path":root.join("a"),"drawing":root.join("source.kcad")});
    let planned = done(call(&mut doc, a.clone(), CallOp::Plan, None));
    assert!(planned["source"]["resourcesSha256"].is_string());
    assert!(planned["source"]["contextSha256"].is_string());
    let issued = done(call(&mut doc, a, CallOp::Execute, Some("first")));
    assert_eq!(issued["source"], planned["source"]);
    assert!(!doc.to_snapshot_v2().requests.is_empty());
    let b =
        json!({"path":root.join("b"),"drawing":root.join("source.kcad"),"pin":planned["source"]});
    let repeated = done(call(&mut doc, b.clone(), CallOp::Plan, None));
    assert_eq!(repeated["source"], planned["source"]);
    fs::copy(
        fixture.join(fx["imageB"].as_str().unwrap()),
        root.join("logo.png"),
    )
    .unwrap();
    let refused = call(&mut doc, b, CallOp::Execute, None);
    assert!(matches!(refused.result,CommandResult::Conflict{error} if error.code=="pin_mismatch"));
    assert!(!root.join("b").exists());
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("a/yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["snapshot"]["source"], planned["source"]);
    assert_eq!(manifest["snapshot"]["laterEditsExcluded"], true);
    fs::remove_dir_all(root).unwrap();
}
