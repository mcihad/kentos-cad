//! Openings in walls (docs/adr/0341 §2, 0719) against the independent reference's cases
//! (fixtures/walls/v1/openings.json, scripts/fixtures/opening_cases.py): each opening's jambs (none when it is not on
//! one edge within its wall), its pieces (dashed when plan does not cut it), and its wall's lines, caps, bands, body
//! and a niche's or flue's inner sides with the opening's hole.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::opening::{self, Spec as OpeningSpec};
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::{WallIn, solve_with};
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/openings.json");
/// Revolving, pocket and folding doors and the sliding door's track (docs/adr/0777; scripts/fixtures/special_doors_cases.py).
const SPECIAL: &str = include_str!("../../../../../fixtures/walls/v1/special-doors.json");
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

fn same_piece(name: &str, got: &opening::Mark, want: &Value) {
    assert_eq!(
        got.dashed,
        want["dashed"].as_bool().unwrap_or(false),
        "{name}: parçanın kesikliliği {got:?}, başvuru {want}"
    );
    match (want["kind"].as_str().unwrap(), &got.shape) {
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
        ("circle", Shape::Circle { c, r }) => {
            assert!(
                close(*c, pt(&want["c"])) && near(*r, want["r"].as_f64().unwrap()),
                "{name}: daire {c:?} {r}, başvuru {want}"
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
    cut_as_the_reference_says(CASES);
}

/// A revolving door's drum and wings, a pocket door's leaves and its pockets in its wall open at its jambs, a folding
/// door's panels and the sliding doors' track, with their sweeps (docs/adr/0777).
#[test]
fn special_doors_cut_their_walls_as_the_reference_says() {
    cut_as_the_reference_says(SPECIAL);
}

fn cut_as_the_reference_says(text: &str) {
    let cases: Value = serde_json::from_str(text).unwrap();
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
        let mut intervals: Vec<Vec<opening::Hole>> = vec![Vec::new(); walls.len()];
        for (k, o) in c["openings"].as_array().unwrap().iter().enumerate() {
            let w = o["wall"].as_u64().unwrap() as usize;
            let spec = OpeningSpec::read_flat(&Json::parse(&o.to_string()).unwrap()).unwrap();
            let (at, width) = (o["at"].as_f64().unwrap(), o["width"].as_f64().unwrap());
            let (s0, s1) = opening::span(at, spec.anchor, width);
            intervals[w].extend(opening::holes(at, width, &spec));
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
                    let symbol = opening::pieces(&j, &spec);
                    let ws = c["symbols"][k].as_array().unwrap();
                    assert_eq!(symbol.len(), ws.len(), "{name}: {k}. açıklığın simgesinin parçaları {symbol:?}");
                    for (g, w) in symbol.iter().zip(ws) {
                        same_piece(name, g, w);
                    }
                    if let Some(zones) = c["zones"].get(k).filter(|z| !z.is_null()) {
                        let got = opening::swing_zones(&j, &spec);
                        let want = zones.as_array().unwrap();
                        assert_eq!(got.len(), want.len(), "{name}: {k}. açıklığın süpürme alanları {got:?}");
                        for (g, w) in got.iter().zip(want) {
                            let wp: Vec<Vec2> = w.as_array().unwrap().iter().map(pt).collect();
                            assert_eq!(g.len(), wp.len(), "{name}: süpürme alanının köşeleri");
                            for (a, b) in g.iter().zip(&wp) {
                                assert!(close(*a, *b), "{name}: süpürme alanı {a:?}, başvuru {b:?}");
                            }
                        }
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
            let same_areas = |what: &str, got: &[kentos_geometry_core::geom::wall::Body], want: &[Value]| {
                assert_eq!(got.len(), want.len(), "{name}: {k}. duvarın {what} parçaları");
                for ((ring, holes), wa) in got.iter().zip(want) {
                    let a = ring_area(ring) - holes.iter().flatten().map(|h| ring_area(h)).sum::<f64>();
                    let wa = wa.as_f64().unwrap();
                    assert!(
                        (a - wa).abs() <= TOL * js_max(wa, 1.0),
                        "{name}: {k} {what} alanı {a}, başvuru {wa}"
                    );
                }
            };
            for (ci, (g, want)) in d.bands.iter().zip(bands).enumerate() {
                same_areas(&format!("{ci}. bandının"), g, want.as_array().unwrap());
            }
            // The body between the faces and a niche's or flue's inner sides (docs/adr/0719 §3).
            same_areas("dolgusunun", &d.fill, w["fill"].as_array().unwrap());
            let recesses = w["recesses"].as_array().unwrap();
            assert_eq!(d.recesses.len(), recesses.len(), "{name}: {k}. duvarın oyuk kenarları {:?}", d.recesses);
            for (st, ws) in d.recesses.iter().zip(recesses) {
                let wp: Vec<Vec2> = ws["pts"].as_array().unwrap().iter().map(pt).collect();
                assert_eq!(st.line.pts.len(), wp.len(), "{name}: oyuk kenarı {:?} başvuru {wp:?}", st.line.pts);
                for (a, b) in st.line.pts.iter().zip(&wp) {
                    assert!(close(*a, *b), "{name}: oyuk kenarının köşesi {a:?}, başvuru {b:?}");
                }
            }
        }
    }
}

/// A niche on a quarter-circle wall (docs/adr/0719 §3): plan cuts it with its back an arc round the wall's centre
/// `depth` in from its face; above the cut height its dashed back is that arc too.
#[test]
fn a_niche_on_a_curved_wall_has_its_back_round_the_walls_centre() {
    let bulge = kentos_geometry_core::jsmath::tan(kentos_geometry_core::jsmath::PI / 8.0);
    let pts = [Vec2::new(5.0, 0.0), Vec2::new(0.0, 5.0)];
    let bulges = [bulge, 0.0];
    let wall = Spec {
        width: 0.3,
        ..Spec::default()
    };
    // The path runs counter-clockwise: its left is the inner face, radius 4.85.
    let path = opening::wall_path(&pts, Some(&bulges), false).unwrap();
    let (s0, s1) = (3.6, 4.2);
    let jambs = opening::jambs(&path, &wall, s0, s1).unwrap();
    let mut niche = OpeningSpec {
        form: opening::Form::Niche,
        depth: Some(0.1),
        sill: Some(1.2),
        head: Some(1.8),
        ..OpeningSpec::default()
    };
    let marks = opening::pieces(&jambs, &niche);
    assert_eq!(marks.len(), 3, "{marks:?}");
    assert!(marks.iter().all(|m| m.dashed), "above the cut: dashed");
    match &marks[1].shape {
        Shape::Arc { c, r, .. } => {
            assert!(c.x.abs() < 1e-9 && c.y.abs() < 1e-9, "round the wall's centre: {c:?}");
            assert!((r - 4.95).abs() < 1e-9, "the face's radius plus its depth: {r}");
        }
        other => panic!("its back is an arc: {other:?}"),
    }
    niche.sill = Some(0.9);
    assert!(opening::pieces(&jambs, &niche).is_empty(), "a cut niche is the wall's");
    let at = (s0 + s1) / 2.0;
    let holes = vec![opening::hole(at, s1 - s0, &niche).into_iter().collect::<Vec<_>>()];
    let ins = [WallIn {
        pts: &pts,
        bulges: Some(&bulges),
        spec: &wall,
    }];
    let d = solve_with(&ins, &holes, None).drawn[0].clone().unwrap();
    assert_eq!(d.recesses.len(), 1, "its back: {:?}", d.recesses);
    let back = &d.recesses[0].line;
    assert!(back.bulges.iter().any(|b| b.abs() > 1e-6), "an arc: {back:?}");
    for p in &back.pts {
        assert!((js_hypot_of(*p) - 4.95).abs() < 1e-9, "on the back's circle: {p:?}");
    }
    // The inner face is broken at the niche, the outer one is whole.
    assert_eq!(d.lines[0].len(), 2);
    assert_eq!(d.lines[1].len(), 1);
}

fn js_hypot_of(p: Vec2) -> f64 {
    kentos_geometry_core::jsmath::js_hypot(p.x, p.y)
}

/// Pocket doors in arc walls (docs/adr/0787 §2; scripts/fixtures/arc_pocket_cases.py): the jambs radial, the leaves
/// curved bands about the wall's centre, the wall's faces broken over the door, the cavity's caps and its sides arcs.
const ARC_POCKETS: &str = include_str!("../../../../../fixtures/walls/v1/arc-pockets.json");

#[test]
fn pocket_doors_in_arc_walls_cut_their_walls_as_the_reference_says() {
    let cases: Value = serde_json::from_str(ARC_POCKETS).unwrap();
    let same_line = |name: &str, what: &str, got: &kentos_geometry_core::geom::wall::Line, want: &Value| {
        let wp: Vec<Vec2> = want["pts"].as_array().unwrap().iter().map(pt).collect();
        let wb: Vec<f64> = want["bulges"].as_array().unwrap().iter().map(|b| b.as_f64().unwrap()).collect();
        assert_eq!(got.pts.len(), wp.len(), "{name}: {what} köşeleri {got:?}, başvuru {want}");
        for (a, b) in got.pts.iter().zip(&wp) {
            assert!(close8(*a, *b), "{name}: {what} köşesi {a:?}, başvuru {b:?}");
        }
        for (a, b) in got.bulges.iter().zip(&wb) {
            assert!(near8(*a, *b), "{name}: {what} bükümü {a}, başvuru {b}");
        }
    };
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let w = &c["wall"];
        let pts: Vec<Vec2> = w["pts"].as_array().unwrap().iter().map(pt).collect();
        let bulges: Vec<f64> = w["bulges"].as_array().unwrap().iter().map(|b| b.as_f64().unwrap()).collect();
        let wall = Spec::read_flat(&Json::parse(&w.to_string()).unwrap()).unwrap();
        let o = &c["opening"];
        let spec = OpeningSpec::read_flat(&Json::parse(&o.to_string()).unwrap()).unwrap();
        let (at, width) = (o["at"].as_f64().unwrap(), o["width"].as_f64().unwrap());
        let (s0, s1) = opening::span(at, spec.anchor, width);
        let path = opening::wall_path(&pts, Some(&bulges), false).unwrap();
        let j = opening::jambs(&path, &wall, s0, s1).unwrap_or_else(|| panic!("{name}: pervaz yok"));
        let wj: Vec<Vec2> = c["jambs"].as_array().unwrap().iter().map(pt).collect();
        for (a, b) in j.iter().zip(&wj) {
            assert!(close8(*a, *b), "{name}: pervaz {a:?}, başvuru {b:?}");
        }
        let symbol = opening::pieces(&j, &spec);
        let ws = c["symbol"].as_array().unwrap();
        assert_eq!(symbol.len(), ws.len(), "{name}: simgenin parçaları {symbol:?}");
        for (g, want) in symbol.iter().zip(ws) {
            same_piece_in(name, g, want);
            if let (Some(wb), Shape::Polyline { bulges, .. }) = (want.get("bulges"), &g.shape) {
                let got = bulges.clone().unwrap_or_default();
                let wb: Vec<f64> = wb.as_array().unwrap().iter().map(|b| b.as_f64().unwrap()).collect();
                assert_eq!(got.len(), wb.len(), "{name}: kanadın bükümleri {got:?}");
                for (a, b) in got.iter().zip(&wb) {
                    assert!(near8(*a, *b), "{name}: kanadın bükümü {a}, başvuru {b}");
                }
            }
        }
        let holes = vec![opening::holes(at, width, &spec)];
        let ins = [WallIn {
            pts: &pts,
            bulges: Some(&bulges),
            spec: &wall,
        }];
        let d = solve_with(&ins, &holes, None).drawn[0].clone().unwrap();
        let lines = c["lines"].as_array().unwrap();
        assert_eq!(d.lines.len(), lines.len(), "{name}: çizgiler");
        for (li, (g, want)) in d.lines.iter().zip(lines).enumerate() {
            let want = want.as_array().unwrap();
            assert_eq!(g.len(), want.len(), "{name}: {li}. çizginin parçaları {g:?}");
            for (st, ws) in g.iter().zip(want) {
                same_line(name, &format!("{li}. çizgi"), &st.line, ws);
            }
        }
        let caps = c["caps"].as_array().unwrap();
        assert_eq!(d.caps.len(), caps.len(), "{name}: uç çizgileri {:?}", d.caps);
        for (g, want) in d.caps.iter().zip(caps) {
            assert!(
                close8(g.0, pt(&want[0])) && close8(g.1, pt(&want[1])),
                "{name}: uç çizgisi {g:?}, başvuru {want}"
            );
        }
        let rec = c["recesses"].as_array().unwrap();
        assert_eq!(d.recesses.len(), rec.len(), "{name}: cebin iç çizgileri {:?}", d.recesses);
        for (g, want) in d.recesses.iter().zip(rec) {
            same_line(name, "cebin iç çizgisi", &g.line, want);
        }
        // The body between the faces, its pieces' outlines: their arcs are drawn as points, each chord losing
        // r²(θ − sin θ)/2 of the exact area, so the reference's exact area is met within half a percent.
        let body: f64 = d
            .fill
            .iter()
            .map(|(ring, holes)| ring_area(ring) - holes.iter().flatten().map(|h| ring_area(h)).sum::<f64>())
            .sum();
        let want = c["body"].as_f64().unwrap();
        assert!((body - want).abs() <= 5e-3 * want, "{name}: gövdenin alanı {body}, başvuru {want}");
    }
}

/// The arc cases' tolerance: 10 nm, as a point 4 400 km from its origin has its last bits there.
const TOL8: f64 = 1e-8;

fn close8(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= TOL8 && (a.y - b.y).abs() <= TOL8
}

fn near8(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOL8
}

/// [`same_piece`] within [`TOL8`]: a ring's points (its bulges are compared by the caller).
fn same_piece_in(name: &str, got: &opening::Mark, want: &Value) {
    match (want["kind"].as_str().unwrap(), &got.shape) {
        ("ring", Shape::Polyline { pts, closed, .. }) => {
            let wp: Vec<Vec2> = want["pts"].as_array().unwrap().iter().map(pt).collect();
            assert!(*closed, "{name}: halka kapalı olmalı");
            assert_eq!(pts.len(), wp.len(), "{name}: halkanın köşeleri");
            for (a, b) in pts.iter().zip(&wp) {
                assert!(close8(*a, *b), "{name}: halka köşesi {a:?}, başvuru {b:?}");
            }
        }
        (kind, got) => panic!("{name}: başvuru {kind}, çekirdek {got:?}"),
    }
}
