//! Walls where they meet (docs/adr/0340 §2) against the independent reference's cases (fixtures/walls/v1/joins.json,
//! scripts/fixtures/wall_join_cases.py, exact fractions): the joints and their places, what is not joined and the fix
//! said for it, and each wall's lines in their pieces (every vertex within 1e-9), its caps and its bands' areas.

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::{WallIn, solve};
use kentos_geometry_core::jsmath::{js_max, js_round};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/walls/v1/joins.json");
const TOL: f64 = 1e-9;

/// An issue's code, walls and fix (wall, end, mode).
type Issue = (String, Vec<usize>, Option<(usize, String, String)>);

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL
}

fn ring_area(r: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += a.x * b.y - a.y * b.x;
    }
    s.abs() / 2.0
}

#[test]
fn walls_join_as_the_reference_says() {
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
        let got = solve(&ins, None);

        let mut joints: Vec<(String, i64, i64)> = got
            .joints
            .iter()
            .map(|j| {
                (
                    j.kind.name().to_owned(),
                    js_round(j.at.x * 1e6) as i64,
                    js_round(j.at.y * 1e6) as i64,
                )
            })
            .collect();
        let mut want: Vec<(String, i64, i64)> = c["joints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| {
                let at = &j["at"];
                (
                    j["kind"].as_str().unwrap().to_owned(),
                    js_round(at[0].as_f64().unwrap() * 1e6) as i64,
                    js_round(at[1].as_f64().unwrap() * 1e6) as i64,
                )
            })
            .collect();
        joints.sort();
        want.sort();
        assert_eq!(joints, want, "{name}: birleşimler");

        let issues: Vec<Issue> = got
            .issues
            .iter()
            .map(|i| {
                (
                    i.code.code().to_owned(),
                    i.walls.clone(),
                    i.fix
                        .map(|f| (f.wall, f.end.name().to_owned(), f.mode.name().to_owned())),
                )
            })
            .collect();
        let want_issues: Vec<Issue> = c["issues"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                let f = &i["fix"];
                (
                    i["code"].as_str().unwrap().to_owned(),
                    i["walls"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|w| w.as_u64().unwrap() as usize)
                        .collect(),
                    f.is_object().then(|| {
                        (
                            f["wall"].as_u64().unwrap() as usize,
                            f["end"].as_str().unwrap().to_owned(),
                            f["mode"].as_str().unwrap().to_owned(),
                        )
                    }),
                )
            })
            .collect();
        assert_eq!(issues, want_issues, "{name}: sorunlar");

        for (k, w) in c["drawn"].as_array().unwrap().iter().enumerate() {
            let d = got.drawn[k]
                .as_ref()
                .unwrap_or_else(|| panic!("{name}: {k}. duvar çizilmedi"));
            let lines = w["lines"].as_array().unwrap();
            assert_eq!(
                d.lines.len(),
                lines.len(),
                "{name}: {k}. duvarın çizgi sayısı"
            );
            for (li, (g, want)) in d.lines.iter().zip(lines).enumerate() {
                let want = want.as_array().unwrap();
                assert_eq!(
                    g.len(),
                    want.len(),
                    "{name}: {k}. duvarın {li}. çizgisinin parça sayısı"
                );
                for (si, (st, ws)) in g.iter().zip(want).enumerate() {
                    let wp: Vec<Vec2> = ws["pts"].as_array().unwrap().iter().map(pt).collect();
                    assert_eq!(
                        st.closed,
                        ws["closed"].as_bool().unwrap(),
                        "{name}: {k}/{li}/{si} kapalılık"
                    );
                    assert_eq!(
                        st.line.pts.len(),
                        wp.len(),
                        "{name}: {k}/{li}/{si} köşeler {:?} başvuru {wp:?}",
                        st.line.pts
                    );
                    for (a, b) in st.line.pts.iter().zip(&wp) {
                        assert!(
                            close(*a, *b),
                            "{name}: {k}/{li}/{si} köşe {a:?}, başvuru {b:?}"
                        );
                    }
                }
            }
            let caps = w["caps"].as_array().unwrap();
            assert_eq!(
                d.caps.len(),
                caps.len(),
                "{name}: {k}. duvarın uç çizgileri"
            );
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
                assert_eq!(
                    g.len(),
                    want.len(),
                    "{name}: {k}. duvarın {ci}. bandının parçaları"
                );
                for ((ring, holes), wa) in g.iter().zip(want) {
                    let a =
                        ring_area(ring) - holes.iter().flatten().map(|h| ring_area(h)).sum::<f64>();
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
