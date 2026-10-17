//! The constraint solver (docs/adr/0334) against the independent reference in `fixtures/constraints/v1/cases.json`
//! (`scripts/fixtures/constraint_cases.py`: the equations written again with sympy, ranks and solutions in mpmath at
//! 50 digits; no KentOS code). With `KENTOS_CONSTRAINTS_OUT=FILE` the solved shapes are written for the reference's
//! `--outputs FILE`, which checks the cases with freedoms left, whose solution is not unique.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::constraints::{
    Axis, Constraint, Element, Hold, PointAt, Problem, Ref, Rule, Status, solve,
};
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/constraints/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(f(&v[0]), f(&v[1]))
}

fn shape(v: &Value) -> Shape {
    let (kind, v) = v
        .as_object()
        .and_then(|o| o.iter().next())
        .expect("a shape");
    match kind.as_str() {
        "point" => Shape::Point {
            p: pt(v),
            z: None,
            parts: None,
        },
        "line" => Shape::Line {
            a: pt(&v[0]),
            b: pt(&v[1]),
        },
        "circle" => Shape::Circle {
            c: pt(&v[0]),
            r: f(&v[1]),
        },
        "arc" => Shape::Arc {
            c: pt(&v[0]),
            r: f(&v[1]),
            a0: f(&v[2]),
            a1: f(&v[3]),
        },
        "polyline" | "polygon" => {
            let pts = v["pts"]
                .as_array()
                .expect("points")
                .iter()
                .map(pt)
                .collect();
            let bulges = v["bulges"].as_array().map(|b| b.iter().map(f).collect());
            if kind == "polygon" {
                Shape::Polygon {
                    pts,
                    bulges,
                    holes: None,
                    parts: None,
                }
            } else {
                Shape::Polyline {
                    pts,
                    bulges,
                    holes: None,
                    parts: None,
                    closed: v["closed"].as_bool().unwrap_or(false),
                    widths: None,
                }
            }
        }
        other => panic!("{other}"),
    }
}

fn shape_json(s: &Shape) -> Value {
    let p = |v: &Vec2| json!([v.x, v.y]);
    match s {
        Shape::Point { p: q, .. } => json!({ "point": p(q) }),
        Shape::Line { a, b } => json!({ "line": [p(a), p(b)] }),
        Shape::Circle { c, r } => json!({ "circle": [p(c), r] }),
        Shape::Arc { c, r, a0, a1 } => json!({ "arc": [p(c), r, a0, a1] }),
        Shape::Polygon { pts, .. } => {
            json!({ "polygon": { "pts": pts.iter().map(p).collect::<Vec<_>>() } })
        }
        Shape::Polyline {
            pts,
            bulges,
            closed,
            ..
        } => json!({ "polyline": {
            "pts": pts.iter().map(p).collect::<Vec<_>>(), "bulges": bulges, "closed": closed } }),
        _ => Value::Null,
    }
}

fn triple(v: &Value) -> (u32, u32, u32) {
    let n = |i: usize| v[i].as_u64().expect("an index") as u32;
    (n(0), n(1), n(2))
}

fn element(v: &Value) -> Element {
    if v == "circle" {
        return Element::Circle;
    }
    if let Some(s) = v.get("segment") {
        let (part, ring, index) = triple(s);
        return Element::Segment { part, ring, index };
    }
    if let Some(s) = v.get("vertex") {
        let (part, ring, index) = triple(s);
        return Element::Point(PointAt::Vertex { part, ring, index });
    }
    Element::Point(match v["point"].as_str().expect("a point") {
        "start" => PointAt::Start,
        "end" => PointAt::End,
        "center" => PointAt::Center,
        "node" => PointAt::Node(0),
        other => panic!("{other}"),
    })
}

fn reference(v: &Value) -> Ref {
    Ref {
        object: v[0].as_u64().expect("an object") as usize,
        element: element(&v[1]),
    }
}

fn constraint(v: &Value) -> Constraint {
    let a = reference(&v["a"]);
    let b = || reference(&v["b"]);
    let flag = |k: &str| v[k].as_bool().unwrap_or(false);
    let rule = match v["rule"].as_str().expect("a rule") {
        "coincident" => Rule::Coincident { a, b: b() },
        "horizontal" => Rule::Horizontal {
            a,
            b: v.get("b").map(reference),
        },
        "vertical" => Rule::Vertical {
            a,
            b: v.get("b").map(reference),
        },
        "parallel" => Rule::Parallel { a, b: b() },
        "perpendicular" => Rule::Perpendicular { a, b: b() },
        "tangent" => Rule::Tangent {
            a,
            b: b(),
            inside: flag("inside"),
            left: flag("left"),
        },
        "concentric" => Rule::Concentric { a, b: b() },
        "collinear" => Rule::Collinear { a, b: b() },
        "equal" => Rule::Equal { a, b: b() },
        "symmetric" => Rule::Symmetric {
            a,
            b: b(),
            axis: reference(&v["axis"]),
        },
        "fix" => Rule::Fix { a },
        "distance" => Rule::Distance {
            a,
            b: b(),
            axis: match v["axis"].as_str().unwrap_or("aligned") {
                "horizontal" => Axis::Horizontal,
                "vertical" => Axis::Vertical,
                _ => Axis::Aligned,
            },
            left: flag("left"),
        },
        "angle" => Rule::Angle {
            a,
            b: b(),
            reverse_a: flag("reverseA"),
            reverse_b: flag("reverseB"),
        },
        "radius" => Rule::Radius { a },
        "diameter" => Rule::Diameter { a },
        other => panic!("{other}"),
    };
    Constraint {
        rule,
        value: v["value"].as_f64().unwrap_or(0.0),
        reference: flag("reference"),
    }
}

fn status(s: Status) -> &'static str {
    match s {
        Status::Fully => "fully",
        Status::Under => "under",
        Status::Over => "over",
        Status::Inconsistent => "inconsistent",
        Status::Singular => "singular",
        Status::NonConvergent => "nonConvergent",
        Status::Cancelled => "cancelled",
    }
}

fn indices(v: &Value) -> Vec<usize> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|i| i.as_u64().expect("an index") as usize)
        .collect()
}

fn close_shape(got: &Shape, want: &Value, what: &str) {
    let got = shape_json(got);
    let flat = |v: &Value, out: &mut Vec<f64>| {
        fn walk(v: &Value, out: &mut Vec<f64>) {
            match v {
                Value::Number(n) => out.push(n.as_f64().unwrap_or(f64::NAN)),
                Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                Value::Object(o) => {
                    if let Some(p) = o.get("pts") {
                        walk(p, out)
                    } else {
                        o.values().for_each(|x| walk(x, out))
                    }
                }
                _ => {}
            }
        }
        walk(v, out)
    };
    let (mut g, mut w) = (Vec::new(), Vec::new());
    flat(&got, &mut g);
    flat(want, &mut w);
    assert_eq!(g.len(), w.len(), "{what}");
    for (a, b) in g.iter().zip(&w) {
        // Arc angles may differ by a turn.
        let d = (a - b).abs();
        let turn = std::f64::consts::TAU;
        assert!(
            d < 1e-8 || (d - turn).abs() < 1e-8,
            "{what}: {got} ≠ {want}"
        );
    }
}

#[test]
fn the_solver_meets_the_reference() {
    let all = cases();
    let mut written = Vec::new();
    for case in all["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let shapes: Vec<Shape> = case["shapes"]
            .as_array()
            .expect("shapes")
            .iter()
            .map(shape)
            .collect();
        let fix_from: Option<Vec<Shape>> = case
            .get("fixFrom")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(shape).collect());
        let constraints: Vec<Constraint> = case["constraints"]
            .as_array()
            .expect("constraints")
            .iter()
            .map(constraint)
            .collect();
        let held: Vec<Hold> = case
            .get("held")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|h| Hold {
                        object: h[0].as_u64().expect("an object") as usize,
                        element: (!h[1].is_null()).then(|| element(&h[1])),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let out = solve(
            &Problem {
                shapes: &shapes,
                constraints: &constraints,
                held: &held,
                fix_from: fix_from.as_deref(),
            },
            &|| false,
        );
        let e = &case["expect"];
        assert_eq!(status(out.status), e["status"], "{name}: {out:?}");
        assert_eq!(out.dof as u64, e["dof"].as_u64().expect("dof"), "{name}");
        assert_eq!(out.redundant, indices(&e["redundant"]), "{name}: fazla");
        assert_eq!(
            out.conflicting,
            indices(&e["conflicting"]),
            "{name}: çelişen"
        );
        assert_eq!(out.singular, indices(&e["singular"]), "{name}: tekil");
        assert_eq!(out.broken, indices(&e["broken"]), "{name}: kopuk");
        assert_eq!(out.free, indices(&e["free"]), "{name}: serbest");
        let measured = e["measured"].as_array().expect("measured");
        assert_eq!(out.measured.len(), measured.len(), "{name}: ölçülen");
        for ((i, v), m) in out.measured.iter().zip(measured) {
            assert_eq!(*i as u64, m[0].as_u64().expect("an index"), "{name}");
            assert!(
                (v - f(&m[1])).abs() < 1e-9,
                "{name}: ölçülen {v} ≠ {}",
                m[1]
            );
        }
        if let Some(want) = e.get("shapes").and_then(Value::as_array) {
            for (k, (g, w)) in out.shapes.iter().zip(want).enumerate() {
                close_shape(g, w, &format!("{name}: nesne {k}"));
            }
        }
        written.push(json!({
            "name": name,
            "status": status(out.status),
            "shapes": out.shapes.iter().map(shape_json).collect::<Vec<_>>(),
        }));
    }
    if let Ok(path) = std::env::var("KENTOS_CONSTRAINTS_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&written).expect("JSON"))
            .expect("written");
    }
}
