//! The convenience operations leave a readable book and use the same source identities (ADR 0592).
use kentos_commands::{CommandResult, SheetsSet};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, sheets_create, sheets_set};
use serde_json::{Value, json};

#[test]
fn full_sheet_read_returns_the_book_templates_and_opt_in_bytes_without_editing() {
    let snapshot =
        serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/sheets.json")).unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let revision = doc.revision();
    let generation = doc.generation();
    let input = SheetsSet {
        details: true,
        pictures: true,
        ..Default::default()
    };
    let CommandResult::Completed { output, .. } = sheets_set::plan_in(&doc, &input) else {
        panic!()
    };
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    assert_eq!(output.book.unwrap(), serde_json::to_value(&book).unwrap());
    assert_eq!(output.pictures.unwrap(), doc.sheets().unwrap().pictures);
    let templates = output.templates.unwrap();
    assert!(templates.len() >= 10);
    for template in templates {
        let t: kentos_sheet::template::Template = serde_json::from_value(template).unwrap();
        kentos_sheet::template::validate_template(&t, true).unwrap();
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.generation(), generation);
    // Default reads remain compact and backward compatible.
    let CommandResult::Completed { output, .. } = sheets_set::plan_in(&doc, &SheetsSet::default())
    else {
        panic!()
    };
    assert!(output.book.is_none() && output.templates.is_none() && output.pictures.is_none());
    // Creation uses the real gallery template and remaps its internal links.
    let input=serde_json::from_value(json!({"id":"mimari","source":{"type":"system","id":"sys:cad-mimari"},"options":{"scale":50,"center":{"x":2.5,"y":1.5}}})).unwrap();
    let result = sheets_create::execute(&mut ExecutionContext::new(&mut doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    let made = book.sheet("mimari").unwrap();
    assert_eq!(made.origin.as_ref().unwrap().template_id, "sys:cad-mimari");
    for item in &made.items {
        if let Some(link) = item.kind.map_link() {
            assert!(made.items.iter().any(|i| &i.id == link));
        }
    }
}

#[test]
fn a_long_valid_sheet_id_does_not_overflow_derived_item_ids() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.sheets.create.json"
    ))
    .unwrap();
    let mut doc =
        Document::from_snapshot(serde_json::from_value(fixture["setup"].clone()).unwrap()).unwrap();
    let input = serde_json::from_value(
        json!({"id":"ç".repeat(128),"source":{"type":"system","id":"sys:cad-mimari"}}),
    )
    .unwrap();
    let result = sheets_create::execute(&mut ExecutionContext::new(&mut doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    assert_eq!(book.sheets[0].id.chars().count(), 128);
    assert!(
        book.sheets[0]
            .items
            .iter()
            .all(|i| kentos_sheet::validate::valid_id(&i.id))
    );
}
