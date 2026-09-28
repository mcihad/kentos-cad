//! Parçala and Çizimi temizle (docs/adr/0140). Parçala cuts line work into
//! separate objects: where the chosen objects cross one another, into equal
//! parts, or into pieces of a given length from one end; arcs stay arcs,
//! polylines keep their arc edges. Çizimi temizle finds what a drawing
//! carries twice or empty: objects repeated exactly on the same layer,
//! objects with no length or no area, and vertices repeated in a row.
//! Nothing is snapped and no tolerance is widened (CLAUDE.md §23.3): a
//! repeat is the same geometry to 1e-9 m.

use crate::api::Op;
use crate::api::json::{Json, ToJson, field};
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{BulgePath, clean_bulge_path};
use crate::geom::intersect::Edge;
use crate::jsmath::{js_hypot, js_max};
use crate::op;
use crate::ops::edges::entity_edges;
use crate::ops::path::{Division, Path, cuts_on, division_params, path_of, sub_path};
use crate::vec2::Vec2;

/// The kinds Parçala cuts: lines, open polylines, arcs and circles.
fn splittable(s: &Shape) -> bool {
    matches!(
        s,
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. } | Shape::Circle { .. }
    )
}

/// The object cut at arc lengths `cuts` (sorted, inside the path): its
/// pieces in order along it, each with the object's other fields. A circle
/// needs two cuts at least (one leaves it whole). `None` when nothing is cut.
pub fn pieces_at(e: &Entity, path: &Path, cuts: &[f64]) -> Option<Vec<Entity>> {
    let circle = matches!(e.shape, Shape::Circle { .. });
    if cuts.is_empty() || (circle && cuts.len() < 2) {
        return None;
    }
    let mut bounds: Vec<f64> = Vec::with_capacity(cuts.len() + 2);
    if path.closed {
        bounds.extend_from_slice(cuts);
        // The last piece runs round past the start to the first cut.
        bounds.push(cuts[0] + path.length);
    } else {
        bounds.push(0.0);
        bounds.extend_from_slice(cuts);
        bounds.push(path.length);
    }
    let eps = 1e-9 * js_max(1.0, path.length);
    let pieces: Vec<Entity> = bounds
        .windows(2)
        .filter(|w| w[1] - w[0] > eps)
        .map(|w| e.with(sub_path(path, w[0], w[1], &e.shape).shape))
        .collect();
    (pieces.len() > 1).then_some(pieces)
}

/// One object's pieces: which of the given objects it is, and its pieces.
pub struct Pieces {
    pub index: usize,
    pub pieces: Vec<Entity>,
}

impl ToJson for Pieces {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "index", &(self.index as f64));
        field(out, &mut first, "pieces", &self.pieces);
        out.push('}');
    }
}

/// Parçala, Kesişimlerden: each of `list` that can be cut is cut where the
/// others cross it (every object with edges cuts, whatever its kind; an
/// object does not cut itself). Only the objects that came apart are named.
pub fn split_at_crossings(list: &[Entity]) -> Vec<Pieces> {
    let edges: Vec<Vec<Edge>> = list.iter().map(|e| entity_edges(&e.shape)).collect();
    let mut out = Vec::new();
    for (i, e) in list.iter().enumerate() {
        if !splittable(&e.shape) {
            continue;
        }
        let Some(path) = path_of(&e.shape) else {
            continue;
        };
        let others: Vec<Edge> = edges
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .flat_map(|(_, es)| es.iter().cloned())
            .collect();
        let cuts = cuts_on(&path, &others);
        if let Some(pieces) = pieces_at(e, &path, &cuts) {
            out.push(Pieces { index: i, pieces });
        }
    }
    out
}

/// Parçala, Eşit parçalara: the object in `parts` pieces of equal length
/// (2 to 10 000). `None` for a kind Parçala does not cut or a bad count.
pub fn split_equal(e: &Entity, parts: f64) -> Option<Vec<Entity>> {
    if !splittable(&e.shape) || !(2.0..=10_000.0).contains(&parts) {
        return None;
    }
    let path = path_of(&e.shape)?;
    let cuts = division_params(&path, &Division::Parts(parts));
    pieces_at(e, &path, &cuts)
}

/// Parçala, Uzunluktan: pieces `length` metres long from the start (or from
/// the end, `from_end`), the last one shorter. A circle is measured from its
/// east point counter-clockwise. `None` for a kind Parçala does not cut, a
/// length not above zero, one that leaves the object whole, or more than
/// 10 000 pieces.
pub fn split_by_length(e: &Entity, length: f64, from_end: bool) -> Option<Vec<Entity>> {
    if !splittable(&e.shape) || !(length > 0.0) {
        return None;
    }
    let path = path_of(&e.shape)?;
    if path.length / length > 10_000.0 {
        return None;
    }
    let mut cuts = division_params(&path, &Division::Step(length));
    if path.closed {
        // A circle's first piece starts at its start too.
        cuts.insert(0, 0.0);
    }
    if from_end && !path.closed {
        cuts = cuts.iter().rev().map(|s| path.length - s).collect();
    }
    pieces_at(e, &path, &cuts)
}

/// What Çizimi temizle found: ids (as the objects gave them) of the repeats
/// and of the empty objects, which go, and the objects whose repeated
/// vertices were dropped, as they will be.
pub struct Findings {
    /// Every repeat after the first of its group, in the given order.
    pub repeats: Vec<Json>,
    /// Objects with no length or no area: a line with both ends together, a
    /// circle or an arc of radius 0, a path whose vertices all fall together.
    pub empty: Vec<Json>,
    /// Paths that repeated a vertex in a row, cleaned (the same object otherwise).
    pub cleaned: Vec<Entity>,
    /// How many vertices went from `cleaned`.
    pub vertices: usize,
}

impl ToJson for Findings {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "repeats", &self.repeats);
        field(out, &mut first, "empty", &self.empty);
        field(out, &mut first, "cleaned", &self.cleaned);
        field(out, &mut first, "vertices", &(self.vertices as f64));
        out.push('}');
    }
}

/// Two numbers the same to 1e-9 (m, or radians for angles).
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9
}

fn same_pt(a: Vec2, b: Vec2) -> bool {
    same(a.x, b.x) && same(a.y, b.y)
}

fn same_pts(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| same_pt(*p, *q))
}

fn same_bulges(a: Option<&[f64]>, b: Option<&[f64]>) -> bool {
    let n = a.map_or(0, <[f64]>::len).max(b.map_or(0, <[f64]>::len));
    (0..n).all(|i| {
        same(
            a.and_then(|x| x.get(i)).copied().unwrap_or(0.0),
            b.and_then(|x| x.get(i)).copied().unwrap_or(0.0),
        )
    })
}

fn same_rings(a: Option<&[Ring]>, b: Option<&[Ring]>) -> bool {
    let (a, b) = (a.unwrap_or(&[]), b.unwrap_or(&[]));
    a.len() == b.len()
        && a.iter().zip(b).all(|(r, s)| {
            same_pts(&r.pts, &s.pts) && same_bulges(r.bulges.as_deref(), s.bulges.as_deref())
        })
}

/// The same object drawn twice: the same kind and geometry. A line drawn the
/// other way round is the same line; a path drawn the other way is not
/// (its direction is data), nor text with other words.
fn same_shape(a: &Shape, b: &Shape) -> bool {
    match (a, b) {
        (Shape::Point { p, z }, Shape::Point { p: q, z: w }) => {
            same_pt(*p, *q) && z.unwrap_or(0.0) == w.unwrap_or(0.0)
        }
        (Shape::Line { a, b }, Shape::Line { a: c, b: d }) => {
            (same_pt(*a, *c) && same_pt(*b, *d)) || (same_pt(*a, *d) && same_pt(*b, *c))
        }
        (
            Shape::Polyline { pts, bulges, holes },
            Shape::Polyline {
                pts: p2,
                bulges: b2,
                holes: h2,
            },
        )
        | (
            Shape::Polygon { pts, bulges, holes },
            Shape::Polygon {
                pts: p2,
                bulges: b2,
                holes: h2,
            },
        ) => {
            same_pts(pts, p2)
                && same_bulges(bulges.as_deref(), b2.as_deref())
                && same_rings(holes.as_deref(), h2.as_deref())
        }
        (Shape::Circle { c, r }, Shape::Circle { c: d, r: s }) => same_pt(*c, *d) && same(*r, *s),
        (
            Shape::Arc { c, r, a0, a1 },
            Shape::Arc {
                c: d,
                r: s,
                a0: b0,
                a1: b1,
            },
        ) => same_pt(*c, *d) && same(*r, *s) && same(*a0, *b0) && same(*a1, *b1),
        (Shape::Spline { pts, closed }, Shape::Spline { pts: q, closed: k }) => {
            closed == k && same_pts(pts, q)
        }
        (
            Shape::Text {
                p,
                text,
                height,
                rotation,
            },
            Shape::Text {
                p: q,
                text: t,
                height: h,
                rotation: r,
            },
        ) => same_pt(*p, *q) && text == t && same(*height, *h) && same(*rotation, *r),
        _ => a == b,
    }
}

fn field_of<'a>(e: &'a Entity, name: &str) -> Option<&'a Json> {
    e.rest.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// Whether an object has nothing to draw: no length (a line, a path) or no
/// radius (a circle, an arc).
fn is_empty(s: &Shape) -> bool {
    let tiny = |l: f64| l <= 1e-9;
    match s {
        Shape::Line { a, b } => tiny(js_hypot(b.x - a.x, b.y - a.y)),
        Shape::Circle { r, .. } | Shape::Arc { r, .. } => tiny(*r),
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } | Shape::Spline { pts, .. } => {
            pts.windows(2).all(|w| same_pt(w[0], w[1]))
        }
        Shape::Text { text, .. } => text.trim().is_empty(),
        _ => false,
    }
}

/// A path's vertices repeated in a row dropped (and a closed ring's last one
/// that repeats its first); `None` when none repeats.
fn without_repeats(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Option<BulgePath> {
    let clean = clean_bulge_path(pts, bulges, closed, 1e-9);
    (clean.pts.len() < pts.len()).then_some(clean)
}

/// Çizimi temizle: what `list` (the selection, or the whole drawing) carries
/// twice or empty. Repeats are only on the same layer (another layer is
/// another meaning); the first of a group stays. An empty object is not also
/// a repeat, and a repeat is not cleaned.
pub fn cleanup_findings(list: &[Entity]) -> Findings {
    let mut repeats = Vec::new();
    let mut empty = Vec::new();
    let mut cleaned = Vec::new();
    let mut vertices = 0;
    let mut kept: Vec<usize> = Vec::new();
    for (i, e) in list.iter().enumerate() {
        let id = field_of(e, "id").cloned().unwrap_or(Json::Null);
        if is_empty(&e.shape) {
            empty.push(id);
            continue;
        }
        let layer = field_of(e, "layerId");
        if kept.iter().any(|&k| {
            field_of(&list[k], "layerId") == layer && same_shape(&list[k].shape, &e.shape)
        }) {
            repeats.push(id);
            continue;
        }
        kept.push(i);
        let shape = match &e.shape {
            Shape::Polyline { pts, bulges, holes } => {
                without_repeats(pts, bulges.as_deref(), false).map(|c| Shape::Polyline {
                    pts: c.pts,
                    bulges: c.bulges,
                    holes: holes.clone(),
                })
            }
            Shape::Polygon { pts, bulges, holes } => {
                let outer = without_repeats(pts, bulges.as_deref(), true);
                let inner: Option<Vec<Option<BulgePath>>> = holes.as_ref().map(|hs| {
                    hs.iter()
                        .map(|h| without_repeats(&h.pts, h.bulges.as_deref(), true))
                        .collect()
                });
                let any_hole = inner
                    .as_ref()
                    .is_some_and(|v| v.iter().any(Option::is_some));
                (outer.is_some() || any_hole).then(|| {
                    let o = outer.unwrap_or_else(|| BulgePath {
                        pts: pts.clone(),
                        bulges: bulges.clone(),
                    });
                    Shape::Polygon {
                        pts: o.pts,
                        bulges: o.bulges,
                        holes: holes.as_ref().map(|hs| {
                            hs.iter()
                                .zip(inner.unwrap_or_default())
                                .map(|(h, c)| match c {
                                    Some(c) => Ring {
                                        pts: c.pts,
                                        bulges: c.bulges,
                                    },
                                    None => h.clone(),
                                })
                                .collect()
                        }),
                    }
                })
            }
            _ => None,
        };
        if let Some(shape) = shape {
            vertices += count(&e.shape) - count(&shape);
            cleaned.push(e.with(shape));
        }
    }
    Findings {
        repeats,
        empty,
        cleaned,
        vertices,
    }
}

fn count(s: &Shape) -> usize {
    match s {
        Shape::Polyline { pts, holes, .. } | Shape::Polygon { pts, holes, .. } => {
            pts.len()
                + holes
                    .as_ref()
                    .map_or(0, |h| h.iter().map(|r| r.pts.len()).sum())
        }
        _ => 0,
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("splitAtCrossings", |list: Vec<Entity>| split_at_crossings(
        &list
    )),
    op!("splitEqual", |e: Entity, parts: f64| split_equal(&e, parts)),
    op!("splitByLength", |e: Entity, length: f64, from_end: bool| {
        split_by_length(&e, length, from_end)
    }),
    op!("cleanupFindings", |list: Vec<Entity>| cleanup_findings(
        &list
    )),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsmath::PI;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn with_id(id: f64, layer: &str, shape: Shape) -> Entity {
        Entity {
            shape,
            rest: vec![
                ("id".into(), Json::Num(id)),
                ("layerId".into(), Json::Str(layer.into())),
            ],
        }
    }

    fn line(ax: f64, ay: f64, bx: f64, by: f64) -> Entity {
        Entity::new(Shape::Line {
            a: v(ax, ay),
            b: v(bx, by),
        })
    }

    #[test]
    fn crossing_lines_come_apart_where_they_cross() {
        // A plus sign and a line that meets nothing.
        let list = vec![
            line(-5.0, 0.0, 5.0, 0.0),
            line(0.0, -5.0, 0.0, 5.0),
            line(10.0, 10.0, 20.0, 10.0),
        ];
        let out = split_at_crossings(&list);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].index, 0);
        assert_eq!(
            out[0]
                .pieces
                .iter()
                .map(|p| p.shape.clone())
                .collect::<Vec<_>>(),
            vec![
                Shape::Line {
                    a: v(-5.0, 0.0),
                    b: v(0.0, 0.0)
                },
                Shape::Line {
                    a: v(0.0, 0.0),
                    b: v(5.0, 0.0)
                },
            ]
        );
        assert_eq!(out[1].pieces.len(), 2);
    }

    #[test]
    fn a_circle_needs_two_cuts_and_its_pieces_are_arcs() {
        let circle = Entity::new(Shape::Circle {
            c: v(0.0, 0.0),
            r: 5.0,
        });
        // One line through the centre crosses it twice.
        let out = split_at_crossings(&[circle.clone(), line(-10.0, 0.0, 10.0, 0.0)]);
        let arcs: Vec<&Pieces> = out.iter().filter(|p| p.index == 0).collect();
        assert_eq!(arcs.len(), 1);
        assert_eq!(arcs[0].pieces.len(), 2);
        for p in &arcs[0].pieces {
            let Shape::Arc { r, a0, a1, .. } = p.shape else {
                panic!("an arc");
            };
            assert!((r - 5.0).abs() < 1e-12);
            let sweep = (a1 - a0).rem_euclid(2.0 * PI);
            assert!((sweep - PI).abs() < 1e-9, "half circles");
        }
        // A tangent line touches it once: it stays whole.
        let out = split_at_crossings(&[circle, line(-10.0, 5.0, 10.0, 5.0)]);
        assert!(out.iter().all(|p| p.index != 0));
    }

    #[test]
    fn equal_parts_and_lengths_from_either_end() {
        let l = with_id(
            4.0,
            "a",
            Shape::Line {
                a: v(0.0, 0.0),
                b: v(10.0, 0.0),
            },
        );
        let parts = split_equal(&l, 4.0).expect("four");
        assert_eq!(parts.len(), 4);
        assert_eq!(
            parts[1].shape,
            Shape::Line {
                a: v(2.5, 0.0),
                b: v(5.0, 0.0)
            }
        );
        assert_eq!(parts[1].rest, l.rest, "the pieces keep the object's fields");
        let by = split_by_length(&l, 3.0, false).expect("pieces");
        assert_eq!(by.len(), 4);
        assert_eq!(
            by[3].shape,
            Shape::Line {
                a: v(9.0, 0.0),
                b: v(10.0, 0.0)
            }
        );
        let back = split_by_length(&l, 3.0, true).expect("pieces");
        assert_eq!(
            back[0].shape,
            Shape::Line {
                a: v(0.0, 0.0),
                b: v(1.0, 0.0)
            }
        );
        assert!(
            split_by_length(&l, 20.0, false).is_none(),
            "longer than the line: whole"
        );
        assert!(split_equal(&l, 1.0).is_none());
        assert!(split_by_length(&l, 0.0, false).is_none());
        let area = Entity::new(Shape::Polygon {
            pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)],
            bulges: None,
            holes: None,
        });
        assert!(
            split_equal(&area, 2.0).is_none(),
            "areas split with Alan böl"
        );
    }

    #[test]
    fn a_polyline_keeps_its_arc_edges_in_its_pieces() {
        let p = Entity::new(Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(20.0, 0.0)],
            bulges: Some(vec![0.0, 1.0, 0.0]),
            holes: None,
        });
        let parts = split_equal(&p, 2.0).expect("two");
        assert_eq!(parts.len(), 2);
        let arc_pieces = parts
            .iter()
            .filter(|e| match &e.shape {
                Shape::Polyline { bulges, .. } => {
                    bulges.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0))
                }
                Shape::Arc { .. } => true,
                _ => false,
            })
            .count();
        assert!(arc_pieces >= 1);
    }

    #[test]
    fn cleanup_finds_repeats_empties_and_repeated_vertices() {
        let square = |id: f64, layer: &str| {
            with_id(
                id,
                layer,
                Shape::Polygon {
                    pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)],
                    bulges: None,
                    holes: None,
                },
            )
        };
        let list = vec![
            square(1.0, "a"),
            square(2.0, "a"),
            square(3.0, "b"),
            with_id(
                4.0,
                "a",
                Shape::Line {
                    a: v(5.0, 5.0),
                    b: v(5.0, 5.0),
                },
            ),
            with_id(
                5.0,
                "a",
                Shape::Line {
                    a: v(0.0, 0.0),
                    b: v(3.0, 0.0),
                },
            ),
            with_id(
                6.0,
                "a",
                Shape::Line {
                    a: v(3.0, 0.0),
                    b: v(0.0, 0.0),
                },
            ),
            with_id(
                7.0,
                "a",
                Shape::Polyline {
                    pts: vec![v(0.0, 5.0), v(1.0, 5.0), v(1.0, 5.0), v(2.0, 5.0)],
                    bulges: None,
                    holes: None,
                },
            ),
        ];
        let f = cleanup_findings(&list);
        assert_eq!(
            f.repeats,
            vec![Json::Num(2.0), Json::Num(6.0)],
            "another layer is no repeat"
        );
        assert_eq!(f.empty, vec![Json::Num(4.0)]);
        assert_eq!(f.vertices, 1);
        assert_eq!(f.cleaned.len(), 1);
        let Shape::Polyline { pts, .. } = &f.cleaned[0].shape else {
            panic!("polyline");
        };
        assert_eq!(pts.len(), 3);
        assert_eq!(field_of(&f.cleaned[0], "id"), Some(&Json::Num(7.0)));
    }

    #[test]
    fn nothing_is_snapped_a_millimetre_is_no_repeat() {
        let a = with_id(
            1.0,
            "a",
            Shape::Line {
                a: v(0.0, 0.0),
                b: v(3.0, 0.0),
            },
        );
        let b = with_id(
            2.0,
            "a",
            Shape::Line {
                a: v(0.0, 0.0),
                b: v(3.001, 0.0),
            },
        );
        let f = cleanup_findings(&[a, b]);
        assert!(f.repeats.is_empty() && f.empty.is_empty() && f.cleaned.is_empty());
    }
}
