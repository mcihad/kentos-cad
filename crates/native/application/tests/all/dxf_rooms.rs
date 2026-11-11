//! Mahaller DXF'te (docs/adr/8046 §1) on the section building (fixtures/arch/v1/section-building.kcad): written as
//! another program reads them (KentOS's data left out), a room is its rings and its label's text — number, name, area,
//! the drawing's lines — on its layer; a void's cross goes with it; a tagged room's label is its tag, so it writes none
//! of its own. Before, a room was its rings alone.

use kentos_commands::DxfWriteInput;
use kentos_commands::{CallOp, DocumentSnapshotV2, EntityId};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// Zemin kat's rooms and the tags written as another program reads them, read back: each text's layer and words.
/// `KENTOS_ROOMS_DXF=path` keeps the file as `path-<keep>.dxf` (the evidence's external reading).
fn texts(doc: &Document, keep: &str) -> Vec<(String, String)> {
    let chosen: Vec<Slot> = doc
        .entities_with_uids()
        .filter(|(_, e)| match e {
            kentos_commands::Entity::Space(s) => s.space.arch.level.as_deref() == Some("zemin"),
            kentos_commands::Entity::Tag(_) => true,
            _ => false,
        })
        .filter_map(|(u, _)| doc.slot_of(u))
        .collect();
    let mut input: DxfWriteInput = serde_json::from_value(json!({
        "entities": chosen.iter().map(|s| doc.get(*s).unwrap().clone()).collect::<Vec<_>>(),
        "layers": [{"id": "cizim", "name": "MM-XXXX-MAHAL", "path": [], "color": "#FFFFFF", "visible": true,
                    "locked": false, "lineType": "continuous", "lineWeight": 0.18}],
        "scale": 100.0, "lengthDecimals": 3, "grads": false, "unit": "m"
    }))
    .unwrap();
    input.uids = chosen
        .iter()
        .map(|&s| (s.0, EntityId(doc.uid(s).unwrap().into_bytes())))
        .collect();
    input.annotations.metadata = false;
    let (bytes, report) = kentos_formats::dxf::write(&input);
    if let Ok(path) = std::env::var("KENTOS_ROOMS_DXF") {
        std::fs::write(format!("{path}-{keep}.dxf"), &bytes).expect("the DXF kept");
    }
    assert!(
        serde_json::to_string(&report)
            .unwrap()
            .contains("etiketsiz mahalin yazısı"),
        "the report says what a room became"
    );
    let read = kentos_formats::dxf::read(&bytes, &kentos_commands::DxfReadOptions::default())
        .expect("the DXF reads");
    read.entities
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some((t.base.layer_id.clone(), t.text.clone())),
            _ => None,
        })
        .collect()
}

fn lines_on(doc: &Document, layer: &str) -> usize {
    let chosen: Vec<kentos_commands::Entity> = doc
        .entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Space(_)))
        .cloned()
        .collect();
    let mut input: DxfWriteInput = serde_json::from_value(json!({
        "entities": chosen,
        "layers": [{"id": "cizim", "name": "MM-XXXX-MAHAL", "path": [], "color": "#FFFFFF", "visible": true,
                    "locked": false, "lineType": "continuous", "lineWeight": 0.18}],
        "scale": 100.0, "lengthDecimals": 3, "grads": false, "unit": "m"
    }))
    .unwrap();
    input.annotations.metadata = false;
    let (bytes, _) = kentos_formats::dxf::write(&input);
    let read = kentos_formats::dxf::read(&bytes, &kentos_commands::DxfReadOptions::default())
        .expect("the DXF reads");
    read.entities
        .iter()
        .filter(|e| matches!(e, kentos_commands::Entity::Line(_)) && e.base().layer_id == layer)
        .count()
}

#[test]
fn a_room_s_label_and_a_void_s_cross_go_out_and_a_tagged_room_s_label_is_its_tag() {
    let mut doc = building();
    // Zemin kat's five rooms: a text each on their layer, number, name and area one under another.
    let said = texts(&doc, "mahaller");
    let mut words: Vec<String> = said.iter().map(|(_, t)| t.replace('\n', " / ")).collect();
    words.sort();
    assert_eq!(
        words,
        [
            "001 / Yatak odası / 17.85 m²",
            "002 / Salon / 17.85 m²",
            "003 / Merdiven holü / 24.51 m²",
            "004 / Banyo / 17.85 m²",
            "005 / Mutfak / 17.85 m²",
        ]
    );
    assert!(
        said.iter().all(|(layer, _)| layer == "MM-XXXX-MAHAL"),
        "on the room's layer: {said:?}"
    );
    // A void (Şaft): its cross, two lines corner to corner (docs/adr/0708), and its words.
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    assert_eq!(lines_on(&doc, "MM-XXXX-MAHAL"), 0);
    run(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": "cizim", "spaces": [{
            "parts": [{"outer": {"pts": [p(13.0, 0.0), p(14.6, 0.0), p(14.6, 2.1), p(13.0, 2.1)]}}],
            "method": "net", "name": "Şaft", "void": true, "arch": {"level": "zemin"}
        }]}),
    );
    assert_eq!(lines_on(&doc, "MM-XXXX-MAHAL"), 2, "the void's cross");
    // The living room tagged: its tag's words go out, its own label not (the tag is its label).
    let (uid, _) = doc
        .entities_with_uids()
        .find(|(_, e)| {
            matches!(e, kentos_commands::Entity::Space(s)
                if s.space.name.as_deref() == Some("Salon") && s.space.arch.level.as_deref() == Some("zemin"))
        })
        .expect("the living room");
    run(
        &mut doc,
        "cad.tags.create",
        json!({"layerId": "cizim", "tags": [{"of": uid.to_string(), "at": p(2.3, 7.4)}]}),
    );
    let after = texts(&doc, "bosluk-etiket");
    let mut words: Vec<String> = after.iter().map(|(_, t)| t.replace('\n', " / ")).collect();
    words.sort();
    assert_eq!(
        words,
        [
            "001 / Yatak odası / 17.85 m²",
            "002",
            "003 / Merdiven holü / 24.51 m²",
            "004 / Banyo / 17.85 m²",
            "005 / Mutfak / 17.85 m²",
            "17.85 m²",
            "Salon",
            "Şaft / 3.36 m²",
        ],
        "the tag's three lines in place of the living room's label"
    );
}
