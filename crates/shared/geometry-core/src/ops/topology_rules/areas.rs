//! The rules over areas (docs/adr/0202 §2): overlaps, gaps, slivers, the
//! vertices a neighbour misses, and what lies outside another layer's areas.

use super::{Finding, MeasureKind, Spot, Taken, dist, middle_of, pairs, written};
use crate::entity::{Shape, area_parts};
use crate::geom::arrangement::WindingIndex;
use crate::geom::arrangement::{Area, Ring, Rule, edge_len};
use crate::geom::centroid::areas_centroid;
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geom::overlay::overlay;
use crate::geom::region::{area_source, net_area};
use crate::geometry::Bounds;
use crate::jsmath::{js_cmp, stable_sort};
use crate::ops::areas::area_of_entity;
use crate::ops::geoprocess::clip::within;
use crate::ops::geoprocess::{
    Class, Place, Region, boundary_edges, intersect_all, subtract_all, union_all,
};
use crate::vec2::Vec2;

/// A region the rules measure: its area, perimeter and centroid.
#[derive(Clone)]
pub(super) struct Piece {
    pub area: Area,
    pub a: f64,
    pub p: f64,
    pub c: Vec2,
}

impl Piece {
    fn of(area: Area) -> Option<Piece> {
        let a = net_area(&area);
        let p: f64 = boundary_edges(std::slice::from_ref(&area))
            .iter()
            .map(edge_len)
            .sum();
        let c = areas_centroid(std::slice::from_ref(&area))?;
        Some(Piece { area, a, p, c })
    }

    /// Its mean width 2A/P.
    pub fn width(&self) -> f64 {
        if self.p > 0.0 {
            2.0 * self.a / self.p
        } else {
            0.0
        }
    }
}

/// The pieces wider than the tolerance (docs/adr/0202 §2), in the overlay's order.
fn counted(areas: Vec<Area>, t: f64) -> Vec<Piece> {
    areas
        .into_iter()
        .filter_map(Piece::of)
        .filter(|p| p.width() > t)
        .collect()
}

/// The largest piece (the first of equal ones).
fn largest(pieces: &[Piece]) -> &Piece {
    let mut best = &pieces[0];
    for p in &pieces[1..] {
        if p.a > best.a {
            best = p;
        }
    }
    best
}

fn areas_of(taken: &[Taken], i: usize) -> &[Area] {
    match &taken[i].class {
        Class::Areas(a) => a,
        _ => &[],
    }
}

/// Çakışmamalı and … ile çakışmamalı: every pair of the layer's areas (or
/// of an area and the other layer's) and the counted pieces of what both cover.
pub(super) fn overlaps(
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    with: Option<&[usize]>,
    t: f64,
) -> Vec<Finding> {
    let boxes: Vec<Bounds> = idx.iter().map(|&i| taken[i].bounds).collect();
    // Boxes that overlap by no more than `t` across hold no piece wider than
    // `t` (a region within a strip of width w has 2A/P ≤ w): neighbours that
    // only share a boundary are not overlaid.
    let found = match with {
        None => pairs(&boxes, None, -t)
            .into_iter()
            .map(|(a, b)| (idx[a], idx[b]))
            .collect::<Vec<_>>(),
        Some(with) => {
            let other: Vec<Bounds> = with.iter().map(|&j| taken[j].bounds).collect();
            pairs(&boxes, Some(&other), -t)
                .into_iter()
                .map(|(a, b)| (idx[a], with[b]))
                .filter(|(i, j)| i != j)
                .collect()
        }
    };
    let mut out = Vec::new();
    for (i, j) in found {
        let (ai, aj) = (areas_of(taken, i), areas_of(taken, j));
        let both = intersect_all(ai, aj);
        let whole: f64 = both.iter().map(net_area).sum();
        let pieces = counted(both, t);
        if pieces.is_empty() {
            continue;
        }
        let at = largest(&pieces).c;
        let total = pieces.iter().map(|p| p.a).sum();
        // A side the other covers whole has nothing left to subtract from.
        let mut fixes = Vec::new();
        if ai.iter().map(net_area).sum::<f64>() - whole > t * t {
            fixes.push("subtractFirst");
        }
        if aj.iter().map(net_area).sum::<f64>() - whole > t * t {
            fixes.push("subtractSecond");
        }
        let regions = pieces.into_iter().map(|p| p.area).collect();
        out.push(
            Finding::new(rule, "overlap", vec![i, j], at)
                .measured(MeasureKind::Area, total)
                .fixes(&fixes)
                .shown(regions, Vec::new()),
        );
    }
    out
}

/// The length of `boundary` (cut at the other's vertices within `t` of it)
/// whose pieces' middles lie within `t` of the other's edges: the shared
/// boundary of docs/adr/0202 §2.2.
pub(super) fn shared_length(boundary: &[Edge], other: &[Edge], vertices: &[Vec2], t: f64) -> f64 {
    let mut total = 0.0;
    for e in boundary {
        let mut cuts = vec![0.0, 1.0];
        for v in vertices {
            let q = closest_on_edge(e, *v);
            if q.d <= t && q.t > 0.0 && q.t < 1.0 {
                cuts.push(q.t);
            }
        }
        stable_sort(&mut cuts, &mut |a, b| a.total_cmp(b));
        let l = edge_len(e);
        for w in cuts.windows(2) {
            if w[1] - w[0] <= 1e-15 {
                continue;
            }
            let m = point_at(e, (w[0] + w[1]) / 2.0);
            if other.iter().any(|f| closest_on_edge(f, m).d <= t) {
                total += (w[1] - w[0]) * l;
            }
        }
    }
    total
}

fn ring_vertices(areas: &[Area]) -> Vec<Vec2> {
    let mut out = Vec::new();
    for a in areas {
        out.extend_from_slice(&a.outer.pts);
        for h in &a.holes {
            out.extend_from_slice(&h.pts);
        }
    }
    out
}

/// The areas sharing a region's boundary, the longest shared first (the
/// first in order of equal ones): what Komşuya kat merges into.
fn neighbours(
    taken: &[Taken],
    region: &Area,
    idx: &[usize],
    skip: Option<usize>,
    t: f64,
) -> Vec<usize> {
    let boundary = boundary_edges(std::slice::from_ref(region));
    let mut b = crate::geometry::empty_bounds();
    for e in &boundary {
        super::grow(&mut b, &crate::geom::arrangement::edge_box(e));
    }
    let mut found: Vec<(usize, f64)> = Vec::new();
    for &k in idx {
        if Some(k) == skip {
            continue;
        }
        let kb = &taken[k].bounds;
        if kb.min_x > b.max_x + t
            || kb.max_x < b.min_x - t
            || kb.min_y > b.max_y + t
            || kb.max_y < b.min_y - t
        {
            continue;
        }
        let areas = areas_of(taken, k);
        let shared = shared_length(&boundary, &boundary_edges(areas), &ring_vertices(areas), t);
        if shared > 0.0 {
            found.push((k, shared));
        }
    }
    stable_sort(&mut found, &mut |a, b| js_cmp(b.1 - a.1, 0.0));
    found.into_iter().map(|(k, _)| k).collect()
}

fn frame(b: &Bounds, margin: f64) -> Area {
    let (x0, y0, x1, y1) = (
        b.min_x - margin,
        b.min_y - margin,
        b.max_x + margin,
        b.max_y + margin,
    );
    Area {
        outer: Ring {
            pts: vec![
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ],
            bulges: None,
        },
        holes: Vec::new(),
    }
}

/// The gaps of a layer: what the areas' union closes in (docs/adr/0202 §2.2),
/// north to south, then west to east.
pub(super) fn gap_pieces(taken: &[Taken], idx: &[usize], t: f64) -> Vec<Piece> {
    let all: Vec<Area> = idx
        .iter()
        .flat_map(|&i| areas_of(taken, i).iter().cloned())
        .collect();
    if all.is_empty() {
        return Vec::new();
    }
    let mut b = crate::geometry::empty_bounds();
    for &i in idx {
        super::grow(&mut b, &taken[i].bounds);
    }
    let f = frame(&b, t + 1.0);
    let edge = f.outer.pts[0].x;
    let rest = overlay(
        &[area_source(std::slice::from_ref(&f)), area_source(&all)],
        Rule::FirstNotOthers,
    );
    let mut pieces: Vec<Piece> = counted(
        rest.into_iter()
            .filter(|a| !a.outer.pts.iter().any(|p| p.x <= edge + 1e-9))
            .collect(),
        t,
    );
    stable_sort(&mut pieces, &mut |p, q| {
        let by_y = js_cmp(q.c.y - p.c.y, 0.0);
        if by_y != std::cmp::Ordering::Equal {
            by_y
        } else {
            js_cmp(p.c.x - q.c.x, 0.0)
        }
    });
    pieces
}

/// Boşluk olmamalı.
pub(super) fn gaps(taken: &[Taken], rule: usize, idx: &[usize], t: f64) -> Vec<Finding> {
    gap_pieces(taken, idx, t)
        .into_iter()
        .map(|g| {
            let around = neighbours(taken, &g.area, idx, None, t);
            let fixes: &[&'static str] = if around.is_empty() {
                &[]
            } else {
                &["mergeNeighbour"]
            };
            let into = around.first().copied();
            let mut f = Finding::new(rule, "gap", around, g.c)
                .measured(MeasureKind::Area, g.a)
                .fixes(fixes)
                .shown(vec![g.area], Vec::new());
            f.subject = into;
            f
        })
        .collect()
}

/// İnce alan olmamalı: every part of every area, by its mean width.
pub(super) fn slivers(
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    least: f64,
    t: f64,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for &i in idx {
        for (k, area) in areas_of(taken, i).iter().enumerate() {
            let Some(p) = Piece::of(area.clone()) else {
                continue;
            };
            let w = p.width();
            if w >= least {
                continue;
            }
            let around = neighbours(taken, area, idx, Some(i), t);
            let mut fixes = Vec::new();
            if !around.is_empty() {
                fixes.push("mergeNeighbour");
            }
            fixes.push("deletePart");
            let mut f = Finding::new(rule, "sliver", vec![i], p.c)
                .measured(MeasureKind::Length, w)
                .fixes(&fixes)
                .shown(vec![p.area], Vec::new());
            f.spot = Some(Spot {
                part: k,
                ring: 0,
                index: 0,
            });
            f.subject = around.first().copied();
            out.push(f);
        }
    }
    out
}

/// Ortak sınırda köşe eksik olmamalı: a vertex of one area within `t` of
/// another's boundary and of none of its vertices; owner by owner, then
/// the other, then the vertices in order.
pub(super) fn missing_vertices(
    shapes: &[Shape],
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    t: f64,
) -> Vec<Finding> {
    let boxes: Vec<Bounds> = idx.iter().map(|&i| taken[i].bounds).collect();
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); idx.len()];
    for (a, b) in pairs(&boxes, None, t) {
        near[a].push(b);
        near[b].push(a);
    }
    let rings: Vec<Vec<super::Written>> = idx.iter().map(|&i| written(&shapes[i])).collect();
    let mut out = Vec::new();
    for (a, &i) in idx.iter().enumerate() {
        let mut others = near[a].clone();
        others.sort_unstable();
        for b in others {
            let j = idx[b];
            let theirs = &rings[b];
            for w in &rings[a] {
                for &v in &w.pts {
                    let mut best: Option<(f64, Spot)> = None;
                    for r in theirs {
                        for k in 0..r.edge_count() {
                            let d = closest_on_edge(&r.edge(k), v).d;
                            if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                                best = Some((
                                    d,
                                    Spot {
                                        part: r.part,
                                        ring: r.ring,
                                        index: k,
                                    },
                                ));
                            }
                        }
                    }
                    let Some((d, spot)) = best else {
                        continue;
                    };
                    if d > t
                        || theirs
                            .iter()
                            .any(|r| r.pts.iter().any(|q| dist(*q, v) <= t))
                    {
                        continue;
                    }
                    let mut f = Finding::new(rule, "missingVertex", vec![j, i], v)
                        .measured(MeasureKind::Distance, d)
                        .fixes(&["addVertex"]);
                    f.spot = Some(spot);
                    f.target = Some(v);
                    out.push(f);
                }
            }
        }
    }
    out
}

/// … içinde kalmalı: what of each object lies outside the other layer's areas.
pub(super) fn covered_by(
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    by: &[usize],
    t: f64,
) -> Vec<Finding> {
    let cover: Vec<Area> = by
        .iter()
        .flat_map(|&j| areas_of(taken, j).iter().cloned())
        .collect();
    let union = union_all(&cover);
    let region = (!union.is_empty()).then(|| Region::of(&union));
    let mut out = Vec::new();
    for &i in idx {
        match &taken[i].class {
            Class::Areas(areas) => {
                let pieces = counted(subtract_all(areas, &union), t);
                if pieces.is_empty() {
                    continue;
                }
                let at = largest(&pieces).c;
                let total = pieces.iter().map(|p| p.a).sum();
                let inside: f64 = intersect_all(areas, &union).iter().map(net_area).sum();
                let fixes: &[&'static str] = if inside > t * t {
                    &["clipOutside"]
                } else {
                    &[]
                };
                out.push(
                    Finding::new(rule, "outside", vec![i], at)
                        .measured(MeasureKind::Area, total)
                        .fixes(fixes)
                        .shown(pieces.into_iter().map(|p| p.area).collect(), Vec::new()),
                );
            }
            Class::Paths(paths) => {
                let Class::Paths(runs) = within(&taken[i].class, &union, false) else {
                    continue;
                };
                let runs: Vec<Vec<Edge>> = runs
                    .into_iter()
                    .filter(|r| r.iter().map(edge_len).sum::<f64>() > t)
                    .collect();
                if runs.is_empty() {
                    continue;
                }
                let lengths: Vec<f64> = runs.iter().map(|r| r.iter().map(edge_len).sum()).collect();
                let mut long = 0;
                for (k, l) in lengths.iter().enumerate() {
                    if *l > lengths[long] {
                        long = k;
                    }
                }
                let total: f64 = lengths.iter().sum();
                let whole: f64 = paths.iter().flatten().map(edge_len).sum();
                let fixes: &[&'static str] = if whole - total > t {
                    &["clipOutside"]
                } else {
                    &[]
                };
                let at = middle_of(&runs[long]);
                out.push(
                    Finding::new(rule, "outside", vec![i], at)
                        .measured(MeasureKind::Length, total)
                        .fixes(fixes)
                        .shown(Vec::new(), runs.into_iter().flatten().collect()),
                );
            }
            Class::Points(points) => {
                let index = region.as_ref().map(|r| WindingIndex::new(r.edges()));
                for &p in points {
                    let d = match (&region, &index) {
                        (Some(r), Some(ix)) => {
                            if r.place(ix, p) != Place::Outside {
                                continue;
                            }
                            r.edges()
                                .iter()
                                .map(|e| closest_on_edge(e, p).d)
                                .fold(f64::INFINITY, crate::jsmath::js_min)
                        }
                        _ => f64::INFINITY,
                    };
                    if d <= t {
                        continue;
                    }
                    let mut f = Finding::new(rule, "outside", vec![i], p);
                    if d.is_finite() {
                        f = f.measured(MeasureKind::Distance, d);
                    } else {
                        f.measure_kind = Some(MeasureKind::Distance);
                    }
                    out.push(f);
                }
            }
            Class::None => {}
        }
    }
    out
}

/// A part of an area by its place among the parts (the written order).
pub(super) fn part_area(s: &Shape, k: usize) -> Option<Area> {
    area_parts(s).get(k).and_then(area_of_entity)
}
