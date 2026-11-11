//! The building as IFC 4.3 (docs/adr/8047) on the section building (fixtures/arch/v1/section-building.kcad): every wall,
//! door, window and room of the drawing is one IfcWall, IfcDoor, IfcWindow and IfcSpace, named as EK-5 §5.5 names it.
//! `KENTOS_WRITE_IFC_BUILDING=1` writes fixtures/arch/v1/ifc/kesit-binasi.ifc, which `scripts/fixtures/arch_ifc_check.py`
//! reads with IfcOpenShell against the fixture's own definition (scripts/fixtures/section_building.py), not KentOS's.

use kentos_commands::{DocumentSnapshotV2, Entity, OpeningForm};
use kentos_domain::Document;
use kentos_ifc::building::{Class, Value};
use kentos_native_application::arch_ifc;

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

#[test]
fn the_drawing_s_walls_doors_windows_and_rooms_are_the_files() {
    let doc = building();
    let count = |f: &dyn Fn(&Entity) -> bool| doc.entities().filter(|e| f(e)).count();
    let walls = count(&|e| matches!(e, Entity::Wall(_)));
    let doors = count(&|e| matches!(e, Entity::Opening(o) if o.opening.form == OpeningForm::Door));
    let windows =
        count(&|e| matches!(e, Entity::Opening(o) if o.opening.form == OpeningForm::Window));
    let rooms = count(&|e| matches!(e, Entity::Space(_)));
    let (b, written) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    assert_eq!(
        (
            written.walls,
            written.doors,
            written.windows,
            written.spaces,
            written.storeys
        ),
        (walls, doors, windows, rooms, 2),
        "{written:?}"
    );
    assert!(walls > 0 && doors > 0 && windows > 0 && rooms > 0);
    // EK-5's names: MM, the class's code, the description without white space or "-".
    for e in &b.elements {
        let mut parts = e.name.splitn(3, '-');
        assert_eq!(parts.next(), Some("MM"), "{}", e.name);
        assert_eq!(parts.next(), Some(e.class.code()), "{}", e.name);
        assert!(
            parts
                .next()
                .is_none_or(|d| !d.contains(char::is_whitespace) && !d.contains('-')),
            "{}",
            e.name
        );
    }
    // Pset_WallCommon.IsExternal: the outer walls (Dış duvar 25), not the inner ones, though the columns cut the outer
    // ones apart and leave each piece an outline of its own.
    for e in b.elements.iter().filter(|e| e.class == Class::Wall) {
        let external = e
            .psets
            .iter()
            .flat_map(|p| &p.props)
            .find(|(k, _)| k == "IsExternal");
        assert_eq!(
            external.map(|(_, v)| v),
            Some(&Value::Bool(e.name.contains("Dış_duvar"))),
            "{}",
            e.name
        );
    }
    let text = kentos_ifc::building::write(&b);
    assert!(text.starts_with("ISO-10303-21;") && text.contains("FILE_SCHEMA(('IFC4X3_ADD2'));"));
    if std::env::var_os("KENTOS_WRITE_IFC_BUILDING").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/arch/v1/ifc/kesit-binasi.ifc");
        std::fs::write(out, &text).expect("the file kept");
    }
}
