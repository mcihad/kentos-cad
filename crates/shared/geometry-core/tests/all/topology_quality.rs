//! MAP-07's acceptance over the shared core (docs/adr/1260 §5): neighbouring areas with a curved common edge, an area
//! with a hole and its island, a neighbour replaced (a source row read again under the same uid, another added and one
//! removed) and an exception covering its objects anywhere: after each edit the recheck from the last result and the
//! full check give the same findings, and the overlaps' measures are the closed-form areas of circular segments and
//! rectangles (not the core's own functions). And how far a repair takes source vertices (`shift`): kept bit for bit,
//! moved by the distance to the new geometry, a deletion not measured, no rounding at the storage precision.

use std::collections::HashSet;

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::topology_rules::incremental::{Previous, recheck};
use kentos_geometry_core::ops::topology_rules::{
    Checked, Exception, Kind, Objects, Rule, check, shift,
};
use kentos_geometry_core::vec2::Vec2;

const T: f64 = 0.001;
const E: f64 = 487_000.0;
const N: f64 = 4_420_000.0;

fn p(x: f64, y: f64) -> Vec2 {
    Vec2::new(E + x, N + y)
}

fn area(pts: Vec<Vec2>, bulges: Option<Vec<f64>>, holes: Vec<Vec<Vec2>>) -> Shape {
    Shape::Polygon {
        pts,
        bulges,
        holes: (!holes.is_empty()).then(|| {
            holes
                .into_iter()
                .map(|mut pts| {
                    pts.reverse();
                    Ring { pts, bulges: None }
                })
                .collect()
        }),
        parts: None,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Vec2> {
    vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]
}

/// A circular segment's area over a chord `c` of an edge of bulge `b` (tan of a quarter of its sweep): the sweep
/// θ = 4·atan|b|, the radius c / (2 sin(θ/2)), the area r²/2 (θ − sin θ).
fn segment(c: f64, b: f64) -> f64 {
    let theta = 4.0 * b.abs().atan();
    let r = c / (2.0 * (theta / 2.0).sin());
    r * r / 2.0 * (theta - theta.sin())
}

struct Drawing {
    shapes: Vec<Shape>,
    layers: Vec<String>,
    uids: Vec<String>,
}

impl Drawing {
    fn objects(&self) -> Objects<'_> {
        Objects {
            shapes: &self.shapes,
            layers: &self.layers,
            uids: &self.uids,
        }
    }
}

fn bounds(s: &Shape) -> Bounds {
    let mut b = kentos_geometry_core::geometry::empty_bounds();
    for q in kentos_geometry_core::entity::entity_vertices(s) {
        kentos_geometry_core::geometry::extend_bounds(&mut b, q, 0.0);
    }
    // An arc bulges past its ends: room for it.
    Bounds {
        min_x: b.min_x - 5.0,
        min_y: b.min_y - 5.0,
        max_x: b.max_x + 5.0,
        max_y: b.max_y + 5.0,
    }
}

fn said(c: &Checked, uids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = c
        .findings
        .iter()
        .map(|f| {
            format!(
                "{} {} {:?} {:.6} {:.6} {:?} {}",
                f.rule,
                f.problem,
                f.objects.iter().map(|&i| uids[i].as_str()).collect::<Vec<_>>(),
                f.at.x,
                f.at.y,
                f.measure.map(|m| format!("{m:.6}")),
                f.exception
            )
        })
        .collect();
    out.sort();
    out
}

fn rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "r1".into(),
            kind: Kind::MustNotOverlap,
            layer: "parsel".into(),
            other: None,
            value: None,
        },
        Rule {
            id: "r2".into(),
            kind: Kind::MustNotHaveGaps,
            layer: "parsel".into(),
            other: None,
            value: None,
        },
    ]
}

/// The edits in order, each the object's new shape (none: removed; a new uid: added), checked again after each.
fn edits_agree(mut d: Drawing, exceptions: &[Exception], edits: Vec<(String, Option<Shape>)>) -> Vec<Checked> {
    let rules = rules();
    let mut last = check(&d.objects(), &rules, T, exceptions);
    let mut last_uids = d.uids.clone();
    let mut all = vec![last.clone()];
    for (step, (uid, shape)) in edits.into_iter().enumerate() {
        let mut touched = Vec::new();
        let at = d.uids.iter().position(|u| *u == uid);
        if let Some(i) = at {
            touched.push(bounds(&d.shapes[i]));
        }
        match (at, shape) {
            (Some(i), Some(s)) => {
                touched.push(bounds(&s));
                d.shapes[i] = s;
            }
            (Some(i), None) => {
                d.shapes.remove(i);
                d.layers.remove(i);
                d.uids.remove(i);
            }
            (None, Some(s)) => {
                touched.push(bounds(&s));
                d.shapes.push(s);
                d.layers.push("parsel".into());
                d.uids.push(uid.clone());
            }
            (None, None) => {}
        }
        let changed: HashSet<&str> = [uid.as_str()].into_iter().collect();
        let again = recheck(
            &d.objects(),
            &rules,
            T,
            exceptions,
            &Previous {
                findings: &last.findings,
                uids: &last_uids,
            },
            &changed,
            &touched,
        );
        let full = check(&d.objects(), &rules, T, exceptions);
        assert_eq!(
            said(&again, &d.uids),
            said(&full, &d.uids),
            "step {step}: the recheck and the full check differ"
        );
        assert_eq!(again.looked, full.looked, "step {step}");
        last_uids = d.uids.clone();
        all.push(full);
        last = again;
    }
    all
}

fn overlaps(c: &Checked) -> Vec<f64> {
    c.findings
        .iter()
        .filter(|f| f.problem == "overlap")
        .filter_map(|f| f.measure)
        .collect()
}

/// A and B share their curved edge (A's right edge bulging into B, B's left edge the same arc the other way): no
/// finding. B's edge bulging less leaves an overlap of the two segments' difference; B as a square then overlaps by
/// A's whole segment; the shared arc again, nothing.
#[test]
fn neighbours_with_a_curved_edge() {
    let (b1, b2) = (0.2, 0.1);
    let a = area(rect(0.0, 0.0, 10.0, 10.0), Some(vec![0.0, b1, 0.0, 0.0]), Vec::new());
    let with = |b: f64| area(rect(10.0, 0.0, 20.0, 10.0), Some(vec![0.0, 0.0, 0.0, -b]), Vec::new());
    let d = Drawing {
        shapes: vec![a, with(b1)],
        layers: vec!["parsel".into(); 2],
        uids: vec!["A".into(), "B".into()],
    };
    let checks = edits_agree(
        d,
        &[],
        vec![
            ("B".into(), Some(with(b2))),
            ("B".into(), Some(area(rect(10.0, 0.0, 20.0, 10.0), None, Vec::new()))),
            ("B".into(), Some(with(b1))),
        ],
    );
    assert!(overlaps(&checks[0]).is_empty(), "{:?}", checks[0].findings);
    let lens = segment(10.0, b1) - segment(10.0, b2);
    let got = overlaps(&checks[1]);
    assert_eq!(got.len(), 1, "{:?}", checks[1].findings);
    assert!((got[0] - lens).abs() < 1e-6, "overlap {} against {lens}", got[0]);
    let got = overlaps(&checks[2]);
    assert!((got[0] - segment(10.0, b1)).abs() < 1e-6, "overlap {:?}", got);
    assert!(overlaps(&checks[3]).is_empty());
}

/// An area with a hole and the island that fills it; the island moved 1 m east overlaps the area by 1 × 10 and
/// leaves a 1 × 10 gap in the hole; put back, nothing. The island read again as another row (removed, a new uid
/// added in the same place) and a neighbour added that overlaps the area by 2 × 30: the same findings either way.
#[test]
fn an_area_with_a_hole_its_island_and_a_neighbour_read_again() {
    let holed = area(rect(0.0, 0.0, 30.0, 30.0), None, vec![rect(10.0, 10.0, 20.0, 20.0)]);
    let island = |dx: f64| area(rect(10.0 + dx, 10.0, 20.0 + dx, 20.0), None, Vec::new());
    let d = Drawing {
        shapes: vec![holed, island(0.0)],
        layers: vec!["parsel".into(); 2],
        uids: vec!["H".into(), "I".into()],
    };
    let checks = edits_agree(
        d,
        &[],
        vec![
            ("I".into(), Some(island(1.0))),
            ("I".into(), Some(island(0.0))),
            ("I".into(), None),
            ("I2".into(), Some(island(0.0))),
            ("K".into(), Some(area(rect(28.0, 0.0, 40.0, 30.0), None, Vec::new()))),
        ],
    );
    assert!(checks[0].findings.is_empty(), "{:?}", checks[0].findings);
    let got = overlaps(&checks[1]);
    assert_eq!(got.len(), 1);
    assert!((got[0] - 10.0).abs() < 1e-6, "{got:?}");
    let gaps: Vec<f64> = checks[1]
        .findings
        .iter()
        .filter(|f| f.problem == "gap")
        .filter_map(|f| f.measure)
        .collect();
    assert_eq!(gaps.len(), 1, "{:?}", checks[1].findings);
    assert!((gaps[0] - 10.0).abs() < 1e-6, "{gaps:?}");
    assert!(checks[2].findings.is_empty());
    // The hole without its island is a gap of the layer's union (docs/adr/0524): no overlap is left.
    assert!(overlaps(&checks[3]).is_empty());
    assert!(checks[4].findings.is_empty(), "{:?}", checks[4].findings);
    let got = overlaps(&checks[5]);
    assert_eq!(got.len(), 1);
    assert!((got[0] - 60.0).abs() < 1e-6, "{got:?}");
}

/// An exception covering its objects anywhere (docs/adr/1260 §1) keeps the overlap an exception wherever it moves;
/// one bound to its place lets it go when it moves away. The recheck agrees with the full check either way.
#[test]
fn an_exception_covering_its_objects_anywhere() {
    let d = || Drawing {
        shapes: vec![
            area(rect(0.0, 0.0, 10.0, 10.0), None, Vec::new()),
            area(rect(9.0, 0.0, 20.0, 10.0), None, Vec::new()),
        ],
        layers: vec!["parsel".into(); 2],
        uids: vec!["A".into(), "B".into()],
    };
    let first = check(&d().objects(), &rules(), T, &[]);
    let o = first.findings.iter().find(|f| f.problem == "overlap").expect("an overlap");
    let mark = |anywhere| Exception {
        rule: "r1".into(),
        objects: vec!["A".into(), "B".into()],
        at: o.at,
        anywhere,
    };
    let moved = vec![("B".into(), Some(area(rect(9.0, 5.0, 20.0, 15.0), None, Vec::new())))];
    let anywhere = edits_agree(d(), &[mark(true)], moved.clone());
    let placed = edits_agree(d(), &[mark(false)], moved);
    let is_exception = |c: &Checked| c.findings.iter().find(|f| f.problem == "overlap").map(|f| f.exception);
    assert_eq!(is_exception(&anywhere[0]), Some(true));
    assert_eq!(is_exception(&anywhere[1]), Some(true));
    assert_eq!(is_exception(&placed[0]), Some(true));
    assert_eq!(is_exception(&placed[1]), Some(false));
}

/// `shift` (docs/adr/1260 §4): A − B keeps the four corners bit for bit; a rectangle cut back 2 m moves two corners
/// 2 m; a vertex taken away from a ring lies 10/√401 m from the edge that replaces it; a vertex moved 0.00005 m is
/// moved, and counted under a 0.0001 m precision; a point moved 3-4-5 is 5 m away.
#[test]
fn how_far_a_repair_takes_source_vertices() {
    let a = area(rect(0.0, 0.0, 10.0, 10.0), None, Vec::new());
    let notched = area(
        vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 2.0), p(8.0, 2.0), p(8.0, 8.0), p(10.0, 8.0), p(10.0, 10.0), p(0.0, 10.0)],
        None,
        Vec::new(),
    );
    let (s, under) = shift(&a, &notched, 0.0001);
    assert_eq!((s.kept, s.moved, s.max, s.total, under), (4, 0, 0.0, 0.0, 0));

    let b = area(rect(8.0, 2.0, 18.0, 8.0), None, Vec::new());
    let cut = area(rect(10.0, 2.0, 18.0, 8.0), None, Vec::new());
    let (s, _) = shift(&b, &cut, 0.0);
    assert_eq!((s.kept, s.moved, s.max, s.total), (2, 2, 2.0, 4.0));

    let five = area(
        vec![p(50.0, 0.0), p(60.0, 0.0), p(60.0, 10.0), p(50.5, 10.0), p(50.0, 10.0)],
        None,
        Vec::new(),
    );
    let four = area(vec![p(50.0, 0.0), p(60.0, 0.0), p(60.0, 10.0), p(50.5, 10.0)], None, Vec::new());
    let (s, _) = shift(&five, &four, 0.0);
    assert_eq!((s.kept, s.moved), (4, 1));
    assert!((s.max - 10.0 / 401f64.sqrt()).abs() < 1e-9, "{}", s.max);

    let nudged = area(
        vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.00005, 10.0)],
        None,
        Vec::new(),
    );
    let (s, under) = shift(&a, &nudged, 0.0001);
    assert_eq!((s.kept, s.moved, under), (3, 1, 1));
    assert!(s.max > 0.0 && s.max < 0.0001, "{}", s.max);

    let pt = |x, y| Shape::Point { p: p(x, y), z: None, parts: None };
    let (s, _) = shift(&pt(0.0, 0.0), &pt(3.0, 4.0), 0.0);
    assert_eq!((s.kept, s.moved), (0, 1));
    assert!((s.max - 5.0).abs() < 1e-9);
}
