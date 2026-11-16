//! The reading face against the files KentOS writes, an independent reader's graph of them, broken files and the
//! limits (docs/adr/8061 §7; plan/ifc-uyum.md IFC-07 "İlk kabul").
//!
//! ```text
//! cargo test -p kentos-ifc --features read read::
//! ```

use super::*;

/// The files KentOS wrote (fixtures/arch/v1/ifc/, fixtures/road/v1/ifc/out/): name and bytes.
pub(super) const WRITTEN: [(&str, &[u8]); 11] = [
    ("kesit-binasi", include_bytes!("../../../../../fixtures/arch/v1/ifc/kesit-binasi.ifc")),
    ("kesit-binasi-kunye", include_bytes!("../../../../../fixtures/arch/v1/ifc/kesit-binasi-kunye.ifc")),
    ("blok-a", include_bytes!("../../../../../fixtures/arch/v1/ifc/blok-a.ifc")),
    ("blok-b", include_bytes!("../../../../../fixtures/arch/v1/ifc/blok-b.ifc")),
    ("cati-alti-mahal", include_bytes!("../../../../../fixtures/arch/v1/ifc/cati-alti-mahal.ifc")),
    ("egimli-doseme", include_bytes!("../../../../../fixtures/arch/v1/ifc/egimli-doseme.ifc")),
    ("egri-bina", include_bytes!("../../../../../fixtures/arch/v1/ifc/egri-bina.ifc")),
    ("egri-bina-referans", include_bytes!("../../../../../fixtures/arch/v1/ifc/egri-bina-referans.ifc")),
    ("katmanli-cati", include_bytes!("../../../../../fixtures/arch/v1/ifc/katmanli-cati.ifc")),
    ("yerel-sistem", include_bytes!("../../../../../fixtures/arch/v1/ifc/yerel-sistem.ifc")),
    ("yol", include_bytes!("../../../../../fixtures/road/v1/ifc/out/yol.ifc")),
];

/// Files another program wrote (fixtures/ifc/v1/acik/, scripts/fixtures/ifc_other_writer_cases.py): IFC4 and IFC 4.3.
pub(super) const OTHERS: [(&str, &[u8]); 2] = [
    ("ifcopenshell-ifc4", include_bytes!("../../../../../fixtures/ifc/v1/acik/ifcopenshell-ifc4.ifc")),
    ("ifcopenshell-ifc4x3", include_bytes!("../../../../../fixtures/ifc/v1/acik/ifcopenshell-ifc4x3.ifc")),
];

/// Real-world files (fixtures/ifc/v1/disaridan/README.md: CC BY 4.0, buildingSMART International): IFC 4.3, IFC4 and
/// IFC2x3, written by five programs.
pub(super) const OUTSIDE: [(&str, &[u8]); 7] = [
    ("bsi-ifc4x3-building-architecture", include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc")),
    ("bsi-ifc4x3-infra-road", include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4x3-infra-road.ifc")),
    ("bsi-ifc4x3-infra-rail", include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4x3-infra-rail.ifc")),
    (
        "bsi-ifc4-wall-with-opening-and-window",
        include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4-wall-with-opening-and-window.ifc"),
    ),
    ("bsi-ifc4-basin-tessellation", include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4-basin-tessellation.ifc")),
    (
        "bsi-ifc4-column-straight-rectangle-tessellation",
        include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc4-column-straight-rectangle-tessellation.ifc"),
    ),
    ("bsi-ifc2x3-building-architecture", include_bytes!("../../../../../fixtures/ifc/v1/disaridan/bsi-ifc2x3-building-architecture.ifc")),
];

/// Every sample: KentOS's, another program's, then the real-world ones.
pub(super) fn samples() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    WRITTEN.iter().chain(OTHERS.iter()).chain(OUTSIDE.iter()).copied()
}

pub(super) fn never() -> bool {
    false
}

pub(super) fn read_ok(bytes: &[u8]) -> Model {
    read(bytes, &Limits::default(), &never).unwrap_or_else(|e| panic!("reads: {e}"))
}

fn errors(m: &Model) -> Vec<&Finding> {
    m.findings.iter().filter(|f| f.severity == Severity::Error).collect()
}

#[test]
fn every_file_kentos_writes_reads_back_whole() {
    for (name, bytes) in WRITTEN {
        let m = read_ok(bytes);
        assert_eq!(m.summary.schema, "IFC4X3_ADD2", "{name}");
        assert_eq!(m.summary.family, Family::Ifc4x3, "{name}");
        assert!(errors(&m).is_empty(), "{name}: {:#?}", errors(&m));
        // One root, the project; everything else hangs from it (types and openings aside).
        let roots: Vec<&str> = m.roots.iter().map(|&i| m.objects[i].class.as_str()).collect();
        assert_eq!(roots, ["IfcProject"], "{name}");
        assert!(m.summary.records > 0 && !m.objects.is_empty(), "{name}");
        assert_eq!(m.summary.units.length.as_ref().map(|u| u.to_si), Some(1.0), "{name}: metres");
        assert!(m.summary.header.preprocessor.as_deref().is_some_and(|p| p.starts_with("KentOS")), "{name}: {:?}", m.summary.header);
    }
}

#[test]
fn turkish_letters_come_back_as_they_were_written() {
    let m = read_ok(WRITTEN[0].1);
    assert_eq!(m.summary.header.name.as_deref(), Some("Kesit binası.ifc"));
    let road = read_ok(WRITTEN[10].1);
    assert_eq!(road.summary.header.name.as_deref(), Some("Yol alışverişi.ifc"));
    // Every name the writer escaped (\X2\…\X0\) reads as UTF-8, no escape left.
    for (name, bytes) in WRITTEN {
        let m = read_ok(bytes);
        for o in &m.objects {
            for s in [&o.name, &o.description, &o.object_type, &o.long_name].into_iter().flatten() {
                assert!(!s.contains("\\X"), "{name}: {s}");
            }
        }
    }
}

#[test]
fn the_tree_parts_containment_and_the_inverse_index() {
    let m = read_ok(WRITTEN[0].1);
    let project = m.roots[0];
    let site = m.objects[project].children.iter().map(|&i| &m.objects[i]).find(|o| o.class == "IfcSite").expect("a site");
    assert!(site.parent == Some(project));
    let storeys: Vec<&Object> = m.objects.iter().filter(|o| o.class == "IfcBuildingStorey").collect();
    assert_eq!(storeys.len(), 2);
    // Walls are contained in a storey, their openings void them, their types are walls' types.
    let wall = m.objects.iter().find(|o| o.class == "IfcWall").expect("a wall");
    let container = wall.container.map(|c| m.objects[c].class.as_str());
    assert_eq!(container, Some("IfcBuildingStorey"));
    assert!(wall.has_shape);
    let classes: Vec<&str> = m.relations_of(wall.step).iter().map(|&r| m.relations[r].class.as_str()).collect();
    assert!(classes.contains(&"IfcRelContainedInSpatialStructure"), "{classes:?}");
    assert!(classes.contains(&"IfcRelDefinesByProperties"), "{classes:?}");
    assert_eq!(m.object(wall.step).map(|o| o.global_id.as_str()), Some(wall.global_id.as_str()));
}

#[test]
fn a_wall_s_sets_are_read_when_asked() {
    let bytes = WRITTEN[0].1;
    let m = read_ok(bytes);
    let i = m.objects.iter().position(|o| o.class == "IfcWall").expect("a wall");
    let sets = m.property_sets(bytes, i).expect("its sets");
    let names: Vec<&str> = sets.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Pset_WallCommon"), "{names:?}");
    let common = sets.iter().find(|s| s.name == "Pset_WallCommon").expect("Pset_WallCommon");
    assert_eq!(common.kind, SetKind::Properties);
    let external = common.values.iter().find(|v| v.name == "IsExternal").expect("IsExternal");
    assert!(matches!(external.value, ValueKind::Logical(Some(_))), "{external:?}");
    if let Some(q) = sets.iter().find(|s| s.kind == SetKind::Quantities) {
        assert!(q.values.iter().all(|v| matches!(v.value, ValueKind::Number { measure: Some(_), .. })), "{q:#?}");
    }
}

// ── Broken files: an error or a finding, never a panic (docs/adr/8061 §7 "Bozuk girdi") ─────────────────────────────

fn small() -> &'static [u8] {
    WRITTEN[3].1
}

fn text() -> String {
    String::from_utf8(small().to_vec()).expect("UTF-8")
}

#[test]
fn what_is_not_an_ifc_file_is_said_so() {
    let limits = Limits::default();
    assert_eq!(read(b"", &limits, &never).err(), Some(ReadError::Empty));
    assert_eq!(read(b"PK\x03\x04 zip", &limits, &never).err(), Some(ReadError::NotStep));
    assert_eq!(read(&[0xff; 64], &limits, &never).err(), Some(ReadError::NotStep));
    let schema = text().replace("IFC4X3_ADD2", "IFC9");
    assert_eq!(read(schema.as_bytes(), &limits, &never).err(), Some(ReadError::Schema { declared: Some("IFC9".into()) }));
    let head_only = text().split("DATA;").next().unwrap_or_default().to_owned() + "DATA;\nENDSEC;\nEND-ISO-10303-21;\n";
    assert_eq!(read(head_only.as_bytes(), &limits, &never).err(), Some(ReadError::NoData));
    let small_limit = Limits { bytes: 100, ..Limits::default() };
    assert!(matches!(read(small(), &small_limit, &never), Err(ReadError::TooLarge { .. })));
    let few = Limits { records: 10, ..Limits::default() };
    assert_eq!(read(small(), &few, &never).err(), Some(ReadError::TooManyRecords { limit: 10 }));
}

#[test]
fn a_cut_file_reads_up_to_the_cut_and_says_where() {
    let t = text();
    for at in [t.len() / 3, t.len() / 2, t.len() - 40] {
        match read(&t.as_bytes()[..at], &Limits::default(), &never) {
            Ok(m) => assert!(!m.findings.is_empty() || m.summary.records > 0, "cut at {at}"),
            Err(e) => assert!(matches!(e, ReadError::NoData), "cut at {at}: {e}"),
        }
    }
    // A record cut in its middle, the file going on: the record is a finding with its line.
    let cut = t.replacen("IFCWALL(", "IFCWALL('kesik", 1);
    let m = read_ok(cut.as_bytes());
    assert!(!m.findings.is_empty(), "the cut record is said");
}

#[test]
fn a_reference_to_nothing_is_a_finding_with_its_line() {
    let t = text();
    // The first relative placement refers to #9999991 instead of the placement it is relative to.
    let at = t.find("=IFCLOCALPLACEMENT(#").expect("a relative placement") + "=IFCLOCALPLACEMENT(".len();
    let digits = t[at + 1..].find(',').expect("its next attribute");
    let broken = format!("{}#9999991{}", &t[..at], &t[at + 1 + digits..]);
    let line = broken[..at].matches('\n').count() + 1;
    let m = read_ok(broken.as_bytes());
    let f = m.findings.iter().find(|f| f.code == "reference_missing").expect("said");
    assert_eq!(f.line, Some(line), "{f:?}");
    assert!(f.message.contains("#9999991"), "{}", f.message);
}

#[test]
fn cycles_in_the_tree_and_in_the_placements_are_findings() {
    let t = text();
    let m = read_ok(small());
    let project = m.objects[m.roots[0]].step;
    let storey = m.objects.iter().find(|o| o.class == "IfcBuildingStorey").expect("a storey").step;
    // The project made a part of a storey: project → site → building → storey → project.
    let cycle = t.replace(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("#9999990=IFCRELAGGREGATES('3Kd1lX2Y4Fz8uQ0p9wLmNa',$,$,$,#{storey},(#{project}));\nENDSEC;\nEND-ISO-10303-21;"),
    );
    let m = read_ok(cycle.as_bytes());
    assert!(m.findings.iter().any(|f| f.code == "tree_cycle"), "{:#?}", m.findings);
    assert!(m.roots.is_empty(), "nothing in the cycle is a root");
    // A placement relative to itself.
    let placement = t.lines().find(|l| l.contains("=IFCLOCALPLACEMENT(#")).expect("a relative placement");
    let id = &placement[1..placement.find('=').expect("=")];
    let own = format!("#{id}=IFCLOCALPLACEMENT(#{id},{}", &placement[placement.find(',').expect(",") + 1..]);
    let m = read_ok(t.replace(placement, &own).as_bytes());
    assert!(m.findings.iter().any(|f| f.code == "placement_cycle"), "{:#?}", m.findings);
}

#[test]
fn record_numbers_and_global_ids_said_twice_or_malformed_are_findings() {
    let t = text();
    let m = read_ok(small());
    let mut walls = m.objects.iter().filter(|o| o.class == "IfcWall");
    let (wall, other) = (walls.next().expect("a wall"), walls.next().expect("another wall"));
    let twice = t.replace(&other.global_id, &wall.global_id);
    let m2 = read_ok(twice.as_bytes());
    assert!(m2.findings.iter().any(|f| f.code == "global_id_duplicate"), "{:#?}", m2.findings);
    let bad = t.replacen(&wall.global_id, "kısa", 1);
    assert!(read_ok(bad.as_bytes()).findings.iter().any(|f| f.code == "global_id_invalid"));
    let dup = t.replace("ENDSEC;\nEND-ISO-10303-21;", "#1=IFCCARTESIANPOINT((0.,0.,0.));\nENDSEC;\nEND-ISO-10303-21;");
    assert!(read_ok(dup.as_bytes()).findings.iter().any(|f| f.code == "step_duplicate"));
    let huge = t.replace("ENDSEC;\nEND-ISO-10303-21;", "#99999999999=IFCCARTESIANPOINT((0.,0.,0.));\nENDSEC;\nEND-ISO-10303-21;");
    assert!(read_ok(huge.as_bytes()).findings.iter().any(|f| f.code == "step_number_too_large"));
    let unknown = t.replace("ENDSEC;\nEND-ISO-10303-21;", "#9999992=IFCNOTACLASS('x');\nENDSEC;\nEND-ISO-10303-21;");
    assert!(read_ok(unknown.as_bytes()).findings.iter().any(|f| f.code == "class_unknown"));
}

#[test]
fn deep_and_long_records_stop_the_read_before_the_engine_sees_them() {
    let t = text();
    let end = "ENDSEC;\nEND-ISO-10303-21;";
    // 100 000 lists in lists: past the nesting limit, said with its line, no stack overflow.
    let deep = format!("#9999993=IFCCARTESIANPOINTLIST3D({}{});\n", "(".repeat(100_000), ")".repeat(100_000));
    let file = t.replace(end, &(deep + end));
    let line = file[..file.find("#9999993").expect("the record")].matches('\n').count() + 1;
    assert_eq!(
        read(file.as_bytes(), &Limits::default(), &never).err(),
        Some(ReadError::Record { line, what: RecordLimit::Nesting, limit: 32 })
    );
    // A list longer than the limit.
    let items = Limits { items: 1000, ..Limits::default() };
    let long = format!("#9999994=IFCCARTESIANPOINTLIST3D((({})));\n", vec!["0."; 3000].join(","));
    let file = t.replace(end, &(long + end));
    assert!(matches!(read(file.as_bytes(), &items, &never), Err(ReadError::Record { what: RecordLimit::Items, .. })));
    let short = Limits { record_bytes: 64, ..Limits::default() };
    assert!(matches!(read(small(), &short, &never), Err(ReadError::Record { what: RecordLimit::Length, .. })));
}

#[test]
fn the_host_stops_a_read_and_a_panic_is_an_error() {
    // Enough records for the host to be asked.
    let mut t = text();
    let points: String = (0..10_000).map(|i| format!("#{}=IFCCARTESIANPOINT(({i}.,0.,0.));\n", 5_000_000 + i)).collect();
    t = t.replace("ENDSEC;\nEND-ISO-10303-21;", &(points + "ENDSEC;\nEND-ISO-10303-21;"));
    let asked = std::cell::Cell::new(0);
    let stop = || {
        asked.set(asked.get() + 1);
        true
    };
    assert_eq!(read(t.as_bytes(), &Limits::default(), &stop).err(), Some(ReadError::Cancelled));
    assert_eq!(asked.get(), 1, "stopped at the first question");
    let caught = guarded::<()>(|| panic!("motor çöktü"));
    assert_eq!(caught, Err(ReadError::Engine { message: "motor çöktü".into() }));
}

// ── The engine's name stays in one manifest (docs/adr/8061 §3.3 rule 7) ──────────────────────────────────────────

#[test]
fn the_engine_s_name_is_in_this_crate_s_manifest_alone() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let own = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let mut stack = vec![root];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".run" | ".claude" | "dist") {
                    stack.push(p);
                }
            } else if name == "Cargo.toml"
                && std::fs::read_to_string(&p).is_ok_and(|t| t.contains("ifc-lite-"))
                && p.canonicalize().ok() != own.canonicalize().ok()
            {
                found.push(p);
            }
        }
    }
    assert!(found.is_empty(), "the engine named outside kentos-ifc: {found:?}");
    // Nor on this crate's public face: no public item of the reading face names the engine.
    for (file, text) in [("mod.rs", include_str!("mod.rs")), ("sets.rs", include_str!("sets.rs"))] {
        for line in text.lines().filter(|l| l.trim_start().starts_with("pub")) {
            assert!(!line.contains("ifc_lite"), "{file}: {line}");
        }
    }
}
