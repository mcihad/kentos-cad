//! A roof's faces and plan lines (docs/adr/8044 §2) against the independent reference's cases
//! (fixtures/arch/v1/roofs.json, scripts/fixtures/roof_cases.py, exact fractions): each face's edge, outline (the same
//! points in the same turn, from any of them) and slope; the eaves and gable ends in order, the ridges and hips as a
//! set (from their higher end; a level ridge either way round); the rise, within 1e-7.

use kentos_geometry_core::geom::roof::{Line, RoofSpec, faces, height_at, lines, ridge_rise};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/roofs.json");
const TOL: f64 = 1e-7;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL
}

fn same_ring(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len()
        && (0..a.len()).any(|k| (0..a.len()).all(|i| close(a[(i + k) % a.len()], b[i])))
}

fn kind(word: &str) -> Line {
    match word {
        "eaves" => Line::Eaves,
        "gable" => Line::Gable,
        "ridge" => Line::Ridge,
        "hip" => Line::Hip,
        _ => Line::Valley,
    }
}

#[test]
fn every_roof_case_matches_the_reference() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let mut read = 0;
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let pts: Vec<Vec2> = c["pts"].as_array().unwrap().iter().map(pt).collect();
        let spec = RoofSpec {
            slopes: c["slopes"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::as_f64)
                .collect(),
            base: 0.0,
            ..Default::default()
        };
        let want = &c["expect"];
        let got = faces(&pts, &spec).unwrap();
        let want_faces = want["faces"].as_array().unwrap();
        assert_eq!(got.len(), want_faces.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(want_faces) {
            let ring: Vec<Vec2> = w["ring"].as_array().unwrap().iter().map(pt).collect();
            assert_eq!(g.edge as u64, w["edge"].as_u64().unwrap(), "{name}");
            assert!(
                same_ring(&g.ring, &ring),
                "{name}: face {} {:?} ≠ {ring:?}",
                g.edge,
                g.ring
            );
            assert!(
                (g.slope - w["slope"].as_f64().unwrap()).abs() <= TOL,
                "{name}"
            );
        }
        let got = lines(&pts, &spec).unwrap();
        let want_lines = want["lines"].as_array().unwrap();
        assert_eq!(got.len(), want_lines.len(), "{name}: {got:?}");
        let n = pts.len();
        for (k, (g, w)) in got.iter().zip(want_lines).enumerate().take(n) {
            assert_eq!(
                g.kind,
                kind(w["kind"].as_str().unwrap()),
                "{name}: edge {k}"
            );
            assert!(
                close(g.a, pt(&w["a"])) && close(g.b, pt(&w["b"])),
                "{name}: edge {k}"
            );
        }
        for w in &want_lines[n..] {
            let (k, a, b) = (kind(w["kind"].as_str().unwrap()), pt(&w["a"]), pt(&w["b"]));
            // A level ridge has no higher end: either way round.
            let z = |p: Vec2| height_at(&pts, &spec, p).unwrap();
            let level = k == Line::Ridge && (z(a) - z(b)).abs() <= TOL;
            assert!(
                got[n..].iter().any(|g| g.kind == k
                    && ((close(g.a, a) && close(g.b, b))
                        || (level && close(g.a, b) && close(g.b, a)))),
                "{name}: no {k:?} from {a:?} to {b:?} in {:?}",
                &got[n..]
            );
        }
        assert!(
            (ridge_rise(&pts, &spec).unwrap() - want["rise"].as_f64().unwrap()).abs() <= TOL,
            "{name}"
        );
        read += 1;
    }
    assert_eq!(read, 9, "every case read");
}
