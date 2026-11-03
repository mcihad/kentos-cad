//! Deterministic recognition of rigid/similar analytic 2D collections (CAD-ADV-07).
//! This only finds candidates; it neither groups objects nor discards their identities/relations.
//! Every member must match, once. A spatial index narrows comparisons, never decides equality.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::block::placement;
use crate::entity::Shape;
use crate::geom::affine::translation;
use crate::geom::arc::sweep;
use crate::geometry::{Bounds, dist};
use crate::jsmath::{atan2, cos, js_max, sin};
use crate::ops::transform::transform_shape;
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

mod path;
mod places;
mod pose;
pub use places::{PlaceMatch, map_place};

/// Bound path preparation as well as the subsequent analytic comparisons.
pub const VERTEX_LIMIT: usize = 200_000;
pub const PATH_VERTEX_LIMIT: usize = 4096;

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub rotations: bool,
    pub mirrors: bool,
    pub scales: bool,
    pub text_variants: bool,
    pub attribute_variants: bool,
    /// Absolute model-space tolerance; never relative to distance from world origin.
    pub tolerance: f64,
    pub limit: usize,
    /// A bounded scan reports partial results explicitly instead of blocking indefinitely.
    pub budget: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rotations: true,
            mirrors: false,
            scales: false,
            text_variants: false,
            attribute_variants: false,
            tolerance: 1e-6,
            limit: 1000,
            budget: 2_000_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    /// Target object indices, aligned with the input source order; never one target twice.
    pub members: Vec<usize>,
    /// Relative to `origin`, mirror in the x axis, scale/rotate, then place at `at`.
    pub origin: Vec2,
    pub at: Vec2,
    pub rotation: f64,
    pub scale: f64,
    pub mirror: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    pub matches: Vec<Match>,
    pub truncated: bool,
    pub comparisons: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidOptions,
    InvalidSource,
    UnsupportedSource(usize),
    GeometryLimit,
    Cancelled,
}

/// First recognition slice: analytic primitives only. Unsupported source shapes are refused, never skipped.
pub fn supported(shape: &Shape) -> bool {
    match shape {
        Shape::Line { a, b } => finite(*a) && finite(*b) && dist(*a, *b) > 0.0,
        Shape::Circle { c, r } => finite(*c) && r.is_finite() && *r > 0.0,
        Shape::Arc { c, r, a0, a1 } => {
            finite(*c) && r.is_finite() && *r > 0.0 && a0.is_finite() && a1.is_finite()
        }
        Shape::Polyline { .. } => path::supported(shape),
        Shape::Text { .. } | Shape::Insert { .. } => pose::supported(shape),
        _ => false,
    }
}

fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn centre(shape: &Shape) -> Option<Vec2> {
    match shape {
        Shape::Line { a, b } => Some(Vec2::new(a.x + (b.x - a.x) / 2.0, a.y + (b.y - a.y) / 2.0)),
        Shape::Circle { c, .. } | Shape::Arc { c, .. } => Some(*c),
        Shape::Polyline { .. } => path::centre(shape),
        Shape::Text { p, .. } | Shape::Insert { p, .. } => Some(*p),
        _ => None,
    }
}

fn on(c: Vec2, r: f64, angle: f64) -> Vec2 {
    Vec2::new(c.x + r * cos(angle), c.y + r * sin(angle))
}

/// Strict analytic equality: endpoint direction is immaterial for lines, arc span is not.
fn equal(a: &Shape, b: &Shape, tolerance: f64, check: &mut impl FnMut() -> bool) -> Option<bool> {
    equal_with(a,b,tolerance,check,false,false)
}
/// Strict geometry/pose/content verification at the final world placement; never ignores text values.
pub fn equivalent(a:&Shape,b:&Shape,tolerance:f64,check:&mut impl FnMut()->bool)->Option<bool> {
    equal(a,b,tolerance,check)
}
fn equal_with(a: &Shape, b: &Shape, tolerance: f64, check: &mut impl FnMut() -> bool, text_variants:bool, attribute_variants:bool) -> Option<bool> {
    Some(match (a, b) {
        (Shape::Text { .. },Shape::Text { .. }) | (Shape::Insert { .. },Shape::Insert { .. }) => {
            for _ in 0..pose::work(a).max(pose::work(b)) { if !check() {return None;} }
            pose::equal(a,b,tolerance,text_variants,attribute_variants)
        }
        (Shape::Polyline { .. }, Shape::Polyline { .. }) => {
            return path::equal(a, b, tolerance, check);
        }
        (Shape::Line { a, b }, Shape::Line { a: c, b: d }) => {
            (dist(*a, *c) <= tolerance && dist(*b, *d) <= tolerance)
                || (dist(*a, *d) <= tolerance && dist(*b, *c) <= tolerance)
        }
        (Shape::Circle { c, r }, Shape::Circle { c: d, r: s }) => {
            dist(*c, *d) <= tolerance && (r - s).abs() <= tolerance
        }
        (
            Shape::Arc { c, r, a0, a1 },
            Shape::Arc {
                c: d,
                r: s,
                a0: b0,
                a1: b1,
            },
        ) => {
            let (sa, sb) = (sweep(*a0, *a1), sweep(*b0, *b1));
            dist(*c, *d) <= tolerance
                && (r - s).abs() <= tolerance
                && (sa - sb).abs() * js_max(*r, *s) <= tolerance
                && [0.0, 0.5, 1.0]
                    .iter()
                    .all(|t| dist(on(*c, *r, a0 + sa * t), on(*d, *s, b0 + sb * t)) <= tolerance)
        }
        _ => false,
    })
}

/// Prefer a genuine directed feature. Circle-only collections use their centres (a concentric set needs no turn).
fn anchor(scene: &[Shape], source: &[usize]) -> (usize, Vec2, Vec2, bool) {
    for &i in source {
        if let Shape::Line { a, b } = scene[i] {
            return (i, a, b, false);
        }
    }
    for &i in source {
        if let Shape::Arc { c, r, a0, .. } = scene[i] {
            return (i, c, on(c, r, a0), false);
        }
    }
    for &i in source {
        if let Some((a, b)) = path::anchor(&scene[i]) {
            return (i, a, b, false);
        }
    }
    for &i in source {
        if let Some((p,size,angle))=pose::frame(&scene[i]) {return (i,p,on(p,size,angle),false);}
    }
    let i = source[0];
    let Shape::Circle { c, r } = scene[i] else {
        unreachable!()
    };
    for &j in source {
        if let Shape::Circle { c: d, .. } = scene[j]
            && dist(c, d) > 0.0
        {
            return (i, c, d, true);
        }
    }
    (i, c, Vec2::new(c.x + r, c.y), false)
}

fn pairs(scene: &[Shape], source: &Shape, target: usize, circle_pair: bool) -> Vec<(Vec2, Vec2)> {
    match (source, &scene[target]) {
        (Shape::Text { .. }, Shape::Text { .. }) | (Shape::Insert { .. }, Shape::Insert { .. }) =>
            pose::frame(&scene[target]).map(|(p,size,angle)|vec![(p,on(p,size,angle))]).unwrap_or_default(),
        (Shape::Polyline { .. }, Shape::Polyline { .. }) => path::pairs(&scene[target]),
        (Shape::Line { .. }, Shape::Line { a, b }) => vec![(*a, *b), (*b, *a)],
        (Shape::Arc { .. }, Shape::Arc { c, r, a0, a1 }) => {
            vec![(*c, on(*c, *r, *a0)), (*c, on(*c, *r, *a1))]
        }
        (Shape::Circle { .. }, Shape::Circle { c, r }) if !circle_pair => {
            vec![(*c, Vec2::new(c.x + r, c.y))]
        }
        // The caller checks cancellation/budget for each pair. No pair is accepted without the complete collection.
        (Shape::Circle { .. }, Shape::Circle { c, .. }) => scene
            .iter()
            .filter_map(|s| match s {
                Shape::Circle { c: d, .. } if dist(*c, *d) > 0.0 => Some((*c, *d)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A small bipartite augmenting path preserves duplicate multiplicity without greedy false negatives.
fn assign(
    member: usize,
    edges: &[Vec<usize>],
    assigned: &mut HashMap<usize, usize>,
    visited: &mut HashSet<usize>,
) -> bool {
    for &target in &edges[member] {
        if !visited.insert(target) {
            continue;
        }
        if assigned
            .get(&target)
            .copied()
            .is_none_or(|other| assign(other, edges, assigned, visited))
        {
            assigned.insert(target, member);
            return true;
        }
    }
    false
}

/// Find equivalent collections. `compatible` compares application-owned layer/style/data policy.
/// Source and scene indices are stable caller identities, and the result is deterministic for their order.
pub fn find(
    scene: &[Shape],
    source: &[usize],
    options: Options,
    compatible: impl Fn(usize, usize) -> bool,
    mut stopped: impl FnMut() -> bool,
) -> Result<Found, Error> {
    if !options.tolerance.is_finite()
        || options.tolerance <= 0.0
        || options.limit == 0
        || options.budget == 0
        || scene.len() > u32::MAX as usize
    {
        return Err(Error::InvalidOptions);
    }
    if source.is_empty()
        || source.len() > 128
        || source.iter().any(|i| *i >= scene.len())
        || source.iter().collect::<HashSet<_>>().len() != source.len()
    {
        return Err(Error::InvalidSource);
    }
    for &i in source {
        if !supported(&scene[i]) {
            return Err(Error::UnsupportedSource(i));
        }
    }
    if stopped() {
        return Err(Error::Cancelled);
    }
    if scene
        .iter()
        .try_fold(0usize, |n, s| n.checked_add(path::vertices(s)))
        .is_none_or(|n| n > VERTEX_LIMIT)
    {
        return Err(Error::GeometryLimit);
    }
    let mut entries = Vec::new();
    let mut eligible = vec![false; scene.len()];
    for (i, s) in scene.iter().enumerate() {
        if stopped() {
            return Err(Error::Cancelled);
        }
        if supported(s)
            && let Some(p) = centre(s).filter(|p| finite(*p))
        {
            eligible[i] = true;
            entries.push((
                i as u32,
                Bounds {
                    min_x: p.x,
                    min_y: p.y,
                    max_x: p.x,
                    max_y: p.y,
                },
            ));
        }
    }
    let index = PackedTree::build(&entries);
    let (anchor, origin, end, circle_pair) = anchor(scene, source);
    // Localize before similarity transforms: survey coordinates must not set the recognition precision.
    let local: Vec<_> = source
        .iter()
        .map(|i| transform_shape(&scene[*i], &translation(-origin.x, -origin.y)))
        .collect();
    let mut found = Found::default();
    let mut seen = BTreeSet::new();
    let mut nearby = Vec::new();
    for target in 0..scene.len() {
        if stopped() {
            return Err(Error::Cancelled);
        }
        if !eligible[target] {
            continue;
        }
        for (at, toward) in pairs(scene, &scene[anchor], target, circle_pair) {
            for mirror in [false, true] {
                if mirror && !options.mirrors {
                    continue;
                }
                found.comparisons += 1;
                if found.comparisons > options.budget {
                    found.truncated = true;
                    return Ok(found);
                }
                if stopped() {
                    return Err(Error::Cancelled);
                }
                let source_angle = pose::frame(&scene[anchor]).map(|(_,_,a)|if mirror {-a}else{a}).unwrap_or_else(||atan2(
                    if mirror {
                        origin.y - end.y
                    } else {
                        end.y - origin.y
                    },
                    end.x - origin.x,
                ));
                let rotation = if options.rotations {
                    pose::frame(&scene[target]).map(|(_,_,a)|a).unwrap_or_else(||atan2(toward.y - at.y, toward.x - at.x)) - source_angle
                } else {
                    0.0
                };
                let scale = if options.scales {
                    match (pose::frame(&scene[anchor]),pose::frame(&scene[target])) {
                        (Some((_,a,_)),Some((_,b,_)))=>b/a,
                        _=>dist(at, toward) / dist(origin, end)
                    }
                } else {
                    1.0
                };
                if !scale.is_finite() || scale <= 0.0 {
                    continue;
                }
                let matrix = placement(at, scale, rotation, mirror);
                let mut edges = Vec::with_capacity(source.len());
                for (member, shape) in local.iter().enumerate() {
                    found.comparisons += path::vertices(shape) as u64;
                    if found.comparisons > options.budget {
                        found.truncated = true;
                        return Ok(found);
                    }
                    if stopped() {
                        return Err(Error::Cancelled);
                    }
                    let mapped = transform_shape(shape, &matrix);
                    let Some(p) = centre(&mapped).filter(|p| finite(*p)) else {
                        break;
                    };
                    let t = options.tolerance;
                    nearby.clear();
                    // The anchor identity is already fixed by this trial. Searching coincident
                    // objects here adds quadratic work without adding a possible assignment.
                    // Its geometry and application-owned properties are still checked below.
                    if source[member] == anchor {
                        nearby.push(target as u32);
                    } else {
                        index.search(
                            &Bounds {
                                min_x: p.x - t,
                                min_y: p.y - t,
                                max_x: p.x + t,
                                max_y: p.y + t,
                            },
                            &mut nearby,
                        );
                        nearby.sort_unstable();
                    }
                    let mut matches = Vec::new();
                    for &j in &nearby {
                        found.comparisons += 1;
                        if found.comparisons > options.budget {
                            found.truncated = true;
                            return Ok(found);
                        }
                        if stopped() {
                            return Err(Error::Cancelled);
                        }
                        if compatible(source[member], j as usize) {
                            let mut cancelled = false;
                            let matched = equal_with(&mapped, &scene[j as usize], t, &mut || {
                                if stopped() {
                                    cancelled = true;
                                    return false;
                                }
                                found.comparisons += 1;
                                found.comparisons <= options.budget
                            }, options.text_variants, options.attribute_variants);
                            match matched {
                                Some(true) => matches.push(j as usize),
                                Some(false) => {}
                                None if cancelled => return Err(Error::Cancelled),
                                None => {
                                    found.truncated = true;
                                    return Ok(found);
                                }
                            }
                        }
                    }
                    if matches.is_empty() {
                        break;
                    }
                    edges.push(matches);
                }
                if edges.len() != source.len() {
                    continue;
                }
                let mut assigned = HashMap::with_capacity(source.len());
                if !(0..source.len()).all(|i| assign(i, &edges, &mut assigned, &mut HashSet::new()))
                {
                    continue;
                }
                let mut members = vec![0; source.len()];
                for (j, i) in assigned {
                    members[i] = j;
                }
                let mut key = members.clone();
                key.sort_unstable();
                if !seen.insert(key) {
                    continue;
                }
                if found.matches.len() == options.limit {
                    found.truncated = true;
                    return Ok(found);
                }
                found.matches.push(Match {
                    members,
                    origin,
                    at,
                    rotation,
                    scale,
                    mirror,
                });
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol() -> Vec<Shape> {
        vec![
            Shape::Line {
                a: Vec2::new(0.0, 0.0),
                b: Vec2::new(2.0, 0.0),
            },
            Shape::Line {
                a: Vec2::new(0.0, 0.0),
                b: Vec2::new(0.0, 1.0),
            },
            Shape::Circle {
                c: Vec2::new(0.6, 0.3),
                r: 0.13,
            },
            Shape::Arc {
                c: Vec2::new(1.0, 0.5),
                r: 0.2,
                a0: 0.2,
                a1: 1.7,
            },
        ]
    }

    #[test]
    fn forty_rotated_mirrored_symbols_match_once_and_a_similar_symbol_does_not() {
        let source = symbol();
        let mut scene = Vec::new();
        for i in 0..40 {
            let m = placement(
                Vec2::new((i % 10) as f64 * 10.0, (i / 10) as f64 * 10.0),
                1.0,
                i as f64 * 0.17,
                i % 2 == 1,
            );
            scene.extend(source.iter().map(|s| transform_shape(s, &m)));
        }
        let mut wrong = source.clone();
        let Shape::Circle { r, .. } = &mut wrong[2] else {
            unreachable!()
        };
        *r = 0.14;
        scene.extend(
            wrong
                .iter()
                .map(|s| transform_shape(s, &translation(200.0, 10.0))),
        );
        let options = Options {
            mirrors: true,
            ..Default::default()
        };
        let a = find(&scene, &[0, 1, 2, 3], options, |_, _| true, || false).unwrap();
        assert_eq!(a.matches.len(), 40);
        assert!(!a.truncated);
        for group in &a.matches {
            assert_eq!(group.members.len(), 4);
            assert!(group.members.iter().all(|i| *i < 160));
        }
        assert_eq!(
            a,
            find(&scene, &[0, 1, 2, 3], options, |_, _| true, || false).unwrap()
        );
        assert_eq!(
            find(
                &scene,
                &[0, 1, 2, 3],
                Options::default(),
                |_, _| true,
                || false
            )
            .unwrap()
            .matches
            .len(),
            20
        );
    }

    #[test]
    fn survey_coordinates_and_explicit_scale_and_property_policy_are_preserved() {
        let source = symbol();
        let mut scene = source.clone();
        scene.extend(source.iter().map(|s| {
            transform_shape(
                s,
                &placement(Vec2::new(452345.123, 4412345.678), 2.0, 0.31, true),
            )
        }));
        let options = Options {
            mirrors: true,
            scales: true,
            ..Default::default()
        };
        assert_eq!(
            find(&scene, &[0, 1, 2, 3], options, |_, _| true, || false)
                .unwrap()
                .matches
                .len(),
            2
        );
        assert_eq!(
            find(
                &scene,
                &[0, 1, 2, 3],
                options,
                |i, j| i / 4 == j / 4,
                || false
            )
            .unwrap()
            .matches
            .len(),
            1
        );
        assert_eq!(
            find(
                &scene,
                &[0, 1, 2, 3],
                Options {
                    mirrors: true,
                    ..Default::default()
                },
                |_, _| true,
                || false
            )
            .unwrap()
            .matches
            .len(),
            1
        );
    }

    #[test]
    fn circles_only_and_duplicate_multiplicity_are_not_greedily_collapsed() {
        let source = vec![
            Shape::Circle {
                c: Vec2::new(0.0, 0.0),
                r: 0.4,
            },
            Shape::Circle {
                c: Vec2::new(2.0, 1.0),
                r: 0.2,
            },
        ];
        let mut scene = source.clone();
        scene.extend(
            source
                .iter()
                .map(|s| transform_shape(s, &placement(Vec2::new(10.0, 10.0), 1.0, 0.8, false))),
        );
        assert_eq!(
            find(&scene, &[0, 1], Options::default(), |_, _| true, || false)
                .unwrap()
                .matches
                .len(),
            2
        );
        let mut scene = vec![source[0].clone(), source[0].clone()];
        scene.push(transform_shape(&source[0], &translation(10.0, 0.0)));
        let found = find(&scene, &[0, 1], Options::default(), |_, _| true, || false).unwrap();
        assert_eq!(found.matches.len(), 1);
        assert_ne!(found.matches[0].members[0], found.matches[0].members[1]);
        let singles = find(&scene, &[0], Options::default(), |_, _| true, || false).unwrap();
        assert_eq!(
            singles.matches.len(),
            3,
            "coincident objects still have separate identities"
        );
    }

    #[test]
    fn spatial_index_bounds_exact_comparisons_in_a_large_unrelated_drawing() {
        let mut scene = symbol();
        for i in 0..20_000 {
            scene.push(Shape::Line {
                a: Vec2::new(100.0 + i as f64 * 10.0, 100.0),
                b: Vec2::new(103.0 + i as f64 * 10.0, 100.0),
            });
        }
        let found = find(
            &scene,
            &[0, 1, 2, 3],
            Options::default(),
            |_, _| true,
            || false,
        )
        .unwrap();
        assert_eq!(found.matches.len(), 1);
        assert!(!found.truncated);
        assert!(
            found.comparisons < 100_000,
            "{}: no full scene scan per source member",
            found.comparisons
        );
    }

    #[test]
    fn dense_coincident_anchors_keep_distinct_identities_without_quadratic_scans() {
        let mut scene = vec![
            Shape::Circle {
                c: Vec2::new(0.0, 0.0),
                r: 0.4
            };
            10_000
        ];
        // A different radius and rejected properties must still fail the complete check.
        scene[0] = Shape::Circle {
            c: Vec2::new(0.0, 0.0),
            r: 0.5,
        };
        let found = find(&scene, &[1], Options::default(), |_, j| j != 2, || false).unwrap();
        assert_eq!(found.matches.len(), 1000);
        assert!(found.truncated, "the explicit result limit still applies");
        assert_eq!(found.matches[0].members, vec![1]);
        assert_eq!(found.matches[1].members, vec![3]);
        assert_eq!(found.matches[999].members, vec![1001]);
        assert!(
            found.comparisons < 3000,
            "{} comparisons",
            found.comparisons
        );
        assert_eq!(
            found
                .matches
                .iter()
                .map(|m| m.members[0])
                .collect::<HashSet<_>>()
                .len(),
            1000
        );
    }

    #[test]
    fn cancellation_limits_and_unsupported_sources_never_claim_a_complete_scan() {
        let scene = symbol();
        assert_eq!(
            find(&scene, &[0], Options::default(), |_, _| true, || true),
            Err(Error::Cancelled)
        );
        assert!(
            find(
                &scene,
                &[0],
                Options {
                    budget: 1,
                    ..Default::default()
                },
                |_, _| true,
                || false
            )
            .unwrap()
            .truncated
        );
        assert_eq!(
            find(&scene, &[0, 0], Options::default(), |_, _| true, || false),
            Err(Error::InvalidSource)
        );
        let unsupported = [Shape::Point {
            p: Vec2::new(0.0, 0.0),
            z: None,
            parts: None,
        }];
        assert_eq!(
            find(
                &unsupported,
                &[0],
                Options::default(),
                |_, _| true,
                || false
            ),
            Err(Error::UnsupportedSource(0))
        );
    }
}
