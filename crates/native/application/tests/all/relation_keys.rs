//! A relation by a parent key through the commands (docs/adr/9135): `cad.relations.edit` refuses a key the parent layer
//! lacks, saying where; `cad.records.page` gives a neighbourhood's works by its code (M1: two) and a work's
//! neighbourhood by the code it holds.

use kentos_commands::{CommandResult, DocumentSnapshotV1, Entity, RelationsEdit, RelationsOp};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, records_page, relations_edit};

fn drawing() -> Document {
    let text = |n: &str| format!(r#"{{"name":"{n}","kind":"text"}}"#);
    let layer = |id: &str, fields: String| {
        format!(
            r#"{{"id":"{id}","name":"{id}","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"ink","lineType":"continuous","lineWeight":0.25}},"children":[],"fields":[{fields}]}}"#
        )
    };
    let layers = [layer("mahalle", [text("kod"), text("ad")].join(",")), layer("bakim", [text("kod"), text("mahalle")].join(","))]
        .join(",");
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Mahalle","settings":{{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
        "origin":{{"x":0,"y":0}},"layers":[{layers}],"activeLayer":"bakim","entities":[],
        "styles":{{"items":[],"categories":[]}}}}"#
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("reads")).expect("opens")
}

fn record(id: u32, layer: &str, attrs: serde_json::Value) -> Entity {
    serde_json::from_value(serde_json::json!({
        "kind": "point", "id": id, "layerId": layer, "attrs": attrs, "p": {"x": f64::from(id), "y": 0}
    }))
    .expect("a point")
}

fn create(doc: &mut Document, parent_key: &str) -> CommandResult<kentos_commands::RelationsEdited> {
    let relation = serde_json::from_value(serde_json::json!({
        "id": "mahalle-bakim", "name": "Bakımlar", "parent": "mahalle", "child": "bakim", "field": "mahalle",
        "parentKey": parent_key
    }))
    .expect("a relation");
    relations_edit::execute(
        &mut ExecutionContext::new(doc),
        RelationsEdit { edit: RelationsOp::Create { relation }, expected_revision: None },
    )
}

#[test]
fn the_commands_tie_by_the_parent_key() {
    let mut doc = drawing();
    // A key the parent layer lacks is refused where it is written.
    match create(&mut doc, "yok") {
        CommandResult::Failed { error } => {
            assert_eq!(error.path.as_deref(), Some("edit.relation.parentKey"));
            assert!(error.message.contains("“yok” alanı yok"), "{}", error.message);
        }
        other => panic!("not refused: {other:?}"),
    }
    assert!(matches!(create(&mut doc, "kod"), CommandResult::Completed { .. }));
    let m1 = doc.add(record(1, "mahalle", serde_json::json!({"kod": "M1", "ad": "Çınar"}))).expect("a slot");
    let m2 = doc.add(record(2, "mahalle", serde_json::json!({"kod": "M2", "ad": "İstasyon"}))).expect("a slot");
    for (i, code) in ["M1", "M1", "M2", "M9"].iter().enumerate() {
        doc.add(record(10 + i as u32, "bakim", serde_json::json!({"kod": format!("B0{}", i + 1), "mahalle": code})))
            .expect("a slot");
    }
    let page = |doc: &mut Document, of: kentos_domain::Slot, side: &str| {
        let input = serde_json::from_value(serde_json::json!({
            "relation": "mahalle-bakim", "of": doc.uid(of).expect("a uid").to_string(), "side": side, "fields": ["kod"]
        }))
        .expect("an input");
        match records_page::execute(&mut ExecutionContext::new(doc), input) {
            CommandResult::Completed { output, .. } => {
                output.records.iter().map(|r| r.attrs.get("kod").cloned().unwrap_or_default()).collect::<Vec<_>>()
            }
            other => panic!("not read: {other:?}"),
        }
    };
    let mut works = page(&mut doc, m1, "children");
    works.sort();
    assert_eq!(works, ["B01", "B02"]);
    // A work's neighbourhood by the code it holds.
    let b03 = doc.by_layer_with_uids("bakim").nth(2).map(|(u, _)| u).expect("B03");
    let b03 = doc.slot_of(b03).expect("a slot");
    assert_eq!(page(&mut doc, b03, "parents"), ["M2"]);
    let _ = m2;
}
