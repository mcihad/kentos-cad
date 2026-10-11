//! The Genelleştirme tools' runs (docs/adr/0221): the desktop's tools call
//! the functions, the web (`model/ops/generalize.ts`) the operations with
//! whole lists, one call a run.

use super::aggregate::{Aggregated, Aggregation, aggregate};
use super::building::{Building, BuildingRing, simplify_rings};
use super::coverage::{self, Coverage};
use super::eliminate::{Elimination, eliminate};
use super::orthogonal::{Squaring, square_track};
use super::parts::{PartRule, drop_parts};
use super::simplify::simplify;
use super::smooth::{Smoothing, smooth_track};
use super::{Track, parts_of, polygon_of, tracks_of, vertex_count, with_tracks};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::Area;
use crate::geom::region::net_area;
use crate::jsmath::{PI, js_max};
use crate::op;
use crate::ops::areas::{areas_of_entity, polygon_of_area};
use crate::ops::convert::{Elevations, Zs};
use crate::ops::geoprocess::validity::problems;

fn is_path_kind(s: &Shape) -> bool {
    matches!(s, Shape::Polygon { .. } | Shape::Polyline { .. })
}

fn net(s: &Shape) -> f64 {
    areas_of_entity(s).iter().map(net_area).sum()
}

/// The rings of every area object, for a coverage: (object, its track) in order.
fn coverage_of(tracks: &[Vec<Track>], shapes: &[&Shape]) -> (Coverage, Vec<(usize, usize)>) {
    let mut at = Vec::new();
    let mut rings: Vec<&Track> = Vec::new();
    for (k, (list, s)) in tracks.iter().zip(shapes).enumerate() {
        if !matches!(s, Shape::Polygon { .. }) {
            continue;
        }
        for (i, t) in list.iter().enumerate() {
            if t.closed {
                at.push((k, i));
                rings.push(t);
            }
        }
    }
    (coverage::build(&rings), at)
}

/// Yumuşat (§2) on objects: each one's shape and elevations, none for a kind it does not take.
pub fn smooth_run(
    shapes: &[&Shape],
    zs: &[Option<&Zs>],
    s: &Smoothing,
    shared: bool,
) -> Vec<Option<Elevations>> {
    let mut tracks: Vec<Vec<Track>> = shapes
        .iter()
        .zip(zs)
        .map(|(s, z)| {
            if is_path_kind(s) {
                tracks_of(s, *z)
            } else {
                Vec::new()
            }
        })
        .collect();
    let mut done: Vec<Vec<bool>> = tracks.iter().map(|l| vec![false; l.len()]).collect();
    if shared {
        let (c, at) = coverage_of(&tracks, shapes);
        let edges: Vec<Track> = c.edges.iter().map(|e| smooth_track(e, s)).collect();
        for (k, &(o, i)) in at.iter().enumerate() {
            tracks[o][i] = coverage::ring(&c, &edges, k);
            done[o][i] = true;
        }
    }
    shapes
        .iter()
        .zip(tracks.iter_mut().zip(&done))
        .map(|(shape, (list, done))| {
            if !is_path_kind(shape) {
                return None;
            }
            for (t, &d) in list.iter_mut().zip(done) {
                if !d {
                    *t = smooth_track(t, s);
                }
            }
            Some(with_tracks(shape, list))
        })
        .collect()
}

/// Dik açılandır's answer for an object: its shape, whether it moved and its largest step (m).
#[derive(Clone, Debug, PartialEq)]
pub struct Squared {
    pub out: Elevations,
    pub moved: bool,
    pub shift: f64,
}

/// Dik açılandır (§3) on one object; none for a kind it does not take.
pub fn square_run(shape: &Shape, zs: Option<&Zs>, rule: &Squaring) -> Option<Squared> {
    if !is_path_kind(shape) {
        return None;
    }
    let mut moved = false;
    let mut shift: f64 = 0.0;
    let tracks: Vec<Track> = tracks_of(shape, zs)
        .iter()
        .map(|t| {
            let (out, m) = square_track(t, rule);
            moved |= m;
            for (a, b) in t.pts.iter().zip(&out.pts) {
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                shift = js_max(shift, (dx * dx + dy * dy).sqrt());
            }
            out
        })
        .collect();
    Some(Squared {
        out: with_tracks(shape, &tracks),
        moved,
        shift,
    })
}

/// What Bina sadeleştir did to an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingCase {
    /// Edges went.
    Changed,
    /// No edge could go.
    Same,
    /// The result was more than twice as large: the building stays.
    Grew,
    /// The result was not valid: the building stays.
    Invalid,
}

impl BuildingCase {
    pub fn key(self) -> &'static str {
        match self {
            BuildingCase::Changed => "changed",
            BuildingCase::Same => "same",
            BuildingCase::Grew => "grew",
            BuildingCase::Invalid => "invalid",
        }
    }
}

/// Bina sadeleştir's answer for an object: its shape (the building as it was unless changed), the edges that went, what happened.
#[derive(Clone, Debug, PartialEq)]
pub struct Built {
    pub shape: Shape,
    pub edits: usize,
    pub case: BuildingCase,
}

/// Bina sadeleştir (§4) on one area object; none for another kind.
pub fn building_run(shape: &Shape, rule: &Building) -> Option<Built> {
    if !matches!(shape, Shape::Polygon { .. }) {
        return None;
    }
    let ring = |t: &Track| BuildingRing {
        pts: t.pts.clone(),
        bulges: t.bulges.clone(),
    };
    let back = |r: &BuildingRing| Track {
        zs: vec![None; r.pts.len()],
        pts: r.pts.clone(),
        bulges: r.bulges.clone(),
        closed: true,
    };
    let mut edits = 0;
    let mut parts = Vec::new();
    for (outer, holes) in parts_of(shape, None) {
        let mut rings: Vec<BuildingRing> =
            std::iter::once(&outer).chain(&holes).map(ring).collect();
        edits += simplify_rings(&mut rings, rule);
        let mut it = rings.iter().map(back);
        let o = it.next()?;
        parts.push((o, it.collect::<Vec<_>>()));
    }
    let same = |case| {
        Some(Built {
            shape: shape.clone(),
            edits: 0,
            case,
        })
    };
    if edits == 0 {
        return same(BuildingCase::Same);
    }
    let new = polygon_of(&parts)?.shape;
    if net(&new) > 2.0 * net(shape) {
        return same(BuildingCase::Grew);
    }
    if !problems(&new).is_empty() && problems(shape).is_empty() {
        return same(BuildingCase::Invalid);
    }
    Some(Built {
        shape: new,
        edits,
        case: BuildingCase::Changed,
    })
}

/// Küçük parçaları ayıkla's answer for an object: its shape and elevations, the parts and holes that went.
#[derive(Clone, Debug, PartialEq)]
pub struct Trimmed {
    pub out: Elevations,
    pub parts: usize,
    pub holes: usize,
}

/// Küçük parçaları ayıkla (§6) on one area object; none for another kind.
pub fn parts_run(shape: &Shape, zs: Option<&Zs>, rule: &PartRule) -> Option<Trimmed> {
    let Shape::Polygon { holes, parts, .. } = shape else {
        return None;
    };
    // One part without holes has nothing to drop: its part stays when it is the largest.
    let simple =
        holes.as_ref().is_none_or(Vec::is_empty) && parts.as_ref().is_none_or(Vec::is_empty);
    let kept = if simple {
        None
    } else {
        Some(drop_parts(&parts_of(shape, zs), rule))
    };
    let Some(kept) = kept.filter(|k| k.parts_gone > 0 || k.holes_gone > 0) else {
        return Some(Trimmed {
            out: Elevations {
                shape: shape.clone(),
                zs: zs.cloned(),
            },
            parts: 0,
            holes: 0,
        });
    };
    Some(Trimmed {
        out: polygon_of(&kept.parts)?,
        parts: kept.parts_gone,
        holes: kept.holes_gone,
    })
}

/// Ortak kenarları koruyarak sadeleştir (§8) on objects: each area object's shape and elevations, none for another kind.
pub fn coverage_run(
    shapes: &[&Shape],
    zs: &[Option<&Zs>],
    tolerance: f64,
    keep_boundary: bool,
) -> Vec<Option<Elevations>> {
    let mut tracks: Vec<Vec<Track>> = shapes
        .iter()
        .zip(zs)
        .map(|(s, z)| {
            if matches!(s, Shape::Polygon { .. }) {
                tracks_of(s, *z)
            } else {
                Vec::new()
            }
        })
        .collect();
    let (c, at) = coverage_of(&tracks, shapes);
    let edges = simplify(&c, tolerance, keep_boundary);
    for (k, &(o, i)) in at.iter().enumerate() {
        tracks[o][i] = coverage::ring(&c, &edges, k);
    }
    shapes
        .iter()
        .zip(&tracks)
        .map(|(s, list)| matches!(s, Shape::Polygon { .. }).then(|| with_tracks(s, list)))
        .collect()
}

/// Alanları kümele (§5) on objects grouped by `groups` (one group per distinct value, in first-seen order).
pub fn aggregate_run(
    shapes: &[&Shape],
    groups: &[String],
    rule: &Aggregation,
) -> Vec<(String, Aggregated)> {
    let mut names: Vec<&String> = Vec::new();
    for g in groups {
        if !names.contains(&g) {
            names.push(g);
        }
    }
    names
        .into_iter()
        .map(|name| {
            let index: Vec<usize> = (0..shapes.len()).filter(|&i| &groups[i] == name).collect();
            let members: Vec<&Shape> = index.iter().map(|&i| shapes[i]).collect();
            let mut got = aggregate(&members, rule);
            for r in &mut got.results {
                for m in &mut r.members {
                    *m = index[*m];
                }
            }
            (name.clone(), got)
        })
        .collect()
}

/// Küçükleri komşuya kat (§7) on area objects.
pub fn eliminate_run(shapes: &[&Shape], rule: &Elimination) -> super::eliminate::Eliminated {
    let areas: Vec<Vec<Area>> = shapes.iter().map(|s| areas_of_entity(s)).collect();
    eliminate(&areas, rule)
}

/// Areas as one (multi-part) area shape, the largest part first.
pub fn area_shape(areas: &[Area]) -> Option<Shape> {
    crate::ops::parts::one_area(areas).map(|e| e.shape)
}

/// One area as a polygon shape.
pub fn polygon_shape(a: &Area) -> Shape {
    polygon_of_area(a).shape
}

// ── The web's operations ───────────────────────────────────────────────

/// A result with its vertices and net area before and after (the reports'),
/// read by the same functions the desktop's tools read them with.
struct Out {
    got: Elevations,
    vertices: [f64; 2],
    area: [f64; 2],
}

impl Out {
    fn of(before: &Shape, got: Elevations) -> Out {
        Out {
            vertices: [vertex_count(before) as f64, vertex_count(&got.shape) as f64],
            area: [net(before), net(&got.shape)],
            got,
        }
    }
}

impl ToJson for Out {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(
            out,
            &mut first,
            "shape",
            &Entity::new(self.got.shape.clone()),
        );
        field(out, &mut first, "zs", &self.got.zs);
        field(out, &mut first, "vertices", &self.vertices.to_vec());
        field(out, &mut first, "area", &self.area.to_vec());
        out.push('}');
    }
}

fn zs_at(zs: &[Option<Zs>], i: usize) -> Option<&Zs> {
    zs.get(i).and_then(Option::as_ref)
}

fn refs(entities: &[Entity]) -> Vec<&Shape> {
    entities.iter().map(|e| &e.shape).collect()
}

fn degrees(d: f64) -> f64 {
    d * PI / 180.0
}

fn outs(entities: &[Entity], results: Vec<Option<Elevations>>) -> Vec<Option<Out>> {
    entities
        .iter()
        .zip(results)
        .map(|(e, r)| r.map(|r| Out::of(&e.shape, r)))
        .collect()
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "generalizeSmooth",
        |entities: Vec<Entity>,
         zs: Vec<Option<Zs>>,
         iterations: f64,
         offset: f64,
         max_angle: f64,
         shared: bool| {
            let z: Vec<Option<&Zs>> = (0..entities.len()).map(|i| zs_at(&zs, i)).collect();
            let s = Smoothing {
                iterations: if iterations >= 1.0 {
                    iterations as u32
                } else {
                    1
                },
                offset,
                max_turn: degrees(max_angle),
            };
            outs(&entities, smooth_run(&refs(&entities), &z, &s, shared))
        }
    ),
    op!(
        "generalizeSquare",
        |entities: Vec<Entity>, zs: Vec<Option<Zs>>, threshold: f64, iterations: f64| {
            let rule = Squaring {
                threshold,
                iterations: if iterations >= 1.0 {
                    iterations as u32
                } else {
                    1
                },
            };
            entities
                .iter()
                .enumerate()
                .map(|(i, e)| square_run(&e.shape, zs_at(&zs, i), &rule).map(SquaredOut))
                .collect::<Vec<_>>()
        }
    ),
    op!(
        "generalizeBuilding",
        |entities: Vec<Entity>, min_edge: f64, parallel: f64, orthogonal: f64| {
            let rule = Building {
                min_edge,
                parallel: degrees(parallel),
                orthogonal: degrees(orthogonal),
            };
            entities
                .iter()
                .map(|e| {
                    building_run(&e.shape, &rule).map(|b| BuiltOut {
                        measured: Out::of(
                            &e.shape,
                            Elevations {
                                shape: b.shape.clone(),
                                zs: None,
                            },
                        ),
                        built: b,
                    })
                })
                .collect::<Vec<_>>()
        }
    ),
    op!("generalizeParts", |entities: Vec<Entity>,
                            zs: Vec<Option<Zs>>,
                            min_area: f64,
                            percent: f64,
                            parts: bool,
                            holes: bool| {
        let rule = PartRule {
            min_area,
            percent,
            parts,
            holes,
        };
        entities
            .iter()
            .enumerate()
            .map(|(i, e)| parts_run(&e.shape, zs_at(&zs, i), &rule).map(TrimmedOut))
            .collect::<Vec<_>>()
    }),
    op!(
        "generalizeCoverage",
        |entities: Vec<Entity>, zs: Vec<Option<Zs>>, tolerance: f64, keep_boundary: bool| {
            let z: Vec<Option<&Zs>> = (0..entities.len()).map(|i| zs_at(&zs, i)).collect();
            outs(
                &entities,
                coverage_run(&refs(&entities), &z, tolerance, keep_boundary),
            )
        }
    ),
    op!(
        "generalizeAggregate",
        |entities: Vec<Entity>,
         groups: Vec<String>,
         distance: f64,
         min_area: f64,
         min_hole: f64,
         fill_notches: bool| {
            let rule = Aggregation {
                distance,
                min_area,
                min_hole,
                fill_notches,
            };
            AggregatedOut(aggregate_run(&refs(&entities), &groups, &rule))
        }
    ),
    op!(
        "generalizeEliminate",
        |entities: Vec<Entity>, min_area: f64, largest: bool| {
            EliminatedOut(eliminate_run(
                &refs(&entities),
                &Elimination { min_area, largest },
            ))
        }
    ),
];

struct SquaredOut(Squared);

impl ToJson for SquaredOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(
            out,
            &mut first,
            "shape",
            &Entity::new(self.0.out.shape.clone()),
        );
        field(out, &mut first, "zs", &self.0.out.zs);
        field(out, &mut first, "moved", &self.0.moved);
        field(out, &mut first, "shift", &self.0.shift);
        out.push('}');
    }
}

struct BuiltOut {
    built: Built,
    measured: Out,
}

impl ToJson for BuiltOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(
            out,
            &mut first,
            "shape",
            &Entity::new(self.built.shape.clone()),
        );
        field(out, &mut first, "edits", &(self.built.edits as f64));
        field(out, &mut first, "case", &self.built.case.key());
        field(
            out,
            &mut first,
            "vertices",
            &self.measured.vertices.to_vec(),
        );
        field(out, &mut first, "area", &self.measured.area.to_vec());
        out.push('}');
    }
}

struct TrimmedOut(Trimmed);

impl ToJson for TrimmedOut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        // An unchanged object is written as it was: its shape does not come back.
        if self.0.parts + self.0.holes > 0 {
            field(
                out,
                &mut first,
                "shape",
                &Entity::new(self.0.out.shape.clone()),
            );
            field(out, &mut first, "zs", &self.0.out.zs);
        }
        field(out, &mut first, "parts", &(self.0.parts as f64));
        field(out, &mut first, "holes", &(self.0.holes as f64));
        out.push('}');
    }
}

/// The groups' results: each result's shape, its objects (indices) and its group; how many were too small.
struct AggregatedOut(Vec<(String, Aggregated)>);

impl ToJson for AggregatedOut {
    fn write_json(&self, out: &mut String) {
        let mut shapes = Vec::new();
        let mut members: Vec<Vec<f64>> = Vec::new();
        let mut groups: Vec<String> = Vec::new();
        let mut dropped = 0.0;
        for (name, got) in &self.0 {
            dropped += got.dropped as f64;
            for r in &got.results {
                shapes.push(Entity::new(polygon_shape(&r.area)));
                members.push(r.members.iter().map(|&m| m as f64).collect());
                groups.push(name.clone());
            }
        }
        out.push('{');
        let mut first = true;
        field(out, &mut first, "shapes", &shapes);
        field(out, &mut first, "members", &members);
        field(out, &mut first, "groups", &groups);
        field(out, &mut first, "dropped", &dropped);
        out.push('}');
    }
}

/// The objects that took others in (indices) and their shapes now, the joinings with the small ones' areas, the small ones left.
struct EliminatedOut(super::eliminate::Eliminated);

impl ToJson for EliminatedOut {
    fn write_json(&self, out: &mut String) {
        let mut changed: Vec<usize> = self.0.joined.iter().map(|j| j.into).collect();
        changed.sort_unstable();
        changed.dedup();
        let shapes: Vec<Option<Entity>> = changed
            .iter()
            .map(|&i| {
                self.0.areas[i]
                    .as_ref()
                    .and_then(|a| area_shape(a))
                    .map(Entity::new)
            })
            .collect();
        let num = |v: &[usize]| v.iter().map(|&i| i as f64).collect::<Vec<f64>>();
        let small: Vec<usize> = self.0.joined.iter().map(|j| j.small).collect();
        let into: Vec<usize> = self.0.joined.iter().map(|j| j.into).collect();
        let shared: Vec<f64> = self.0.joined.iter().map(|j| j.shared).collect();
        let size: Vec<f64> = self
            .0
            .joined
            .iter()
            .map(|j| self.0.sizes[j.small])
            .collect();
        out.push('{');
        let mut first = true;
        field(out, &mut first, "changed", &num(&changed));
        field(out, &mut first, "shapes", &shapes);
        field(out, &mut first, "small", &num(&small));
        field(out, &mut first, "into", &num(&into));
        field(out, &mut first, "shared", &shared);
        field(out, &mut first, "smallArea", &size);
        field(out, &mut first, "left", &num(&self.0.left));
        out.push('}');
    }
}
