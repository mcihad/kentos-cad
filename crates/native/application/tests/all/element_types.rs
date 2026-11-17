//! BIM-12's first acceptance (docs/adr/8106; plan/bim-plan.md BIM-12): a column type made from a drawn column, its
//! seventeen columns tied to it, one given its own width by an edit (an exception); the type's size changed, sixteen
//! follow and the one with its own width keeps it, the source of each value said. And IFC-04's next part: the section
//! building's elements typed by their own values, each type written as its class's IFC type, its elements typed by it;
//! the file is kept (fixtures/arch/v1/ifc/kesit-binasi-eleman-tipli.ifc) with the types as the drawing says them
//! (kesit-binasi-eleman-tipli.json) for the independent reader (scripts/fixtures/ifc_types_check.py).
//! `KENTOS_WRITE_IFC_TYPES=1` writes them again.

use std::path::PathBuf;

use kentos_commands::{
    ArchChange, ArchEdit, ArchOperation, CommandResult, ElementTypeKind, ElementTypeRefused, ElementTypesEdit,
    ElementTypesOperation, Entity, element_type_values, fields_off_type, type_link,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, arch_edit, arch_ifc, element_types_edit};
use serde_json::{Map, Value, json};

fn uid(n: usize) -> String {
    format!("0192f5a1-7720-7000-8000-{n:012x}")
}

/// Seventeen columns of their own, 30/50, on a 5 m grid.
fn columns() -> Document {
    let entities: Vec<Value> = (1..=17)
        .map(|k| {
            json!({
                "kind": "column", "id": k, "layerId": "kolon", "attrs": {},
                "at": { "x": 5.0 * ((k - 1) % 6) as f64, "y": 5.0 * ((k - 1) / 6) as f64 },
                "width": 0.3, "depth": 0.5, "fill": true
            })
        })
        .collect();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(json!({
        "format": "kentos.document", "version": 2, "name": "Kolonlar",
        "uids": (1..=17).map(uid).collect::<Vec<_>>(),
        "settings": { "srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 50, "workspace": "cad" },
        "origin": { "x": 0, "y": 0 },
        "layers": [{ "id": "kolon", "name": "Kolon", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "kolon",
        "styles": { "items": [], "categories": [] },
        "entities": entities
    }))
    .expect("the drawing");
    Document::from_snapshot_v2(snapshot).expect("opens")
}

fn types(doc: &mut Document, operation: ElementTypesOperation) -> kentos_commands::ElementTypesEdited {
    let input = ElementTypesEdit { operation, on_refused: ElementTypeRefused::Stop, expected_revision: None };
    match element_types_edit::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn column(doc: &Document, n: usize) -> Entity {
    let slot = doc.slot_of(uid(n).parse().expect("a uid")).expect("in the drawing");
    doc.get(slot).cloned().expect("the column")
}

fn width(e: &Entity) -> f64 {
    match e {
        Entity::Column(c) => c.column.width,
        _ => panic!("a column"),
    }
}

#[test]
fn the_type_s_size_changes_sixteen_columns_follow_and_the_one_with_its_own_width_keeps_it() {
    let mut doc = columns();
    // Önce çiz, beğenince tipe çevir: the first column becomes S1 and follows it.
    let made = types(&mut doc, ElementTypesOperation::Define {
        name: "S1".into(),
        id: None,
        element: None,
        copy_of: None,
        from: Some(uid(1)),
        values: Map::new(),
    });
    assert_eq!(made.id.as_deref(), Some("s1"));
    assert_eq!(Value::Object(made.types[0].values.clone()), json!({ "width": 0.3, "depth": 0.5, "fill": true }));
    types(&mut doc, ElementTypesOperation::Assign { kind: "S1".into(), uids: (2..=17).map(uid).collect() });
    // The seventeenth's width its own, by an edit through Öznitelikler's command: an exception.
    let edit = ArchEdit {
        uids: vec![uid(17)],
        operation: ArchOperation::Set(Box::new(ArchChange { width: Some(0.45), ..Default::default() })),
        expected_revision: None,
    };
    let CommandResult::Completed { .. } = arch_edit::execute(&mut ExecutionContext::new(&mut doc), edit) else {
        panic!("edited")
    };
    let s1 = doc.element_types()[0].clone();
    assert_eq!(fields_off_type(&column(&doc, 17), &s1), ["width"], "Örnekten: its width");
    assert_eq!(type_link(&column(&doc, 17)).map(|l| l.overrides.clone()), Some(vec!["width".to_owned()]));
    assert!(fields_off_type(&column(&doc, 3), &s1).is_empty(), "Tipten: every value");
    // The type's size changed.
    let changed = types(&mut doc, ElementTypesOperation::Update {
        kind: "S1".into(),
        values: json!({ "width": 0.35 }).as_object().cloned().unwrap_or_default(),
    });
    assert_eq!(changed.report.applied.len(), 17, "all take the new version");
    assert_eq!(changed.report.kept.len(), 1);
    assert_eq!(changed.report.kept[0].uid, uid(17));
    let followed = (1..=17).filter(|&n| (width(&column(&doc, n)) - 0.35).abs() < 1e-12).count();
    assert_eq!(followed, 16, "sixteen follow");
    assert_eq!(width(&column(&doc, 17)), 0.45, "the one with its own width keeps it");
    let s1 = doc.element_types()[0].clone();
    assert_eq!(s1.version, 2);
    assert!((1..=17).all(|n| type_link(&column(&doc, n)).is_some_and(|l| l.version == 2)));
    if let Entity::Column(c) = column(&doc, 9) {
        assert_eq!((c.column.depth, c.column.kind.as_deref()), (Some(0.5), Some("S1")));
    }
    // One undo step gives the type and its columns back.
    assert_eq!(doc.undo().as_deref(), Some(element_types_edit::STEP));
    assert_eq!(doc.element_types()[0].version, 1);
    assert_eq!((1..=16).filter(|&n| width(&column(&doc, n)) == 0.3).count(), 16);
}

/// The section building's elements of each kind a type serves, grouped by their own values: each group a type made
/// from its first element ("{Kind} {n}"), the others tied to it — no element changes but its link.
fn typed_building() -> Document {
    let mut doc = crate::ifc_required::declared();
    let mut groups: Vec<(ElementTypeKind, Map<String, Value>, Vec<String>)> = Vec::new();
    for (u, e) in doc.entities_with_uids() {
        let Some((kind, mut values)) = element_type_values(e) else { continue };
        // A column's type text is the type's name, not one of its values.
        values.remove("type");
        match groups.iter_mut().find(|(k, v, _)| *k == kind && *v == values) {
            Some(g) => g.2.push(u.to_string()),
            None => groups.push((kind, values, vec![u.to_string()])),
        }
    }
    let mut counts: std::collections::BTreeMap<ElementTypeKind, usize> = Default::default();
    for (kind, _, members) in groups {
        let n = counts.entry(kind).or_default();
        *n += 1;
        let made = types(&mut doc, ElementTypesOperation::Define {
            name: format!("{} {n}", kind.label()),
            id: None,
            element: None,
            copy_of: None,
            from: Some(members[0].clone()),
            values: Map::new(),
        });
        if members.len() > 1 {
            let id = made.id.expect("its id");
            types(&mut doc, ElementTypesOperation::Assign { kind: id, uids: members[1..].to_vec() });
        }
    }
    doc
}

#[test]
fn every_element_type_is_its_class_s_ifc_type_and_its_elements_are_typed_by_it() {
    let doc = typed_building();
    let made = doc.element_types().to_vec();
    assert!(made.iter().any(|t| t.kind == ElementTypeKind::Column), "{made:?}");
    let (b, _) = arch_ifc::building(&doc, "2026-11-17T12:00:00", "KentOS Mimari");
    let text = kentos_ifc::building::write(&b);
    // The wall styles' types and one for each element type with an element of its class.
    let element_types: Vec<&kentos_ifc::building::TypeObject> =
        b.types.iter().filter(|t| t.entity != "IFCWALLTYPE").collect();
    assert_eq!(element_types.len(), made.len(), "{:?}", b.types.iter().map(|t| t.entity).collect::<Vec<_>>());
    for t in &element_types {
        assert!(!t.members.is_empty(), "{}", t.name);
        assert!(text.contains(&format!("{}('", t.entity)), "{}", t.entity);
    }
    assert_eq!(text.matches("IFCRELDECLARES(").count(), 1, "one declaration of every type");
    // The types as the drawing says them, beside the file: each its name, its IFC class and its elements' ids.
    let note_types: Vec<Value> = made
        .iter()
        .map(|t| {
            let members: Vec<String> = doc
                .entities_with_uids()
                .filter(|(_, e)| type_link(e).is_some_and(|l| l.id == t.id))
                .map(|(u, _)| u.to_string())
                .collect();
            let class = element_types.iter().find(|x| x.name == t.name).map_or("", |x| x.entity);
            json!({ "name": t.name, "kind": t.kind, "ifc": class, "members": members })
        })
        .collect();
    let note = serde_json::to_string_pretty(&json!({ "file": "kesit-binasi-eleman-tipli.ifc", "types": note_types }))
        .expect("the note")
        + "\n";
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc");
    if std::env::var_os("KENTOS_WRITE_IFC_TYPES").is_some() {
        std::fs::write(dir.join("kesit-binasi-eleman-tipli.ifc"), &text).expect("writes the file");
        std::fs::write(dir.join("kesit-binasi-eleman-tipli.json"), &note).expect("writes the note");
    }
    assert_eq!(std::fs::read_to_string(dir.join("kesit-binasi-eleman-tipli.ifc")).expect("the kept file"), text);
    assert_eq!(std::fs::read_to_string(dir.join("kesit-binasi-eleman-tipli.json")).expect("the kept note"), note);
}
