//! A wall's drawing (docs/adr/0339 §1) against the independent reference's cases (fixtures/walls/v1/cases.json,
//! scripts/fixtures/wall_cases.py): each line of the wall (its vertices and bulges within 1e-9), the body's area, and
//! the pieces at the three details in their order: the fill or the hatches (a pattern's name, turn and scale), the two
//! faces, the caps, the boundaries between shown components (each on its line); a wall that cannot be drawn refused.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::wall::{self, Detail, Spec, View};
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/cases.json");
const TOL: f64 = 1e-9;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn num(name: &str, what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= TOL * js_max(want.abs(), 1.0),
        "{name}: {what} {got}, başvuru {want}"
    );
}

fn wall_of(v: &Value) -> (Vec<Vec2>, Option<Vec<f64>>, Spec) {
    let pts = v["pts"].as_array().unwrap().iter().map(pt).collect();
    let bulges = v["bulges"]
        .as_array()
        .map(|b| b.iter().map(|x| x.as_f64().unwrap()).collect());
    let spec = Spec::read_flat(&Json::parse(&v.to_string()).unwrap()).unwrap();
    (pts, bulges, spec)
}

fn same_line(name: &str, what: &str, got: &wall::Line, want: &Value) {
    let pts = want["pts"].as_array().unwrap();
    assert_eq!(got.pts.len(), pts.len(), "{name}: {what} köşe sayısı");
    for (k, (g, w)) in got.pts.iter().zip(pts).enumerate() {
        let w = pt(w);
        num(name, &format!("{what} {k}. köşe x"), g.x, w.x);
        num(name, &format!("{what} {k}. köşe y"), g.y, w.y);
    }
    for (k, (g, w)) in got
        .bulges
        .iter()
        .zip(want["bulges"].as_array().unwrap())
        .enumerate()
    {
        num(name, &format!("{what} {k}. büküm"), *g, w.as_f64().unwrap());
    }
}

#[test]
fn every_wall_draws_as_the_reference_says() {
    let doc: Value = serde_json::from_str(CASES).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 16);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let (pts, bulges, spec) = wall_of(&c["wall"]);
        let got = wall::lines(&pts, bulges.as_deref(), &spec);
        if c["collapses"].as_bool() == Some(true) {
            assert!(got.is_err(), "{name}: çizilmemeli");
            continue;
        }
        let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        let want = c["lines"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{name}: çizgi sayısı");
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            same_line(name, &format!("{k}. çizgi"), g, w);
        }
        num(
            name,
            "alan",
            wall::area(&pts, bulges.as_deref(), &spec).unwrap(),
            c["area"].as_f64().unwrap(),
        );
        // The same through the shape the store expands.
        let shape = Shape::Wall {
            pts: pts.clone(),
            bulges: bulges.clone(),
            wall: spec.clone(),
        };
        num(
            name,
            "nesnenin alanı",
            kentos_geometry_core::entity::entity_area(&shape).unwrap(),
            c["area"].as_f64().unwrap(),
        );
        for (detail, scale, key) in [
            (Detail::Schematic, 100.0, "schematic"),
            (Detail::Standard, 50.0, "standard"),
            (Detail::Detailed, 20.0, "detailed"),
        ] {
            let pieces =
                wall::pieces(&pts, bulges.as_deref(), &spec, View { detail, scale }).unwrap();
            let want = c["pieces"][key].as_array().unwrap();
            assert_eq!(pieces.len(), want.len(), "{name} ({key}): parça sayısı");
            let mut faces = 0;
            for (k, (p, w)) in pieces.iter().zip(want).enumerate() {
                let at = format!("{name} ({key}) {k}. parça");
                match (w["kind"].as_str().unwrap(), &p.shape) {
                    ("fill", Shape::Hatch { pattern, .. }) => {
                        assert_eq!(pattern.kind, "solid", "{at}");
                        assert_eq!(p.color.as_deref(), w["color"].as_str(), "{at}: renk");
                    }
                    ("hatch", Shape::Hatch { pattern, .. }) if w["pattern"] == "SOLID" => {
                        assert_eq!(pattern.kind, "solid", "{at}");
                    }
                    ("hatch", Shape::Hatch { pattern, .. }) => {
                        assert_eq!(pattern.kind, "pattern", "{at}");
                        assert_eq!(
                            pattern.name.as_deref(),
                            w["pattern"].as_str(),
                            "{at}: desen"
                        );
                        num(&at, "açı", pattern.angle, w["angle"].as_f64().unwrap());
                        num(
                            &at,
                            "ölçek",
                            pattern.scale.unwrap(),
                            w["scale"].as_f64().unwrap(),
                        );
                        assert!(
                            pattern.lines.as_ref().is_some_and(|l| !l.is_empty()),
                            "{at}: desenin çizgileri"
                        );
                    }
                    ("face", Shape::Polyline { pts: fp, .. }) => {
                        let line = if faces == 0 {
                            &got[0]
                        } else {
                            &got[got.len() - 1]
                        };
                        faces += 1;
                        assert_eq!(fp, &line.pts, "{at}: yüz");
                    }
                    ("cap", Shape::Line { .. }) => {}
                    ("boundary", Shape::Polyline { pts: bp, .. }) => {
                        let k = w["at"].as_u64().unwrap() as usize;
                        assert_eq!(bp, &got[k].pts, "{at}: {k}. sınır");
                    }
                    (kind, shape) => panic!("{at}: başvuru {kind}, çekirdek {shape:?}"),
                }
            }
        }
    }
}
