//! Plan yolu çizimi (docs/adr/0198): a road's areas from its axis (the road,
//! the carriageway inside its kerbs, the median closed with half circles),
//! the inner corners of an area rounded (Kavşak temizle's ada and kaldırım
//! köşeleri, toplu yol yuvarlatma) after the areas of one kind are joined,
//! and two lines closed into a median (Refüj kapat).
//!
//! The tools draw and pick. The independent reference is
//! `scripts/fixtures/plan_road_cases.py` (`fixtures/plan-road/v1/cases.json`).

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::bulge::{BulgePath, bulge_at};
use crate::geom::offset::offset_path;
use crate::geom::parallel::{clean_axis, corridor_area};
use crate::geom::region::union_areas;
use crate::geometry::{dist, signed_area};
use crate::op;
use crate::ops::fillet::CornerOp;
use crate::ops::reshape::{corners_of_path_where, turns_inward};
use crate::vec2::Vec2;

/// A road's areas (§2): the road, the carriageway when it has kerbs, the
/// median when it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadParts {
    pub road: Area,
    pub carriageway: Option<Area>,
    pub median: Option<Area>,
}

crate::json_struct!(out RoadParts { road, carriageway, median });

/// The areas of a road `width` wide along `axis` (an open path), with kerbs
/// `kerb` wide on both sides and a median `median` wide in the middle (0:
/// none). None for an axis of fewer than two points, a width not above
/// zero, kerbs that leave no carriageway or a median wider than it.
pub fn road_parts(axis: &[Vec2], width: f64, kerb: f64, median: f64) -> Option<RoadParts> {
    let fits = width > 0.0 && kerb >= 0.0 && median >= 0.0 && 2.0 * kerb < width;
    if !fits || median >= width - 2.0 * kerb {
        return None;
    }
    let pts = clean_axis(axis, false);
    if pts.len() < 2 {
        return None;
    }
    let half = width / 2.0;
    let road = corridor_area(&pts, half, half, false)?;
    let carriageway = if kerb > 0.0 {
        Some(corridor_area(&pts, half - kerb, half - kerb, false)?)
    } else {
        None
    };
    let median = if median > 0.0 {
        let left = offset_path(&pts, median / 2.0, false);
        let right = offset_path(&pts, -median / 2.0, false);
        Some(Area {
            outer: median_ring(&left, None, &right, None, true)?,
            holes: Vec::new(),
        })
    } else {
        None
    };
    Some(RoadParts {
        road,
        carriageway,
        median,
    })
}

/// Two lines closed into a median (§4): the second as it is when its start
/// is nearer the first's end than its end is, else backwards; the ring the
/// first's points then the second's, counter-clockwise by its vertices; the
/// two edges joining their ends half circles (`round`) or straight. None for
/// a line of fewer than two points.
pub fn median_ring(
    first: &[Vec2],
    first_bulges: Option<&[f64]>,
    second: &[Vec2],
    second_bulges: Option<&[f64]>,
    round: bool,
) -> Option<Ring> {
    let (n1, n2) = (first.len(), second.len());
    if n1 < 2 || n2 < 2 {
        return None;
    }
    let mut bulges: Vec<f64> = (0..n1 - 1).map(|i| bulge_at(first_bulges, i)).collect();
    let mut tail = second.to_vec();
    let mut tail_bulges: Vec<f64> = (0..n2 - 1).map(|i| bulge_at(second_bulges, i)).collect();
    let end = first[n1 - 1];
    if dist(tail[0], end) >= dist(tail[n2 - 1], end) {
        tail.reverse();
        tail_bulges.reverse();
        tail_bulges.iter_mut().for_each(|b| *b = -*b + 0.0);
    }
    let mut pts = first.to_vec();
    pts.extend(tail);
    bulges.push(0.0);
    bulges.extend(tail_bulges);
    bulges.push(0.0);
    let m = pts.len();
    let mut caps = vec![false; m];
    caps[n1 - 1] = true;
    caps[m - 1] = true;
    if signed_area(&pts) < 0.0 {
        pts.reverse();
        // Edge k of the reversed ring is edge m − 2 − k of the ring, run backwards.
        bulges = (0..m).map(|k| -bulges[(2 * m - 2 - k) % m] + 0.0).collect();
        caps = (0..m).map(|k| caps[(2 * m - 2 - k) % m]).collect();
    }
    if round {
        for (b, cap) in bulges.iter_mut().zip(&caps) {
            if *cap {
                *b = 1.0;
            }
        }
    }
    let arcs = bulges.iter().any(|b| *b != 0.0);
    Some(Ring {
        pts,
        bulges: arcs.then_some(bulges),
    })
}

/// An area with its inner corners rounded, and the counts: corners rounded,
/// inner corners left (too tight for the radius, sharing an edge with a
/// corner it does not leave room for, or next to an arc edge).
#[derive(Clone, Debug, PartialEq)]
pub struct Rounded {
    pub area: Area,
    pub done: usize,
    pub skipped: usize,
}

impl ToJson for Rounded {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "area", &self.area);
        field(out, &mut first, "done", &(self.done as f64));
        field(out, &mut first, "skipped", &(self.skipped as f64));
        out.push('}');
    }
}

/// One ring's inner corners rounded: the area's inside on the ring's left
/// (`area_left`) makes a right turn inner, else a left turn (§3).
fn round_ring(ring: &Ring, area_left: bool, radius: f64) -> (Ring, usize, usize) {
    let bulges = ring.bulges.as_deref();
    let (path, done, skipped) =
        corners_of_path_where(&ring.pts, bulges, true, &CornerOp::Radius(radius), |i| {
            turns_inward(&ring.pts, bulges, i, area_left)
        });
    let BulgePath { pts, bulges } = path;
    (Ring { pts, bulges }, done, skipped)
}

/// The area's inner corners (its outer ring's and its holes') rounded with
/// `radius` (§3). A radius not above zero leaves the area as it is.
pub fn round_inner_corners(area: &Area, radius: f64) -> Rounded {
    if !(radius > 0.0) {
        return Rounded {
            area: area.clone(),
            done: 0,
            skipped: 0,
        };
    }
    let outer_left = signed_area(&area.outer.pts) > 0.0;
    let (outer, mut done, mut skipped) = round_ring(&area.outer, outer_left, radius);
    let holes = area
        .holes
        .iter()
        .map(|h| {
            let (ring, d, s) = round_ring(h, signed_area(&h.pts) < 0.0, radius);
            done += d;
            skipped += s;
            ring
        })
        .collect();
    Rounded {
        area: Area { outer, holes },
        done,
        skipped,
    }
}

/// Kavşak temizle's work on one kind of road (§3): the areas joined (one is
/// left as it is), then their inner corners rounded with `radius`.
#[derive(Clone, Debug, PartialEq)]
pub struct Joined {
    pub areas: Vec<Area>,
    pub done: usize,
    pub skipped: usize,
}

impl ToJson for Joined {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "areas", &self.areas);
        field(out, &mut first, "done", &(self.done as f64));
        field(out, &mut first, "skipped", &(self.skipped as f64));
        out.push('}');
    }
}

pub fn road_junctions(areas: &[Area], radius: f64) -> Joined {
    let joined = if areas.len() > 1 {
        union_areas(areas)
    } else {
        areas.to_vec()
    };
    let (mut done, mut skipped) = (0, 0);
    let areas = joined
        .iter()
        .map(|a| {
            let r = round_inner_corners(a, radius);
            done += r.done;
            skipped += r.skipped;
            r.area
        })
        .collect();
    Joined {
        areas,
        done,
        skipped,
    }
}

pub(crate) static OPS: &[Op] = &[
    // A road's areas from its axis; null when the widths do not fit.
    op!("roadParts", |axis: Vec<Vec2>,
                      width: f64,
                      kerb: f64,
                      median: f64| road_parts(
        &axis, width, kerb, median
    )),
    op!("roundInnerCorners", |area: Area, radius: f64| {
        round_inner_corners(&area, radius)
    }),
    op!("roadJunctions", |areas: Vec<Area>, radius: f64| {
        road_junctions(&areas, radius)
    }),
    // Two lines closed into a median; null for a line of fewer than two points.
    op!("medianRing", |first: Ring, second: Ring, round: bool| {
        median_ring(
            &first.pts,
            first.bulges.as_deref(),
            &second.pts,
            second.bulges.as_deref(),
            round,
        )
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn widths_that_do_not_fit_make_no_road() {
        let axis = [v(0.0, 0.0), v(50.0, 0.0)];
        assert!(road_parts(&axis, 0.0, 0.0, 0.0).is_none());
        assert!(road_parts(&axis, 10.0, 5.0, 0.0).is_none());
        assert!(road_parts(&axis, 10.0, 3.0, 4.0).is_none());
        let parts = road_parts(&axis, 10.0, 3.0, 2.0).expect("a road");
        assert!(parts.carriageway.is_some() && parts.median.is_some());
    }

    #[test]
    fn two_crossing_roads_join_with_four_rounded_corners() {
        let a = road_parts(&[v(-30.0, 0.0), v(30.0, 0.0)], 10.0, 0.0, 0.0).expect("east");
        let b = road_parts(&[v(0.0, -30.0), v(0.0, 30.0)], 10.0, 0.0, 0.0).expect("north");
        let joined = road_junctions(&[a.road, b.road], 4.0);
        assert_eq!(joined.areas.len(), 1);
        assert_eq!((joined.done, joined.skipped), (4, 0));
        let arcs = joined.areas[0]
            .outer
            .bulges
            .as_deref()
            .expect("arcs")
            .iter()
            .filter(|b| **b != 0.0)
            .count();
        assert_eq!(arcs, 4);
    }
}
