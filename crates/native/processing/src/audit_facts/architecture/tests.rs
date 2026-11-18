use std::path::PathBuf;
use std::time::Instant;

use kentos_commands::DocumentSnapshotV2;
use kentos_extensions::audit_facts::ProjectAuditFacts;

use super::super::{MOST_OBJECTS, project_facts};
use super::*;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/audit-facts/v1")
}

/// The architecture drawing (scripts/fixtures/audit_facts_mimari.py made it with KentOS's commands).
fn plan() -> DocumentSnapshotV2 {
    kentos_kcad::decode(&std::fs::read(root().join("mimari.kcad")).unwrap()).unwrap()
}

fn never() -> bool {
    false
}

fn facts(s: DocumentSnapshotV2) -> ProjectAuditFacts {
    project_facts(
        &Document::from_snapshot_v2(s).unwrap(),
        MOST_OBJECTS,
        &never,
    )
    .unwrap()
}

fn room<'a>(a: &'a ArchitectureFacts, level: &str, name: &str) -> &'a RoomFacts {
    a.rooms
        .iter()
        .find(|r| r.level.as_deref() == Some(level) && r.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{level} {name}"))
}

/// What the plan holds, as KentOS Mimari measures it: two storeys, the ground one with two exits (its stair's room and
/// the corridor's door out), the first with one; the archive's farthest point 22.52 m from the stair's door, all of it
/// one way, and the first floor's corridor a 21.82 m dead end, as Mimari denetim's fire rules say them (BYKHY 32/4,
/// 4/i); flat 1's living room 4.21 m from its door (48/2); flat 2's door 6.30 m from the stair (48/4).
#[test]
fn the_plan_s_storeys_rooms_groups_and_walks() {
    let f = facts(plan());
    let a = f.architecture.as_ref().expect("the plan has rooms");
    let levels: Vec<(&str, f64, Option<f64>)> = a
        .levels
        .iter()
        .map(|l| (l.id.as_str(), l.elevation, l.height))
        .collect();
    assert_eq!(levels, [("zemin", 0.0, Some(3.0)), ("k1", 3.0, Some(3.0))]);
    let exits: Vec<(Option<&str>, u64)> = a
        .exits
        .iter()
        .map(|e| (e.level.as_deref(), e.count))
        .collect();
    assert_eq!(exits, [(Some("zemin"), 2), (Some("k1"), 1)]);
    assert_eq!(a.rooms.len(), 15);
    let walk = |level: &str, name: &str| {
        let e = room(a, level, name).escape.as_ref().expect("its walks");
        e.exit.clone().expect("a walk to an exit")
    };
    let archive = walk("k1", "Arşiv");
    assert_eq!(
        (archive.length, archive.one_way, archive.exits),
        (22.52, Some(22.52), Some(1))
    );
    assert_eq!(walk("k1", "Koridor").one_way, Some(21.819), "a dead end");
    assert_eq!(walk("zemin", "Koridor").one_way, Some(0.0), "two exits");
    let living = room(a, "zemin", "Salon")
        .escape
        .as_ref()
        .expect("its walks");
    assert!(living.has_entrance && living.storey_exits);
    assert_eq!(living.entrance.as_ref().map(|w| w.length), Some(4.21));
    for (level, name) in [
        ("zemin", "Merdiven evi"),
        ("zemin", "Asansör boşluğu"),
        ("k1", "Balkon"),
    ] {
        assert!(
            room(a, level, name).escape.is_none(),
            "{name} is not walked"
        );
    }
    assert!(room(a, "k1", "Merdiven evi").stair);
    assert!(room(a, "zemin", "Asansör boşluğu").void);
    assert!(room(a, "k1", "Balkon").outdoor);
    let ids: Vec<&str> = a.units.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(
        ids,
        ["d1", "d2", "dk", "of", "dp", "ortak"],
        "the program's order"
    );
    let unit = |id: &str| a.units.iter().find(|u| u.id == id).expect("the group");
    assert_eq!(
        unit("d1").annex_gross,
        unit("dp").gross,
        "its storage annex"
    );
    assert!(unit("d1").dwelling && !unit("of").dwelling);
    assert_eq!(unit("ortak").share, None);
    let shares: f64 = a.units.iter().filter_map(|u| u.share).sum();
    assert!(
        (shares - unit("ortak").gross).abs() < 1e-3,
        "the common areas shared out"
    );
    // A group's net is its counted rooms' (voids and outdoor rooms out), less the places under its stairs lower than
    // 1.80 m (4/1-ğ-3, docs/adr/8080): the common stair hall's first flight.
    let groups = Document::from_snapshot_v2(plan()).unwrap().paiy_areas();
    let under = |id: &str| groups.iter().find(|(g, _)| g == id).map_or(0.0, |(_, g)| g.under_stair);
    assert!(under("ortak") > 0.0 && under("d1") == 0.0, "the stair stands in the hall");
    for u in &a.units {
        let rooms: f64 = a
            .rooms
            .iter()
            .filter(|r| r.unit.as_deref() == Some(u.id.as_str()) && !r.void && !r.outdoor)
            .map(|r| r.net)
            .sum();
        assert!(
            (rooms - under(&u.id) - u.net).abs() < 1e-3,
            "{}: {rooms} − {} ≠ {}",
            u.id,
            under(&u.id),
            u.net
        );
    }
    let doors: Vec<(&str, Option<f64>)> = a
        .doors
        .iter()
        .map(|d| (d.unit.as_str(), d.exit.as_ref().and_then(|w| w.one_way)))
        .collect();
    assert_eq!(doors.len(), 2);
    assert!(doors.contains(&("d2", Some(6.3))) && doors.contains(&("d1", Some(0.0))));
}

/// The same objects give the same bytes: measured twice, in the reverse order and in another order.
#[test]
fn the_same_plan_in_any_order_gives_the_same_bytes() {
    let first = facts(plan()).to_json();
    assert_eq!(facts(plan()).to_json(), first, "twice the same");
    let mut s = plan();
    s.entities.reverse();
    s.uids.reverse();
    assert_eq!(facts(s).to_json(), first);
    let mut s = plan();
    let n = s.entities.len();
    assert_ne!(n % 7, 0, "7 is prime and no divisor: every object once");
    let order: Vec<usize> = (0..n).map(|i| (i * 7) % n).collect();
    s.entities = order.iter().map(|&i| s.entities[i].clone()).collect();
    s.uids = order.iter().map(|&i| s.uids[i]).collect();
    assert_eq!(facts(s).to_json(), first);
}

/// A drawing without rooms has no section: its facts are the layers' alone, as before it.
/// A project with an İmar durumu gives its zoning: the plan's ratios and the areas the Emsal ve TAKS table reads, the
/// same numbers as KentOS Mimari's own report (docs/adr/8057 §92); the fixture's plan writes none and gives none.
#[test]
fn the_parcels_zoning_is_read_as_the_emsal_and_taks_table_reads_it() {
    assert!(
        facts(plan())
            .architecture
            .as_ref()
            .unwrap()
            .zoning
            .is_none()
    );
    let mut s = plan();
    let arch = s.settings.arch.get_or_insert_with(Default::default);
    arch.layout = Some(kentos_commands::BuildingLayout::Attached);
    arch.zoning = Some(kentos_commands::Zoning {
        parcel_area: 300.0,
        far: Some(1.5),
        coverage: Some(0.4),
        far_source: Some("İmar durum belgesi 12.3.2025/1234".into()),
        coverage_source: None,
    });
    let doc = Document::from_snapshot_v2(s.clone()).unwrap();
    let report = doc.zoning_report().expect("a report");
    let f = facts(s);
    let z = f
        .architecture
        .as_ref()
        .unwrap()
        .zoning
        .as_ref()
        .expect("the zoning");
    assert_eq!((z.parcel, z.far, z.coverage), (300.0, Some(1.5), Some(0.4)));
    // Where the emsal is read from goes with it (docs/adr/8140); the TAKS says none.
    assert_eq!(
        (z.far_source.as_deref(), z.coverage_source.as_deref()),
        (Some("İmar durum belgesi 12.3.2025/1234"), None)
    );
    // So does the building's layout (docs/adr/8141), which 5/6 reads the TAKS by.
    assert_eq!(z.layout.as_deref(), Some("attached"));
    let near = |a: f64, b: f64| (a - b).abs() < 1e-4;
    assert!(
        near(z.floors, report.floors) && near(z.footprint, report.footprint),
        "{z:?}"
    );
    assert!(z.floors > 0.0 && z.footprint > 0.0, "{z:?}");
    assert_eq!(z.cap.is_some(), report.cap.is_some());
}

#[test]
fn a_drawing_without_rooms_has_no_architecture() {
    let text = std::fs::read_to_string(root().join("denetim.kcad")).unwrap();
    let f = facts(serde_json::from_str(&text).unwrap());
    assert_eq!(f.architecture, None);
    assert!(!f.to_json().contains("architecture"));
}

/// Asked to stop, the architecture's measure gives nothing.
#[test]
fn a_stopped_architecture_measure_gives_nothing() {
    let doc = Document::from_snapshot_v2(plan()).unwrap();
    assert!(matches!(measure(&doc, &|| true), Err(Stopped)));
    assert!(matches!(measure(&doc, &never), Ok(Some(_))));
}

/// The facts are the fixture's, byte for byte (fixtures/audit-facts/v1/mimari.json): what a cloud job and the SDK's
/// Python tests compare with. A change of the facts is written there by
/// `KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib -- audit_facts`.
#[test]
fn the_architecture_facts_are_the_fixture_s() {
    let made = serde_json::to_string_pretty(&facts(plan())).unwrap() + "\n";
    let path = root().join("mimari.json");
    if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
        std::fs::write(&path, &made).unwrap();
        return;
    }
    assert!(
        std::fs::read_to_string(&path).unwrap() == made,
        "the facts changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib -- audit_facts"
    );
}

/// What a mid-size plan costs (docs/adr/8057 §39): `KENTOS_AUDIT_FACTS_PLAN=<a .kcad> cargo test -p kentos-processing
/// --lib -- --ignored measure_the_cost --nocapture` prints the whole measure and its architecture part.
#[test]
#[ignore]
fn measure_the_cost() {
    let Some(path) = std::env::var_os("KENTOS_AUDIT_FACTS_PLAN") else {
        return;
    };
    let doc =
        Document::from_snapshot_v2(kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap())
            .unwrap();
    let rooms = doc
        .entities()
        .filter(|e| matches!(e, Entity::Space(_)))
        .count();
    for _ in 0..2 {
        let began = Instant::now();
        let all = project_facts(&doc, MOST_OBJECTS, &never).unwrap();
        let whole = began.elapsed();
        let began = Instant::now();
        let _ = measure(&doc, &never);
        let part = began.elapsed();
        eprintln!(
            "{} objects, {rooms} rooms: facts {whole:?}, the architecture {part:?} ({} bytes)",
            doc.entities().count(),
            all.to_json().len()
        );
    }
}
