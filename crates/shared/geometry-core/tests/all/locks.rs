//! The digitizing locks (docs/adr/0166) against the independent reference in
//! `fixtures/locks/v1/cases.json` (`scripts/fixtures/lock_cases.py`: exact
//! fractions, the angles with 50-digit mpmath, no KentOS code): the locked
//! point, the cursor rules with the locks, the directions of a typed angle
//! and of a deflection, Dik kapat's corner, lock text and a picked edge's
//! direction; points within
//! 1e-8 m (a TM northing's double steps 0.93 nm), unit directions within
//! 1e-14, none where the reference has none, lock text exactly. The ops the
//! web calls are run on the same cases through the call table.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::tools::locks::{
    Direction, LockText, Locks, constrain_locked, deflected, direction_of, edge_direction,
    lock_point, parse_lock_text, square_corner,
};
use kentos_geometry_core::tools::point_input::{AngleFrom, Angles};
use serde_json::{Value, json};

const POINT: f64 = 1e-8;
const UNIT: f64 = 1e-14;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/locks/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.locks");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(num(&v[0]), num(&v[1]))
}

fn opt_pt(v: &Value) -> Option<Vec2> {
    (!v.is_null()).then(|| pt(v))
}

fn opt_num(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| num(v))
}

fn angles(case: &Value) -> Angles {
    Angles {
        from: if case["fromNorth"].as_bool().expect("fromNorth") {
            AngleFrom::North
        } else {
            AngleFrom::East
        },
        grads: case["grads"].as_bool().expect("grads"),
    }
}

fn direction(case: &Value) -> Option<Direction> {
    opt_pt(&case["u"]).map(|u| Direction {
        u,
        both: case["both"].as_bool().expect("both"),
    })
}

/// `got` against the expected point (or none), within `tol`.
fn same(name: &str, got: Option<Vec2>, expected: &Value, tol: f64) {
    match (got, opt_pt(expected)) {
        (None, None) => {}
        (Some(g), Some(e)) => assert!(
            (g.x - e.x).abs() <= tol && (g.y - e.y).abs() <= tol,
            "{name}: ({}, {}) ≠ ({}, {})",
            g.x,
            g.y,
            e.x,
            e.y
        ),
        (g, e) => panic!("{name}: {g:?} ≠ {e:?}"),
    }
}

/// A point as the call table writes it, back to a point (null to none).
fn called(name: &str, args: Value) -> Option<Vec2> {
    let out: Value =
        serde_json::from_str(&run_named(name, &args.to_string()).expect("the op runs"))
            .expect("op JSON");
    (!out.is_null()).then(|| Vec2::new(num(&out["x"]), num(&out["y"])))
}

fn wire(p: Vec2) -> Value {
    json!({ "x": p.x, "y": p.y })
}

fn wire_opt(p: Option<Vec2>) -> Value {
    p.map_or(Value::Null, wire)
}

#[test]
fn the_locked_point() {
    let file = fixture();
    for case in file["lockPoint"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let (o, c) = (pt(&case["o"]), pt(&case["c"]));
        let length = opt_num(&case["length"]);
        let got = lock_point(o, c, length, direction(case));
        same(name, got, &case["expect"], POINT);
        let op = called(
            "lockPoint",
            json!([
                wire(o),
                wire(c),
                case["length"],
                wire_opt(opt_pt(&case["u"])),
                case["both"]
            ]),
        );
        assert_eq!(op, got, "{name}: the op");
    }
}

#[test]
fn the_cursor_rules_with_the_locks() {
    let file = fixture();
    for case in file["constrainLocked"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let got = constrain_locked(
            opt_pt(&case["from"]),
            pt(&case["world"]),
            case["exact"].as_bool().expect("exact"),
            case["ortho"].as_bool().expect("ortho"),
            opt_num(&case["polarStep"]),
            num(&case["tol"]),
            Locks {
                length: opt_num(&case["length"]),
                direction: direction(case),
            },
        );
        let expected = &case["expect"];
        same(name, got.map(|c| c.point), &expected["point"], POINT);
        match (got.and_then(|c| c.tracking), expected.get("tracking")) {
            (None, None | Some(Value::Null)) => {}
            (Some(t), Some(e)) if !e.is_null() => {
                same(name, Some(t.origin), &e["origin"], POINT);
                assert!((t.angle - num(&e["angle"])).abs() <= 1e-9, "{name}: angle");
            }
            (t, e) => panic!("{name}: tracking {t:?} ≠ {e:?}"),
        }
        let args = json!([
            wire_opt(opt_pt(&case["from"])),
            wire(pt(&case["world"])),
            case["exact"],
            case["ortho"],
            case["polarStep"],
            case["tol"],
            case["length"],
            wire_opt(opt_pt(&case["u"])),
            case["both"]
        ]);
        let out: Value = serde_json::from_str(
            &run_named("constrainLocked", &args.to_string()).expect("the op runs"),
        )
        .expect("op JSON");
        let op =
            (!out.is_null()).then(|| Vec2::new(num(&out["point"]["x"]), num(&out["point"]["y"])));
        assert_eq!(op, got.map(|c| c.point), "{name}: the op");
    }
}

#[test]
fn the_directions_of_angles_and_deflections() {
    let file = fixture();
    for case in file["direction"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let got = direction_of(num(&case["angle"]), angles(case));
        same(name, Some(got), &case["expect"], UNIT);
        let op = called(
            "lockDirection",
            json!([case["angle"], case["fromNorth"], case["grads"]]),
        );
        assert_eq!(op, Some(got), "{name}: the op");
    }
    for case in file["deflected"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let (prev, from) = (pt(&case["prev"]), pt(&case["from"]));
        let got = deflected(prev, from, num(&case["angle"]), angles(case));
        same(name, got, &case["expect"], UNIT);
        let op = called(
            "lockDeflected",
            json!([
                wire(prev),
                wire(from),
                case["angle"],
                case["fromNorth"],
                case["grads"]
            ]),
        );
        assert_eq!(op, got, "{name}: the op");
    }
}

#[test]
fn the_square_corner() {
    let file = fixture();
    for case in file["squareCorner"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let [first, second, prev, last] = ["first", "second", "prev", "last"].map(|k| pt(&case[k]));
        let got = square_corner(first, second, prev, last);
        same(name, got, &case["expect"], POINT);
        let op = called(
            "squareCorner",
            json!([wire(first), wire(second), wire(prev), wire(last)]),
        );
        assert_eq!(op, got, "{name}: the op");
    }
}

/// An edge as the cases write it (the web's `Edge`).
fn edge(v: &Value) -> Edge {
    match v["kind"].as_str().expect("kind") {
        "seg" => Edge::Seg {
            a: Vec2::new(num(&v["a"]["x"]), num(&v["a"]["y"])),
            b: Vec2::new(num(&v["b"]["x"]), num(&v["b"]["y"])),
        },
        _ => Edge::Arc {
            c: Vec2::new(num(&v["c"]["x"]), num(&v["c"]["y"])),
            r: num(&v["r"]),
            a0: num(&v["a0"]),
            sweep: num(&v["sweep"]),
        },
    }
}

#[test]
fn the_direction_of_a_picked_edge() {
    let file = fixture();
    for case in file["edgeDirection"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("name");
        let p = pt(&case["p"]);
        let got = edge_direction(&edge(&case["edge"]), p);
        same(name, got, &case["expect"], UNIT);
        let op = called("lockEdgeDirection", json!([case["edge"], wire(p)]));
        assert_eq!(op, got, "{name}: the op");
    }
}

#[test]
fn lock_text() {
    let file = fixture();
    for case in file["lockText"].as_array().expect("cases") {
        let text = case["text"].as_str().expect("text");
        let got = parse_lock_text(text).map(|LockText::Angle(a)| a);
        assert_eq!(got, opt_num(&case["expect"]), "{text:?}");
    }
}
