//! Openings in walls (docs/adr/0341 §2) against the independent reference's cases (fixtures/walls/v1/openings.json,
//! scripts/fixtures/opening_cases.py): each opening's jambs (none when it is not on one edge within its wall), its
//! symbol's pieces, and its wall's lines, caps and bands with the opening's gap.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::opening::{self, Spec as OpeningSpec};
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::{WallIn, solve_with};
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/openings.json");
const TOL: f64 = 1e-9;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOL
}

fn ring_area(r: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += a.x * b.y - a.y * b.x;
    }
    s.abs() / 2.0
}

fn same_piece(name: &str, got: &Shape, want: &Value) {
    match (want["kind"].as_str().unwrap(), got) {
        ("line", Shape::Line { a, b }) => {
            assert!(
                close(*a, pt(&want["a"])) && close(*b, pt(&want["b"])),
                "{name}: çizgi {a:?} {b:?}, başvuru {want}"
            );
        }
        ("arc", Shape::Arc { c, r, a0, a1 }) => {
            assert!(
                close(*c, pt(&want["c"]))
                    && near(*r, want["r"].as_f64().unwrap())
                    && near(*a0, want["a0"].as_f64().unwrap())
                    && near(*a1, want["a1"].as_f64().unwrap()),
                "{name}: yay {c:?} {r} {a0} {a1}, başvuru {want}"
            );
        }
        ("ring", Shape::Polyline { pts, closed, .. }) => {
            let wp: Vec<Vec2> = want["pts"].as_array().unwrap().iter().map(pt).collect();
            assert!(*closed, "{name}: kasa kapalı halka olmalı");
            assert_eq!(pts.len(), wp.len(), "{name}: kasanın köşeleri");
            for (a, b) in pts.iter().zip(&wp) {
                assert!(close(*a, *b), "{name}: kasa köşesi {a:?}, başvuru {b:?}");
            }
        }
        (kind, got) => panic!("{name}: başvuru {kind}, çekirdek {got:?}"),
    }
}

#[test]
fn openings_cut_their_walls_as_the_reference_says() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let walls: Vec<(Vec<Vec2>, Spec)> = c["walls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                let pts = w["pts"].as_array().unwrap().iter().map(pt).collect();
                (pts, Spec::read_flat(&Json::parse(&w.to_string()).unwrap()).unwrap())
            })
            .collect();
        let mut intervals: Vec<Vec<(f64, f64)>> = vec![Vec::new(); walls.len()];
        for (k, o) in c["openings"].as_array().unwrap().iter().enumerate() {
            let w = o["wall"].as_u64().unwrap() as usize;
            let spec = OpeningSpec::read_flat(&Json::parse(&o.to_string()).unwrap()).unwrap();
            let (s0, s1) = opening::span(o["at"].as_f64().unwrap(), spec.anchor, o["width"].as_f64().unwrap());
            intervals[w].push((s0, s1));
            let (pts, wall) = &walls[w];
            let path = opening::wall_path(pts, None, wall.closed).unwrap();
            let got = opening::jambs(&path, wall, s0, s1);
            let want = &c["jambs"][k];
            match (got, want.is_null()) {
                (None, true) => continue,
                (Some(j), false) => {
                    let wj: Vec<Vec2> = want.as_array().unwrap().iter().map(pt).collect();
                    for (a, b) in j.iter().zip(&wj) {
                        assert!(close(*a, *b), "{name}: {k}. açıklığın pervazı {a:?}, başvuru {b:?}");
                    }
                    let symbol = opening::symbol(&j, &spec);
                    let ws = c["symbols"][k].as_array().unwrap();
                    assert_eq!(symbol.len(), ws.len(), "{name}: {k}. açıklığın simgesinin parçaları {symbol:?}");
                    for (g, w) in symbol.iter().zip(ws) {
                        same_piece(name, g, w);
                    }
                }
                (got, _) => panic!("{name}: {k}. açıklığın pervazları {got:?}, başvuru {want}"),
            }
        }
        let ins: Vec<WallIn<'_>> = walls
            .iter()
            .map(|(pts, spec)| WallIn {
                pts,
                bulges: None,
                spec,
            })
            .collect();
        let got = solve_with(&ins, &intervals, None);
        for (k, w) in c["drawn"].as_array().unwrap().iter().enumerate() {
            let d = got.drawn[k].as_ref().unwrap_or_else(|| panic!("{name}: {k}. duvar çizilmedi"));
            let lines = w["lines"].as_array().unwrap();
            assert_eq!(d.lines.len(), lines.len(), "{name}: {k}. duvarın çizgi sayısı");
            for (li, (g, want)) in d.lines.iter().zip(lines).enumerate() {
                let want = want.as_array().unwrap();
                assert_eq!(g.len(), want.len(), "{name}: {k}. duvarın {li}. çizgisinin parça sayısı");
                for (si, (st, ws)) in g.iter().zip(want).enumerate() {
                    let wp: Vec<Vec2> = ws["pts"].as_array().unwrap().iter().map(pt).collect();
                    assert_eq!(st.closed, ws["closed"].as_bool().unwrap(), "{name}: {k}/{li}/{si} kapalılık");
                    assert_eq!(
                        st.line.pts.len(),
                        wp.len(),
                        "{name}: {k}/{li}/{si} köşeler {:?} başvuru {wp:?}",
                        st.line.pts
                    );
                    for (a, b) in st.line.pts.iter().zip(&wp) {
                        assert!(close(*a, *b), "{name}: {k}/{li}/{si} köşe {a:?}, başvuru {b:?}");
                    }
                }
            }
            let caps = w["caps"].as_array().unwrap();
            assert_eq!(d.caps.len(), caps.len(), "{name}: {k}. duvarın uç çizgileri {:?}", d.caps);
            for (g, want) in d.caps.iter().zip(caps) {
                assert!(
                    close(g.0, pt(&want[0])) && close(g.1, pt(&want[1])),
                    "{name}: uç çizgisi {g:?}, başvuru {want}"
                );
            }
            let bands = w["bands"].as_array().unwrap();
            assert_eq!(d.bands.len(), bands.len(), "{name}: {k}. duvarın bantları");
            for (ci, (g, want)) in d.bands.iter().zip(bands).enumerate() {
                let want = want.as_array().unwrap();
                assert_eq!(g.len(), want.len(), "{name}: {k}. duvarın {ci}. bandının parçaları");
                for ((ring, holes), wa) in g.iter().zip(want) {
                    let a = ring_area(ring) - holes.iter().flatten().map(|h| ring_area(h)).sum::<f64>();
                    let wa = wa.as_f64().unwrap();
                    assert!(
                        (a - wa).abs() <= TOL * js_max(wa, 1.0),
                        "{name}: {k}/{ci} bandın alanı {a}, başvuru {wa}"
                    );
                }
            }
        }
    }
}
