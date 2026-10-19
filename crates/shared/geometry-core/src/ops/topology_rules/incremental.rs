//! The findings kept as the drawing changes (docs/adr/0524): a lasting key for each finding, and a check of what a
//! change can touch alone. The objects changed (their uids) and where they were and are (`touched`) make the
//! affected set: they, the objects near them, and for Boşluk olmamalı the whole cluster of areas joined to them
//! (a gap is a hole of the union of a cluster, the islands in it included). The rules run over the affected set
//! and its neighbours; the new findings that name an affected object replace the old ones that did, the others
//! are kept. The result is the full check's: the same findings, the same exceptions, the same counts
//! (`incremental_cases` holds it to that over random edits).

use std::collections::{HashMap, HashSet};

use super::{
    Checked, Exception, Finding, Kind, Objects, Rule, Taken, check, is_areas, is_paths, is_points,
    taken,
};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min, js_round};
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// A finding's lasting key: its rule's id, its objects' uids in the finding's order and its place on the
/// tolerance's grid, as 16 hexadecimal digits (FNV-1a, 64 bits). The same finding found again keeps its key; a
/// finding whose objects or place changed is another.
pub fn finding_key(rule: &str, uids: &[&str], at: Vec2, tolerance: f64) -> String {
    let step = if tolerance > 0.0 { tolerance } else { 1e-6 };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h ^= 0xff;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    eat(rule.as_bytes());
    for u in uids {
        eat(u.as_bytes());
    }
    eat(&(js_round(at.x / step) as i64).to_le_bytes());
    eat(&(js_round(at.y / step) as i64).to_le_bytes());
    format!("{h:016x}")
}

/// A shape's box as the rules see it: what a change touched.
pub fn shape_bounds(s: &crate::entity::Shape) -> Bounds {
    taken(s).bounds
}

/// What the last check found: its findings, its objects' uids by their places then.
pub struct Previous<'a> {
    pub findings: &'a [Finding],
    pub uids: &'a [String],
}

/// Whether two boxes come within `t` of each other.
fn near(a: &Bounds, b: &Bounds, t: f64) -> bool {
    a.min_x <= b.max_x + t
        && b.min_x <= a.max_x + t
        && a.min_y <= b.max_y + t
        && b.min_y <= a.max_y + t
}

/// The distance from a point to a box (0 inside).
fn box_to_point(b: &Bounds, p: Vec2) -> f64 {
    let dx = js_max(js_max(b.min_x - p.x, p.x - b.max_x), 0.0);
    let dy = js_max(js_max(b.min_y - p.y, p.y - b.max_y), 0.0);
    js_hypot(dx, dy)
}

/// The distance between two boxes (0 when they meet).
fn box_to_box(a: &Bounds, b: &Bounds) -> f64 {
    let dx = js_max(js_max(a.min_x - b.max_x, b.min_x - a.max_x), 0.0);
    let dy = js_max(js_max(a.min_y - b.max_y, b.min_y - a.max_y), 0.0);
    js_hypot(dx, dy)
}

/// The farthest a point of one box is from the nearest point of the other: no point of `a` is farther than it
/// from everything in `b`.
fn farthest(a: &Bounds, b: &Bounds) -> f64 {
    let dx = js_max(js_max(a.max_x - b.min_x, b.max_x - a.min_x), 0.0);
    let dy = js_max(js_max(a.max_y - b.min_y, b.max_y - a.min_y), 0.0);
    js_hypot(dx, dy)
}

fn grown(b: &Bounds, t: f64) -> Bounds {
    Bounds {
        min_x: b.min_x - t,
        min_y: b.min_y - t,
        max_x: b.max_x + t,
        max_y: b.max_y + t,
    }
}

/// The objects a rule looks at, as `check` takes them: its layer's and its other layer's of the classes it reads.
fn looked_at(objects: &Objects<'_>, taken: &[Taken], rule: &Rule) -> usize {
    let of = |layer: &str, want: &[fn(&crate::ops::geoprocess::Class) -> bool]| {
        (0..objects.shapes.len())
            .filter(|&i| objects.layers[i] == layer && want.iter().any(|w| w(&taken[i].class)))
            .count()
    };
    let l = rule.layer.as_str();
    match rule.kind {
        Kind::MustNotOverlap
        | Kind::MustNotOverlapWith
        | Kind::MustNotHaveGaps
        | Kind::MustNotHaveSlivers
        | Kind::BoundaryMustBeCoveredBy => of(l, &[is_areas]),
        Kind::MustNotHaveMissingVertices => (0..objects.shapes.len())
            .filter(|&i| {
                objects.layers[i] == l
                    && is_areas(&taken[i].class)
                    && matches!(objects.shapes[i], crate::entity::Shape::Polygon { .. })
            })
            .count(),
        Kind::MustBeCoveredBy => of(l, &[is_areas, is_paths, is_points]),
        Kind::MustNotHaveDuplicates => of(l, &[is_paths, is_points]),
        Kind::MustNotHaveDangles => of(l, &[is_paths]),
        Kind::MustNotHaveShortEdges | Kind::MustNotHaveSmallAngles | Kind::MustBeValid => {
            of(l, &[is_paths, is_areas])
        }
        Kind::MustBeOnEndOf => of(l, &[is_points]),
    }
}

/// The check again after a change (docs/adr/0524): `changed` the uids of the objects added, changed or removed
/// since `previous`, `touched` the boxes they had and have. The rules, the tolerance and the exceptions are the
/// previous check's (else the caller checks everything again).
pub fn recheck(
    objects: &Objects<'_>,
    rules: &[Rule],
    tolerance: f64,
    exceptions: &[Exception],
    previous: &Previous<'_>,
    changed: &HashSet<&str>,
    touched: &[Bounds],
) -> Checked {
    let t = tolerance;
    let n = objects.shapes.len();
    let taken: Vec<Taken> = objects.shapes.iter().map(taken).collect();
    let entries: Vec<(u32, Bounds)> = taken
        .iter()
        .enumerate()
        .map(|(i, k)| (i as u32, grown(&k.bounds, t)))
        .collect();
    let tree = PackedTree::build(&entries);
    let around = |b: &Bounds, out: &mut Vec<u32>| {
        out.clear();
        tree.search(&grown(b, t), out);
    };
    // The affected: the changed objects and those near where the changed were or are.
    let mut affected = vec![false; n];
    let mut hits = Vec::new();
    for (i, uid) in objects.uids.iter().enumerate() {
        if changed.contains(uid.as_str()) {
            affected[i] = true;
        }
    }
    for b in touched {
        around(b, &mut hits);
        for &i in &hits {
            if near(&taken[i as usize].bounds, b, t) {
                affected[i as usize] = true;
            }
        }
    }
    // Sarkan uç, … çizgilerinin ucunda olmalı and … içinde kalmalı measure to the nearest connector, however
    // far: an old finding
    // whose reach (its distance, or everything without one) a change came into is found again.
    let now: HashMap<&str, usize> = objects
        .uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.as_str(), i))
        .collect();
    for f in previous.findings {
        let kind = rules.get(f.rule).map(|r| r.kind);
        if !matches!(
            kind,
            Some(Kind::MustNotHaveDangles | Kind::MustBeOnEndOf | Kind::MustBeCoveredBy)
        ) {
            continue;
        }
        // A distance to the nearest reaches as far; another measure (an area outside) is the objects' own.
        let reach = match f.measure_kind {
            Some(super::MeasureKind::Distance) => f.measure.unwrap_or(f64::INFINITY) + t,
            _ => t,
        };
        if touched.iter().any(|b| box_to_point(b, f.at) <= reach) {
            for &k in &f.objects {
                if let Some(&i) = previous.uids.get(k).and_then(|u| now.get(u.as_str())) {
                    affected[i] = true;
                }
            }
        }
    }
    // Boşluk olmamalı: a gap is a hole of a cluster's union, less the clusters inside it. The layer's areas in
    // clusters (joined within the tolerance); a cluster with an affected area, or whose box meets where a change
    // was, is checked whole, and so is every cluster whose box meets a checked one's (one inside its hole, or
    // around it), until none is added.
    for rule in rules.iter().filter(|r| r.kind == Kind::MustNotHaveGaps) {
        let areas: Vec<usize> = (0..n)
            .filter(|&i| objects.layers[i] == rule.layer && is_areas(&taken[i].class))
            .collect();
        let mut parent: HashMap<usize, usize> = areas.iter().map(|&i| (i, i)).collect();
        fn root(parent: &mut HashMap<usize, usize>, mut i: usize) -> usize {
            while parent[&i] != i {
                let up = parent[&parent[&i]];
                parent.insert(i, up);
                i = up;
            }
            i
        }
        for &i in &areas {
            around(&taken[i].bounds, &mut hits);
            for &j in &hits {
                let j = j as usize;
                if j != i && parent.contains_key(&j) && near(&taken[i].bounds, &taken[j].bounds, t)
                {
                    let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                    if a != b {
                        parent.insert(a.max(b), a.min(b));
                    }
                }
            }
        }
        let mut clusters: HashMap<usize, (Vec<usize>, Bounds)> = HashMap::new();
        for &i in &areas {
            let r = root(&mut parent, i);
            let c = clusters
                .entry(r)
                .or_insert_with(|| (Vec::new(), crate::geometry::empty_bounds()));
            c.0.push(i);
            super::grow(&mut c.1, &taken[i].bounds);
        }
        let mut chosen: HashSet<usize> = clusters
            .iter()
            .filter(|(_, (members, b))| {
                members.iter().any(|&i| affected[i]) || touched.iter().any(|x| near(b, x, t))
            })
            .map(|(&r, _)| r)
            .collect();
        loop {
            let boxes: Vec<Bounds> = chosen.iter().map(|r| clusters[r].1).collect();
            let more: Vec<usize> = clusters
                .iter()
                .filter(|(r, (_, b))| !chosen.contains(r) && boxes.iter().any(|x| near(b, x, t)))
                .map(|(&r, _)| r)
                .collect();
            if more.is_empty() {
                break;
            }
            chosen.extend(more);
        }
        for r in chosen {
            for &i in &clusters[&r].0 {
                affected[i] = true;
            }
        }
    }
    // The rules run over the affected and their neighbours.
    let mut part = affected.clone();
    for i in 0..n {
        if !affected[i] {
            continue;
        }
        around(&taken[i].bounds, &mut hits);
        for &j in &hits {
            if near(&taken[i].bounds, &taken[j as usize].bounds, t) {
                part[j as usize] = true;
            }
        }
    }
    // An affected end's nearest connector is in the check, wherever it is: every connector whose box is no
    // farther than the nearest box's farthest corner (an edge in a box is no farther than its far corner).
    for rule in rules.iter().filter(|r| {
        matches!(
            r.kind,
            Kind::MustNotHaveDangles | Kind::MustBeOnEndOf | Kind::MustBeCoveredBy
        )
    }) {
        let any: fn(&crate::ops::geoprocess::Class) -> bool =
            |c| is_areas(c) || is_paths(c) || is_points(c);
        let (subject, connector): (fn(&crate::ops::geoprocess::Class) -> bool, &str) =
            match rule.kind {
                Kind::MustNotHaveDangles => (is_paths, rule.layer.as_str()),
                Kind::MustBeOnEndOf => (is_points, rule.other.as_deref().unwrap_or("")),
                _ => (any, rule.other.as_deref().unwrap_or("")),
            };
        let connects = |j: usize| {
            objects.layers[j] == connector
                && match rule.kind {
                    Kind::MustNotHaveDangles => {
                        is_paths(&taken[j].class) || is_areas(&taken[j].class)
                    }
                    Kind::MustBeOnEndOf => is_paths(&taken[j].class),
                    _ => is_areas(&taken[j].class),
                }
        };
        let connectors: Vec<usize> = (0..n).filter(|&j| connects(j)).collect();
        for i in 0..n {
            if !affected[i] || objects.layers[i] != rule.layer || !subject(&taken[i].class) {
                continue;
            }
            let b = &taken[i].bounds;
            let reach = connectors
                .iter()
                .filter(|&&j| j != i)
                .map(|&j| farthest(b, &taken[j].bounds))
                .fold(f64::INFINITY, js_min);
            for &j in &connectors {
                if box_to_box(b, &taken[j].bounds) <= reach + t {
                    part[j] = true;
                }
            }
        }
    }
    let places: Vec<usize> = (0..n).filter(|&i| part[i]).collect();
    let shapes: Vec<_> = places.iter().map(|&i| objects.shapes[i].clone()).collect();
    let layers: Vec<String> = places.iter().map(|&i| objects.layers[i].clone()).collect();
    let uids: Vec<String> = places.iter().map(|&i| objects.uids[i].clone()).collect();
    let found = check(
        &Objects {
            shapes: &shapes,
            layers: &layers,
            uids: &uids,
        },
        rules,
        t,
        exceptions,
    );
    let back = |k: usize| places[k];
    let mut findings: Vec<Finding> = found
        .findings
        .into_iter()
        .map(|mut f| {
            f.objects = f.objects.iter().map(|&k| back(k)).collect();
            f.subject = f.subject.map(back);
            f
        })
        .filter(|f| f.objects.iter().any(|&i| affected[i]))
        .collect();
    // The old findings no change touched, their objects at their places now.
    for f in previous.findings {
        let Some(at): Option<Vec<usize>> = f
            .objects
            .iter()
            .map(|&k| {
                previous
                    .uids
                    .get(k)
                    .and_then(|u| now.get(u.as_str()).copied())
            })
            .collect()
        else {
            continue;
        };
        if at.iter().any(|&i| affected[i]) {
            continue;
        }
        let subject = match f.subject {
            Some(k) => match previous.uids.get(k).and_then(|u| now.get(u.as_str())) {
                Some(&i) => Some(i),
                None => continue,
            },
            None => None,
        };
        let mut kept = f.clone();
        kept.objects = at;
        kept.subject = subject;
        findings.push(kept);
    }
    // The rules' order, as the full check gives them.
    findings.sort_by_key(|f| f.rule);
    Checked {
        findings,
        looked: rules
            .iter()
            .map(|r| looked_at(objects, &taken, r))
            .collect(),
    }
}
