//! Spaces from walls (docs/adr/0342 §2) against the independent reference's cases (fixtures/walls/v1/spaces.json,
//! scripts/fixtures/space_cases.py): the net face round the seed, the face moved by each method, their areas and
//! perimeters. Rings are compared as their vertices without collinear ones, in any order.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::space::{self, Method};
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::jsmath::{js_max, js_round};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/spaces.json");
const TOL: f64 = 1e-9;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

/// The ring's vertices without collinear ones, sorted.
fn plain(r: &Ring) -> Vec<Vec2> {
    let mut out = r.pts.clone();
    loop {
        let n = out.len();
        if n <= 3 {
            break;
        }
        let k = (0..n).find(|&i| {
            let (a, b, c) = (out[(i + n - 1) % n], out[i], out[(i + 1) % n]);
            let cr = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            cr.abs() <= 1e-9
        });
        match k {
            Some(i) => {
                out.remove(i);
            }
            None => break,
        }
    }
    let key = |p: &Vec2| (js_round(p.x * 1e6) as i64, js_round(p.y * 1e6) as i64);
    out.sort_by_key(key);
    out
}

fn same_points(name: &str, got: &[Vec2], want: &Value) {
    let want: Vec<Vec2> = want.as_array().unwrap().iter().map(pt).collect();
    assert_eq!(
        got.len(),
        want.len(),
        "{name}: köşeler {got:?}, başvuru {want:?}"
    );
    for (a, b) in got.iter().zip(&want) {
        assert!(
            (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL,
            "{name}: köşe {a:?}, başvuru {b:?}"
        );
    }
}

#[test]
fn spaces_are_found_and_measured_as_the_reference_says() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let walls: Vec<(Vec<Vec2>, Spec)> = c["walls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                let pts = w["pts"].as_array().unwrap().iter().map(pt).collect();
                (
                    pts,
                    Spec::read_flat(&Json::parse(&w.to_string()).unwrap()).unwrap(),
                )
            })
            .collect();
        let ins: Vec<WallIn<'_>> = walls
            .iter()
            .map(|(pts, spec)| WallIn {
                pts,
                bulges: None,
                spec,
            })
            .collect();
        let faces = space::wall_faces(&ins);
        let net = space::net_face(&faces, pt(&c["seed"]));
        if c["net"].is_null() {
            assert!(net.is_none(), "{name}: başvuruda mahal yok, çekirdek buldu");
            continue;
        }
        let net = net.unwrap_or_else(|| panic!("{name}: çekirdek mahali bulamadı"));
        for (key, method) in [
            ("net", Method::Net),
            ("centerline", Method::Centerline),
            ("gross", Method::Gross),
        ] {
            let want = &c[key];
            let a = space::by_method(&net, &faces, method);
            let label = format!("{name} ({key})");
            same_points(&label, &plain(&a.outer), &want["outer"]);
            let mut holes: Vec<Vec<Vec2>> = a.holes.iter().map(plain).collect();
            holes.sort_by(|x, y| x[0].x.total_cmp(&y[0].x).then(x[0].y.total_cmp(&y[0].y)));
            let wh = want["holes"].as_array().unwrap();
            assert_eq!(holes.len(), wh.len(), "{label}: delikler");
            for (h, w) in holes.iter().zip(wh) {
                same_points(&label, h, w);
            }
            let area = space::area(std::slice::from_ref(&a));
            let wa = want["area"].as_f64().unwrap();
            assert!(
                (area - wa).abs() <= TOL * js_max(wa, 1.0),
                "{label}: alan {area}, başvuru {wa}"
            );
            let per = space::perimeter(std::slice::from_ref(&a));
            let wp = want["perimeter"].as_f64().unwrap();
            assert!(
                (per - wp).abs() <= TOL * js_max(wp, 1.0),
                "{label}: çevre {per}, başvuru {wp}"
            );
        }
    }
}

/// A void's cross (docs/adr/0708): a rectangle's diagonals run corner to corner; an L's are kept inside it; a hole's
/// part of them is left out.
#[test]
fn a_void_is_crossed_inside_its_parts() {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::arrangement::Area;
    let ring = |pts: &[(f64, f64)]| Ring {
        pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        bulges: None,
    };
    let lines = |shapes: Vec<Shape>| -> Vec<(Vec2, Vec2)> {
        shapes
            .into_iter()
            .map(|s| match s {
                Shape::Line { a, b } => (a, b),
                other => panic!("a line, not {other:?}"),
            })
            .collect()
    };
    let near = |p: Vec2, x: f64, y: f64| (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12;
    // A 0.8 × 0.6 shaft.
    let shaft = Area { outer: ring(&[(1.0, 1.0), (1.8, 1.0), (1.8, 1.6), (1.0, 1.6)]), holes: vec![] };
    let cross = lines(space::void_cross(&[shaft]));
    assert_eq!(cross.len(), 2);
    assert!(near(cross[0].0, 1.0, 1.0) && near(cross[0].1, 1.8, 1.6), "{cross:?}");
    assert!(near(cross[1].0, 1.8, 1.0) && near(cross[1].1, 1.0, 1.6), "{cross:?}");
    // An L (4 × 4 less its upper right 2 × 2): the rectangle's diagonal from (0, 0) leaves it at (2, 2); the other
    // runs from (4, 0) to (2, 2) and again from (2, 2) to (0, 4).
    let l = Area {
        outer: ring(&[(0.0, 0.0), (4.0, 0.0), (4.0, 2.0), (2.0, 2.0), (2.0, 4.0), (0.0, 4.0)]),
        holes: vec![],
    };
    let cross = lines(space::void_cross(&[l]));
    let length: f64 = cross
        .iter()
        .map(|(a, b)| kentos_geometry_core::jsmath::js_hypot(b.x - a.x, b.y - a.y))
        .sum();
    assert!((length - 3.0 * 8f64.sqrt()).abs() < 1e-9, "{cross:?}");
    // A 4 × 4 void with a 2 × 2 hole in its middle: each diagonal in two pieces.
    let holed = Area {
        outer: ring(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]),
        holes: vec![ring(&[(1.0, 1.0), (1.0, 3.0), (3.0, 3.0), (3.0, 1.0)])],
    };
    let cross = lines(space::void_cross(&[holed]));
    assert_eq!(cross.len(), 4, "{cross:?}");
    // The space's pieces carry the cross only when it is a void.
    let room = space::Spec::default();
    let shaft = Area { outer: ring(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]), holes: vec![] };
    let plain = space::pieces(std::slice::from_ref(&shaft), &room, 50.0);
    let void = space::pieces(&[shaft], &space::Spec { void: true, ..room }, 50.0);
    assert_eq!(void.len(), plain.len() + 2);
}
