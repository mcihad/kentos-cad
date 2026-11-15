use std::cell::Cell;
use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV2;
use serde_json::{Value, json};

use super::*;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/audit-facts/v1")
}

/// The fixture drawing as JSON: a v2 file, every object's persistent id written in it.
fn snapshot() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join("denetim.kcad")).unwrap()).unwrap()
}

fn open(v: Value) -> Document {
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(v).unwrap()).unwrap()
}

fn never() -> bool {
    false
}

fn facts(v: Value) -> ProjectAuditFacts {
    project_facts(&open(v), MOST_OBJECTS, &never).unwrap()
}

fn uid(n: u32) -> String {
    format!("0192f5a1-2590-7000-8000-{n:012}")
}

/// The fixture's objects (and their ids) in the order `order` gives, each under another local id.
fn reordered(order: impl Iterator<Item = usize>) -> Value {
    let mut v = snapshot();
    let entities = v["entities"].as_array().unwrap().clone();
    let uids = v["uids"].as_array().unwrap().clone();
    let (mut e, mut u) = (Vec::new(), Vec::new());
    for (k, i) in order.enumerate() {
        let mut entity = entities[i].clone();
        entity["id"] = json!(100 + k);
        e.push(entity);
        u.push(uids[i].clone());
    }
    assert_eq!(e.len(), entities.len());
    v["entities"] = Value::Array(e);
    v["uids"] = Value::Array(u);
    v
}

/// What each layer holds, as the drawing was made to hold it (fixtures/audit-facts/v1/denetim.kcad): parcels 2 and 3
/// overlapping by 5 × 10 m, a 10 × 6 m gap closed in by five parcels, three buildings whose shapes are not valid, two
/// points, a road and a half circle.
#[test]
fn the_facts_say_what_each_layer_holds() {
    let f = facts(snapshot());
    assert_eq!(f.tolerance, 0.001, "the default, the project names none");
    assert!(!f.cut);
    let ids: Vec<&str> = f.layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(
        ids,
        ["parsel", "yapi", "nokta", "yol"],
        "the tree's order; a layer without objects is not there"
    );
    let parsel = &f.layers[0];
    assert_eq!(parsel.name, "Parsel");
    assert_eq!(
        parsel.objects,
        ObjectCounts {
            points: 0,
            lines: 0,
            areas: 8,
            other: 1
        },
        "the label is not measured"
    );
    assert_eq!(parsel.area, 1140.0);
    assert_eq!(
        parsel.overlaps,
        [Overlap {
            a: uid(2),
            b: uid(3),
            area: 50.0,
            at: [17.5, 5.0]
        }]
    );
    assert_eq!(parsel.gaps.len(), 1, "{:#?}", parsel.gaps);
    let gap = &parsel.gaps[0];
    assert_eq!((gap.area, gap.at), (60.0, [15.0, 33.0]));
    assert_eq!(
        gap.around.first(),
        Some(&uid(6)),
        "the neighbour sharing the longest boundary first"
    );
    let mut around = gap.around.clone();
    around.sort();
    assert_eq!(around, [uid(4), uid(5), uid(6), uid(7), uid(8)]);
    assert!(parsel.invalid.is_empty());
    assert_eq!(parsel.totals.overlaps, 1);
    let yapi = &f.layers[1];
    let problems: Vec<(String, &str, [f64; 2])> = yapi
        .invalid
        .iter()
        .map(|x| (x.uid.clone(), x.problem.key(), x.at))
        .collect();
    assert_eq!(
        problems,
        [
            (uid(10), "ringCrossing", [105.0, 5.0]),
            (uid(11), "repeated", [130.0, 0.0]),
            (uid(12), "zeroArea", [140.0, 0.0])
        ]
    );
    assert_eq!(
        yapi.area, 200.0,
        "a bow tie's halves cancel, a ring on one line has none"
    );
    assert!(yapi.overlaps.is_empty());
    assert_eq!(f.layers[2].objects.points, 2);
    let yol = &f.layers[3];
    assert_eq!(yol.objects.lines, 2);
    assert_eq!(
        yol.length, 45.708,
        "30 m and a half circle of 5 m radius, its arc exact (5π)"
    );
}

/// The facts are the fixture's, byte for byte (fixtures/audit-facts/v1/denetim.json): what a cloud job and the SDK's
/// Python tests compare with. A change of the facts is written there by
/// `KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib -- audit_facts`.
#[test]
fn the_facts_are_the_fixture_s() {
    let made = serde_json::to_string_pretty(&facts(snapshot())).unwrap() + "\n";
    let path = root().join("denetim.json");
    if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
        std::fs::write(&path, &made).unwrap();
        return;
    }
    assert!(
        std::fs::read_to_string(&path).unwrap() == made,
        "the facts changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib -- audit_facts"
    );
}

/// The same objects give the same bytes: measured twice, in the reverse order, in another order, under other local
/// ids.
#[test]
fn the_same_objects_in_any_order_give_the_same_bytes() {
    let first = facts(snapshot()).to_json();
    assert_eq!(facts(snapshot()).to_json(), first, "twice the same");
    let n = snapshot()["entities"].as_array().unwrap().len();
    assert_eq!(facts(reordered((0..n).rev())).to_json(), first);
    assert_ne!(n % 7, 0, "7 is prime and no divisor: every object once");
    assert_eq!(
        facts(reordered((0..n).map(|i| (i * 7) % n))).to_json(),
        first
    );
}

/// Past `most` objects the rest are left out by persistent id, whatever the drawing's order, and `cut` says so.
#[test]
fn past_most_objects_the_facts_are_cut_by_persistent_id() {
    let n = snapshot()["entities"].as_array().unwrap().len();
    for v in [snapshot(), reordered((0..n).rev())] {
        let f = project_facts(&open(v), 3, &never).unwrap();
        assert!(f.cut);
        assert_eq!(f.layers.len(), 1, "parcels 1, 2 and 3 only");
        assert_eq!(f.layers[0].objects.areas, 3);
        assert_eq!(f.layers[0].overlaps.len(), 1);
        assert!(f.layers[0].gaps.is_empty());
    }
}

/// The audit's Durdur: asked to stop before the check or while it runs, it gives nothing.
#[test]
fn a_stopped_measure_gives_nothing() {
    let doc = open(snapshot());
    assert!(project_facts(&doc, MOST_OBJECTS, &|| true).is_none());
    let asked = Cell::new(0);
    let later = || {
        asked.set(asked.get() + 1);
        asked.get() > 2
    };
    assert!(project_facts(&doc, MOST_OBJECTS, &later).is_none());
    assert!(asked.get() > 2, "asked as it went");
}

/// The tolerance is the project's: an overlap no wider than it is none.
#[test]
fn the_tolerance_is_the_project_s() {
    let mut v = snapshot();
    v["entities"][2]["pts"] = json!([
        { "x": 19.8, "y": 0 }, { "x": 29.8, "y": 0 }, { "x": 29.8, "y": 10 }, { "x": 19.8, "y": 10 }
    ]);
    let tight = facts(v.clone());
    let areas: Vec<f64> = tight.layers[0].overlaps.iter().map(|o| o.area).collect();
    assert_eq!(areas, [2.0], "0.2 × 10 m");
    v["settings"]["topology"] = json!({ "tolerance": 0.5 });
    let loose = facts(v);
    assert_eq!(loose.tolerance, 0.5);
    assert!(
        loose.layers[0].overlaps.is_empty(),
        "{:?}",
        loose.layers[0].overlaps
    );
}

/// Every problem the facts name is one the geometry core finds.
#[test]
fn every_problem_named_is_the_core_s() {
    use kentos_geometry_core::ops::geoprocess::validity::Kind as Core;
    for p in ValidityProblem::ALL {
        assert_eq!(Core::of_key(p.key()).map(Core::key), Some(p.key()));
    }
}

/// An area's hole is a gap of its layer (what the union of its areas closes in and none covers), measured exact: a
/// 20 × 20 m area with a 4 × 5 m hole is 380 m² and one 20 m² gap around which it alone lies.
#[test]
fn an_area_s_hole_is_a_gap_of_its_layer() {
    let mut v = snapshot();
    let corners = |x0: f64, y0: f64, x1: f64, y1: f64| json!([{ "x": x0, "y": y0 }, { "x": x1, "y": y0 }, { "x": x1, "y": y1 }, { "x": x0, "y": y1 }]);
    v["entities"].as_array_mut().unwrap().push(json!({
        "kind": "polygon", "id": 99, "layerId": "bos", "attrs": {},
        "pts": corners(200.0, 0.0, 220.0, 20.0), "holes": [{ "pts": corners(205.0, 5.0, 209.0, 10.0) }]
    }));
    v["uids"].as_array_mut().unwrap().push(json!(uid(99)));
    let f = facts(v);
    let bos = f
        .layers
        .iter()
        .find(|l| l.id == "bos")
        .expect("the layer now holds an area");
    assert_eq!(bos.area, 380.0);
    assert_eq!(bos.gaps.len(), 1, "{:#?}", bos.gaps);
    assert_eq!(
        (bos.gaps[0].area, bos.gaps[0].at, bos.gaps[0].around.clone()),
        (20.0, [207.0, 7.5], vec![uid(99)])
    );
}
