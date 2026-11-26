//! A fonts folder's family in the project file (docs/adr/9313): the project's typeface, a text style's, a text's and a
//! dimension's may name a family by its name. The file writes and reads the name as it is, the typed columns carry it,
//! and KentOS's own faces keep their ids.

use kentos_kcad::contracts::{DocumentSnapshotV2, DrawingFont, Entity};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn content(name: &str) -> DocumentSnapshotV2 {
    let bytes = std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn family(name: &str) -> DrawingFont {
    DrawingFont::from_name(name).unwrap_or_else(|| panic!("“{name}” is a family's name"))
}

/// Every text and dimension of the styles fixture, its styles and the project, in folder families.
fn in_families() -> DocumentSnapshotV2 {
    let mut doc = content("styles.json");
    doc.settings.drawing_font = Some(family("Kurum Sans"));
    for s in &mut doc.settings.text_styles {
        s.font = family("Roboto");
    }
    for e in &mut doc.entities {
        match e {
            Entity::Text(t) => t.face.font = Some(family("Source Sans 3")),
            Entity::Dimension(d) => d.look.font = Some(family("Kurum Sans")),
            _ => {}
        }
    }
    doc
}

fn json(value: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

#[test]
fn a_folder_family_comes_back_by_its_name() {
    let doc = in_families();
    let bytes = kentos_kcad::encode(&doc).expect("writes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(json(&back), json(&doc));
    assert_eq!(back.settings.drawing_font.map(DrawingFont::id), Some("Kurum Sans"));
    // The name is in the file as it is written, beside the own faces' ids.
    let named = |name: &str| bytes.windows(name.len()).any(|w| w == name.as_bytes());
    assert!(named("Source Sans 3") && named("Roboto") && named("Kurum Sans"));
}

#[test]
fn the_typed_columns_carry_a_folder_family() {
    let doc = in_families();
    let (_, cols) = kentos_kcad::split(doc.clone()).expect("splits");
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!(uids, doc.uids);
    assert_eq!(json(&entities), json(&doc.entities));
    let texts = entities.iter().filter(|e| matches!(e, Entity::Text(t) if t.face.font == Some(family("Source Sans 3"))));
    assert!(texts.count() > 0, "the fixture has texts");
}

#[test]
fn own_faces_keep_their_bytes() {
    // The styles fixture names only KentOS's own faces: it is written as it always was.
    let doc = content("styles.json");
    let bytes = kentos_kcad::encode(&doc).expect("writes");
    let kept = std::fs::read(format!("{DIR}styles.kcad")).expect("styles.kcad");
    assert!(bytes == kept, "the reference's bytes");
}
