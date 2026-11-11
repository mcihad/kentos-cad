//! IFC dışa aktar (bina) (arch_ifc.rs, docs/adr/8047): the section building goes out as IFC 4.3 from Dosya › Dışa
//! aktar, and the message says what the file holds.

use kentos_interaction::Level;

use crate::app::{App, Message};

/// The section building (fixtures/arch/v1/section-building.kcad): two storeys of walls, openings, rooms, slabs,
/// columns, beams and a stair.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!(
        "../../../fixtures/arch/v1/section-building.kcad"
    ))
    .expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_building_goes_out_as_ifc_and_the_message_says_what_it_holds() {
    let mut app = app();
    let out = std::env::temp_dir().join(format!("kentos-desktop-bina-{}.ifc", std::process::id()));
    let _ = app.update(Message::IfcBuildingExport(Some(out.clone())));
    let text = std::fs::read_to_string(&out).expect("written");
    assert!(text.starts_with("ISO-10303-21;") && text.contains("FILE_SCHEMA(('IFC4X3_ADD2'));"));
    assert_eq!(text.matches("=IFCWALL(").count(), 16);
    assert_eq!(text.matches("=IFCDOOR(").count(), 5);
    assert_eq!(text.matches("=IFCSPACE(").count(), 10);
    let said = app.log.last().expect("said");
    assert_eq!(said.level, Level::Success);
    assert!(
        said.text.starts_with("IFC 4.3 yazıldı: 2 kat, 16 duvar, 5 kapı, 9 pencere, 10 mahal, 3 döşeme, 16 kolon, 8 kiriş, 1 merdiven"),
        "{}",
        said.text
    );
    let _ = std::fs::remove_file(out);
    // The drawing does not change.
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(!doc.can_undo());
}

#[test]
fn a_file_name_without_its_extension_gets_ifc() {
    let mut app = app();
    let bare = std::env::temp_dir().join(format!(
        "kentos-desktop-bina-uzantisiz-{}",
        std::process::id()
    ));
    let _ = app.update(Message::IfcBuildingExport(Some(bare.clone())));
    let written = bare.with_extension("ifc");
    assert!(written.exists(), "{} written", written.display());
    let _ = std::fs::remove_file(written);
}
