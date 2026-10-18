//! Grids, columns, stairs and ramps drawn (docs/adr/0343 §2) against the independent reference's cases
//! (fixtures/arch/v1/cases.json, scripts/fixtures/arch_cases.py): bubbles, a column's corners, a stair's risers, band,
//! landings and checks, a ramp's sloped run, slope, levels and checks.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/cases.json");
const TOL: f64 = 1e-9;

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= TOL, "{what}: {a} ≠ {b}");
}

fn points(got: &[kentos_geometry_core::vec2::Vec2], want: &Value, what: &str) {
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{what}: {} nokta", got.len());
    for (g, w) in got.iter().zip(want) {
        close(g.x, w[0].as_f64().unwrap(), what);
        close(g.y, w[1].as_f64().unwrap(), what);
    }
}

fn codes(got: &[arch::Check]) -> Vec<String> {
    got.iter().map(|c| c.code.to_owned()).collect()
}

fn want_codes(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_owned()).collect()
}

#[test]
fn arch_elements_draw_as_the_reference_says() {
    let file: Value = serde_json::from_str(CASES).unwrap();
    for c in file["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let shape = Shape::from_json(&Json::parse(&c["shape"].to_string()).unwrap()).unwrap();
        let scale = c["scale"].as_f64().unwrap();
        let e = &c["expect"];
        match &shape {
            Shape::Grid { .. } => {
                let circles: Vec<_> = arch::pieces(&shape, scale)
                    .into_iter()
                    .filter_map(|p| match p.shape {
                        Shape::Circle { c, r } => Some((c, r)),
                        _ => None,
                    })
                    .collect();
                let centres: Vec<_> = circles.iter().map(|(c, _)| *c).collect();
                points(&centres, &e["bubbles"], name);
                for (_, r) in circles {
                    close(r, e["radius"].as_f64().unwrap(), name);
                }
            }
            Shape::Column { at, column } => points(&arch::column_corners(*at, column), &e["corners"], name),
            Shape::Stair { pts, stair } => {
                let walk = arch::Walk::of(pts);
                let (stations, missing) = arch::riser_stations(&walk, stair);
                let want: Vec<f64> = e["stations"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
                assert_eq!(stations.len(), want.len(), "{name}: rıhtlar");
                for (g, w) in stations.iter().zip(&want) {
                    close(*g, *w, name);
                }
                assert_eq!(f64::from(missing), e["missing"].as_f64().unwrap(), "{name}: sığmayan");
                assert_eq!(codes(&arch::stair_checks(pts, stair)), want_codes(&e["codes"]), "{name}");
                points(&arch::band(pts, stair.width / 2.0), &e["band"], name);
                let lands = walk.landings(stair.landing());
                let want = e["landings"].as_array().unwrap();
                assert_eq!(lands.len(), want.len(), "{name}: sahanlıklar");
                for ((a, b), w) in lands.iter().zip(want) {
                    close(*a, w[0].as_f64().unwrap(), name);
                    close(*b, w[1].as_f64().unwrap(), name);
                }
            }
            Shape::Ramp { pts, ramp } => {
                let run = arch::ramp_run(&arch::Walk::of(pts), ramp);
                close(run, e["run"].as_f64().unwrap(), name);
                close(ramp.rise / run * 100.0, e["slope"].as_f64().unwrap(), name);
                assert_eq!(codes(&arch::ramp_checks(pts, ramp)), want_codes(&e["codes"]), "{name}");
                let texts: Vec<String> = arch::pieces(&shape, scale)
                    .into_iter()
                    .filter_map(|p| match p.shape {
                        Shape::Text { text, .. } => Some(text),
                        _ => None,
                    })
                    .collect();
                let levels: Vec<String> =
                    e["levels"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
                assert_eq!(texts, levels, "{name}: kotlar");
                points(&arch::band(pts, ramp.width / 2.0), &e["band"], name);
            }
            other => panic!("{name}: beklenmeyen şekil {other:?}"),
        }
    }
}
