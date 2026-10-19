//! The incremental check (docs/adr/0524) gives the full check's findings: a block of parcels with an overlap, a gap,
//! a sliver and an island, roads with dangles and points to be covered, edited at random (moved, grown, removed,
//! added) 150 times from three seeds; after each edit the recheck from the last result and the full check are the same
//! findings (rule, problem, objects by uid, place, measure, exception), the same counts. Lasting keys stay.

use std::collections::HashSet;

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::topology_rules::incremental::{Previous, finding_key, recheck};
use kentos_geometry_core::ops::topology_rules::{Checked, Exception, Kind, Objects, Rule, check};
use kentos_geometry_core::vec2::Vec2;

const T: f64 = 0.001;

fn square(x: f64, y: f64, w: f64, h: f64) -> Shape {
    Shape::Polygon {
        pts: vec![
            Vec2::new(x, y),
            Vec2::new(x + w, y),
            Vec2::new(x + w, y + h),
            Vec2::new(x, y + h),
        ],
        bulges: None,
        holes: None,
        parts: None,
    }
}

struct Drawing {
    shapes: Vec<Shape>,
    layers: Vec<String>,
    uids: Vec<String>,
    next: usize,
}

impl Drawing {
    fn push(&mut self, s: Shape, layer: &str) {
        self.shapes.push(s);
        self.layers.push(layer.into());
        self.uids.push(format!("u{}", self.next));
        self.next += 1;
    }
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
    for p in kentos_geometry_core::entity::entity_vertices(s) {
        kentos_geometry_core::geometry::extend_bounds(&mut b, p, 0.0);
    }
    b
}

/// A finding as compared: its rule, problem, objects' uids, place and measure on a fine grid, exception.
fn said(c: &Checked, uids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = c
        .findings
        .iter()
        .map(|f| {
            format!(
                "{} {} {:?} {:.6} {:.6} {:?} {}",
                f.rule,
                f.problem,
                f.objects
                    .iter()
                    .map(|&i| uids[i].as_str())
                    .collect::<Vec<_>>(),
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

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() % 10_000) as f64 / 10_000.0
    }
}

#[test]
fn the_recheck_finds_what_the_full_check_finds() {
    for seed in [
        0x9e37_79b9_7f4a_7c15_u64,
        0x2545_f491_4f6c_dd1d,
        0x1234_5678_9abc_def1,
    ] {
        edited(seed, 150);
    }
}

fn edited(seed: u64, steps: usize) {
    let (e, n) = (487_000.0, 4_420_000.0);
    let mut d = Drawing {
        shapes: Vec::new(),
        layers: Vec::new(),
        uids: Vec::new(),
        next: 0,
    };
    for i in 0..8 {
        for j in 0..8 {
            if (i, j) == (4, 4) {
                continue; // a gap
            }
            let grow = if (i, j) == (2, 3) { 0.4 } else { 0.0 }; // an overlap
            d.push(
                square(
                    e + f64::from(i) * 20.0,
                    n + f64::from(j) * 20.0,
                    20.0 + grow,
                    20.0,
                ),
                "parsel",
            );
        }
    }
    d.push(square(e + 85.0, n + 85.0, 6.0, 6.0), "parsel"); // an island in the gap
    d.push(square(e + 200.0, n, 30.0, 0.05), "parsel"); // a sliver
    for k in 0..6 {
        let y = n - 10.0 - f64::from(k) * 4.0;
        d.push(
            Shape::Line {
                a: Vec2::new(e, y),
                b: Vec2::new(e + 160.0, y),
            },
            "yol",
        );
        d.push(
            Shape::Point {
                p: Vec2::new(
                    e + 10.0 + f64::from(k) * 25.0,
                    n + 10.0 + f64::from(k) * 20.0,
                ),
                z: None,
                parts: None,
            },
            "nokta",
        );
    }
    let rule = |id: &str, kind, layer: &str, other: Option<&str>| Rule {
        id: id.into(),
        kind,
        layer: layer.into(),
        other: other.map(Into::into),
        value: None,
    };
    let rules = vec![
        rule("r1", Kind::MustNotOverlap, "parsel", None),
        rule("r2", Kind::MustNotHaveGaps, "parsel", None),
        rule("r3", Kind::MustNotHaveSlivers, "parsel", None),
        rule("r4", Kind::MustNotHaveDangles, "yol", None),
        rule("r5", Kind::MustBeCoveredBy, "nokta", Some("parsel")),
        rule("r6", Kind::MustNotHaveShortEdges, "parsel", None),
        rule("r7", Kind::MustBeValid, "parsel", None),
        rule("r8", Kind::MustNotHaveMissingVertices, "parsel", None),
        rule("r9", Kind::MustBeOnEndOf, "nokta", Some("yol")),
        rule("r10", Kind::MustNotHaveSmallAngles, "parsel", None),
        rule("r11", Kind::MustNotHaveDuplicates, "nokta", None),
        rule("r12", Kind::BoundaryMustBeCoveredBy, "parsel", Some("yol")),
    ];
    let first = check(&d.objects(), &rules, T, &[]);
    // One overlap left on purpose.
    let o = first
        .findings
        .iter()
        .find(|f| f.problem == "overlap")
        .expect("an overlap");
    let exceptions = vec![Exception {
        rule: "r1".into(),
        objects: o.objects.iter().map(|&i| d.uids[i].clone()).collect(),
        at: o.at,
    }];
    let mut last = check(&d.objects(), &rules, T, &exceptions);
    let mut last_uids = d.uids.clone();
    let mut rng = Rng(seed);
    for step in 0..steps {
        let mut changed: HashSet<String> = HashSet::new();
        let mut touched: Vec<Bounds> = Vec::new();
        let i = rng.below(d.shapes.len());
        match rng.below(4) {
            0 | 1 => {
                // Moved a little, or grown.
                touched.push(bounds(&d.shapes[i]));
                let (dx, dy) = (rng.unit() - 0.5, rng.unit() - 0.5);
                let s = match &d.shapes[i] {
                    Shape::Polygon { pts, .. } => {
                        let b = bounds(&d.shapes[i]);
                        square(
                            b.min_x + dx,
                            b.min_y + dy,
                            b.max_x - b.min_x + dx.abs(),
                            pts[2].y - pts[1].y,
                        )
                    }
                    Shape::Line { a, b } => Shape::Line {
                        a: Vec2::new(a.x + dx * 5.0, a.y),
                        b: *b,
                    },
                    Shape::Point { p, .. } => Shape::Point {
                        p: Vec2::new(p.x + dx * 30.0, p.y + dy * 30.0),
                        z: None,
                        parts: None,
                    },
                    s => s.clone(),
                };
                touched.push(bounds(&s));
                d.shapes[i] = s;
                changed.insert(d.uids[i].clone());
            }
            2 => {
                touched.push(bounds(&d.shapes[i]));
                changed.insert(d.uids[i].clone());
                d.shapes.remove(i);
                d.layers.remove(i);
                d.uids.remove(i);
            }
            _ => {
                let s = square(
                    e + rng.unit() * 160.0,
                    n + rng.unit() * 160.0,
                    5.0 + rng.unit() * 10.0,
                    7.0,
                );
                touched.push(bounds(&s));
                d.push(s, "parsel");
                changed.insert(d.uids.last().expect("added").clone());
            }
        }
        let names: HashSet<&str> = changed.iter().map(String::as_str).collect();
        let again = recheck(
            &d.objects(),
            &rules,
            T,
            &exceptions,
            &Previous {
                findings: &last.findings,
                uids: &last_uids,
            },
            &names,
            &touched,
        );
        let full = check(&d.objects(), &rules, T, &exceptions);
        let (a, b) = (said(&again, &d.uids), said(&full, &d.uids));
        let only_a: Vec<&String> = a.iter().filter(|x| !b.contains(x)).collect();
        let only_b: Vec<&String> = b.iter().filter(|x| !a.contains(x)).collect();
        assert!(
            only_a.is_empty() && only_b.is_empty() && a.len() == b.len(),
            "seed {seed:x} step {step}: only in the recheck {only_a:?}, only in the full check {only_b:?}"
        );
        assert_eq!(again.looked, full.looked, "seed {seed:x} step {step}");
        last = again;
        last_uids = d.uids.clone();
    }
}

#[test]
fn a_finding_keeps_its_key_and_another_has_another() {
    let at = Vec2::new(487_010.0, 4_420_010.0);
    let a = finding_key("r1", &["u1", "u2"], at, 0.001);
    assert_eq!(
        a,
        finding_key("r1", &["u1", "u2"], Vec2::new(at.x + 0.0001, at.y), 0.001)
    );
    assert_ne!(a, finding_key("r1", &["u2", "u1"], at, 0.001));
    assert_ne!(a, finding_key("r2", &["u1", "u2"], at, 0.001));
    assert_ne!(
        a,
        finding_key("r1", &["u1", "u2"], Vec2::new(at.x + 0.01, at.y), 0.001)
    );
    assert_eq!(a.len(), 16);
}

/// 10 000 parcels, one moved: the full check and the recheck, rule by rule; Boşluk olmamalı checks its cluster
/// whole (the block is one). `cargo test --release -p kentos-geometry-core --test all topology_incremental::timing
/// -- --ignored --nocapture`.
#[test]
#[ignore]
fn timing() {
    let (e, n) = (487_000.0, 4_420_000.0);
    let mut d = Drawing {
        shapes: Vec::new(),
        layers: Vec::new(),
        uids: Vec::new(),
        next: 0,
    };
    for i in 0..100 {
        for j in 0..100 {
            d.push(
                square(e + f64::from(i) * 20.0, n + f64::from(j) * 20.0, 20.0, 20.0),
                "parsel",
            );
        }
    }
    for kind in [
        Kind::MustNotOverlap,
        Kind::MustNotHaveMissingVertices,
        Kind::MustNotHaveShortEdges,
        Kind::MustNotHaveSmallAngles,
        Kind::MustBeValid,
        Kind::MustNotHaveGaps,
    ] {
        let rules = [Rule {
            id: "r".into(),
            kind,
            layer: "parsel".into(),
            other: None,
            value: None,
        }];
        let before = check(&d.objects(), &rules, T, &[]);
        let mut moved = Drawing {
            shapes: d.shapes.clone(),
            layers: d.layers.clone(),
            uids: d.uids.clone(),
            next: d.next,
        };
        let was = bounds(&moved.shapes[5050]);
        moved.shapes[5050] = square(was.min_x + 0.3, was.min_y, 20.0, 20.0);
        let changed: HashSet<&str> = std::iter::once(moved.uids[5050].as_str()).collect();
        let t0 = std::time::Instant::now();
        let full = check(&moved.objects(), &rules, T, &[]);
        let whole = t0.elapsed().as_secs_f64();
        let t0 = std::time::Instant::now();
        let again = recheck(
            &moved.objects(),
            &rules,
            T,
            &[],
            &Previous {
                findings: &before.findings,
                uids: &d.uids,
            },
            &changed,
            &[was, bounds(&moved.shapes[5050])],
        );
        let part = t0.elapsed().as_secs_f64();
        assert_eq!(said(&again, &moved.uids), said(&full, &moved.uids));
        println!(
            "{:<28} bütün {whole:.3} s  değişen {part:.3} s  {} bulgu",
            kind.key(),
            full.findings.len()
        );
    }
}
