//! Vaziyet planı (docs/adr/8032; ADV-22, M32): the parcel's garden distances as Planlı Alanlar İmar Yönetmeliği 23 sets
//! them, the building's footprint (oturum) against them, the setback line (yapı yaklaşma sınırı) the document keeps, the
//! trees (23/6) and ±0.00.
//!
//! - **The parcel** is a closed line or a polygon the site plan names (`SitePlan::parcel`); its edges run from each
//!   corner to the next, a bulge taken as its chord.
//! - **Each edge's distance:** the plan's own (`SiteEdge::setback`), else its kind's: 5 m on a road or a public area,
//!   3 m at the side and the back with half a metre for each storey past four (23/1-a…ç).
//! - **The storeys** for that increase (23/1-d): the storeys standing on or above ±0.00, their heights summed and
//!   divided by 3, a remainder over 2.50 m one storey more.
//! - **The footprint** is the ground storey's rooms' contours together (the TAKS's, docs/adr/8030); each edge's drawn
//!   distance is the nearest the footprint comes to it, 0 when it crosses it.
//! - **The setback line:** the parcel less every point nearer an edge than its distance (a band along each edge with
//!   round ends), so a corner keeps both distances.
//! - **Trees** (23/6): one for each whole 30 m² of the parcel outside the footprint.
//!
//! A distance is the drawing's measure, not an approval; the plan's notes and the higher buildings' rules (23/1-g, ğ)
//! are read apart.

use kentos_contracts::{Entity, SitePlan};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region;

use crate::Document;
use crate::history::Op;
use crate::store::Stored;

/// One parcel edge as the site plan reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct SiteEdgeReport {
    pub kind: kentos_contracts::SiteEdgeKind,
    pub a: Vec2,
    pub b: Vec2,
    /// The garden distance it asks, m.
    pub required: f64,
    /// The nearest the footprint comes, m; none without a footprint.
    pub drawn: Option<f64>,
    /// Where: the footprint's point and the edge's (the site plan's distance dimension, docs/adr/8032 §2).
    pub near: Option<(Vec2, Vec2)>,
}

impl SiteEdgeReport {
    /// Whether the footprint comes nearer than its distance.
    pub fn short(&self) -> bool {
        self.drawn.is_some_and(|d| d + 1e-6 < self.required)
    }
}

/// The site plan read against the drawing (docs/adr/8032).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SiteReport {
    /// The parcel's corners, anticlockwise.
    pub parcel: Vec<Vec2>,
    pub area: f64,
    /// The storeys 23/1-d counts.
    pub floors: u32,
    pub edges: Vec<SiteEdgeReport>,
    pub footprint: f64,
    /// The setback line's parts.
    pub envelope: Vec<Area>,
    pub trees: u32,
    pub datum: Option<f64>,
}

impl SiteReport {
    /// Whether an edge's distance is not kept.
    pub fn short(&self) -> bool {
        self.edges.iter().any(SiteEdgeReport::short)
    }
}

/// Twice the signed area of a ring (anticlockwise positive).
fn signed2(pts: &[Vec2]) -> f64 {
    (0..pts.len())
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            p.x * q.y - q.x * p.y
        })
        .sum()
}

fn crosses(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let side = |p: Vec2, q: Vec2, r: Vec2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// The nearest point of segment `ab` to `p`.
fn foot(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Vec2 {
        x: a.x + t * dx,
        y: a.y + t * dy,
    }
}

/// The nearest two segments come and where: the point on `ab` and the one on `cd`; 0 when they cross.
fn segment_segment(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> (f64, Vec2, Vec2) {
    if crosses(a, b, c, d) {
        return (0.0, a, a);
    }
    let pairs = [
        (a, foot(a, c, d)),
        (b, foot(b, c, d)),
        (foot(c, a, b), c),
        (foot(d, a, b), d),
    ];
    pairs
        .into_iter()
        .map(|(p, q)| ((p.x - q.x).hypot(p.y - q.y), p, q))
        .min_by(|x, y| x.0.total_cmp(&y.0))
        // Four pairs: there is always a nearest.
        .unwrap_or((0.0, a, a))
}

/// The stretch of `ab` (its parameters 0…1) segment `cd` faces when parallel to it; none otherwise.
fn facing(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<(f64, f64)> {
    let (ux, uy) = (b.x - a.x, b.y - a.y);
    let len2 = ux * ux + uy * uy;
    let (vx, vy) = (d.x - c.x, d.y - c.y);
    if len2 <= 1e-18 || (ux * vy - uy * vx).abs() > 1e-9 * len2.sqrt() * vx.hypot(vy).max(1e-9) {
        return None;
    }
    let t = |p: Vec2| ((p.x - a.x) * ux + (p.y - a.y) * uy) / len2;
    let (t0, t1) = (t(c).min(t(d)).max(0.0), t(c).max(t(d)).min(1.0));
    (t1 > t0).then_some((t0, t1))
}

/// The points within `d` of segment `ab`: a band with round ends, anticlockwise, its arcs in `n` steps.
fn band(a: Vec2, b: Vec2, d: f64, n: usize) -> Ring {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let start = dy.atan2(dx) - std::f64::consts::FRAC_PI_2;
    let mut pts = Vec::with_capacity(2 * n + 2);
    // Around b from the right side to the left, then around a back to the right.
    for k in 0..=n {
        let t = start + std::f64::consts::PI * k as f64 / n as f64;
        pts.push(Vec2 {
            x: b.x + d * t.cos(),
            y: b.y + d * t.sin(),
        });
    }
    for k in 0..=n {
        let t = start + std::f64::consts::PI + std::f64::consts::PI * k as f64 / n as f64;
        pts.push(Vec2 {
            x: a.x + d * t.cos(),
            y: a.y + d * t.sin(),
        });
    }
    Ring { pts, bulges: None }
}

impl Document {
    /// The storeys 23/1-d counts for the side and back gardens: those standing on or above ±0.00, their heights summed
    /// and divided by 3, a remainder over 2.50 m one more; 1 without storeys.
    pub fn site_floors(&self) -> u32 {
        let Some(arch) = self.settings().arch.as_ref() else {
            return 1;
        };
        let mut levels: Vec<&kentos_contracts::ArchLevel> = arch.levels.iter().collect();
        levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let mut total = 0.0;
        for (i, l) in levels.iter().enumerate() {
            if l.elevation < -1e-6 {
                continue;
            }
            let next = levels
                .get(i + 1)
                .map(|n| n.elevation - l.elevation)
                .filter(|h| *h > 0.0);
            total += l.height.or(next).unwrap_or(3.0);
        }
        if total <= 0.0 {
            return 1;
        }
        let whole = (total / 3.0).floor();
        let rest = total - whole * 3.0;
        (whole as u32 + u32::from(rest > 2.5 + 1e-9)).max(1)
    }

    /// The parcel the site plan names: its corners, anticlockwise; none when it names no closed line or polygon.
    pub fn site_parcel(&self, site: &SitePlan) -> Option<Vec<Vec2>> {
        let uid = crate::identity::Uuid::parse_str(site.parcel.as_deref()?).ok()?;
        let slot = self.slot_of(uid)?;
        let pts = match self.get(slot)? {
            Entity::Polyline(p) if p.closed => &p.pts,
            Entity::Polygon(p) => &p.pts,
            _ => return None,
        };
        let mut ring: Vec<Vec2> = pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect();
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        (ring.len() >= 3).then_some(ring)
    }

    /// The site plan of the settings read against the drawing (docs/adr/8032); none without one or without its parcel.
    pub fn site_report(&self) -> Option<SiteReport> {
        let site = self.settings().arch.as_ref()?.site.clone()?;
        self.site_report_for(&site)
    }

    /// `site` read against the drawing: the parcel, each edge's distance and the footprint's, the setback line, the
    /// trees; none without its parcel.
    pub fn site_report_for(&self, site: &SitePlan) -> Option<SiteReport> {
        let corners = self.site_parcel(site)?;
        // The edges as the boundary runs; anticlockwise for the areas, the kinds kept on their edges.
        let floors = self.site_floors();
        let (footprint, shape) = self.footprint();
        let n = corners.len();
        let mut edges = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            let e = site.edge(i);
            let required = e.setback.unwrap_or_else(|| e.kind.setback(floors));
            // The footprint's nearest segment pair: the distance, the edge's point and the footprint's; where facing
            // pieces of the footprint stand at that distance (a facade in several pieces), the middle of all of them,
            // where a site plan dimensions it.
            let segments: Vec<(Vec2, Vec2)> = shape
                .iter()
                .flat_map(|area| std::iter::once(&area.outer).chain(&area.holes))
                .flat_map(|r| {
                    (0..r.pts.len()).map(move |k| (r.pts[k], r.pts[(k + 1) % r.pts.len()]))
                })
                .collect();
            let mut nearest = segments
                .iter()
                .map(|&(c, d)| segment_segment(a, b, c, d))
                .min_by(|x, y| x.0.total_cmp(&y.0));
            if let Some((dmin, ..)) = nearest {
                let stretch = segments
                    .iter()
                    .filter(|&&(c, d)| (segment_segment(a, b, c, d).0 - dmin).abs() <= 1e-6)
                    .filter_map(|&(c, d)| facing(a, b, c, d).map(|t| (t, c, d)))
                    .fold(
                        None::<(f64, f64, Vec2, Vec2)>,
                        |acc, ((t0, t1), c, d)| match acc {
                            None => Some((t0, t1, c, d)),
                            Some((s0, s1, c0, d0)) => Some((s0.min(t0), s1.max(t1), c0, d0)),
                        },
                    );
                if let Some((t0, t1, c, d)) = stretch {
                    let m = (t0 + t1) / 2.0;
                    let on_edge = Vec2 {
                        x: a.x + m * (b.x - a.x),
                        y: a.y + m * (b.y - a.y),
                    };
                    // The footprint's line through the facing pieces, at the same distance.
                    let (vx, vy) = (d.x - c.x, d.y - c.y);
                    let len2 = vx * vx + vy * vy;
                    let t = ((on_edge.x - c.x) * vx + (on_edge.y - c.y) * vy) / len2;
                    let on_footprint = Vec2 {
                        x: c.x + t * vx,
                        y: c.y + t * vy,
                    };
                    nearest = Some((dmin, on_edge, on_footprint));
                }
            }
            edges.push(SiteEdgeReport {
                kind: e.kind,
                a,
                b,
                required,
                drawn: nearest.map(|n| n.0).filter(|d| d.is_finite()),
                near: nearest.map(|(_, on_edge, on_footprint)| (on_footprint, on_edge)),
            });
        }
        let mut ring = corners.clone();
        if signed2(&ring) < 0.0 {
            ring.reverse();
        }
        let parcel = Area {
            outer: Ring {
                pts: ring.clone(),
                bulges: None,
            },
            holes: Vec::new(),
        };
        let area = signed2(&ring) / 2.0;
        let bands: Vec<Area> = edges
            .iter()
            .filter(|e| e.required > 0.0)
            .map(|e| Area {
                outer: band(e.a, e.b, e.required, 16),
                holes: Vec::new(),
            })
            .collect();
        let envelope = if bands.is_empty() {
            vec![parcel]
        } else {
            region::subtract_areas(&[parcel], &region::union_areas(&bands))
        };
        let garden = (area - footprint).max(0.0);
        Some(SiteReport {
            parcel: ring,
            area,
            floors,
            edges,
            footprint,
            envelope,
            trees: (garden / 30.0 + 1e-9).floor() as u32,
            datum: site.datum,
        })
    }

    /// The setback line's corners now (its largest part's outer ring), as the closed line the site plan keeps writes
    /// them; none when it has none.
    pub fn site_envelope_ring(&self, site: &SitePlan) -> Option<Vec<kentos_contracts::Vec2>> {
        let r = self.site_report_for(site)?;
        let best = r
            .envelope
            .iter()
            .max_by(|a, b| region::net_area(a).total_cmp(&region::net_area(b)))?;
        Some(
            best.outer
                .pts
                .iter()
                .map(|p| kentos_contracts::Vec2 { x: p.x, y: p.y })
                .collect(),
        )
    }

    /// The setback line the site plan keeps (docs/adr/8032) written again where the parcel, the storeys or the
    /// footprint moved it: the op that does, none when it stands; the line taken to its closed form.
    pub(crate) fn site_envelope_update(&self) -> Option<Op> {
        let site = self.settings().arch.as_ref()?.site.clone()?;
        let uid = crate::identity::Uuid::parse_str(site.envelope.as_deref()?).ok()?;
        let slot = self.slot_of(uid)?;
        let current = self.store.get(slot)?.clone();
        let Entity::Polyline(line) = &*current.entity else {
            return None;
        };
        let pts = self.site_envelope_ring(&site)?;
        if pts == line.pts && line.closed && line.bulges.is_none() {
            return None;
        }
        let mut next = line.clone();
        next.pts = pts;
        next.closed = true;
        next.bulges = None;
        next.zs = None;
        next.ms = None;
        Some(Op::Update {
            after: Stored {
                uid: current.uid,
                entity: std::sync::Arc::new(Entity::Polyline(next)),
            },
            before: current,
        })
    }

    /// The setback line after a step (docs/adr/8032): written again in it when the step touched the parcel, a room or
    /// a wall (a room follows its walls in the same step) or the storeys.
    pub(crate) fn follow_site(&mut self, ops: &[Op]) -> Vec<Op> {
        let Some(site) = self.settings().arch.as_ref().and_then(|a| a.site.as_ref()) else {
            return Vec::new();
        };
        if site.envelope.is_none() {
            return Vec::new();
        }
        let parcel = site
            .parcel
            .as_deref()
            .and_then(|p| crate::identity::Uuid::parse_str(p).ok());
        let moved = ops.iter().any(|op| match op {
            Op::Add(s) | Op::Remove(s) => {
                matches!(s.entity.kind(), "space" | "wall") || Some(s.uid) == parcel
            }
            Op::Update { before, after } => {
                matches!(after.entity.kind(), "space" | "wall")
                    || Some(after.uid) == parcel
                    || Some(before.uid) == parcel
            }
            Op::TemplateSettings { .. } => true,
            _ => false,
        });
        if !moved {
            return Vec::new();
        }
        match self.site_envelope_update() {
            Some(op) => {
                self.apply_op(&op);
                vec![op]
            }
            None => Vec::new(),
        }
    }

    /// The setback line after the settings changed or a step was undone (docs/adr/8032), without another undo step.
    pub(crate) fn refresh_site(&mut self) -> usize {
        match self.site_envelope_update() {
            Some(op) => {
                self.apply_op(&op);
                1
            }
            None => 0,
        }
    }
}
