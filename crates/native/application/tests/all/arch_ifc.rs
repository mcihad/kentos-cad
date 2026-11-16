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
    // TUREF / TM33 (Sivas), Antalya heights, ±0.00 at 1285.40 m (EK-7 Tablo 7.6–7.7).
    snap.settings.srid = 5255;
    snap.settings.heights = Some(kentos_commands::HeightReference {
        kind: kentos_commands::HeightKind::Orthometric,
        vertical_srid: Some(5775),
        geoid: None,
    });
    if let Some(arch) = snap.settings.arch.as_mut() {
        arch.site = Some(kentos_commands::SitePlan {
            datum: Some(1285.4),
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
    // EK-7 Tablo 7.6–7.7: TUREF's zone in full, Antalya heights, the model raised to ±0.00's elevation.
    let crs = b.crs.as_ref().expect("the project's system");
    assert_eq!(
        (
            crs.name.as_str(),
            crs.geodetic_datum.as_deref(),
            crs.map_projection.as_deref(),
            crs.map_zone.as_deref()
        ),
        (
            "EPSG:5255",
            Some("TUREF"),
            Some("Transverse Mercator"),
            Some("33")
        )
    );
    assert_eq!(
        crs.vertical_datum.as_deref(),
        Some("Antalya height (EPSG:5775)")
    );
    assert_eq!(crs.origin, [0.0, 0.0, 1285.4]);
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
    assert!(
        b.crs.is_none(),
        "a drawing in local coordinates has no system"
    );
    assert_eq!(arch_ifc::file_name(&doc), "Kesit binası.ifc");
}

/// Aks ızgarası (docs/adr/8070 §4; EK-6 Tablo 6.36): the section building's grids A–D (north–south) and 1–3 (east–west),
/// on no storey, are one rectangular IfcGrid of the building: A–D its U axes, 1–3 its V, each axis its tag and its line.
/// Its footprint is drawn in a plan context made only for it. Left with A–D alone (one direction), the grid is no
/// IfcGrid and its axes are on the loss list.
#[test]
fn the_grids_are_one_rectangular_ifc_grid_and_one_direction_alone_is_left_out() {
    let doc = building();
    let (b, written) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    assert_eq!(b.grids.len(), 1, "{:?}", b.grids);
    let g = &b.grids[0];
    let tags = |axes: &[kentos_ifc::building::Axis]| axes.iter().map(|a| a.tag.clone()).collect::<Vec<_>>();
    assert_eq!(tags(&g.u), ["A", "B", "C", "D"]);
    assert_eq!(tags(&g.v), ["1", "2", "3"]);
    assert_eq!((g.predefined, g.storey), ("RECTANGULAR", None));
    assert_eq!(written.grid_axes, 7);
    assert!(g.u.iter().chain(&g.v).all(|a| a.through.is_none()));
    assert_eq!((g.u[1].from, g.u[1].to), ([4.5, -2.0], [4.5, 11.0]));
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCGRIDAXIS(").count(), 7);
    assert_eq!(text.matches("IFCGRID(").count(), 1);
    assert!(text.contains("IFCGRIDAXIS('B',"), "the axes' tags");
    assert!(text.contains(".RECTANGULAR.)"));
    assert!(text.contains("'FootPrint','GeometricCurveSet'"));
    // The grid's axes carry their tags: no axis is missing one.
    let gaps = kentos_ifc::required::gaps(&b);
    assert!(!gaps.iter().any(|g| g.class == "IfcGridAxis"), "{gaps:?}");
    // One direction alone.
    let mut snap = kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    let keep: Vec<bool> = snap
        .entities
        .iter()
        .map(|e| !matches!(e, Entity::Grid(g) if g.grid.label.chars().all(|c| c.is_ascii_digit())))
        .collect();
    let mut k = 0;
    snap.entities.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    let mut k = 0;
    snap.uids.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    for (i, e) in snap.entities.iter_mut().enumerate() {
        e.base_mut().id = i as u32 + 1;
    }
    let one = kentos_domain::Document::from_snapshot_v2(snap).unwrap();
    let (b, written) = arch_ifc::building(&one, "2026-11-11T12:00:00", "KentOS Mimari");
    assert!(b.grids.is_empty());
    let left: Vec<&kentos_ifc::arch::Left> = written.left.iter().filter(|l| l.what == "aks").collect();
    assert_eq!(left.len(), 4, "{:?}", written.left);
    assert!(left.iter().all(|l| l.why == kentos_ifc::arch::Why::OneWay));
}

/// Kitaplık blokları yapı elemanı olarak (docs/adr/8070 §5): KentOS's library's Asansör, Lavabo, Masa and Dolap placed
/// on the section building are an IfcTransportElement ELEVATOR (its rated capacity from its KISI and YUK attributes), an
/// IfcSanitaryTerminal WASHHANDBASIN, an IfcFurniture TABLE and an IfcFurniture USERDEFINED named for its block; each a
/// box over its drawn outline from its floor up its card's height (the lift's 1.20 × 1.50 m cabin drawn with its walls,
/// doors and counterweight: x 0.17 to 1.43, y 0.015 (the landing door's face) to 1.96 (the counterweight's back)); the Kot işareti, no building element, is none. The section model has their
/// boxes on Tefriş's and Asansör's layers. The file writes the kilogram and the lift's capacity.
#[test]
fn the_library_s_fixtures_are_their_ifc_classes_with_their_bodies() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let library = include_str!("../../../../../catalog/v1/blocks/kentos-temel.kblok");
    run(&mut doc, "cad.blocks.edit", serde_json::json!({"operation": "import", "library": library}));
    let place = |doc: &mut Document, name: &str, x: f64, y: f64, extra: serde_json::Value| -> String {
        let mut input = serde_json::json!({"name": name, "at": {"x": x, "y": y}, "layerId": "0",
                                           "arch": {"level": "zemin"}});
        for (k, v) in extra.as_object().into_iter().flatten() {
            input[k] = v.clone();
        }
        let made = run(doc, "cad.blocks.insert", input);
        made["output"]["uid"]
            .as_str()
            .or_else(|| made["output"]["uids"][0].as_str())
            .unwrap()
            .to_owned()
    };
    let lift = place(&mut doc, "Asansör", 5.0, 6.0, serde_json::json!({"attributes": {"KISI": "8", "YUK": "630"}}));
    let basin = place(&mut doc, "Lavabo", 8.5, 0.2, serde_json::Value::Null);
    let table = place(&mut doc, "Masa", 2.0, 6.5, serde_json::Value::Null);
    let wardrobe = place(&mut doc, "Dolap", 0.2, 2.0, serde_json::Value::Null);
    let mark = place(&mut doc, "Kot işareti", 1.0, 1.0, serde_json::Value::Null);
    let (b, written) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let of = |uid: &str| {
        let id = kentos_domain::Uuid::parse_str(uid).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id))
    };
    assert!(of(&mark).is_none(), "a level mark is no building element");
    assert_eq!(written.fixtures, 4);
    let lift_el = of(&lift).expect("the lift");
    assert_eq!((lift_el.class, lift_el.predefined), (Class::TransportElement, "ELEVATOR"));
    assert_eq!(lift_el.name, "MM-TSM-Asansör");
    let capacity = lift_el
        .psets
        .iter()
        .find(|p| p.name == "Pset_TransportElementCommon")
        .expect("its capacity");
    assert!(capacity.props.contains(&("CapacityPeople".into(), Value::Count(8))));
    assert!(capacity.props.contains(&("CapacityWeight".into(), Value::Mass(630.0))));
    let span = |e: &kentos_ifc::building::Element| match &e.shapes[..] {
        [kentos_ifc::building::Shape::Prism { rings, bottom, top }] => {
            let xs: Vec<f64> = rings[0].iter().map(|p| p[0]).collect();
            let ys: Vec<f64> = rings[0].iter().map(|p| p[1]).collect();
            let w = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
            let d = ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
            (w, d, *bottom, *top)
        }
        other => panic!("{other:?}"),
    };
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    let (w, d, bottom, top) = span(lift_el);
    assert!(near(w, 1.26) && near(d, 1.945) && near(bottom, 0.0) && near(top, 2.2), "{w} {d} {bottom} {top}");
    let basin_el = of(&basin).expect("the basin");
    assert_eq!((basin_el.class, basin_el.predefined), (Class::SanitaryTerminal, "WASHHANDBASIN"));
    assert!(near(span(basin_el).3, 0.85));
    let table_el = of(&table).expect("the table");
    assert_eq!((table_el.class, table_el.predefined, table_el.object_type.as_deref()), (Class::Furniture, "TABLE", None));
    assert!(near(span(table_el).3, 0.75));
    let wardrobe_el = of(&wardrobe).expect("the wardrobe");
    assert_eq!(
        (wardrobe_el.predefined, wardrobe_el.object_type.as_deref()),
        ("USERDEFINED", Some("Dolap"))
    );
    // The section model holds their boxes.
    let m = kentos_native_application::sections::model(&doc);
    let layer_of = |uid: &str| {
        m.owners
            .iter()
            .position(|o| o.as_deref() == Some(uid))
            .map(|i| m.looks[i].layer.clone())
    };
    assert_eq!(layer_of(&lift).as_deref(), Some("MM-ASAN-GENEL"));
    assert_eq!(layer_of(&basin).as_deref(), Some("MM-TEFR-VITRIFIYE"));
    assert_eq!(layer_of(&table).as_deref(), Some("MM-TEFR-MOBILYA"));
    assert_eq!(layer_of(&mark), None);
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.)"));
    assert!(text.contains("IFCMASSMEASURE(630.)") && text.contains("IFCCOUNTMEASURE(8)"));
    assert!(text.contains("IFCTRANSPORTELEMENT(") && text.contains("IFCSANITARYTERMINAL(") && text.contains("IFCFURNITURE("));
}

/// Kaplama kalınlıkları (docs/adr/8070 §6; EK-6 Tablo 6.20's first way, the room's own set): the section building's
/// rooms carry their floor's finish (the floor slab's top layer, 2 cm on both storeys) and their ceiling's plaster (the
/// slab over's, 2 cm) as Pset_SpaceCoveringRequirements' thicknesses; a room with an asma tavan has no ceiling
/// thickness (its build-up is not drawn).
#[test]
fn a_room_carries_its_floor_finish_and_ceiling_plaster_thicknesses() {
    let doc = building();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let get = |s: &kentos_ifc::building::Space, key: &str| {
        s.psets
            .iter()
            .filter(|p| p.name == "Pset_SpaceCoveringRequirements")
            .flat_map(|p| p.props.iter())
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    let thick = b
        .spaces
        .iter()
        .filter(|s| get(s, "FloorCoveringThickness") == Some(Value::PositiveLength(0.02)))
        .count();
    assert!(thick >= 9, "{thick} of {} rooms", b.spaces.len());
    assert!(b
        .spaces
        .iter()
        .all(|s| get(s, "CeilingCoveringThickness").is_none_or(|v| v == Value::PositiveLength(0.02))));
    assert!(b.spaces.iter().any(|s| get(s, "CeilingCoveringThickness").is_some()));
    // An asma tavan at 2.60 m: no ceiling thickness.
    let mut snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    let first = snap
        .entities
        .iter_mut()
        .find_map(|e| match e {
            Entity::Space(s) => Some(s),
            _ => None,
        })
        .unwrap();
    first.space.suspended_ceiling = Some(2.6);
    let name = first.space.number.clone();
    let doc = Document::from_snapshot_v2(snap).unwrap();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let room = b
        .spaces
        .iter()
        .find(|s| s.name.contains(name.as_deref().unwrap_or("?")))
        .expect("the room");
    assert_eq!(get(room, "CeilingCoveringThickness"), None);
    assert_eq!(get(room, "FloorCoveringThickness"), Some(Value::PositiveLength(0.02)));
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("IFCPOSITIVELENGTHMEASURE(0.02)"));
}

/// Dikme ve kayıt (docs/adr/8070 §8; EK-6 Tablo 6.40): east of the section building a 10/10 Çelik post 2.40 m tall
/// (its own height, not the storey's 3.00) and a 5/10 Çelik rail along y = 0 from x 20 to 26, its underside 1.00 m over
/// the floor. Worked out here: the post a prism from 0 to 2.40, the rail from 1.00 to 1.10, both on EK-2's Diğer
/// (MM-DIGR-CELIK). In the IFC model each an IfcMember, the post POST, the rail USERDEFINED "Kayıt", Çelik, outside
/// (`Pset_MemberCommon.IsExternal`), with no `Pset_ColumnCommon.LoadBearing`; neither left out, the frame's columns as
/// many as before.
#[test]
fn a_post_and_a_rail_are_ifc_members_of_their_types() {
    use kentos_commands::CallOp;
    use kentos_geometry_core::geom::section::Height;
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let (_, before) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"]["uids"][0].as_str().unwrap().to_owned()
    };
    let post = run(&mut doc, "cad.columns.create", serde_json::json!({"layerId": "0", "columns": [{
        "at": {"x": 20.0, "y": 2.0}, "width": 0.1, "depth": 0.1, "height": 2.4, "material": "Çelik",
        "member": {"predefined": "POST"}, "arch": {"level": "zemin"}
    }]}));
    let rail = run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [{"x": 20.0, "y": 0.0}, {"x": 26.0, "y": 0.0}], "width": 0.05, "height": 0.1, "baseOffset": 1.0,
        "material": "Çelik", "member": {"predefined": "USERDEFINED", "name": "Kayıt"}, "arch": {"level": "zemin"}
    }]}));
    let m = sections::model(&doc);
    let body = |uid: &str| {
        let i = (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(uid)).expect("its solid");
        let Height::Prism { bottom, top } = m.solids[i].height else {
            panic!("a prism")
        };
        (bottom, top, m.looks[i].layer.clone())
    };
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let (b0, t0, layer) = body(&post);
    assert!(near(b0, 0.0) && near(t0, 2.4), "{b0} {t0}");
    assert_eq!(layer, "MM-DIGR-CELIK");
    let (b1, t1, layer) = body(&rail);
    assert!(near(b1, 1.0) && near(t1, 1.1), "{b1} {t1}");
    assert_eq!(layer, "MM-DIGR-CELIK");
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new(), "every object written");
    assert_eq!((written.members, written.columns, written.beams), (2, before.columns, before.beams));
    let element = |uid: &str| {
        let id = kentos_domain::Uuid::parse_str(uid).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id)).expect("its element")
    };
    let p = element(&post);
    assert_eq!((p.class, p.predefined, p.object_type.as_deref()), (Class::Member, "POST", None));
    assert_eq!((p.name.as_str(), p.material.as_deref()), ("MM-ELM-Dikme", Some("Çelik")));
    let r = element(&rail);
    assert_eq!((r.class, r.predefined, r.object_type.as_deref()), (Class::Member, "USERDEFINED", Some("Kayıt")));
    assert_eq!(r.name, "MM-ELM-Kayıt");
    for e in [p, r] {
        let outside = e
            .psets
            .iter()
            .find(|s| s.name == "Pset_MemberCommon")
            .and_then(|s| s.props.iter().find(|(k, _)| k == "IsExternal"))
            .map(|(_, v)| v.clone());
        assert_eq!(outside, Some(Value::Bool(true)), "{}", e.name);
        assert!(!e.psets.iter().any(|s| s.name == "Pset_ColumnCommon"), "{}", e.name);
    }
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCMEMBER(").count(), 2);
    // `KENTOS_WRITE_IFC_MEMBERS=<path>` keeps the file for an outside reader (IfcOpenShell's validator).
    if let Some(path) = std::env::var_os("KENTOS_WRITE_IFC_MEMBERS") {
        std::fs::write(path, &text).expect("the file kept");
    }
}

/// B9's first acceptance (plan/bim-plan.md BIM-09; docs/adr/8070 §9): the section building with a ramp, a curtain wall,
/// the library's lift and a post and a rail. Its IFC model leaves nothing out; each new element has its body in the
/// section model (so the section, the elevation and the 3D view draw it) and its object in the file; the curtain wall is
/// a row of the doors and windows' schedule, the lift of the fixtures' and the post of the columns'.
#[test]
fn the_sample_building_with_a_ramp_a_curtain_wall_and_a_lift_leaves_nothing_out() {
    use kentos_commands::{CallOp, ScheduleOf};
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let uid = |answer: &serde_json::Value| {
        answer["output"]["uids"][0]
            .as_str()
            .or_else(|| answer["output"]["uid"].as_str())
            .unwrap()
            .to_owned()
    };
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let ramp = uid(&run(&mut doc, "cad.ramps.create", serde_json::json!({"layerId": "0", "ramps": [{
        "pts": [p(30.0, 0.0), p(36.0, 0.0), p(36.0, 4.0)], "width": 1.5, "rise": 0.75, "arch": {"level": "zemin"}
    }]})));
    let curtain = uid(&run(&mut doc, "cad.walls.create", serde_json::json!({"layerId": "0", "walls": [{
        "pts": [p(20.0, -3.0), p(26.0, -3.0)], "width": 0.15, "height": 3.0, "arch": {"level": "zemin"},
        "curtain": {"spacing": 1.5, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}
    }]})));
    let library = include_str!("../../../../../catalog/v1/blocks/kentos-temel.kblok");
    run(&mut doc, "cad.blocks.edit", serde_json::json!({"operation": "import", "library": library}));
    let lift = uid(&run(&mut doc, "cad.blocks.insert", serde_json::json!({
        "name": "Asansör", "at": p(5.0, 6.0), "layerId": "0", "arch": {"level": "zemin"},
        "attributes": {"KISI": "8", "YUK": "630"}
    })));
    let post = uid(&run(&mut doc, "cad.columns.create", serde_json::json!({"layerId": "0", "columns": [{
        "at": p(28.0, 2.0), "width": 0.1, "height": 2.4, "material": "Çelik", "member": {"predefined": "POST"},
        "arch": {"level": "zemin"}
    }]})));
    let rail = uid(&run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [p(27.0, 0.0), p(29.0, 0.0)], "width": 0.05, "height": 0.1, "baseOffset": 1.0, "material": "Çelik",
        "member": {"predefined": "USERDEFINED", "name": "Kayıt"}, "arch": {"level": "zemin"}
    }]})));
    // Every new element's body in the section model.
    let m = sections::model(&doc);
    for (what, u) in [("rampa", &ramp), ("giydirme cephe", &curtain), ("asansör", &lift), ("dikme", &post), ("kayıt", &rail)] {
        assert!(m.owners.iter().any(|o| o.as_deref() == Some(u.as_str())), "{what} kesit modelinde yok");
    }
    // The IFC model: nothing left out, each its class.
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new(), "the loss list is empty");
    let class_of = |u: &str| {
        let id = kentos_domain::Uuid::parse_str(u).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id)).map(|e| e.class)
    };
    assert_eq!(class_of(&ramp), Some(Class::Ramp));
    assert_eq!(class_of(&curtain), Some(Class::CurtainWall));
    assert_eq!(class_of(&lift), Some(Class::TransportElement));
    assert_eq!(class_of(&post), Some(Class::Member));
    assert_eq!(class_of(&rail), Some(Class::Member));
    // The schedules: the curtain wall among the doors and windows, the lift among the fixtures, the post among the
    // columns.
    let listed = |of: ScheduleOf, u: &str| {
        doc.schedule_rows(of, &of.default_columns(), None, &[], false, false)
            .members
            .iter()
            .any(|row| row.iter().any(|m| m == u))
    };
    assert!(listed(ScheduleOf::Openings, &curtain), "the curtain wall in the doors and windows' schedule");
    assert!(listed(ScheduleOf::Fixtures, &lift), "the lift in the fixtures' schedule");
    assert!(listed(ScheduleOf::Columns, &post), "the post in the columns' schedule");
}

/// Kaplama (docs/adr/8070 §10; EK-6 Tablo 6.20): an asma tavan drawn from one of the section building's rooms (bound to
/// it, 12.5 mm Alçıpan, its top 2.60 over its storey's floor) and a floor covering drawn by hand. Worked out here: the
/// ceiling a plate from 2.5875 to 2.60 on EK-2's Asma Tavan (MM-ASTV-ALCIPAN), its outline the room's. Neither is a floor
/// of the building: the walls' heights and the rooms' ceilings stay as they were. In the IFC model an IfcCovering
/// CEILING named MM-KPL-Asma_tavan and an IfcCovering FLOORING; no IfcSlab more; nothing left out.
#[test]
fn a_ceiling_from_a_room_and_a_floor_covering_are_ifc_coverings() {
    use kentos_commands::CallOp;
    use kentos_geometry_core::geom::section::Height;
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let (room, room_fields) = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Space(s) if !s.space.void && s.space.arch.level.as_deref() == Some("zemin") => {
                Some((u.to_string(), s.space.clone()))
            }
            _ => None,
        })
        .expect("a ground storey room");
    let walls_before: Vec<Option<f64>> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => Some(doc.wall_height(&w.wall)),
            _ => None,
        })
        .collect();
    let ceiling_before = doc.room_height(&room_fields).map(|h| h.ceiling);
    let (_, before) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    let run = |doc: &mut Document, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), "cad.slabs.create", CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"]["uids"][0].as_str().unwrap().to_owned()
    };
    let ceiling = run(&mut doc, serde_json::json!({"layerId": "0", "slabs": [{
        "space": room, "thickness": 0.0125, "offset": 2.6, "material": "Alçıpan",
        "arch": {"level": "zemin"}, "covering": {"predefined": "CEILING", "name": "Asma tavan"}
    }]}));
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let floor = run(&mut doc, serde_json::json!({"layerId": "0", "slabs": [{
        "parts": [{"outer": {"pts": [p(20.0, 0.0), p(24.0, 0.0), p(24.0, 3.0), p(20.0, 3.0)]}}],
        "thickness": 0.02, "offset": 0.02, "arch": {"level": "zemin"}, "covering": {"predefined": "FLOORING"}
    }]}));
    // Not a floor of the building.
    let walls_after: Vec<Option<f64>> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => Some(doc.wall_height(&w.wall)),
            _ => None,
        })
        .collect();
    assert_eq!(walls_after, walls_before, "the walls do not stop at a covering");
    assert_eq!(doc.room_height(&room_fields).map(|h| h.ceiling), ceiling_before, "nor the room's ceiling");
    // Its plate in the section model.
    let m = sections::model(&doc);
    let i = (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(ceiling.as_str())).expect("its plate");
    let Height::Prism { bottom, top } = m.solids[i].height else {
        panic!("a prism")
    };
    assert!((bottom - 2.5875).abs() < 1e-9 && (top - 2.6).abs() < 1e-9, "{bottom} {top}");
    assert_eq!(m.looks[i].layer, "MM-ASTV-ALCIPAN");
    assert!(!m.looks[i].heavy);
    // The IFC model.
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new());
    assert_eq!((written.coverings, written.slabs), (2, before.slabs));
    let element = |uid: &str| {
        let id = kentos_domain::Uuid::parse_str(uid).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id)).expect("its element")
    };
    let c = element(&ceiling);
    assert_eq!((c.class, c.predefined, c.name.as_str()), (Class::Covering, "CEILING", "MM-KPL-Asma_tavan"));
    assert_eq!(c.material.as_deref(), Some("Alçıpan"));
    let f = element(&floor);
    assert_eq!((f.class, f.predefined, f.name.as_str()), (Class::Covering, "FLOORING", "MM-KPL-Zemin_kaplaması"));
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCCOVERING(").count(), 2);
    if let Some(path) = std::env::var_os("KENTOS_WRITE_IFC_COVERINGS") {
        std::fs::write(path, &text).expect("the file kept");
    }
}
