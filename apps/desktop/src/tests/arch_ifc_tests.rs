//! IFC dışa aktar (bina) (arch_ifc.rs, docs/adr/8047): the section building goes out as IFC 4.3 from Dosya › Dışa
//! aktar, and the message says what the file holds.

use kentos_interaction::Level;

use crate::app::{App, Message};

/// The section building (fixtures/arch/v1/section-building.kcad): two storeys of walls, openings, rooms, slabs,
/// columns, beams and a stair.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!(
        "../../../../fixtures/arch/v1/section-building.kcad"
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
    let _ = app.update(Message::from(crate::arch::Event::IfcBuildingExport(Some(out.clone()))));
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
    let _ = app.update(Message::from(crate::arch::Event::IfcBuildingExport(Some(bare.clone()))));
    let written = bare.with_extension("ifc");
    assert!(written.exists(), "{} written", written.display());
    let _ = std::fs::remove_file(written);
}

/// IFC dışa aktar (bina, kesilmiş gövde) (docs/adr/8071 §4): the same building with its bodies already cut, its
/// openings 'Reference' shapes, its header ReferenceView; the message says so.
#[test]
fn the_cut_bodies_go_out_as_a_reference_view() {
    let mut app = app();
    let out = std::env::temp_dir().join(format!("kentos-desktop-bina-kesik-{}.ifc", std::process::id()));
    let _ = app.update(Message::from(crate::arch::Event::IfcBuildingExportCut(Some(out.clone()))));
    let text = std::fs::read_to_string(&out).expect("written");
    assert!(text.contains("ViewDefinition [ReferenceView]"));
    assert_eq!(text.matches("=IFCWALL(").count(), 16);
    // 14 through the walls and the 1. kat slab's hole over the stair (docs/adr/8071 §5).
    assert_eq!(text.matches("=IFCOPENINGELEMENT(").count(), 15);
    assert_eq!(text.matches(",'Reference','SweptSolid',").count(), 15, "each opening a 'Reference' shape");
    let said = app.log.last().expect("said");
    assert_eq!(said.level, Level::Success);
    assert!(said.text.starts_with("IFC 4.3 yazıldı (gövdeler açıklıkları kesilmiş): 2 kat, 16 duvar"), "{}", said.text);
    let _ = std::fs::remove_file(out);
}

/// Bina başına model (docs/adr/8072 §2): the program's two blocks (the section building's storeys split between them,
/// each its code) go out as two files from one choice: the first where the user said, the second beside it named by
/// EK-2 with its code; the message names both.
#[test]
fn two_blocks_go_out_as_two_files() {
    let mut app = app();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    let arch = settings.arch.as_mut().expect("its levels");
    arch.program.buildings = serde_json::from_value(serde_json::json!([
        {"id": "a", "name": "A Blok", "levels": ["zemin"], "code": "A1"},
        {"id": "b", "name": "B Blok", "levels": ["k1"], "code": "B1", "zero": 3.0}
    ]))
    .unwrap();
    doc.set_settings(settings);
    let dir = std::env::temp_dir().join(format!("kentos-desktop-bloklar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let first = dir.join("a.ifc");
    let _ = app.update(Message::from(crate::arch::Event::IfcBuildingExport(Some(first.clone()))));
    let a = std::fs::read_to_string(&first).expect("A written");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let second = dir.join(kentos_native_application::arch_ifc::file_name_of(doc, "b"));
    let b = std::fs::read_to_string(&second).expect("B written beside it");
    assert_eq!(a.matches("=IFCBUILDINGSTOREY(").count(), 1);
    assert_eq!(b.matches("=IFCBUILDINGSTOREY(").count(), 1);
    assert!(a.contains("'MM-BNA-A_Blok'") && b.contains("'MM-BNA-B_Blok'"));
    assert!(second.display().to_string().contains("B1"), "{}", second.display());
    let said = app.log.last().expect("said");
    assert_eq!(said.level, Level::Success);
    assert!(said.text.starts_with("IFC 4.3, 2 bina ayrı dosyalara yazıldı: A Blok: 1 kat"), "{}", said.text);
    let _ = std::fs::remove_dir_all(dir);
}
