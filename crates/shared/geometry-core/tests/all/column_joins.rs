//! A column in the walls it stands in (docs/adr/8041 §2) against the independent reference's cases
//! (fixtures/walls/v1/column-joins.json, scripts/fixtures/column_join_cases.py, exact fractions): each wall's lines in
//! their pieces and its caps (every end within 1e-9), its bands' areas, and each column's look: whether it is one
//! section with a wall's layer of its material, that layer's pattern and the wall's fill, and how much of its outline
//! is drawn.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::arch::ColumnSpec;
use kentos_geometry_core::geom::column_join::{Foot, WallAt, cut, joined};
use kentos_geometry_core::geom::wall::{Drawn, Spec, drawn};
use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/column-joins.json");
const TOL: f64 = 1e-9;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn level(v: &Value) -> Option<String> {
    v["arch"]["level"].as_str().map(str::to_owned)
}

fn ring_area(r: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += a.x * b.y - a.y * b.x;
    }
    s.abs() / 2.0
}

fn length(pts: &[Vec2]) -> f64 {
    pts.windows(2)
        .map(|w| js_hypot(w[1].x - w[0].x, w[1].y - w[0].y))
        .sum()
}

/// A wall's lines as (height, its pieces from west to east).
fn lines(d: &Drawn) -> Vec<(f64, Vec<(f64, f64)>)> {
    d.lines
        .iter()
        .map(|strokes| {
            let y = strokes[0].line.pts[0].y;
            let mut pieces: Vec<(f64, f64)> = strokes
                .iter()
                .map(|s| {
                    let xs = s.line.pts.iter().map(|p| p.x);
                    (
                        xs.clone().fold(f64::INFINITY, js_min),
                        xs.fold(f64::NEG_INFINITY, js_max),
                    )
                })
                .collect();
            pieces.sort_by(|a, b| a.0.total_cmp(&b.0));
            (y, pieces)
        })
        .collect()
}

#[test]
fn columns_cut_and_join_walls_as_the_reference_says() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let mut seen = 0;
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let walls: Vec<(Vec<Vec2>, Spec, Option<String>)> = c["walls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                let pts = w["pts"].as_array().unwrap().iter().map(pt).collect();
                (
                    pts,
                    Spec::read_flat(&Json::parse(&w.to_string()).unwrap()).unwrap(),
                    level(w),
                )
            })
            .collect();
        let feet: Vec<Foot> = c["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|col| {
                let spec = ColumnSpec::read_flat(&Json::parse(&col.to_string()).unwrap()).unwrap();
                Foot::of(pt(&col["at"]), &spec).unwrap()
            })
            .collect();
        let wholes: Vec<Drawn> = walls
            .iter()
            .map(|(pts, spec, _)| drawn(pts, None, spec).unwrap())
            .collect();
        for (i, ((_, _, lv), whole)) in walls.iter().zip(&wholes).enumerate() {
            let mine: Vec<&Foot> = feet.iter().filter(|f| f.level == *lv).collect();
            let d = cut(whole, &mine).unwrap_or_else(|| whole.clone());
            let want = &c["expect"]["walls"][i];
            let got = lines(&d);
            let want_lines = want["lines"].as_array().unwrap();
            assert_eq!(got.len(), want_lines.len(), "{name}: lines");
            for wl in want_lines {
                let y = wl["y"].as_f64().unwrap();
                let (_, pieces) = got
                    .iter()
                    .find(|(gy, _)| (gy - y).abs() <= TOL)
                    .unwrap_or_else(|| panic!("{name}: no line at {y}: {got:?}"));
                let want_pieces: Vec<(f64, f64)> = wl["pieces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                    .collect();
                assert_eq!(
                    pieces.len(),
                    want_pieces.len(),
                    "{name}: line {y}: {pieces:?}"
                );
                for (g, w) in pieces.iter().zip(&want_pieces) {
                    assert!(
                        (g.0 - w.0).abs() <= TOL && (g.1 - w.1).abs() <= TOL,
                        "{name}: line {y}: {pieces:?}"
                    );
                }
            }
            let mut caps: Vec<(f64, f64, f64)> = d
                .caps
                .iter()
                .map(|(a, b)| (a.x, js_min(a.y, b.y), js_max(a.y, b.y)))
                .collect();
            caps.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
            let want_caps = want["caps"].as_array().unwrap();
            assert_eq!(caps.len(), want_caps.len(), "{name}: caps {caps:?}");
            for (g, w) in caps.iter().zip(want_caps) {
                let w = (
                    w[0].as_f64().unwrap(),
                    w[1].as_f64().unwrap(),
                    w[2].as_f64().unwrap(),
                );
                assert!(
                    (g.0 - w.0).abs() <= TOL
                        && (g.1 - w.1).abs() <= TOL
                        && (g.2 - w.2).abs() <= TOL,
                    "{name}: caps {caps:?}"
                );
            }
            for (k, area) in want["bands"].as_array().unwrap().iter().enumerate() {
                let got: f64 = d.bands[k]
                    .iter()
                    .map(|(ring, holes)| {
                        ring_area(ring) - holes.iter().flatten().map(|h| ring_area(h)).sum::<f64>()
                    })
                    .sum();
                assert!(
                    (got - area.as_f64().unwrap()).abs() <= TOL,
                    "{name}: band {k}: {got}"
                );
            }
        }
        for (k, foot) in feet.iter().enumerate() {
            let at: Vec<WallAt<'_>> = walls
                .iter()
                .zip(&wholes)
                .filter(|((_, _, lv), _)| *lv == foot.level)
                .map(|((_, spec, _), whole)| WallAt {
                    drawn: whole,
                    spec,
                    turn: 0.0,
                })
                .collect();
            let want = &c["expect"]["columns"][k];
            let look = joined(foot, &at);
            assert_eq!(
                look.is_some(),
                want["joined"].as_bool().unwrap(),
                "{name}: column {k}"
            );
            let drawn = match &look {
                Some(j) => j.outline.iter().map(|p| length(p)).sum::<f64>(),
                None => {
                    let mut ring = foot.ring.clone();
                    ring.push(foot.ring[0]);
                    length(&ring)
                }
            };
            assert!(
                (drawn - want["outline"].as_f64().unwrap()).abs() <= TOL,
                "{name}: column {k}: {drawn}"
            );
            if let Some(j) = &look {
                assert_eq!(
                    j.hatch.as_ref().map(|h| h.0.pattern.as_str()),
                    want["pattern"].as_str(),
                    "{name}: column {k}"
                );
                assert_eq!(
                    j.fill.as_deref(),
                    want["fill"].as_str(),
                    "{name}: column {k}"
                );
            }
        }
        seen += 1;
    }
    assert_eq!(seen, 15, "every case read");
}
