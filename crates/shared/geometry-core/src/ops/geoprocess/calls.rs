//! The tools' calls (docs/adr/0201): whole objects in, what each tool
//! writes out. The desktop's tools call the functions with shapes; the web's
//! (`model/ops/geoprocess.ts`) call the operations below with objects.

use super::buffer::{Side, buffer};
use super::clip::{Piece, difference, dissolve, intersection, sym_difference, union, within};
use super::reproject::reproject;
use super::validity::{Kind, problems, repair};
use super::{
    Class, areas_measure, class_of, paths_measure, shape_of, subtract_all, union_all,
    written_vertices,
};
use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::crs::{Choice, System, Unreached};
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::Area;
use crate::op;
use crate::ops::reshape::simplify;
use crate::ops::statistics::{Dec, compare, read_number};
use crate::vec2::Vec2;

// ── Tampon ─────────────────────────────────────────────────────────────

/// One piece Tampon writes: the object it is of (none: every object's,
/// Birleştir), its ring (1 the first), its distance (k · d, the decimal as
/// written times the ring; none when the joined objects' distances differ),
/// its shape.
#[derive(Clone, Debug, PartialEq)]
pub struct BufferPiece {
    pub source: Option<usize>,
    pub ring: usize,
    pub distance: Option<String>,
    pub shape: Shape,
}

/// Tampon's answer: its pieces; the objects whose distance is not read as a
/// number, those with a minus distance while rings go outward only (more
/// than one ring), and those whose buffer is nothing (a path or a point at a
/// minus distance, an area buffered inward past its middle).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Buffered {
    pub pieces: Vec<BufferPiece>,
    pub unread: Vec<usize>,
    pub inward: Vec<usize>,
    pub empty: Vec<usize>,
}

/// `d` × `k` as a decimal text.
fn times(d: Dec, k: usize) -> Option<String> {
    Some(
        Dec {
            m: d.m.checked_mul(i128::try_from(k).ok()?)?,
            scale: d.scale,
        }
        .text(),
    )
}

/// Tampon (docs/adr/0201 §2): each object at its distance (a decimal text,
/// read as Özet istatistik reads values), ring by ring: ring 1 the buffer
/// at d, ring k the band between (k − 1)·d and k·d. With `dissolve` every
/// object's buffers at k·d are joined and the bands taken of the joined.
pub fn buffer_run(
    shapes: &[Shape],
    distances: &[Option<String>],
    side: Side,
    rings: usize,
    dissolve: bool,
) -> Buffered {
    let rings = rings.max(1);
    let mut out = Buffered::default();
    // Each object's distance and its whole buffers at d, 2d …
    let mut per: Vec<(usize, Dec, Vec<Vec<Area>>)> = Vec::new();
    for (i, s) in shapes.iter().enumerate() {
        let Some(d) = distances
            .get(i)
            .and_then(|t| t.as_deref())
            .and_then(read_number)
        else {
            out.unread.push(i);
            continue;
        };
        let x = d.to_f64();
        if x < 0.0 && rings > 1 {
            out.inward.push(i);
            continue;
        }
        let c = class_of(s);
        let full: Vec<Vec<Area>> = (1..=rings)
            .map(|k| buffer(&c, x * k as f64, side))
            .collect();
        if full.iter().all(Vec::is_empty) {
            out.empty.push(i);
            continue;
        }
        per.push((i, d, full));
    }
    let area = |source: Option<usize>, ring: usize, distance: Option<String>, band: Vec<Area>| {
        shape_of(&Class::Areas(band)).map(|shape| BufferPiece {
            source,
            ring,
            distance,
            shape,
        })
    };
    if dissolve {
        let same = per
            .windows(2)
            .all(|w| compare(w[0].1, w[1].1) == std::cmp::Ordering::Equal);
        let mut before: Vec<Area> = Vec::new();
        for k in 0..rings {
            let all: Vec<Area> = per.iter().flat_map(|(_, _, b)| b[k].clone()).collect();
            let now = union_all(&all);
            let band = if k == 0 {
                now.clone()
            } else {
                subtract_all(&now, &before)
            };
            let distance = per
                .first()
                .filter(|_| same)
                .and_then(|(_, d, _)| times(*d, k + 1));
            out.pieces.extend(area(None, k + 1, distance, band));
            before = now;
        }
        return out;
    }
    for (i, d, full) in &per {
        for k in 0..rings {
            let band = if k == 0 {
                full[0].clone()
            } else {
                subtract_all(&full[k], &full[k - 1])
            };
            out.pieces
                .extend(area(Some(*i), k + 1, times(*d, k + 1), band));
        }
    }
    out
}

// ── Kes ────────────────────────────────────────────────────────────────

/// Kes (§3): each object's parts inside or on the cutting areas joined; none where nothing is left.
pub fn clip_run(shapes: &[Shape], cut: &[Shape]) -> Vec<Option<Shape>> {
    let areas: Vec<Area> = cut
        .iter()
        .flat_map(|s| match class_of(s) {
            Class::Areas(a) => a,
            _ => Vec::new(),
        })
        .collect();
    let joined = union_all(&areas);
    shapes
        .iter()
        .map(|s| shape_of(&within(&class_of(s), &joined, true)))
        .collect()
}

// ── Kesişim, Fark, Simetrik fark, Birleşim ─────────────────────────────

/// Which of the overlays (§5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Intersection,
    Difference,
    SymDifference,
    Union,
}

impl Mode {
    pub fn from_key(key: &str) -> Option<Mode> {
        match key {
            "intersection" => Some(Mode::Intersection),
            "difference" => Some(Mode::Difference),
            "symDifference" => Some(Mode::SymDifference),
            "union" => Some(Mode::Union),
            _ => None,
        }
    }
}

/// A piece an overlay writes: its first-side object, its second-side
/// object, its share of the first-side object (area, length or count; none
/// when it is not of one) and its shape.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayPiece {
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub share: Option<f64>,
    pub shape: Shape,
}

/// The overlay's pieces in their order: Kesişim every first-side object
/// with every second-side area it meets; Fark each first-side object less
/// the second side; Simetrik fark the first side less the second, then the
/// second less the first; Birleşim the meetings, then the two differences.
pub fn overlay_run(a: &[Shape], b: &[Shape], mode: Mode) -> Vec<OverlayPiece> {
    let (a, b): (Vec<Class>, Vec<Class>) = (
        a.iter().map(class_of).collect(),
        b.iter().map(class_of).collect(),
    );
    let pieces: Vec<Piece> = match mode {
        Mode::Intersection => intersection(&a, &b),
        Mode::Difference => difference(&a, &b),
        Mode::SymDifference => sym_difference(&a, &b),
        Mode::Union => union(&a, &b),
    };
    pieces
        .into_iter()
        .filter_map(|p| {
            Some(OverlayPiece {
                shape: shape_of(&p.class)?,
                a: p.a,
                b: p.b,
                share: p.share,
            })
        })
        .collect()
}

// ── Birleştir ──────────────────────────────────────────────────────────

/// Birleştir (§4): each group's objects as one, or each joined part on its
/// own; groups by their first object's place, numbered by the caller.
pub fn dissolve_run(shapes: &[Shape], groups: &[usize], multi: bool) -> Vec<(usize, Shape)> {
    let classes: Vec<Class> = shapes.iter().map(class_of).collect();
    dissolve(&classes, groups, multi)
        .into_iter()
        .filter_map(|(g, c)| Some((g, shape_of(&c)?)))
        .collect()
}

// ── Geçerlilik, Onar ───────────────────────────────────────────────────

/// A problem Geçerliliği denetle finds: the object (its place in the
/// list), the kind and the first place it shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub index: usize,
    pub kind: Kind,
    pub at: Vec2,
}

/// Every object's problems, object by object.
pub fn validity_run(shapes: &[Shape]) -> Vec<Found> {
    shapes
        .iter()
        .enumerate()
        .flat_map(|(index, s)| {
            problems(s).into_iter().map(move |p| Found {
                index,
                kind: p.kind,
                at: p.at,
            })
        })
        .collect()
}

/// An object's problems' kinds, each once, in the order found.
pub fn kinds_of(s: &Shape) -> Vec<Kind> {
    let mut out: Vec<Kind> = Vec::new();
    for p in problems(s) {
        if !out.contains(&p.kind) {
            out.push(p.kind);
        }
    }
    out
}

// ── Sadeleştir ─────────────────────────────────────────────────────────

/// Sadeleştir's answer for an object: its shape (as it was when no vertex
/// went), whether it changed, its vertices before and after, its area before
/// and after (none for a path), the largest deviation (m).
#[derive(Clone, Debug, PartialEq)]
pub struct Simplified {
    pub shape: Shape,
    pub changed: bool,
    pub vertices: [usize; 2],
    pub area: Option<[f64; 2]>,
    pub deviation: f64,
}

/// Sadeleştir on one object (§7): ADR 0140's rule, arc edges kept whole.
pub fn simplify_run(s: &Shape, tolerance: f64) -> Simplified {
    let before = class_of(s);
    let area_of = |c: &Class| match c {
        Class::Areas(a) => Some(areas_measure(a)),
        _ => None,
    };
    let changed = simplify(&Entity::new(s.clone()), tolerance).filter(|r| r.done > 0);
    match changed {
        Some(r) => {
            let after = class_of(&r.entity.shape);
            Simplified {
                vertices: [written_vertices(s), written_vertices(&r.entity.shape)],
                area: area_of(&before).zip(area_of(&after)).map(|(a, b)| [a, b]),
                deviation: r.deviation,
                shape: r.entity.shape,
                changed: true,
            }
        }
        None => Simplified {
            vertices: [written_vertices(s); 2],
            area: area_of(&before).map(|a| [a, a]),
            deviation: 0.0,
            shape: s.clone(),
            changed: false,
        },
    }
}

// ── Measures (the shared cases' players) ───────────────────────────────

/// An object as the geometry tools measure it: its parts, its holes, its
/// area (areas), its length (paths) and its points (points).
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub parts: usize,
    pub holes: usize,
    pub area: Option<f64>,
    pub length: Option<f64>,
    pub points: Option<Vec<Vec2>>,
}

crate::json_struct!(out Measured { parts, holes, area, length, points });

pub fn measured(s: &Shape) -> Measured {
    match class_of(s) {
        Class::Areas(a) => Measured {
            parts: a.len(),
            holes: a.iter().map(|x| x.holes.len()).sum(),
            area: Some(areas_measure(&a)),
            length: None,
            points: None,
        },
        Class::Paths(p) => Measured {
            parts: p.len(),
            holes: 0,
            area: None,
            length: Some(paths_measure(&p)),
            points: None,
        },
        Class::Points(p) => Measured {
            parts: p.len(),
            holes: 0,
            area: None,
            length: None,
            points: Some(p),
        },
        Class::None => Measured {
            parts: 0,
            holes: 0,
            area: None,
            length: None,
            points: None,
        },
    }
}

// ── The web's operations ───────────────────────────────────────────────

fn shapes(entities: &[Entity]) -> Vec<Shape> {
    entities.iter().map(|e| e.shape.clone()).collect()
}

struct BufferedOut(Buffered);

impl ToJson for BufferedOut {
    fn write_json(&self, out: &mut String) {
        struct Piece<'a>(&'a BufferPiece);
        impl ToJson for Piece<'_> {
            fn write_json(&self, out: &mut String) {
                out.push('{');
                let mut first = true;
                field(out, &mut first, "source", &self.0.source);
                field(out, &mut first, "ring", &self.0.ring);
                field(out, &mut first, "distance", &self.0.distance);
                field(out, &mut first, "shape", &Entity::new(self.0.shape.clone()));
                out.push('}');
            }
        }
        let b = &self.0;
        out.push('{');
        let mut first = true;
        let pieces: Vec<Piece<'_>> = b.pieces.iter().map(Piece).collect();
        field(out, &mut first, "pieces", &pieces);
        field(out, &mut first, "unread", &b.unread);
        field(out, &mut first, "inward", &b.inward);
        field(out, &mut first, "empty", &b.empty);
        out.push('}');
    }
}

/// An overlay's piece for the web.
struct PieceOut {
    a: Option<usize>,
    b: Option<usize>,
    share: Option<f64>,
    shape: Entity,
}

crate::json_struct!(out PieceOut { a, b, share, shape });

/// A dissolved group's object for the web.
struct GroupOut {
    group: usize,
    shape: Entity,
}

crate::json_struct!(out GroupOut { group, shape });

/// A problem for the web: the object, its kind, its text, its place.
struct ProblemOut {
    index: usize,
    problem: &'static str,
    text: &'static str,
    at: Vec2,
}

crate::json_struct!(out ProblemOut { index, problem, text, at });

/// Onar's answer for an object: its shape (none: nothing is left), its
/// parts, holes, vertices and area (none for a path) before and after, and
/// its problems' kinds and texts.
struct RepairOut {
    shape: Option<Entity>,
    parts: [usize; 2],
    holes: [usize; 2],
    vertices: [usize; 2],
    area: Option<[f64; 2]>,
    problems: Vec<&'static str>,
    texts: Vec<&'static str>,
}

crate::json_struct!(out RepairOut { shape, parts, holes, vertices, area, problems, texts });

struct SimplifyOut {
    shape: Entity,
    changed: bool,
    vertices: [usize; 2],
    area: Option<[f64; 2]>,
    deviation: f64,
}

crate::json_struct!(out SimplifyOut { shape, changed, vertices, area, deviation });

/// Koordinat sistemine dönüştür's answer for an object: its shape, or why
/// it could not be (the first vertex that could not be); the arcs that
/// became chords.
struct ReprojectOut {
    shape: Option<Entity>,
    error: Option<&'static str>,
    chorded: usize,
}

crate::json_struct!(out ReprojectOut { shape, error, chorded });

pub(crate) const OPS: &[Op] = &[
    op!("geoBuffer", |entities: Vec<Entity>,
                      distances: Vec<Option<String>>,
                      side: String,
                      rings: usize,
                      dissolve: bool| {
        let side = Side::from_key(&side).ok_or("bilinmeyen yan")?;
        Ok::<BufferedOut, String>(BufferedOut(buffer_run(
            &shapes(&entities),
            &distances,
            side,
            rings,
            dissolve,
        )))
    }),
    op!("geoClip", |entities: Vec<Entity>, cut: Vec<Entity>| {
        clip_run(&shapes(&entities), &shapes(&cut))
            .into_iter()
            .map(|s| s.map(Entity::new))
            .collect::<Vec<Option<Entity>>>()
    }),
    op!("geoOverlay", |a: Vec<Entity>,
                       b: Vec<Entity>,
                       mode: String| {
        let mode = Mode::from_key(&mode).ok_or("bilinmeyen işlem")?;
        Ok::<Vec<PieceOut>, String>(
            overlay_run(&shapes(&a), &shapes(&b), mode)
                .into_iter()
                .map(|p| PieceOut {
                    a: p.a,
                    b: p.b,
                    share: p.share,
                    shape: Entity::new(p.shape),
                })
                .collect(),
        )
    }),
    op!("geoDissolve", |entities: Vec<Entity>,
                        groups: Vec<usize>,
                        multi: bool| {
        dissolve_run(&shapes(&entities), &groups, multi)
            .into_iter()
            .map(|(group, s)| GroupOut {
                group,
                shape: Entity::new(s),
            })
            .collect::<Vec<GroupOut>>()
    }),
    op!("geoValidity", |entities: Vec<Entity>| {
        validity_run(&shapes(&entities))
            .into_iter()
            .map(|f| ProblemOut {
                index: f.index,
                problem: f.kind.key(),
                text: f.kind.text(),
                at: f.at,
            })
            .collect::<Vec<ProblemOut>>()
    }),
    op!("geoRepair", |entities: Vec<Entity>| {
        entities
            .iter()
            .map(|e| {
                let r = repair(&e.shape);
                let kinds = kinds_of(&e.shape);
                RepairOut {
                    shape: r.shape.map(Entity::new),
                    parts: [r.parts.0, r.parts.1],
                    holes: [r.holes.0, r.holes.1],
                    vertices: [r.vertices.0, r.vertices.1],
                    area: r.area.map(|(a, b)| [a, b]),
                    problems: kinds.iter().map(|k| k.key()).collect(),
                    texts: kinds.iter().map(|k| k.text()).collect(),
                }
            })
            .collect::<Vec<RepairOut>>()
    }),
    op!("geoSimplify", |entities: Vec<Entity>, tolerance: f64| {
        entities
            .iter()
            .map(|e| {
                let r = simplify_run(&e.shape, tolerance);
                SimplifyOut {
                    shape: Entity::new(r.shape),
                    changed: r.changed,
                    vertices: r.vertices,
                    area: r.area,
                    deviation: r.deviation,
                }
            })
            .collect::<Vec<SimplifyOut>>()
    }),
    op!(
        "geoReproject",
        |entities: Vec<Entity>, from: System, to: System, choices: Option<Vec<Choice>>| {
            let choices = choices.unwrap_or_default();
            entities
                .iter()
                .map(|e| {
                    let r = reproject(&e.shape, &from, &to, &choices);
                    ReprojectOut {
                        shape: r.shape.map(Entity::new),
                        error: r.error.map(Unreached::as_str),
                        chorded: r.chorded,
                    }
                })
                .collect::<Vec<ReprojectOut>>()
        }
    ),
    op!("geoMeasure", |entities: Vec<Entity>| {
        entities
            .iter()
            .map(|e| measured(&e.shape))
            .collect::<Vec<Measured>>()
    }),
];
