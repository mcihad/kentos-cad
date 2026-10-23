//! The metadata operation has the same validation, identity and exact inverse in the protocol host.
use kentos_commands::{CallEnvelope, CallOp, CommandResult};
use kentos_domain::Document;
use kentos_native_application::call::call;
use serde_json::{Value, json};
fn edit(doc: &mut Document, input: Value) -> kentos_commands::CallAnswer {
    call(
        doc,
        None,
        CallEnvelope {
            command: "cad.sheets.edit".into(),
            version: None,
            op: CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    )
}
#[test]
fn independent_drawing_set_cases_use_the_native_protocol_and_preserve_model() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/drawing-set.json"
    ))
    .unwrap();
    for c in fx["cases"].as_array().unwrap() {
        if c["ops"].as_array().unwrap().is_empty() {
            continue;
        }
        let mut doc =
            Document::from_snapshot(serde_json::from_value(fx["model"].clone()).unwrap()).unwrap();
        let book: kentos_sheet::SheetBook = serde_json::from_value(c["book"].clone()).unwrap();
        doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let result = edit(&mut doc, json!({"ops":c["ops"]}));
        if let Some(code) = c["code"].as_str() {
            let CommandResult::Failed { error } = result.result else {
                panic!("{} {result:?}", c["name"])
            };
            assert_eq!(error.code, code, "{}", c["name"]);
            assert_eq!(doc.to_snapshot_v2(), before);
            continue;
        }
        let CommandResult::Completed { output, .. } = result.result else {
            panic!("{} {result:?}", c["name"])
        };
        let kept = doc.sheets().unwrap();
        for (id, info) in c["metadata"].as_object().unwrap() {
            let s = kept.book["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == *id)
                .unwrap();
            assert_eq!(&s["drawingInfo"], info, "{}", c["name"]);
            let listed = output["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == *id)
                .unwrap();
            assert_eq!(&listed["drawingInfo"], info);
        }
        let inverse = edit(&mut doc, json!({"ops":output["inverse"]}));
        assert!(
            matches!(inverse.result, CommandResult::Completed { .. }),
            "{inverse:?}"
        );
        let after = doc.to_snapshot_v2();
        assert_eq!(before.sheets, after.sheets);
        assert_eq!(before.entities, after.entities);
        assert_eq!(before.layers, after.layers);
    }
}
