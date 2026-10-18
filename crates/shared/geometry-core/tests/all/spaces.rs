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
