//! Places on objects and how they follow edits (docs/adr/0249) against the
//! independent reference's cases (fixtures/subref/v1/cases.json,
//! scripts/fixtures/subref_cases.py): every place's point, and where a place
//! goes when its object is moved, gets or loses vertices, holes and parts,
//! changes kind or is deleted; places along an edge or a curve, ranges and
//! pieces inside blocks (docs/adr/0263).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::ops::subref::{
    Followed, Place, follow, follow_in, follow_range, place_in, point, point_in, range_length,
};
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
        "vertex" => Place::Vertex {
            part: n("part"),
            ring: n("ring"),
            index: n("index"),
        },
        "mid" => Place::Mid {
            part: n("part"),
            ring: n("ring"),
            index: n("index"),
        },
        "along" => Place::Along {
            part: n("part"),
            ring: n("ring"),
            index: n("index"),
            t: v["t"].as_f64().expect("t"),
        },
        other => panic!("{other} yeri yok"),
    }
}

/// Two places the same; an along place's fraction within 1e-12 (it comes from trigonometry).
fn same_place(name: &str, got: Place, want: Place) {
    match (got, want) {
        (
            Place::Along {
                part,
                ring,
                index,
                t,
            },
            Place::Along {
                part: p,
                ring: r,
                index: i,
                t: u,
            },
        ) => {
            assert_eq!((part, ring, index), (p, r, i), "{name}");
            assert!((t - u).abs() <= 1e-12, "{name}: t {t} ≠ {u}");
        }
        _ => assert_eq!(got, want, "{name}"),
    }
}

fn same(name: &str, got: Vec2, want: &Value, exact: bool) {
    let (x, y) = (want["x"].as_f64().unwrap(), want["y"].as_f64().unwrap());
    if exact {
        assert!(got.x == x && got.y == y, "{name}: {got:?} ≠ ({x}, {y})");
    } else {
        assert!(
            (got.x - x).abs() <= 1e-9 && (got.y - y).abs() <= 1e-9,
            "{name}: {got:?} ≠ ({x}, {y})"
        );
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
            (want, Some(p)) if !want.is_null() => {
                same(name, p, want, c["exact"].as_bool().unwrap())
            }
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
                assert!(
                    want["broken"].is_null(),
                    "{name}: izledi, başvuru {}",
                    want["broken"]
                );
                same_place(name, at, place(&want["at"]));
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
            "insertion" => SnapKind::Insertion,
            "nearest" => SnapKind::Nearest,
            "perpendicular" => SnapKind::Perpendicular,
            "tangent" => SnapKind::Tangent,
            "quadrant" => SnapKind::Quadrant,
            "intersection" => SnapKind::Intersection,
            other => panic!("{other}"),
        };
        let at = Vec2::new(
            c["at"]["x"].as_f64().unwrap(),
            c["at"]["y"].as_f64().unwrap(),
        );
        let got = place_at(&entity(&c["entity"]), kind, at);
        let want = (!c["place"].is_null()).then(|| place(&c["place"]));
        match (got, want) {
            (Some(g), Some(w)) => same_place(name, g, w),
            (g, w) => assert_eq!(g, w, "{name}"),
        }
    }
}

#[test]
fn a_range_has_the_length_the_reference_gives_and_follows_both_ends() {
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let ranges = cases["ranges"].as_array().expect("ranges");
    assert!(ranges.len() >= 10);
    for c in ranges {
        let name = c["name"].as_str().unwrap();
        let got = range_length(&entity(&c["entity"]), place(&c["from"]), place(&c["to"]));
        match (got, c["length"].as_f64()) {
            (None, None) => {}
            (Some(g), Some(w)) => {
                assert!((g - w).abs() <= 1e-9 * (1.0 + w.abs()), "{name}: {g} ≠ {w}")
            }
            (g, w) => panic!("{name}: {g:?}, başvuru {w:?}"),
        }
    }
    for c in cases["rangeFollows"].as_array().expect("range follows") {
        let name = c["name"].as_str().unwrap();
        let after = (!c["after"].is_null()).then(|| entity(&c["after"]));
        let got = follow_range(
            place(&c["from"]),
            place(&c["to"]),
            &entity(&c["before"]),
            after.as_ref(),
        );
        let want = &c["expect"];
        match got {
            Ok((a, b)) => {
                same_place(name, a, place(&want["from"]));
                same_place(name, b, place(&want["to"]));
            }
            Err(r) => assert_eq!(Some(r.code()), want["broken"].as_str(), "{name}"),
        }
    }
}

fn blocks(v: &Value) -> Blocks {
    Blocks::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("definitions")
}

fn path(v: &Value) -> Vec<u32> {
    v.as_array()
        .expect("a path")
        .iter()
        .map(|n| n.as_u64().expect("a number") as u32)
        .collect()
}

#[test]
fn a_piece_inside_a_block_is_where_the_reference_puts_it_and_follows_its_insert() {
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let pieces = cases["pieces"].as_array().expect("pieces");
    assert!(pieces.len() >= 8);
    for c in pieces {
        let name = c["name"].as_str().unwrap();
        let got = point_in(
            &blocks(&c["definitions"]),
            &entity(&c["insert"]),
            &path(&c["path"]),
            place(&c["place"]),
        );
        match (&c["point"], got) {
            (Value::Null, None) => {}
            (want, Some(p)) if !want.is_null() => same(name, p, want, false),
            (want, got) => panic!("{name}: {got:?}, başvuru {want}"),
        }
    }
    let mut gone = 0;
    for c in cases["pieceFollows"].as_array().expect("piece follows") {
        let name = c["name"].as_str().unwrap();
        let (bb, ab) = (
            blocks(&c["before"]["definitions"]),
            blocks(&c["after"]["definitions"]),
        );
        let before = entity(&c["before"]["insert"]);
        let after = (!c["after"]["insert"].is_null()).then(|| entity(&c["after"]["insert"]));
        let got = follow_in(
            place(&c["place"]),
            &path(&c["path"]),
            (&bb, &before),
            after.as_ref().map(|a| (&ab, a)),
        );
        let want = &c["expect"];
        match got {
            Followed::At(at, p) => {
                assert!(
                    want["broken"].is_null(),
                    "{name}: izledi, başvuru {}",
                    want["broken"]
                );
                same_place(name, at, place(&want["at"]));
                same(name, p, &want["point"], false);
            }
            Followed::Broken(r) => {
                gone += usize::from(r.code() == "piece_gone");
                assert_eq!(Some(r.code()), want["broken"].as_str(), "{name}");
            }
        }
    }
    assert!(gone >= 1);
    for c in cases["pieceSnaps"].as_array().expect("piece snaps") {
        use kentos_geometry_core::store::snap::SnapKind;
        let name = c["name"].as_str().unwrap();
        let kind = match c["snap"].as_str().unwrap() {
            "endpoint" => SnapKind::Endpoint,
            "center" => SnapKind::Center,
            "nearest" => SnapKind::Nearest,
            other => panic!("{other}"),
        };
        let at = Vec2::new(
            c["at"]["x"].as_f64().unwrap(),
            c["at"]["y"].as_f64().unwrap(),
        );
        let got = place_in(&blocks(&c["definitions"]), &entity(&c["insert"]), kind, at);
        match (got, &c["expect"]) {
            (None, Value::Null) => {}
            (Some((p, pl)), want) if !want.is_null() => {
                assert_eq!(p, path(&want["path"]), "{name}");
                same_place(name, pl, place(&want["place"]));
            }
            (got, want) => panic!("{name}: {got:?}, başvuru {want}"),
        }
    }
}
