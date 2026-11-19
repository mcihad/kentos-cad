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

/// The usages the file writes (docs/adr/8166): one for each set and top among the elements laid from their top.
fn usages(b: &kentos_ifc::building::Building) -> usize {
    let mut seen: Vec<(String, String, u64)> = Vec::new();
    for (k, top) in &b.layer_usages {
        let Some((_, name, layers)) = b.layer_sets.iter().find(|(i, ..)| i == k) else { continue };
        let key = (name.clone(), format!("{layers:?}"), top.to_bits());
        if !seen.contains(&key) {
            seen.push(key);
        }
    }
    seen.len()
}

/// A layered slab (docs/adr/8166; IFC-04): its build-up as a layer set from the top, laid down from its top over its
/// storey, in place of its one material. The section building's ground floor (on the ground: kaplama 2, şap 5, ısı
/// yalıtımı 5, su yalıtımı 1 over 15 cm of Betonarme): Kaplama an inner finish, Isı yalıtımı insulating, the structure
/// load bearing; every layered slab's layers add up to its whole thickness.
#[test]
fn a_layered_slab_is_its_build_up_laid_from_its_top() {
    let doc = building();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let mut layered = 0;
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Slab(sl) = e else { continue };
        let id = uid.as_u128();
        let at = b.elements.iter().position(|el| el.id == Some(id)).expect("its element");
        let set = b.layer_sets.iter().find(|(k, ..)| *k == at);
        if sl.slab.layers.is_empty() {
            assert!(set.is_none(), "a plain slab keeps its one material");
            continue;
        }
        layered += 1;
        let (_, name, layers) = set.expect("a layered slab's set");
        assert_eq!(name, "Döşeme katmanları");
        let total: f64 = layers.iter().map(|l| l.thickness).sum();
        assert!((total - sl.slab.whole_thickness()).abs() < 1e-9, "{total} {}", sl.slab.whole_thickness());
        let structure = &layers[sl.slab.layers.over.len()];
        assert_eq!(
            (structure.name.as_str(), structure.material.as_deref(), structure.thickness, structure.category),
            ("Taşıyıcı", sl.slab.material.as_deref(), sl.slab.thickness, Some("LoadBearing"))
        );
        assert_eq!(b.layer_usages.iter().find(|(k, _)| *k == at).map(|(_, t)| *t), Some(sl.slab.offset));
        if sl.slab.arch.level.as_deref() == Some("zemin") && sl.slab.layers.under.is_empty() {
            let rows: Vec<(&str, f64, Option<&str>)> =
                layers.iter().map(|l| (l.name.as_str(), l.thickness, l.category)).collect();
            assert_eq!(
                rows,
                [
                    ("Kaplama", 0.02, Some("Inner finish")),
                    ("Şap", 0.05, None),
                    ("Isı yalıtımı", 0.05, Some("Insulation")),
                    ("Su yalıtımı", 0.01, None),
                    ("Taşıyıcı", 0.15, Some("LoadBearing")),
                ]
            );
        }
    }
    assert!(layered > 0, "the section building's slabs are layered");
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCMATERIALLAYERSETUSAGE(").count(), usages(&b));
    let laid = |l: &&str| l.contains(",.AXIS3.,.NEGATIVE.,");
    assert!(text.lines().filter(|l| l.contains("IFCMATERIALLAYERSETUSAGE(")).all(|l| laid(&l)));
}

/// An element rising over its storey (docs/adr/8168): the section building's east wall drawn once over both storeys
/// and on over the roof, as a party wall stands (the ground storey's raised to 6.6 m by Değiştir, the first storey's
/// and the two east beams taken away, the 1. kat's floor and the terrace cut back to the wall's inner face by Kes), is
/// contained in the ground storey and referenced in the first; none of the building's other walls and columns (built
/// up to what is over them, or standing on the structure under the finished floor) spans a storey.
/// `KENTOS_WRITE_IFC_BUILDING=1` keeps fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc, which
/// `scripts/fixtures/storey_spans_check.py` reads with IfcOpenShell.
#[test]
fn a_wall_two_storeys_tall_spans_the_storey_over_its_own() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let east = |pts: &[kentos_commands::Vec2]| {
        pts.first().is_some_and(|p| p.x == 12.0 && p.y == 0.0) && pts.last().is_some_and(|q| q.x == 12.0 && q.y == 9.0)
    };
    let (mut tall, mut gone, mut slabs) = (None, Vec::new(), Vec::new());
    for (u, e) in doc.entities_with_uids() {
        match e {
            Entity::Wall(w) if east(&w.wall.pts) && w.wall.arch.level.as_deref() == Some("zemin") => tall = Some(u),
            Entity::Wall(w) if east(&w.wall.pts) => gone.push(u.to_string()),
            Entity::Beam(b) if east(&b.beam.pts) => gone.push(u.to_string()),
            Entity::Slab(s) if s.slab.arch.level.as_deref() == Some("k1") => slabs.push(u.to_string()),
            _ => {}
        }
    }
    let uid = tall.expect("the ground storey's east wall").to_string();
    assert_eq!((gone.len(), slabs.len()), (3, 2), "the first storey's east wall and both east beams; the two slabs");
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let strip = serde_json::json!({"outer": {"pts": [p(11.875, -0.5), p(12.5, -0.5), p(12.5, 9.5), p(11.875, 9.5)]}});
    for (command, input) in [
        ("cad.walls.edit", serde_json::json!({"uids": [uid], "operation": {"kind": "set", "height": 6.6}})),
        ("cad.entities.delete", serde_json::json!({"uids": gone})),
        ("cad.slabs.edit", serde_json::json!({"uids": slabs, "operation": {"kind": "subtract", "parts": [strip]}})),
    ] {
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), command, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{command}: {answer}");
    }
    let (b, written) = arch_ifc::building(&doc, "2026-11-19T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new());
    let text = kentos_ifc::building::write(&b);
    let id = |l: &str| l.split('=').next().unwrap().to_owned();
    let wall = id(text.lines().find(|l| l.contains("IFCWALL(") && l.contains(&uid)).expect("the tall wall"));
    let storey = |name: &str| text.lines().find(|l| l.contains("IFCBUILDINGSTOREY(") && l.contains(name)).map(id);
    let (upper, lower) = (storey("1._kat").expect("the 1. kat"), storey("Zemin").expect("the ground"));
    // A relation's related objects: `(..,$,$,$,(#a,#b),#storey);`.
    let list = |l: &str| l.split("$,$,$,(").nth(1).and_then(|x| x.split(')').next()).unwrap_or("").to_owned();
    let refs: Vec<&str> = text.lines().filter(|l| l.contains("IFCRELREFERENCEDINSPATIALSTRUCTURE(")).collect();
    assert!(refs.iter().any(|l| l.ends_with(&format!("({wall}),{upper});"))), "{refs:?}");
    let contained = text
        .lines()
        .filter(|l| l.contains("IFCRELCONTAINEDINSPATIALSTRUCTURE("))
        .find(|l| list(l).split(',').any(|x| x == wall))
        .expect("the tall wall's storey");
    assert!(contained.ends_with(&format!(",{lower});")), "{contained}");
    // The building's other walls and columns span none: every reference besides the tall wall's is a zone's.
    let walls_and_columns: Vec<String> = text
        .lines()
        .filter(|l| (l.contains("IFCWALL(") || l.contains("IFCCOLUMN(")) && !l.contains(&uid))
        .map(|l| id(l))
        .collect();
    assert!(!walls_and_columns.is_empty());
    for r in &refs {
        assert!(list(r).split(',').all(|x| !walls_and_columns.iter().any(|w| w == x)), "{r}");
    }
    if std::env::var_os("KENTOS_WRITE_IFC_BUILDING").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc");
        std::fs::write(out, &text).expect("the file kept");
    }
}

/// A curved grid system beside the building (docs/adr/8169), drawn by Eğrisel aks ağı from the east façade's middle:
/// four radial axes (4 to 7, from −45° by 30° turns) and two arcs (E at 6 m, F at 10 m), each 1 m past the others. It
/// is one RADIAL grid of the building, U its radial axes counter-clockwise and V its arcs from the centre out, each arc
/// a circle about the centre trimmed at its ends. The building's own axes (2 and D pass through the centre) stay their
/// rectangular grid. `KENTOS_WRITE_IFC_RADIAL=<path>` keeps the file for `scripts/fixtures/radial_grid_check.py`.
#[test]
fn a_curved_grid_system_is_a_radial_grid() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let turn = 30f64.to_radians();
    let input = serde_json::json!({"layerId": "aks", "radial": {
        "center": {"x": 12.0, "y": 4.5}, "angle": -45f64.to_radians(), "turns": [turn, turn, turn],
        "radius": 6.0, "rings": [4.0], "radialStart": "4", "arcStart": "E", "extension": 1.0}});
    let answer =
        registry::run(&mut ExecutionContext::new(&mut doc), "cad.grids.create", CallOp::Execute, input).unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let (b, written) = arch_ifc::building(&doc, "2026-11-19T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new());
    assert_eq!(written.grid_axes, 13);
    let tags = |list: &[kentos_ifc::building::Axis]| list.iter().map(|a| a.tag.clone()).collect::<Vec<_>>();
    let said: Vec<(&str, Vec<String>, Vec<String>)> =
        b.grids.iter().map(|g| (g.predefined, tags(&g.u), tags(&g.v))).collect();
    let words = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        said,
        [
            ("RECTANGULAR", words(&["A", "B", "C", "D"]), words(&["1", "2", "3"])),
            ("RADIAL", words(&["4", "5", "6", "7"]), words(&["E", "F"])),
        ]
    );
    assert_eq!((b.grids[0].storey, b.grids[1].storey), (None, None));
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCGRID(").count(), 2);
    assert_eq!(text.matches("IFCTRIMMEDCURVE(").count(), 2);
    // Each axis's curve as the grid's footprint applies them: the straight ones polylines, the arcs trimmed circles.
    let line_of = |id: &str| text.lines().find(|l| l.starts_with(&format!("{id}="))).unwrap_or_default();
    for axis in text.lines().filter(|l| l.contains("IFCGRIDAXIS(")) {
        let curve = axis.split(",#").nth(1).and_then(|x| x.split(',').next()).expect("its curve");
        let kind = line_of(&format!("#{curve}"));
        assert!(kind.contains("=IFCPOLYLINE(") || kind.contains("=IFCTRIMMEDCURVE("), "{axis} {kind}");
    }
    if let Some(path) = std::env::var_os("KENTOS_WRITE_IFC_RADIAL") {
        std::fs::write(path, &text).expect("the file kept");
    }
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

/// The fire compartments a caller gives (docs/adr/8088), here the section building's two storeys' rooms each: two
/// zones, `MM-ZON-Yangın_kompartımanı_1` and `_2`, their object type “Yangın kompartımanı”, each grouping its storey's
/// five rooms. `KENTOS_WRITE_IFC_BUILDING=1` keeps fixtures/arch/v1/ifc/kesit-binasi-kompartiman.ifc, which
/// `scripts/fixtures/arch_ifc_check.py` reads with IfcOpenShell.
#[test]
fn the_fire_compartments_are_zones_of_their_rooms() {
    use kentos_commands::{CallOp, IfcCompartment};
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let levels: Vec<String> =
        doc.settings().arch.as_ref().expect("Mimari").levels.iter().map(|l| l.id.clone()).collect();
    let compartments: Vec<IfcCompartment> = levels
        .iter()
        .map(|level| IfcCompartment {
            rooms: doc
                .entities_with_uids()
                .filter(|(_, e)| matches!(e, Entity::Space(s) if s.space.arch.level.as_deref() == Some(level.as_str())))
                .map(|(u, _)| u.to_string())
                .collect(),
        })
        .collect();
    assert_eq!(compartments.iter().map(|c| c.rooms.len()).collect::<Vec<_>>(), [5, 5]);
    let keep = std::env::var_os("KENTOS_WRITE_IFC_BUILDING").is_some();
    let path = if keep {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc/kesit-binasi-kompartiman.ifc")
    } else {
        std::env::temp_dir().join(format!("kentos-kompartiman-{}.ifc", std::process::id()))
    };
    let input = serde_json::json!({
        "path": path.display().to_string(), "stamp": "2026-11-11T12:00:00", "overwrite": true,
        "compartments": compartments,
    });
    let answer = registry::run(&mut ExecutionContext::new(&mut doc), "cad.arch.ifc", CallOp::Execute, input).unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(answer["output"]["compartments"], 2, "{answer}");
    let text = std::fs::read_to_string(&path).expect("the file");
    if !keep {
        let _ = std::fs::remove_file(&path);
    }
    // STEP writes “ı” as \X2\0131\X0\: the names end in their numbers.
    let zones: Vec<&str> = text.lines().filter(|l| l.contains("IFCZONE(") && l.contains("'MM-ZON-Yang")).collect();
    assert_eq!(zones.len(), 2, "{zones:#?}");
    assert!(zones[0].contains("_1'") && zones[1].contains("_2'"), "{zones:#?}");
    // Each zone groups five spaces.
    for z in &zones {
        let id = z.split('=').next().expect("its instance").trim();
        let rel = text
            .lines()
            .find(|l| l.contains("IFCRELASSIGNSTOGROUP(") && l.trim_end().ends_with(&format!(",{id});")))
            .unwrap_or_else(|| panic!("{id}'s members"));
        let members = rel.split("),$,").next().and_then(|x| x.rsplit('(').next()).expect("the list");
        assert_eq!(members.split(',').count(), 5, "{rel}");
    }
}

/// A wall declared load-bearing (docs/adr/8090) writes `Pset_WallCommon.LoadBearing` true, one declared not false; a
/// wall with no declaration writes none (EK-6 Tablo 6.133: optional, never guessed).
#[test]
fn a_wall_s_declared_load_bearing_role_is_its_common_set_s() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let walls: Vec<(u128, String)> = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Wall(_)))
        .map(|(u, _)| (u.as_u128(), u.to_string()))
        .take(2)
        .collect();
    for ((_, uid), bearing) in walls.iter().zip([true, false]) {
        let input = serde_json::json!({"operation": {"kind": "objects", "uids": [uid],
            "loadBearing": {"bearing": bearing, "basis": {"kind": "author"}}}});
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), "cad.arch.performance", CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let bearing = |id: u128| {
        let e = b.elements.iter().find(|e| e.id == Some(id)).expect("the wall");
        e.psets
            .iter()
            .filter(|p| p.name == "Pset_WallCommon")
            .flat_map(|p| p.props.iter())
            .find(|(n, _)| n == "LoadBearing")
            .map(|(_, v)| v.clone())
    };
    assert_eq!(bearing(walls[0].0), Some(Value::Bool(true)));
    assert_eq!(bearing(walls[1].0), Some(Value::Bool(false)));
    let undeclared = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Wall(_)))
        .map(|(u, _)| u.as_u128())
        .nth(2)
        .expect("a third wall");
    assert_eq!(bearing(undeclared), None);
}

/// A wall style declared load-bearing (docs/adr/8137) writes `Pset_WallCommon.LoadBearing` true on its walls that
/// declare nothing of their own; a wall of the style declared not load-bearing keeps its false.
#[test]
fn a_wall_style_s_load_bearing_role_is_its_walls_common_set_s() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let styled: Vec<(u128, String, String)> = doc
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Wall(w) => w.wall.style.clone().map(|s| (u.as_u128(), u.to_string(), s)),
            _ => None,
        })
        .collect();
    let style = styled.first().map(|x| x.2.clone()).expect("a wall with a style");
    let mine: Vec<&(u128, String, String)> = styled.iter().filter(|x| x.2 == style).collect();
    assert!(mine.len() >= 2, "two walls of one style: {mine:?}");
    let mut run = |input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), "cad.arch.performance", CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    run(serde_json::json!({"operation": {"kind": "wallStyle", "style": style,
        "loadBearing": {"bearing": true, "basis": {"kind": "author"}}}}));
    run(serde_json::json!({"operation": {"kind": "objects", "uids": [mine[1].1],
        "loadBearing": {"bearing": false, "basis": {"kind": "author"}}}}));
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let bearing = |id: u128| {
        let e = b.elements.iter().find(|e| e.id == Some(id)).expect("the wall");
        e.psets
            .iter()
            .filter(|p| p.name == "Pset_WallCommon")
            .flat_map(|p| p.props.iter())
            .find(|(n, _)| n == "LoadBearing")
            .map(|(_, v)| v.clone())
    };
    assert_eq!(bearing(mine[0].0), Some(Value::Bool(true)), "the style's");
    assert_eq!(bearing(mine[1].0), Some(Value::Bool(false)), "the wall's own");
}

/// The künye a permit set writes into the drawing's variables (docs/adr/8037), as an office fills it.
const KUNYE: [(&str, &str); 18] = [
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
    ("mimar_soyadi", "Yılmaz"),
    ("mimar_unvan", "Y. Mimar"),
    ("sicil_no", "41234"),
    ("buro_adi", "Örnek Mimarlık"),
    ("vergi_no", "1234567890"),
    ("buro_adres", "Atatürk Cad. No:1 Sivas"),
    // EK-7 Tablo 7.1 (docs/adr/8074 §3): the project's description, and the building kind its project type.
    ("proje_aciklama", "İki katlı konut, kesit örneği"),
    ("yapi_turu", "yeni bina"),
];

pub(crate) fn with_kunye() -> Document {
    Document::from_snapshot_v2(kunye_snapshot()).unwrap()
}

/// The section building's snapshot with its künye, its system and its ±0.00's elevation.
pub(crate) fn kunye_snapshot() -> DocumentSnapshotV2 {
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
    snap
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
    // EK-7 Tablo 7.1: the künye's description, the building kind a new build (IFC 4.3's PEnum_ProjectType).
    assert_eq!(b.project.description.as_deref(), Some("İki katlı konut, kesit örneği"));
    assert_eq!(
        b.project.psets.iter().find(|p| p.name == "Pset_ProjectCommon").map(|p| p.props.clone()),
        Some(vec![("ProjectType".to_owned(), Value::Label("NEWBUILD".into()))])
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
        Some(Value::Id("1234/5".into()))
    );
    assert_eq!(
        prop(&b.site.psets, "Pset_LandRegistration", "LandTitleID"),
        Some(Value::Id("12/345".into()))
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

/// Every standard property and quantity the section building's model writes, with its künye and its İmar durumu, is
/// one IFC 4.3's template has, of the type it asks (docs/adr/8074 §5, 8075: the profile's IDS checks the type; an
/// independent checker found the land registration's identifiers written as labels, the form the footprint misspelt).
#[test]
fn every_standard_property_has_its_template_s_type() {
    // With the parcel's İmar durumu too: the building's footprint and gross area are written then.
    let mut zoned = with_kunye();
    let mut settings = zoned.settings().clone();
    settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    zoned.set_settings(settings);
    for doc in [with_kunye(), building(), zoned] {
        let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
        assert_eq!(kentos_ifc::catalog_schema::mistyped(&b), Vec::new());
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
        [kentos_ifc::building::Shape::Prism { rings, bottom, top, .. }] => {
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

/// Eğimli eleman (docs/adr/8130): a 5/10 Ahşap rafter east of the building, from x 20 to 24 along y 0, its underside
/// 2.50 m over the ground storey's floor at its start, rising 3 m to its end: a 3–4–5 triangle, its grade 3/4, its
/// length 5 m. Its depth across its axis is a level one's, so upright it is 0.10 × 5/4 = 0.125 deep: its top 2.625 at
/// x 20 and 5.625 at x 24. A section across it at x 22 cuts it 5 cm wide from 4.00 to 4.125. In the IFC model it is an
/// IfcMember RAFTER whose Pset_MemberCommon says its slope, atan 3/4 (36.87°), its body the sloped solid. Worked by
/// hand. A frame beam takes no rise, a bent member neither; 0 levels the rafter again.
#[test]
fn a_sloped_rafter_rises_along_its_axis_and_says_its_slope() {
    use kentos_commands::CallOp;
    use kentos_geometry_core::Vec2 as P;
    use kentos_geometry_core::geom::section::{self as sec, Frame, Height};
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| {
        registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap()
    };
    let answer = run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [{"x": 20.0, "y": 0.0}, {"x": 24.0, "y": 0.0}], "width": 0.05, "height": 0.1, "baseOffset": 2.5,
        "material": "Ahşap", "member": {"predefined": "RAFTER"}, "rise": 3.0, "arch": {"level": "zemin"}
    }]}));
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = answer["output"]["uids"][0].as_str().unwrap().to_owned();
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let m = sections::model(&doc);
    let i = (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(uid.as_str())).expect("its solid");
    let Height::Sloped(s) = &m.solids[i].height else { panic!("{:?}", m.solids[i].height) };
    assert!(near(s.dx, 0.75) && near(s.dy, 0.0) && near(s.thickness, 0.125), "{s:?}");
    assert!(near(s.z(P::new(20.0, 0.0)), 2.625) && near(s.z(P::new(24.0, 0.0)), 5.625), "{s:?}");
    let frame = Frame::new(P::new(22.0, -1.0), P::new(22.0, 1.0), true, None).unwrap();
    let cut = sec::section(&frame, &m.solids);
    let piece = cut.cut.iter().find(|c| c.solid == i).expect("cut across");
    let zs: Vec<f64> = piece.ring.iter().map(|q| q.y).collect();
    let (z0, z1) = (zs.iter().copied().fold(f64::INFINITY, f64::min), zs.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    assert!(near(z0, 4.0) && near(z1, 4.125), "{zs:?}");
    // The IFC model.
    let (b, written) = arch_ifc::building(&doc, "2026-11-18T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new(), "every object written");
    let id = kentos_domain::Uuid::parse_str(&uid).unwrap().as_u128();
    let e = b.elements.iter().find(|e| e.id == Some(id)).expect("its element");
    assert_eq!((e.class, e.predefined), (Class::Member, "RAFTER"));
    let slope = e
        .psets
        .iter()
        .find(|p| p.name == "Pset_MemberCommon")
        .and_then(|p| p.props.iter().find(|(k, _)| k == "Slope"))
        .map(|(_, v)| v.clone());
    assert_eq!(slope, Some(Value::PlaneAngle(0.75f64.atan())));
    assert!(matches!(e.shapes.as_slice(), [kentos_ifc::building::Shape::Faces { .. }]), "{:?}", e.shapes);
    // A frame beam takes no rise; nor does a bent member; 0 levels it.
    let frame_beam = run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [{"x": 20.0, "y": 3.0}, {"x": 24.0, "y": 3.0}], "width": 0.25, "height": 0.5, "rise": 1.0
    }]}));
    assert_eq!((frame_beam["error"]["code"].as_str(), frame_beam["error"]["path"].as_str()), (Some("invalid_beam"), Some("beams[0].rise")));
    let bent = run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [{"x": 20.0, "y": 3.0}, {"x": 24.0, "y": 3.0}, {"x": 24.0, "y": 5.0}], "width": 0.05, "height": 0.1,
        "member": {"predefined": "BRACE"}, "rise": 1.0
    }]}));
    assert_eq!(bent["error"]["code"], "invalid_beam", "{bent}");
    let level = run(&mut doc, "cad.arch.edit", serde_json::json!({"uids": [uid], "operation": {"kind": "set", "rise": 0.0}}));
    assert_eq!(level["status"], "completed", "{level}");
    let slot = doc.slot_of(kentos_domain::Uuid::parse_str(&uid).unwrap()).unwrap();
    let Some(kentos_commands::Entity::Beam(beam)) = doc.get(slot) else { panic!("a beam") };
    assert_eq!(beam.beam.rise, None);
}

/// B9's first acceptance (plan/bim-plan.md BIM-09; docs/adr/8070 §9, §11): the section building with a ramp, a curtain
/// wall, the library's lift, a post, a rail and an asma tavan. Its IFC model leaves nothing out; each new element has its
/// body in the section model (so the section, the elevation and the 3D view draw it) and its object in the file; the
/// curtain wall is a row of the doors and windows' schedule, the lift of the fixtures', the post of the columns', the
/// rail of the beams' and the asma tavan of the coverings'.
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
    let ceiling = uid(&run(&mut doc, "cad.slabs.create", serde_json::json!({"layerId": "0", "slabs": [{
        "parts": [{"outer": {"pts": [p(1.0, 1.0), p(4.0, 1.0), p(4.0, 4.0), p(1.0, 4.0)]}}], "thickness": 0.0125,
        "offset": 2.6, "material": "Alçıpan", "arch": {"level": "zemin"},
        "covering": {"predefined": "CEILING", "name": "Asma tavan"}
    }]})));
    // Every new element's body in the section model.
    let m = sections::model(&doc);
    for (what, u) in [
        ("rampa", &ramp),
        ("giydirme cephe", &curtain),
        ("asansör", &lift),
        ("dikme", &post),
        ("kayıt", &rail),
        ("asma tavan", &ceiling),
    ] {
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
    assert_eq!(class_of(&ceiling), Some(Class::Covering));
    // Every body closed (docs/adr/8071 §1): the ramp's faces, the stair's treads, the curtain wall's frames and panels,
    // the lift's box, the walls, slabs, columns and beams.
    for e in &b.elements {
        for s in &e.shapes {
            assert_eq!(kentos_ifc::closed::faults(s), Vec::<String>::new(), "{} {s:?}", e.name);
        }
    }
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
    assert!(listed(ScheduleOf::Beams, &rail), "the rail in the beams' schedule");
    assert!(listed(ScheduleOf::Coverings, &ceiling), "the asma tavan in the coverings' schedule");
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
    // The ceiling bound to its room covers it (docs/adr/8127, `IfcRelCoversSpaces`); the floor covering drawn beside
    // the building stands in no room and covers none.
    let ceiling_at = b.elements.iter().position(|e| e.id == element(&ceiling).id).expect("the ceiling");
    let room_at = b.spaces.iter().position(|sp| sp.id.map(|i| kentos_domain::Uuid::from_u128(i).to_string()) == Some(room.clone()));
    assert_eq!(b.covers, vec![(room_at.expect("the room"), vec![ceiling_at])]);
    assert_eq!(text.matches("IFCRELCOVERSSPACES(").count(), 1);
    // Each covering's plate is a layer set of one, its words, material and thickness (docs/adr/8166; EK-6 Tablo 6.20's
    // Kaplama Malzemesi and Kaplama Kalınlığı), laid down from its top: the ceiling's 2.60 m, the floor's 0.02 m.
    let set_of = |at: usize| b.layer_sets.iter().find(|(k, ..)| *k == at).map(|(_, n, l)| (n.clone(), l.clone()));
    let floor_at = b.elements.iter().position(|e| e.id == element(&floor).id).expect("the floor covering");
    let (name, layers) = set_of(ceiling_at).expect("the ceiling's set");
    assert_eq!(name, "Kaplama katmanları");
    let plate: Vec<(&str, Option<&str>, f64, Option<&str>)> =
        layers.iter().map(|l| (l.name.as_str(), l.material.as_deref(), l.thickness, l.category)).collect();
    assert_eq!(plate, [("Asma tavan", Some("Alçıpan"), 0.0125, None)]);
    let (_, layers) = set_of(floor_at).expect("the floor covering's set");
    assert_eq!(layers.iter().map(|l| (l.name.as_str(), l.thickness)).collect::<Vec<_>>(), [("Zemin kaplaması", 0.02)]);
    assert_eq!(b.layer_usages.iter().find(|(k, _)| *k == ceiling_at).map(|(_, t)| *t), Some(2.6));
    assert_eq!(b.layer_usages.iter().find(|(k, _)| *k == floor_at).map(|(_, t)| *t), Some(0.02));
    assert!(usages(&b) >= 2);
    assert_eq!(text.matches("IFCMATERIALLAYERSETUSAGE(").count(), usages(&b), "one for each set and top");
    if let Some(path) = std::env::var_os("KENTOS_WRITE_IFC_COVERINGS") {
        std::fs::write(path, &text).expect("the file kept");
    }
}

/// Kiriş çizelgesi and Kaplama çizelgesi (docs/adr/8070 §11): placed in the section building, each lists its own (its
/// beams and its rail; its covering, not its floors) and the drawing is written with architecture data 6 and read back
/// as it was.
#[test]
fn beam_and_covering_schedules_list_their_own_and_round_trip_under_arch_data_6() {
    use kentos_commands::{CallOp, ScheduleOf};
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let rail = run(&mut doc, "cad.beams.create", serde_json::json!({"layerId": "0", "beams": [{
        "pts": [p(27.0, 0.0), p(29.0, 0.0)], "width": 0.05, "height": 0.1, "baseOffset": 1.0,
        "member": {"predefined": "USERDEFINED", "name": "Kayıt"}, "arch": {"level": "zemin"}
    }]}))["output"]["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let covering = run(&mut doc, "cad.slabs.create", serde_json::json!({"layerId": "0", "slabs": [{
        "parts": [{"outer": {"pts": [p(20.0, 0.0), p(24.0, 0.0), p(24.0, 3.0), p(20.0, 3.0)]}}],
        "thickness": 0.02, "offset": 0.02, "covering": {"predefined": "FLOORING"}
    }]}))["output"]["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let beams = doc.schedule_rows(ScheduleOf::Beams, &ScheduleOf::Beams.default_columns(), None, &[], false, false);
    assert!(beams.members.iter().any(|r| r.contains(&rail)), "the rail among the beams");
    assert!(beams.members.len() > 1, "the frame's beams too");
    let coverings =
        doc.schedule_rows(ScheduleOf::Coverings, &ScheduleOf::Coverings.default_columns(), None, &[], false, false);
    assert_eq!(coverings.members, vec![vec![covering]], "the covering alone, no floor");
    run(&mut doc, "cad.schedules.create", serde_json::json!({"layerId": "0", "at": p(0.0, -20.0), "of": "beams"}));
    let snap = doc.to_snapshot_v2();
    assert_eq!(kentos_kcad::arch_version(&snap), kentos_kcad::ARCH_WITH_ELEMENT_SCHEDULES);
    let bytes = kentos_kcad::encode(&snap).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), snap, "the schedule read back");
}

/// Geometri doğruluğu (docs/adr/8071; plan/bim-plan.md BIM-10's first acceptance) on the curved building
/// (fixtures/arch/v1/curved-building.kcad, scripts/fixtures/curved_building.py): its north wall an arc, two storeys of
/// different heights at different levels. Against the closed forms the fixture works out without KentOS code
/// (curved-building.json): every wall's and slab's base quantities, its body's bottom and top over ±0.00, each room's
/// net floor area, the spiral stair's treads and post. The curved wall's, the slabs' and the stair's arcs are written as
/// arcs (no chords: nothing said about them), the window through the curved wall runs along its faces' arcs, and every
/// body is closed.
/// The cut body of every wall (its Reference View's) is its net volume. `KENTOS_WRITE_IFC_CURVED=1` writes
/// fixtures/arch/v1/ifc/egri-bina.ifc (Design Transfer) and egri-bina-referans.ifc (Reference View), which
/// `scripts/fixtures/curved_ifc_check.py` reads with IfcOpenShell against the same closed forms.
#[test]
fn the_curved_building_s_bodies_are_their_closed_forms() {
    use kentos_commands::CallOp;
    use kentos_ifc::building::Shape;
    use kentos_native_application::{ExecutionContext, registry};
    let snap: DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/curved-building.kcad")).unwrap();
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/arch/v1/curved-building.json")).unwrap();
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new(), "the loss list is empty");
    let element = |u: &str| {
        let id = kentos_domain::Uuid::parse_str(u).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id)).unwrap_or_else(|| panic!("{u} not written"))
    };
    let number = |v: &Value| match v {
        Value::Length(x) | Value::Area(x) | Value::Volume(x) => *x,
        other => panic!("{other:?}"),
    };
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    for (set, of) in [("walls", "Qto_WallBaseQuantities"), ("slabs", "Qto_SlabBaseQuantities")] {
        for (u, want) in expected[set].as_object().unwrap() {
            let e = element(u);
            let q = e.qtos.iter().find(|q| q.name == of).unwrap_or_else(|| panic!("{}: no {of}", e.name));
            for (k, v) in want.as_object().unwrap() {
                let got = q.quantities.iter().find(|(n, _)| n == k).map(|(_, v)| number(v));
                let v = v.as_f64().unwrap();
                assert!(got.is_some_and(|g| near(g, v)), "{} {k}: {got:?}, the closed form {v}; {:?}", e.name, e.shapes);
            }
        }
    }
    // Each body where its storey's level puts it.
    for (u, want) in expected["bodies"].as_object().unwrap() {
        let e = element(u);
        let floor = b.storeys[e.storey].elevation;
        let (low, high) = e.shapes.iter().fold((f64::MAX, f64::MIN), |(l, h), s| match s {
            Shape::Prism { bottom, top, .. } => (l.min(floor + bottom), h.max(floor + top)),
            Shape::Faces { .. } => (l, h),
        });
        let (bottom, top) = (want[0].as_f64().unwrap(), want[1].as_f64().unwrap());
        assert!(near(low, bottom) && near(high, top), "{}: {low} → {high}, not {bottom} → {top}", e.name);
    }
    // The rooms' quantities (docs/adr/8071 §7), the inner faces' arc measured as an arc, from floor to ceiling.
    for (u, want) in expected["spaces"].as_object().unwrap() {
        let id = kentos_domain::Uuid::parse_str(u).unwrap().as_u128();
        let s = b.spaces.iter().find(|s| s.id == Some(id)).expect("the room written");
        for (k, v) in want.as_object().unwrap() {
            let got = s.qtos.iter().flat_map(|q| &q.quantities).find(|(n, _)| n == k).map(|(_, v)| number(v));
            let v = v.as_f64().unwrap();
            assert!(got.is_some_and(|g| near(g, v)), "{} {k}: {got:?}, the closed form {v}", s.name);
        }
        assert!(near(s.top - s.bottom, want["Height"].as_f64().unwrap()), "{}: {} → {}", s.name, s.bottom, s.top);
        assert!(s.bulges.iter().any(|r| r.iter().any(|x| *x != 0.0)), "{}: its arc written as an arc", s.name);
    }
    // The spiral stair (docs/adr/0709, 8071): its treads pieces of a ring and its centre post a cylinder, their arcs
    // true; each one's bottom, top and plan area what the fixture works out, in the order it climbs, the post last.
    for (u, want) in expected["stairs"].as_object().unwrap() {
        let e = element(u);
        assert_eq!((e.class, e.predefined), (Class::Stair, "SPIRAL_STAIR"), "{}", e.name);
        let floor = b.storeys[e.storey].elevation;
        let got: Vec<[f64; 3]> = e
            .shapes
            .iter()
            .map(|s| match s {
                Shape::Prism { rings, bulges, bottom, top } => {
                    assert!(bulges.iter().flatten().any(|x| *x != 0.0), "{}: its arcs as arcs", e.name);
                    [floor + bottom, floor + top, kentos_ifc::quantities::area(rings, bulges)]
                }
                Shape::Faces { .. } => panic!("{}: a prism", e.name),
            })
            .collect();
        let mut wanted: Vec<[f64; 3]> = want["treads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| [0, 1, 2].map(|k| t[k].as_f64().unwrap()))
            .collect();
        wanted.push([0, 1, 2].map(|k| want["post"][k].as_f64().unwrap()));
        assert_eq!(got.len(), wanted.len(), "{}: its treads and its post", e.name);
        for (g, w) in got.iter().zip(&wanted) {
            assert!((0..3).all(|k| near(g[k], w[k])), "{}: {g:?}, the closed form {w:?}", e.name);
        }
        let v: f64 = e.shapes.iter().map(kentos_ifc::quantities::volume).sum();
        assert!(near(v, want["volume"].as_f64().unwrap()), "{}: {v}", e.name);
    }
    // Every body closed; the curved walls' and the slabs' with their arcs, the curved walls' windows too.
    for e in &b.elements {
        for s in &e.shapes {
            assert_eq!(kentos_ifc::closed::faults(s), Vec::<String>::new(), "{}", e.name);
        }
    }
    let curved: Vec<usize> = (0..b.elements.len())
        .filter(|&k| {
            b.elements[k].shapes.iter().any(|s| matches!(s, Shape::Prism { bulges, .. } if bulges.iter().flatten().any(|x| *x != 0.0)))
        })
        .collect();
    let classes: Vec<Class> = curved.iter().map(|&k| b.elements[k].class).collect();
    assert_eq!(classes.iter().filter(|c| **c == Class::Wall).count(), 2, "the two curved walls: {classes:?}");
    assert_eq!(classes.iter().filter(|c| **c == Class::Slab).count(), 3, "the three slabs: {classes:?}");
    assert_eq!(classes.iter().filter(|c| **c == Class::Stair).count(), 1, "the spiral stair: {classes:?}");
    let arched = b.openings.iter().filter(|o| o.bulges.iter().any(|x| *x != 0.0)).count();
    assert_eq!(arched, 2, "a window through each curved wall along its faces' arcs");
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("IFCARCINDEX(") && text.contains("IFCINDEXEDPOLYCURVE("));
    // Nothing said about chords: the arcs are written as arcs.
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.arch.ifc",
        CallOp::Plan,
        serde_json::json!({"path": "/tmp/egri-bina.ifc"}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let said = answer["warnings"].to_string();
    assert!(!said.contains("düz parçalarla"), "{said}");
    // The cut body (docs/adr/8071 §4): each wall less its openings is its net volume, each band closed, its arcs kept.
    for (u, want) in expected["walls"].as_object().unwrap() {
        let k = b.elements.iter().position(|e| e.id == Some(kentos_domain::Uuid::parse_str(u).unwrap().as_u128())).unwrap();
        let hosted: Vec<&kentos_ifc::building::Opening> = b.openings.iter().filter(|o| o.host == k).collect();
        let body = kentos_ifc::quantities::cut(&b.elements[k].shapes, &hosted);
        let v: f64 = body.iter().map(kentos_ifc::quantities::volume).sum();
        let net = want["NetVolume"].as_f64().unwrap();
        assert!(near(v, net), "{}: the cut body {v}, the closed form {net}", b.elements[k].name);
        for s in &body {
            assert_eq!(kentos_ifc::closed::faults(s), Vec::<String>::new(), "{}", b.elements[k].name);
        }
    }
    let mut reference = b.clone();
    reference.view = kentos_ifc::building::View::Reference;
    let cut_text = kentos_ifc::building::write(&reference);
    assert!(cut_text.contains("ViewDefinition [ReferenceView]") && cut_text.contains(",'Reference','SweptSolid',"));
    if std::env::var_os("KENTOS_WRITE_IFC_CURVED").is_some() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc");
        std::fs::write(dir.join("egri-bina.ifc"), &text).expect("the file kept");
        std::fs::write(dir.join("egri-bina-referans.ifc"), &cut_text).expect("the file kept");
    }
}

/// Döşemede boşluk ve şaft (docs/adr/8071 §5): on the section building a 1 × 1 m shaft (a void, docs/adr/0708) on the
/// 1. kat, clear of the stair's hole. Worked out here, as the quantities read voids (docs/adr/0722 §5: the slabs of the
/// void's storey): the 1. kat's floor slab (12.25 × 9.25 m less the stair's 2.85 × 3.94 m hole) and the terrace over
/// it (12.25 × 9.25 m) lose the shaft's square; the ground's slab does not. In the IFC model each slab's body is its own
/// outline and the hole and the shaft are its openings; their net areas and volumes take them out, in either view;
/// every body closed.
#[test]
fn a_shaft_takes_its_floor_out_of_its_storey_s_slab() {
    use kentos_commands::CallOp;
    use kentos_ifc::building::{Shape, View};
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.spaces.create",
        CallOp::Execute,
        serde_json::json!({"layerId": "0", "spaces": [{
            "parts": [{"outer": {"pts": [p(9.0, 6.0), p(10.0, 6.0), p(10.0, 7.0), p(9.0, 7.0)]}}],
            "number": "1S1", "name": "Şaft", "void": true, "arch": {"level": "k1"}
        }]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let shaft = answer["output"]["uids"][0].as_str().unwrap().to_owned();
    // The section model: the 1. kat's slab cut, the others not.
    let m = sections::model(&doc);
    assert_eq!(m.slab_cuts.len(), 2, "{:?}", m.slab_cuts);
    assert!(m.slab_cuts.iter().all(|c| c.source == shaft));
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new());
    let slabs: Vec<(usize, &kentos_ifc::building::Element)> =
        b.elements.iter().enumerate().filter(|(_, e)| e.class == Class::Slab).collect();
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let (whole, hole, square) = (12.25 * 9.25, 2.85 * 3.94, 1.0);
    let mut cut_slabs = 0;
    for (k, e) in &slabs {
        let hosted: Vec<&kentos_ifc::building::Opening> = b.openings.iter().filter(|o| o.host == *k).collect();
        let q = e.qtos.iter().find(|q| q.name == "Qto_SlabBaseQuantities").expect("its quantities");
        let get = |n: &str| match q.quantities.iter().find(|(k, _)| k == n).map(|(_, v)| v) {
            Some(Value::Area(x) | Value::Volume(x) | Value::Length(x)) => *x,
            other => panic!("{n}: {other:?}"),
        };
        // Its body its own outline: one ring each layer.
        assert!(e.shapes.iter().all(|s| matches!(s, Shape::Prism { rings, .. } if rings.len() == 1)), "{}", e.name);
        assert!(near(get("GrossArea"), whole), "{} {}", e.name, get("GrossArea"));
        let holes = if hosted.iter().any(|o| o.name == "MM-ACL-Döşeme_boşluğu") { hole } else { 0.0 };
        match hosted.len() {
            0 => assert!(near(get("NetArea"), whole), "{}", e.name),
            1 | 2 => {
                cut_slabs += 1;
                assert!(hosted.iter().any(|o| o.name == "MM-ACL-1S1_Şaft"), "{:?}", hosted);
                assert!(near(get("NetArea"), whole - holes - square), "{} {}", e.name, get("NetArea"));
                let depth = get("Width");
                assert!(near(get("NetVolume"), (whole - holes - square) * depth), "{}", e.name);
                let body = kentos_ifc::quantities::cut(&e.shapes, &hosted);
                let v: f64 = body.iter().map(kentos_ifc::quantities::volume).sum();
                assert!(near(v, get("NetVolume")), "{v}");
                for s in &body {
                    assert_eq!(kentos_ifc::closed::faults(s), Vec::<String>::new(), "{}", e.name);
                }
            }
            n => panic!("{}: {n} openings", e.name),
        }
    }
    assert_eq!(cut_slabs, 2, "the 1. kat's floor slab and the terrace over it");
    // Both views write it closed; the Reference View's slab is cut.
    for view in [View::DesignTransfer, View::Reference] {
        let mut v = b.clone();
        v.view = view;
        assert_eq!(kentos_ifc::closed::open_bodies(&v), Vec::<(String, String)>::new());
    }
}

/// Eğimli döşeme (docs/adr/8071 §6): on the section building a 4 × 3 m slab east of it at (20, 0)–(24, 3), 20 cm thick,
/// rising 30 cm northward over its 3 m (10 %), its top at its tail 3.00 over the ground (offset 3.0 on Zemin kat), and
/// a wall under it from (20, 1.5) to (24, 1.5). Worked out here: its plate's upright thickness 0.2 · √1.01, its
/// volume 12 · 0.2 · √1.01 m³, its area as it lies 12 · √1.01 m², its pitch atan 0.1; the wall stops under its
/// structure at its middle (22, 1.5): 3.00 + 0.15 − 0.2 · √1.01 over the ground. In the IFC model a closed face set,
/// whose volume IfcOpenShell measures the same (`KENTOS_WRITE_IFC_SLOPED=1` writes fixtures/arch/v1/ifc/egimli-doseme.ifc
/// for scripts/fixtures/curved_ifc_check.py's sloped part).
#[test]
fn a_sloped_slab_is_a_sloped_plate_of_its_closed_forms() {
    use kentos_commands::CallOp;
    use kentos_geometry_core::geom::section::Height;
    use kentos_ifc::building::Shape;
    use kentos_native_application::{ExecutionContext, registry, sections};
    let mut doc = building();
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> String {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer["output"]["uids"][0].as_str().unwrap().to_owned()
    };
    let slab = run(&mut doc, "cad.slabs.create", serde_json::json!({"layerId": "0", "slabs": [{
        "parts": [{"outer": {"pts": [p(20.0, 0.0), p(24.0, 0.0), p(24.0, 3.0), p(20.0, 3.0)]}}],
        "thickness": 0.2, "offset": 3.0, "arch": {"level": "zemin"},
        "slope": {"from": p(20.0, 0.0), "to": p(20.0, 3.0), "rise": 0.3}
    }]}));
    let wall = run(&mut doc, "cad.walls.create", serde_json::json!({"layerId": "0", "walls": [{
        "pts": [p(20.0, 1.5), p(24.0, 1.5)], "width": 0.2, "arch": {"level": "zemin"}
    }]}));
    let k = (1.0_f64 + 0.01).sqrt();
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    // The section model: a sloped plate, its top 3.00 at y 0 and 3.30 at y 3.
    let m = sections::model(&doc);
    let i = m.owners.iter().position(|o| o.as_deref() == Some(slab.as_str())).expect("its plate");
    let Height::Sloped(s) = &m.solids[i].height else { panic!("{:?}", m.solids[i].height) };
    let z = |x: f64, y: f64| s.z(kentos_geometry_core::Vec2::new(x, y));
    assert!(near(z(20.0, 0.0), 3.0) && near(z(24.0, 3.0), 3.3), "{} {}", z(20.0, 0.0), z(24.0, 3.0));
    assert!(near(s.thickness, 0.2 * k), "{}", s.thickness);
    // The wall under it stops under its structure at the wall's middle.
    let Some(Entity::Wall(w)) = doc.entities_with_uids().find_map(|(u, e)| (u.to_string() == wall).then_some(e)) else {
        panic!("the wall")
    };
    let top = doc.wall_foot(&w.wall) + doc.wall_height(&w.wall).expect("a height");
    assert!(near(top, 3.0 + 0.15 - 0.2 * k), "{top}");
    // The IFC model.
    let (b, written) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert_eq!(written.left, Vec::new());
    let id = kentos_domain::Uuid::parse_str(&slab).unwrap().as_u128();
    let e = b.elements.iter().find(|e| e.id == Some(id)).expect("written");
    assert_eq!(e.class, Class::Slab);
    assert!(e.shapes.iter().all(|s| matches!(s, Shape::Faces { .. })), "{:?}", e.shapes);
    for s in &e.shapes {
        assert_eq!(kentos_ifc::closed::faults(s), Vec::<String>::new());
    }
    let v: f64 = e.shapes.iter().map(kentos_ifc::quantities::volume).sum();
    assert!(near(v, 12.0 * 0.2 * k), "{v}");
    let q = e.qtos.iter().find(|q| q.name == "Qto_SlabBaseQuantities").expect("its quantities");
    let get = |n: &str| match q.quantities.iter().find(|(k, _)| k == n).map(|(_, v)| v) {
        Some(Value::Area(x) | Value::Volume(x) | Value::Length(x)) => *x,
        other => panic!("{n}: {other:?}"),
    };
    assert!(near(get("Width"), 0.2) && near(get("GrossArea"), 12.0 * k) && near(get("NetVolume"), 12.0 * 0.2 * k));
    let pitch = e.psets.iter().flat_map(|p| &p.props).find(|(n, _)| n == "PitchAngle").map(|(_, v)| v.clone());
    assert!(matches!(pitch, Some(Value::PlaneAngle(a)) if near(a, 0.1_f64.atan())), "{pitch:?}");
    // The 3D view's triangles close the same volume.
    let mesh = kentos_ifc::mesh::meshes(&b).into_iter().find(|m| m.id == Some(id)).expect("its mesh");
    let mv: f64 = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|k| mesh.vertices[k as usize]);
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
        })
        .sum();
    assert!(near(mv.abs(), 12.0 * 0.2 * k), "{mv}");
    if std::env::var_os("KENTOS_WRITE_IFC_SLOPED").is_some() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc/egimli-doseme.ifc");
        std::fs::write(out, kentos_ifc::building::write(&b)).expect("the file kept");
    }
}

/// Kapalı mahal hacmi (docs/adr/8071 §7): a ground room of the section building stands from its floor (the ground slab's
/// finished top, ±0.00) to the 1. kat slab's plastered underside: 3.00 less its finish and screed (0.07), its structure
/// (0.15) and its plaster (0.02), 2.76 m. Its quantities say so: Height 2.76, its volumes its areas by it. An asma tavan
/// at 2.60 brings its ceiling down: Height and FinishCeilingHeight 2.60.
#[test]
fn a_room_is_closed_from_its_floor_to_its_ceiling() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let room = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Space(s) if s.space.arch.level.as_deref() == Some("zemin") && !s.space.void => Some(u),
            _ => None,
        })
        .expect("a ground room");
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let quantity = |b: &kentos_ifc::building::Building, n: &str| -> Option<f64> {
        let s = b.spaces.iter().find(|s| s.id == Some(room.as_u128()))?;
        s.qtos.iter().flat_map(|q| &q.quantities).find(|(k, _)| k == n).map(|(_, v)| match v {
            Value::Length(x) | Value::Area(x) | Value::Volume(x) => *x,
            other => panic!("{other:?}"),
        })
    };
    let (b, _) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    let s = b.spaces.iter().find(|s| s.id == Some(room.as_u128())).expect("written");
    assert!(near(s.bottom, 0.0) && near(s.top, 2.76), "{} → {}", s.bottom, s.top);
    assert!(near(quantity(&b, "Height").unwrap(), 2.76));
    assert_eq!(quantity(&b, "FinishCeilingHeight"), None, "no asma tavan");
    let gross = quantity(&b, "GrossFloorArea").unwrap();
    assert!(near(quantity(&b, "GrossVolume").unwrap(), gross * 2.76));
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.spaces.edit",
        CallOp::Execute,
        serde_json::json!({"uids": [room.to_string()], "operation": {"kind": "set", "suspendedCeiling": 2.6}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let (b, _) = arch_ifc::building(&doc, "2026-11-16T12:00:00", "KentOS Mimari");
    assert!(near(quantity(&b, "Height").unwrap(), 2.6) && near(quantity(&b, "FinishCeilingHeight").unwrap(), 2.6));
    for s in &b.spaces {
        let body = kentos_ifc::building::Shape::Prism {
            rings: s.rings.clone(),
            bulges: s.bulges.clone(),
            bottom: s.bottom,
            top: s.top,
        };
        assert_eq!(kentos_ifc::closed::faults(&body), Vec::<String>::new(), "{}", s.name);
    }
}

/// The section building as A Blok over a basement both blocks share, and B Blok 20 m east entered 1.20 m higher (its own
/// ground level at +1.20): its four walls and a room (docs/adr/8072 §2, §4). A Blok (A1) spans the basement, the ground
/// floor and the 1. kat; B Blok (B1, its ±0.00 1.20 over A's) the basement and its own ground level; the basement counts
/// in A Blok; a slab under both on the basement.
pub(crate) fn two_blocks() -> Document {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let mut settings = doc.settings().clone();
    let arch = settings.arch.as_mut().expect("its levels");
    for (id, name, elevation) in [("bodrum", "Bodrum kat", -3.0), ("b-zemin", "B zemin kat", 1.2)] {
        arch.levels.push(serde_json::from_value(serde_json::json!({"id": id, "name": name, "elevation": elevation, "height": 3.0})).unwrap());
    }
    arch.program.buildings = serde_json::from_value(serde_json::json!([
        {"id": "a", "name": "A Blok", "levels": ["bodrum", "zemin", "k1"], "code": "A1", "usage": "Konut", "zero": 0.0},
        {"id": "b", "name": "B Blok", "levels": ["bodrum", "b-zemin"], "code": "B1", "usage": "Ticaret", "zero": 1.2}
    ]))
    .unwrap();
    arch.program.shared = vec![kentos_commands::SharedLevel { level: "bodrum".into(), building: "a".into() }];
    doc.set_settings(settings);
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)];
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
    };
    run(&mut doc, "cad.slabs.create", serde_json::json!({"layerId": "0", "slabs": [{
        "parts": [{"outer": {"pts": rect(-0.125, -0.125, 26.125, 9.125)}}], "thickness": 0.3, "arch": {"level": "bodrum"}
    }]}));
    let corners = [(20.0, 0.0), (26.0, 0.0), (26.0, 5.0), (20.0, 5.0)];
    let walls: Vec<serde_json::Value> = (0..4)
        .map(|k| {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            serde_json::json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.25, "arch": {"level": "b-zemin"}})
        })
        .collect();
    run(&mut doc, "cad.walls.create", serde_json::json!({"layerId": "0", "walls": walls}));
    run(&mut doc, "cad.spaces.create", serde_json::json!({"layerId": "0", "spaces": [{
        "parts": [{"outer": {"pts": rect(20.125, 0.125, 25.875, 4.875)}}], "number": "B01", "name": "Dükkân",
        "arch": {"level": "b-zemin"}
    }]}));
    doc
}

/// Bina başına model (docs/adr/8072 §2) on `two_blocks`: A's file holds its three storeys (−3.00, ±0.00, +3.00 over its
/// ±0.00) and the basement's slab; B's its one storey at ±0.00 over its own ±0.00, the building placed 1.20 up (its walls
/// standing at +1.20 over the project's, no second offset), no basement; the site and the project the same in both.
/// `KENTOS_WRITE_IFC_BLOCKS=1` writes fixtures/arch/v1/ifc/blok-a.ifc and blok-b.ifc for
/// scripts/fixtures/curved_ifc_check.py's blocks part.
#[test]
fn two_blocks_are_two_models_with_their_own_zero_and_the_basement_once() {
    use kentos_ifc::building::{Shape, write};
    let doc = two_blocks();
    let stamp = "2026-11-16T12:00:00";
    let (a, written_a, unplaced) = arch_ifc::building_of(&doc, "a", stamp, "KentOS Mimari").expect("A Blok");
    assert_eq!((unplaced, written_a.left.len()), (0, 0));
    let (b, written_b, _) = arch_ifc::building_of(&doc, "b", stamp, "KentOS Mimari").expect("B Blok");
    assert_eq!(written_b.left, Vec::new());
    assert!(arch_ifc::building_of(&doc, "c", stamp, "KentOS Mimari").is_none());
    let storeys = |x: &kentos_ifc::building::Building| {
        let mut v: Vec<(String, f64)> = x.storeys.iter().map(|s| (s.long_name.clone().unwrap(), s.elevation)).collect();
        v.sort_by(|p, q| p.1.total_cmp(&q.1));
        v
    };
    assert_eq!(
        storeys(&a),
        vec![("Bodrum kat".into(), -3.0), ("Zemin kat".into(), 0.0), ("1. kat".into(), 3.0)]
    );
    assert_eq!(storeys(&b), vec![("B zemin kat".to_owned(), 0.0)]);
    assert_eq!((a.zero, b.zero), (0.0, 1.2));
    let count = |x: &kentos_ifc::building::Building, c: Class| x.elements.iter().filter(|e| e.class == c).count();
    assert_eq!(count(&b, Class::Wall), 4);
    assert_eq!(count(&b, Class::Slab), 0, "the basement is A Blok's");
    assert_eq!(b.spaces.len(), 1);
    assert_eq!(count(&a, Class::Slab), 4, "the ground, the 1. kat's, the terrace and the basement");
    assert!(a.spaces.len() > 1 && a.spaces.iter().all(|s| s.long_name.as_deref() != Some("Dükkân")));
    // B's walls stand at +1.20 over the project's ±0.00: its zero, its storey's elevation over it, their own foot.
    for e in b.elements.iter().filter(|e| e.class == Class::Wall) {
        let low = e
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Prism { bottom, .. } => Some(*bottom),
                Shape::Faces { .. } => None,
            })
            .fold(f64::MAX, f64::min);
        assert!((b.zero + b.storeys[e.storey].elevation + low - 1.2).abs() < 1e-9, "{}: {low}", e.name);
    }
    // One site and one project in both files; each building its own name.
    let (fa, fb) = (write(&a), write(&b));
    let line = |f: &str, what: &str| f.lines().find(|l| l.contains(what)).map(|l| l.split_once('=').unwrap().1.to_owned());
    assert_eq!(line(&fa, "IFCSITE("), line(&fb, "IFCSITE("));
    assert_eq!(line(&fa, "IFCPROJECT("), line(&fb, "IFCPROJECT("));
    assert!(fa.contains("'MM-BNA-A_Blok'") && fb.contains("'MM-BNA-B_Blok'"), "{}", line(&fb, "IFCBUILDING(").unwrap_or_default());
    if std::env::var_os("KENTOS_WRITE_IFC_BLOCKS").is_some() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/arch/v1/ifc");
        std::fs::write(dir.join("blok-a.ifc"), fa).expect("the file kept");
        std::fs::write(dir.join("blok-b.ifc"), fb).expect("the file kept");
    }
}

/// Every GlobalId of a file written from the drawing: the first attribute of each instance that has one.
fn global_ids(doc: &Document, stamp: &str) -> Vec<String> {
    let (b, _) = arch_ifc::building(doc, stamp, "KentOS Mimari");
    let text = kentos_ifc::building::write(&b);
    text.lines()
        .filter_map(|l| {
            let i = l.find("('")?;
            let g = l.get(i + 2..i + 24)?;
            (l.get(i + 24..i + 25) == Some("'") && kentos_ifc::step::unguid(g).is_some()).then(|| g.to_owned())
        })
        .collect()
}

/// The same drawing written on two days keeps every GlobalId (docs/adr/8117; plan/ifc-uyum.md §7 "Kimlik"; IFC 4.3,
/// Software Identity: the IFC-GUID has to be persistent): the storeys, the openings, the zones, the units' groups, the
/// property sets and the relationships too, not only the objects with a KentOS id; none twice. A door added elsewhere
/// leaves every GlobalId the file had.
#[test]
fn a_drawing_written_another_day_keeps_every_global_id() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    // A curtain wall and a library's lift: parts the export makes of their own (mullions, panels, the lift's fixtures).
    run(&mut doc, "cad.walls.create", serde_json::json!({"layerId": "0", "walls": [{
        "pts": [p(20.0, -3.0), p(26.0, -3.0)], "width": 0.15, "height": 3.0, "arch": {"level": "zemin"},
        "curtain": {"spacing": 1.5, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}
    }]}));
    let library = include_str!("../../../../../catalog/v1/blocks/kentos-temel.kblok");
    run(&mut doc, "cad.blocks.edit", serde_json::json!({"operation": "import", "library": library}));
    run(&mut doc, "cad.blocks.insert", serde_json::json!({
        "name": "Asansör", "at": p(5.0, 6.0), "layerId": "0", "arch": {"level": "zemin"},
        "attributes": {"KISI": "8", "YUK": "630"}
    }));
    let before = global_ids(&doc, "2026-11-16T12:00:00");
    assert!(before.len() > 100, "{} GlobalIds", before.len());
    assert_eq!(global_ids(&doc, "2026-11-18T09:30:00"), before, "another day, the same GlobalIds");
    let once: std::collections::BTreeSet<&String> = before.iter().collect();
    assert_eq!(once.len(), before.len(), "no GlobalId twice");
    // A door in a wall of its own, away from the rest.
    let wall = run(&mut doc, "cad.walls.create", serde_json::json!({"layerId": "0", "walls": [{
        "pts": [p(40.0, 0.0), p(46.0, 0.0)], "width": 0.2, "height": 3.0, "arch": {"level": "zemin"}
    }]}));
    let wall = wall["output"]["uids"][0].as_str().expect("the wall").to_owned();
    run(&mut doc, "cad.openings.create", serde_json::json!({"layerId": "0", "openings": [{"host": wall, "at": 3.0, "width": 0.9}]}));
    let after = global_ids(&doc, "2026-11-19T08:00:00");
    let lost: Vec<&String> = before.iter().filter(|g| !after.contains(g)).collect();
    assert!(lost.is_empty(), "GlobalIds the door's change took away: {lost:?}");
    assert!(after.len() > before.len());
}

/// What the drawing says without being told is changed on the element (docs/adr/8124): a wall declared inside writes
/// `IsExternal` false and its doors and windows follow it; a slab declared outside writes true; a column declared not to
/// carry writes `LoadBearing` false. Undeclared, the drawing's reading stands.
#[test]
fn a_declared_reading_is_written_over_the_drawing_s() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = building();
    let prop = |b: &kentos_ifc::building::Building, id: u128, set: &str, key: &str| {
        b.elements
            .iter()
            .find(|e| e.id == Some(id))
            .and_then(|e| e.psets.iter().filter(|p| p.name == set).flat_map(|p| p.props.iter()).find(|(n, _)| n == key))
            .map(|(_, v)| v.clone())
    };
    let (before, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    // An outer wall with an opening in it, a floor slab and a column of the frame.
    let hosts: std::collections::BTreeSet<String> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Opening(o) if o.opening.form == OpeningForm::Window => Some(o.opening.host.clone()),
            _ => None,
        })
        .collect();
    let wall = doc
        .entities_with_uids()
        .find(|(u, e)| {
            matches!(e, Entity::Wall(_))
                && hosts.contains(&u.to_string())
                && prop(&before, u.as_u128(), "Pset_WallCommon", "IsExternal") == Some(Value::Bool(true))
        })
        .map(|(u, _)| u)
        .expect("an outer wall with a window");
    let window = doc
        .entities_with_uids()
        .find(|(_, e)| matches!(e, Entity::Opening(o) if o.opening.host == wall.to_string() && o.opening.form == OpeningForm::Window))
        .map(|(u, _)| u.as_u128())
        .expect("its window");
    let slab = doc
        .entities_with_uids()
        .find(|(_, e)| matches!(e, Entity::Slab(s) if s.slab.covering.is_none() && s.slab.role == kentos_commands::SlabRole::Floor))
        .map(|(u, _)| u)
        .expect("a floor slab");
    let column = doc
        .entities_with_uids()
        .find(|(_, e)| matches!(e, Entity::Column(c) if c.column.member.is_none()))
        .map(|(u, _)| u)
        .expect("a column");
    assert_eq!(prop(&before, window, "Pset_WindowCommon", "IsExternal"), Some(Value::Bool(true)));
    assert_eq!(prop(&before, slab.as_u128(), "Pset_SlabCommon", "IsExternal"), Some(Value::Bool(false)));
    assert_eq!(prop(&before, column.as_u128(), "Pset_ColumnCommon", "LoadBearing"), Some(Value::Bool(true)));
    let run = |doc: &mut Document, op: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), "cad.arch.performance", CallOp::Execute,
            serde_json::json!({"operation": op})).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    let author = serde_json::json!({"kind": "author"});
    run(&mut doc, serde_json::json!({"kind": "objects", "uids": [wall.to_string()], "external": {"external": false, "basis": author}}));
    run(&mut doc, serde_json::json!({"kind": "objects", "uids": [slab.to_string()], "external": {"external": true, "basis": author}}));
    run(&mut doc, serde_json::json!({"kind": "objects", "uids": [column.to_string()], "loadBearing": {"bearing": false, "basis": author}}));
    let (after, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    assert_eq!(prop(&after, wall.as_u128(), "Pset_WallCommon", "IsExternal"), Some(Value::Bool(false)));
    assert_eq!(prop(&after, window, "Pset_WindowCommon", "IsExternal"), Some(Value::Bool(false)), "the window follows its wall");
    assert_eq!(prop(&after, slab.as_u128(), "Pset_SlabCommon", "IsExternal"), Some(Value::Bool(true)));
    assert_eq!(prop(&after, column.as_u128(), "Pset_ColumnCommon", "LoadBearing"), Some(Value::Bool(false)));
    // The fire rules read the same: the wall is no exterior wall, the column no frame's; every wall reads as the model
    // did before (the framed storey's outer walls on the outline, docs/adr/8124).
    let read_before: Vec<(String, bool)> = before
        .elements
        .iter()
        .filter(|e| e.class == Class::Wall)
        .filter_map(|e| {
            let id = kentos_domain::Uuid::from_u128(e.id?);
            let v = prop(&before, e.id?, "Pset_WallCommon", "IsExternal")?;
            Some((id.to_string(), v == Value::Bool(true)))
        })
        .collect();
    let fire = doc.fire_elements();
    let of = |u: kentos_domain::Uuid| fire.iter().find(|f| f.uid == u.to_string()).expect("a fire element").clone();
    assert_eq!((of(wall).exterior, of(wall).outline), (false, true), "declared inside, read on the outline");
    for (u, outside) in &read_before {
        assert_eq!(fire.iter().find(|f| &f.uid == u).map(|f| f.outline), Some(*outside), "{u}");
    }
    assert!(!of(column).frame);
    // And the property panel's reading says why.
    let read = doc.outside_reading(&wall.to_string(), doc.get(doc.slot_of(wall).unwrap()).unwrap()).unwrap();
    assert_eq!((read.value, read.why), (true, "katın dış çizgisinde olduğu için dışarıda"));
}

/// The IFC Tag (docs/adr/8124): an element's own number (a door's mark, a column's tag), else its type's joinery code,
/// else its id.
#[test]
fn an_element_s_tag_is_its_number_else_its_type_s_code_else_its_id() {
    let mut doc = building();
    let column = doc
        .entities_with_uids()
        .find(|(_, e)| matches!(e, Entity::Column(c) if c.column.member.is_none()))
        .map(|(u, _)| u)
        .expect("a column");
    let slot = doc.slot_of(column).unwrap();
    let mut e = doc.get(slot).unwrap().clone();
    if let Entity::Column(c) = &mut e {
        c.column.tag = Some("S1".into());
    }
    assert!(doc.update(slot, e), "the column tagged");
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let tag = |id: u128| b.elements.iter().find(|e| e.id == Some(id)).and_then(|e| e.tag.clone());
    assert_eq!(tag(column.as_u128()).as_deref(), Some("S1"));
    let wall = doc.entities_with_uids().find(|(_, e)| matches!(e, Entity::Wall(_))).map(|(u, _)| u).unwrap();
    assert_eq!(tag(wall.as_u128()), Some(wall.to_string()), "a wall without a number: its id");
    for (u, e) in doc.entities_with_uids() {
        let Entity::Opening(o) = e else { continue };
        if !matches!(o.opening.form, OpeningForm::Door | OpeningForm::Window) {
            continue;
        }
        let code = o
            .opening
            .style
            .as_deref()
            .and_then(|id| doc.settings().arch.as_ref()?.opening_types.iter().find(|t| t.id == id)?.code.clone());
        let want = o.opening.number.clone().or(code).unwrap_or_else(|| u.to_string());
        assert_eq!(tag(u.as_u128()), Some(want), "{u}");
    }
}

/// The project's accepted survey points are survey annotations in the site (docs/adr/8124; EK-7 Tablo 7.9): named by
/// their mark, their source said, their height over ±0.00; without ±0.00's height none is written.
#[test]
fn an_accepted_survey_point_is_a_survey_annotation_of_the_site() {
    let mut snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    let uid = "0192f5a1-8124-7000-8000-000000000001";
    let id = snap.entities.iter().map(|e| e.base().id).max().unwrap_or(0) + 1;
    let layer = snap.entities[0].base().layer_id.clone();
    snap.entities.push(
        serde_json::from_value(serde_json::json!({"kind": "point", "id": id, "layerId": layer, "attrs": {},
            "p": {"x": -3.0, "y": -4.0}, "z": 1285.10}))
        .unwrap(),
    );
    snap.uids.push(kentos_commands::EntityId::parse(uid).unwrap());
    snap.settings.map_survey = Some(
        serde_json::from_value(serde_json::json!({"points": [{"id": "kn-1", "mark": "P1",
            "uid": uid, "result": {"revision": 1, "solutions": ["sc-1"], "method": "single", "lat": 39.75, "lon": 37.02,
            "height": 1320.0, "srid": 5254, "antenna": [], "x": 412345.0, "y": 4401234.0, "z": 1285.10,
            "chain": "complete"}}]}))
        .unwrap(),
    );
    let doc = Document::from_snapshot_v2(snap.clone()).unwrap();
    assert!(arch_ifc::surveys(&doc).is_empty(), "±0.00 of unknown height: the height is not guessed");
    snap.settings.arch.get_or_insert_with(Default::default).site.get_or_insert_with(Default::default).datum = Some(1285.40);
    let doc = Document::from_snapshot_v2(snap).unwrap();
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    assert_eq!(b.surveys.len(), 1);
    let s = &b.surveys[0];
    assert_eq!(s.name, "P1");
    assert!(s.description.contains("kn-1") && s.description.contains("Tek çözüm") && s.description.contains("sc-1"));
    assert!((s.at[2] - -0.30).abs() < 1e-9 && s.at[0] == -3.0 && s.at[1] == -4.0, "{:?}", s.at);
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCANNOTATION(").count(), 1);
    assert!(text.contains("'P1','Ölçüm noktası kn-1") || text.contains("'P1','\\X2\\"), "its name and source");
    assert!(text.contains(".SURVEY.)"));
    assert!(text.contains("'Annotation','Point'"));
}

/// A room's own emsal state (docs/adr/8125) wins over its words: one room counted out with its article, one an emsal
/// increase (DİĞER) with the administration's rule; the report sums the increase apart, the model writes its zone
/// DIGER with the basis as its description. Every room has its kind and `IsExternal`.
#[test]
fn a_room_s_own_emsal_state_counts_and_an_increase_is_a_diger_zone() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    // The section building with a parcel's zoning: its report sums the rooms.
    let mut snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    snap.settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 450.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let rooms: Vec<String> = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Space(s) if !s.space.void))
        .map(|(u, _)| u.to_string())
        .take(2)
        .collect();
    let before = doc.zoning_report().expect("a zoning");
    let gross = |doc: &Document, uid: &str| doc.zoning_rooms().into_iter().find(|r| r.uid == uid).map(|r| r.gross).unwrap();
    let (g0, g1) = (gross(&doc, &rooms[0]), gross(&doc, &rooms[1]));
    for (uid, emsal) in rooms.iter().zip([
        serde_json::json!({"count": "exempt", "basis": "Planlı Alanlar İmar Yönetmeliği 22/1-a"}),
        serde_json::json!({"count": "bonus", "basis": "Sivas İmar Yönetmeliği 22/k"}),
    ]) {
        let input = serde_json::json!({"uids": [uid], "operation": {"kind": "set", "emsal": emsal}});
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), "cad.spaces.edit", CallOp::Execute, input).unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let after = doc.zoning_report().expect("a zoning");
    assert!((after.bonus - g1).abs() < 1e-9, "the increase apart: {} vs {g1}", after.bonus);
    assert!((after.included - (before.included - g0 - g1)).abs() < 1e-6, "both out of katlar alanı");
    assert!((after.exempt - (before.exempt + g0)).abs() < 1e-6);
    let table = doc.zoning_table();
    assert!(table.rows.iter().any(|r| r[0].contains("Emsal artışı (DİĞER)")), "{:?}", table.rows);
    // The model: a DIGER zone on its storey, its basis the description.
    let (b, _) = arch_ifc::building(&doc, "2026-11-11T12:00:00", "KentOS Mimari");
    let diger: Vec<_> = b
        .zones
        .iter()
        .filter(|z| z.psets.iter().flat_map(|p| p.props.iter()).any(|(k, v)| k == "EmsalDurumu" && *v == Value::Label("DIGER".into())))
        .collect();
    assert_eq!(diger.len(), 1);
    assert!(diger[0].description.as_deref().is_some_and(|d| d.contains("Sivas İmar Yönetmeliği 22/k")), "{:?}", diger[0].description);
    // Every room has its kind (EK-6 Tablo 6.58): the section building's are inside, none a car park.
    for s in &b.spaces {
        assert_eq!(s.predefined, "INTERNAL", "{}", s.name);
    }
}
