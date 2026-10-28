//! Grids, columns, stairs and ramps drawn (docs/adr/0343 §2) against the independent reference's cases
//! (fixtures/arch/v1/cases.json, scripts/fixtures/arch_cases.py): bubbles, a column's corners, a stair's risers, band,
//! landings and checks, a ramp's sloped run, slope, levels and checks.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/cases.json");
const TOL: f64 = 1e-9;

/// The railing lines against the reference's (docs/adr/0705 §2): their points and whether they lie past the cut.
fn railings(got: &[(Vec<kentos_geometry_core::vec2::Vec2>, bool)], want: &Value, what: &str) {
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{what}: korkuluk çizgileri");
    for ((line, over), w) in got.iter().zip(want) {
        assert_eq!(*over, w["over"].as_bool().unwrap(), "{what}: kesmenin üstü");
        points(line, &w["pts"], what);
    }
}

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
            Shape::Stair { pts, stair } if stair.bulge != 0.0 => curved_stair(pts, stair, e, name),
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
                let lands = arch::stair_landings(&walk, stair);
                let want = e["landings"].as_array().unwrap();
                assert_eq!(lands.len(), want.len(), "{name}: sahanlıklar");
                for ((a, b), w) in lands.iter().zip(want) {
                    close(*a, w[0].as_f64().unwrap(), name);
                    close(*b, w[1].as_f64().unwrap(), name);
                }
                // docs/adr/0704 §1, §3: the fitted end, the risers dashed past the cut.
                match (arch::stair_fit_end(pts, stair), e["fit"].as_array()) {
                    (Some(p), Some(w)) => {
                        close(p.x, w[0].as_f64().unwrap(), name);
                        close(p.y, w[1].as_f64().unwrap(), name);
                    }
                    (None, None) => {}
                    (got, want) => panic!("{name}: sığdırma {got:?}, beklenen {want:?}"),
                }
                let dashed = arch::stair_pieces(pts, stair, 50.0)
                    .iter()
                    .filter(|p| p.stroke.line_type.as_deref() == Some("dashed"))
                    .count();
                assert_eq!(dashed as u64, e["dashed"].as_u64().unwrap(), "{name}: kesikli rıhtlar");
                railings(&arch::stair_railings(pts, stair), &e["railings"], name);
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
                // Its slope as drawn, then its levels (docs/adr/0723 §5).
                let want: Vec<String> =
                    e["texts"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
                assert_eq!(texts, want, "{name}: eğim ve kotlar");
                points(&arch::band(pts, ramp.width / 2.0), &e["band"], name);
                // docs/adr/0705 §1, §2: the fitted end, the railings.
                match (arch::ramp_fit_end(pts, ramp), e["fit"].as_array()) {
                    (Some(p), Some(w)) => {
                        close(p.x, w[0].as_f64().unwrap(), name);
                        close(p.y, w[1].as_f64().unwrap(), name);
                    }
                    (None, None) => {}
                    (got, want) => panic!("{name}: sığdırma {got:?}, beklenen {want:?}"),
                }
                railings(&arch::ramp_railings(pts, ramp), &e["railings"], name);
            }
            other => panic!("{name}: beklenmeyen şekil {other:?}"),
        }
    }
}

/// A curved stair (docs/adr/0709) against the reference: its risers along its arc, radial; its band of two arcs; its
/// fitted end round its centre; its railings concentric arcs split at the cut.
fn curved_stair(pts: &[kentos_geometry_core::vec2::Vec2], stair: &arch::StairSpec, e: &Value, name: &str) {
    let walk = arch::Walk::curved(pts, stair.bulge);
    assert!(walk.arc.is_some(), "{name}: yay");
    let (stations, missing) = arch::riser_stations(&walk, stair);
    let want: Vec<f64> = e["stations"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    assert_eq!(stations.len(), want.len(), "{name}: rıhtlar");
    for (g, w) in stations.iter().zip(&want) {
        close(*g, *w, name);
    }
    assert_eq!(f64::from(missing), e["missing"].as_f64().unwrap(), "{name}: sığmayan");
    assert_eq!(codes(&arch::stair_checks(pts, stair)), want_codes(&e["codes"]), "{name}");
    assert!(arch::stair_landings(&walk, stair).is_empty(), "{name}: döner merdivende sahanlık yok");
    let Shape::Polygon { pts: band, bulges: Some(bulges), .. } = arch::stair_band(pts, stair) else {
        panic!("{name}: bant iki yaylı halka olmalı");
    };
    points(&band, &e["band"], name);
    for (g, w) in bulges.iter().zip(e["bandBulges"].as_array().unwrap()) {
        close(*g, w.as_f64().unwrap(), name);
    }
    match (arch::stair_fit_arc(pts, stair), e["fitArc"].as_array()) {
        (Some((p, b)), Some(w)) => {
            close(p.x, w[0].as_f64().unwrap(), name);
            close(p.y, w[1].as_f64().unwrap(), name);
            close(b, w[2].as_f64().unwrap(), name);
        }
        (None, None) => {}
        (got, want) => panic!("{name}: sığdırma {got:?}, beklenen {want:?}"),
    }
    let pieces = arch::stair_pieces(pts, stair, 50.0);
    // After the band (no landings): the risers, radial across the width.
    let risers: Vec<_> = pieces
        .iter()
        .filter_map(|p| match p.shape {
            Shape::Line { a, b } => Some([a, b]),
            _ => None,
        })
        .take(stations.len())
        .collect();
    let want = e["risers"].as_array().unwrap();
    assert_eq!(risers.len(), want.len(), "{name}: rıht çizgileri");
    for (g, w) in risers.iter().zip(want) {
        points(g, w, name);
    }
    let dashed = pieces.iter().filter(|p| p.stroke.line_type.as_deref() == Some("dashed")).count();
    assert_eq!(dashed as u64, e["dashed"].as_u64().unwrap(), "{name}: kesikli parçalar");
    assert!(arch::stair_railings(pts, stair).is_empty(), "{name}: düz korkuluk çizgisi yok");
    let arcs = arch::stair_railing_arcs(pts, stair);
    let want = e["railingArcs"].as_array().unwrap();
    assert_eq!(arcs.len(), want.len(), "{name}: korkuluk yayları");
    for ((shape, over), w) in arcs.iter().zip(want) {
        assert_eq!(*over, w["over"].as_bool().unwrap(), "{name}: kesmenin üstü");
        let Shape::Polyline { pts, bulges: Some(b), .. } = shape else {
            panic!("{name}: korkuluk yay olmalı");
        };
        points(pts, &w["pts"], name);
        close(b[0], w["bulge"].as_f64().unwrap(), name);
    }
}

/// En uzun kol (docs/adr/0723 §1): 1.2 m at 5 % is 24 m of slope; with runs of at most 9 m and 1.5 m landings the
/// line gets two straight corners and ends 27 m from its start (9 + 1.5 + 9 + 1.5 + 6), every run at most 9 m. A
/// ramp whose runs are short enough is left alone, and no positive limit means no change.
#[test]
fn long_ramps_get_landings_where_a_run_would_pass_the_limit() {
    use kentos_geometry_core::Vec2;
    let spec = arch::RampSpec { width: 1.5, rise: 1.2, slope: Some(5.0), base: None, landing: Some(1.5), railings: None };
    let line = [Vec2::new(0.0, 0.0), Vec2::new(30.0, 0.0)];
    let pts = arch::ramp_with_landings(&line, &spec, 9.0, true);
    let xs: Vec<f64> = pts.iter().map(|p| p.x).collect();
    assert_eq!(xs.len(), 4, "{xs:?}");
    for (x, want) in xs.iter().zip([0.0, 9.75, 20.25, 27.0]) {
        assert!((x - want).abs() < 1e-9, "{xs:?}");
    }
    let walk = arch::Walk::of(&pts);
    assert!((arch::ramp_run(&walk, &spec) - 24.0).abs() < 1e-9);
    let mut start = 0.0;
    for (a, b) in arch::ramp_landings(&walk, &spec).into_iter().chain([(walk.length(), walk.length())]) {
        assert!(a - start <= 9.0 + 1e-9, "{a} {start}");
        start = b;
    }
    assert_eq!(arch::ramp_with_landings(&line, &spec, 30.0, false), line.to_vec());
    assert_eq!(arch::ramp_with_landings(&line, &spec, 0.0, true), line.to_vec());
}
