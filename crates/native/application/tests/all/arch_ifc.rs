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
    assert_eq!(
        written.left,
        Vec::new(),
        "every object of the building written"
    );
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

/// The künye a permit set writes into the drawing's variables (docs/adr/8037), as an office fills it.
const KUNYE: [(&str, &str); 15] = [
    ("proje_kodu", "260123"),
    ("bina_kodu", "00"),
    ("revizyon_no", "000"),
    ("il", "Sivas"),
    ("ilce", "Merkez"),
    ("mahalle", "Esentepe"),
    ("ada", "1234"),
    ("parsel", "5"),
    ("tapu_kaydi", "12/345"),
    ("mimar", "Ayşe Nur Yılmaz"),
    ("mimar_unvan", "Y. Mimar"),
    ("sicil_no", "41234"),
    ("buro_adi", "Örnek Mimarlık"),
    ("vergi_no", "1234567890"),
    ("buro_adres", "Atatürk Cad. No:1 Sivas"),
];

fn with_kunye() -> Document {
    let mut snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    for (name, value) in KUNYE {
        snap.settings
            .variables
            .push(kentos_commands::ProjectVariable {
                name: name.into(),
                label: name.into(),
                value: kentos_commands::VariableValue::Text(value.into()),
                ..Default::default()
            });
    }
    Document::from_snapshot_v2(snap).unwrap()
}

#[test]
fn the_kunye_is_the_project_s_people_and_place() {
    let doc = with_kunye();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    // EK-1, EK-2: the project's name its code, the file EK-2's model name.
    assert_eq!(b.project.name, "260123");
    assert_eq!(b.file, "260123_00_MM_XXXX_MD_XXX_NNN_01_000");
    assert_eq!(
        arch_ifc::file_name(&doc),
        "260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc"
    );
    // EK-7: the architect, their registration, their office and its tax number.
    let a = &b.actors[0];
    assert_eq!(
        (
            a.identification.as_deref(),
            a.given.as_deref(),
            a.family.as_deref()
        ),
        (Some("41234"), Some("Ayşe Nur"), Some("Yılmaz"))
    );
    assert_eq!(
        (a.role, a.role_description.as_deref()),
        ("ARCHITECT", Some("Y. Mimar"))
    );
    let o = a.organization.as_ref().expect("the office");
    assert_eq!(
        (o.name.as_str(), o.identification.as_deref()),
        ("Örnek Mimarlık", Some("1234567890"))
    );
    // EK-6 Tablo 6.55: the address, the parcel and its title deed.
    let prop = |psets: &[kentos_ifc::building::Pset], set: &str, key: &str| {
        psets
            .iter()
            .find(|p| p.name == set)
            .and_then(|p| p.props.iter().find(|(k, _)| k == key))
            .map(|(_, v)| v.clone())
    };
    assert_eq!(
        prop(&b.site.psets, "Pset_Address", "Town"),
        Some(Value::Label("Sivas".into()))
    );
    assert_eq!(
        prop(&b.site.psets, "Pset_Address", "Region"),
        Some(Value::Label("Merkez".into()))
    );
    assert_eq!(
        prop(&b.site.psets, "Pset_Address", "AddressLines"),
        Some(Value::Labels(vec!["Esentepe".into()]))
    );
    assert_eq!(
        prop(&b.site.psets, "Pset_LandRegistration", "LandID"),
        Some(Value::Label("1234/5".into()))
    );
    assert_eq!(
        prop(&b.site.psets, "Pset_LandRegistration", "LandTitleID"),
        Some(Value::Label("12/345".into()))
    );
    // EK-6 Tablo 6.7: the building's temporary id.
    assert_eq!(
        prop(&b.building.psets, "Pset_BuildingCommon", "BuildingID"),
        Some(Value::Id("260123_00".into()))
    );
    assert_eq!(
        prop(&b.building.psets, "Pset_BuildingCommon", "IsPermanentID"),
        Some(Value::Bool(false))
    );
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("FILE_NAME('260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc'"));
    if std::env::var_os("KENTOS_WRITE_IFC_BUILDING").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/arch/v1/ifc/kesit-binasi-kunye.ifc");
        std::fs::write(out, &text).expect("the file kept");
    }
}

#[test]
fn without_a_kunye_the_drawing_names_the_project_and_the_file() {
    let doc = building();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    assert_eq!(
        (b.project.name.as_str(), b.file.as_str()),
        ("Kesit binası", "Kesit binası")
    );
    assert!(b.actors.is_empty());
    assert_eq!(arch_ifc::file_name(&doc), "Kesit binası.ifc");
}
