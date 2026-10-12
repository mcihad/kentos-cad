//! Places on objects and how they follow edits (docs/adr/0249) against the
//! independent reference's cases (fixtures/subref/v1/cases.json,
//! scripts/fixtures/subref_cases.py): every place's point, and where a place
//! goes when its object is moved, gets or loses vertices, holes and parts,
//! changes kind or is deleted.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::ops::subref::{Followed, Place, follow, point};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/subref/v1/cases.json");

fn entity(v: &Value) -> Entity {
    Entity::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("an entity")
}

fn place(v: &Value) -> Place {
    let n = |k: &str| v[k].as_u64().expect("a number") as usize;
    match v["kind"].as_str().expect("a kind") {
        "node" => Place::Node { part: n("part") },
        "start" => Place::Start,
        "end" => Place::End,
        "center" => Place::Center,
        "vertex" => Place::Vertex { part: n("part"), ring: n("ring"), index: n("index") },
        "mid" => Place::Mid { part: n("part"), ring: n("ring"), index: n("index") },
        other => panic!("{other} yeri yok"),
    }
}

fn same(name: &str, got: Vec2, want: &Value, exact: bool) {
    let (x, y) = (want["x"].as_f64().unwrap(), want["y"].as_f64().unwrap());
    if exact {
        assert!(got.x == x && got.y == y, "{name}: {got:?} ≠ ({x}, {y})");
    } else {
        assert!((got.x - x).abs() <= 1e-9 && (got.y - y).abs() <= 1e-9, "{name}: {got:?} ≠ ({x}, {y})");
    }
}

#[test]
fn every_place_is_where_the_reference_puts_it() {
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let points = cases["points"].as_array().expect("points");
    assert!(points.len() >= 15);
    for c in points {
        let name = c["name"].as_str().unwrap();
        let got = point(&entity(&c["entity"]), place(&c["place"]));
        match (&c["point"], got) {
            (Value::Null, None) => {}
            (want, Some(p)) if !want.is_null() => same(name, p, want, c["exact"].as_bool().unwrap()),
            (want, got) => panic!("{name}: {got:?}, başvuru {want}"),
        }
    }
}

#[test]
fn every_place_follows_its_object_as_the_reference_says() {
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let follows = cases["follows"].as_array().expect("follows");
    assert!(follows.len() >= 20);
    let mut broken = 0;
    for c in follows {
        let name = c["name"].as_str().unwrap();
        let before = entity(&c["before"]);
        let after = (!c["after"].is_null()).then(|| entity(&c["after"]));
        let got = follow(place(&c["place"]), &before, after.as_ref());
        let want = &c["expect"];
        match got {
            Followed::At(at, p) => {
                assert!(want["broken"].is_null(), "{name}: izledi, başvuru {}", want["broken"]);
                assert_eq!(at, place(&want["at"]), "{name}");
                same(name, p, &want["point"], c["exact"].as_bool().unwrap());
            }
            Followed::Broken(reason) => {
                broken += 1;
                assert_eq!(Some(reason.code()), want["broken"].as_str(), "{name}");
            }
        }
    }
    // Every reason is met.
    assert!(broken >= 8, "{broken}");
}

#[test]
fn a_snap_names_the_place_the_reference_names() {
    use kentos_geometry_core::ops::subref::place_at;
    use kentos_geometry_core::store::snap::SnapKind;
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let snaps = cases["snaps"].as_array().expect("snaps");
    assert!(snaps.len() >= 10);
    for c in snaps {
        let name = c["name"].as_str().unwrap();
        let kind = match c["snap"].as_str().unwrap() {
            "endpoint" => SnapKind::Endpoint,
            "midpoint" => SnapKind::Midpoint,
            "center" => SnapKind::Center,
            "node" => SnapKind::Node,
            "nearest" => SnapKind::Nearest,
            other => panic!("{other}"),
        };
        let at = Vec2::new(c["at"]["x"].as_f64().unwrap(), c["at"]["y"].as_f64().unwrap());
        let got = place_at(&entity(&c["entity"]), kind, at);
        let want = (!c["place"].is_null()).then(|| place(&c["place"]));
        assert_eq!(got, want, "{name}");
    }
}
