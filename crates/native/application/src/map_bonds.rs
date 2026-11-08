//! The map bond's parts (docs/adr/2580): a boundary edge (an area's or a single path's straight edge that moves only
//! parallel to itself, Paralel kaydır's rule) and the straight segment it keeps its distance from, read for the bond
//! adapter (`arch_bonds`, docs/adr/0770 §4) and the boundary's shift written back with the core's
//! `ops::edge_shift::shifted` (its neighbours lengthen or shorten to meet it). The adapter solves map bonds with the
//! architectural ones: one constraint set, one solve, one refusal.
//!
//! The shared cases are `fixtures/commands/v1/cad.shapes.edit.bond.json` (`scripts/fixtures/map_bond_cases.py`).

use kentos_commands::{CadElement, ConstraintRule, ElementRef, Entity, EntityId};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, is_multi_part};
use kentos_geometry_core::ops::arch_bonds::{PARALLEL, TIE};
use kentos_geometry_core::ops::edge_shift;
use kentos_geometry_core::ops::subref::{Place, edge_line};

use crate::geometry::{shape, with_shape};

/// A straight edge's line: its start and its unit direction; none on an arc, a vertex the object has not, a
/// zero-length edge.
fn line(s: &Shape, part: u32, ring: u32, index: u32) -> Option<(P, P)> {
    let core = CoreEntity::new(s.clone());
    let place = Place::Mid {
        part: part as usize,
        ring: ring as usize,
        index: index as usize,
    };
    let (a, b) = edge_line(&core, place)?;
    let l = (b.x - a.x).hypot(b.y - a.y);
    (l > 0.0).then(|| (a, P::new((b.x - a.x) / l, (b.y - a.y) / l)))
}

/// A boundary edge's line: an area's or a single path's straight edge only (what Paralel kaydır shifts).
pub(crate) fn boundary_line(s: &Shape, part: u32, ring: u32, index: u32) -> Option<(P, P)> {
    let takes = matches!(s, Shape::Polygon { .. } | Shape::Polyline { .. }) && part == 0 && !is_multi_part(s);
    if !takes {
        return None;
    }
    line(s, part, ring, index)
}

/// A reference segment's line: a line, or any object's straight edge.
pub(crate) fn segment_line(s: &Shape, part: u32, ring: u32, index: u32) -> Option<(P, P)> {
    line(s, part, ring, index)
}

/// The vertices a boundary edge runs between (its index and the next, round a closed ring); none when the ring has
/// not that edge.
pub(crate) fn ends(s: &Shape, ring: u32, index: u32) -> Option<(u32, u32)> {
    let (n, closed) = match s {
        Shape::Polygon { pts, holes, .. } => match ring {
            0 => (pts.len(), true),
            k => (holes.as_ref()?.get(k as usize - 1)?.pts.len(), true),
        },
        Shape::Polyline { pts, closed, .. } if ring == 0 => (pts.len(), *closed),
        _ => return None,
    };
    let n = u32::try_from(n).ok()?;
    let next = index + 1;
    match (next < n, closed && next == n) {
        (true, _) => Some((index, next)),
        (false, true) => Some((index, 0)),
        _ => None,
    }
}

/// Whether a write moved a boundary edge's line: turned it, or moved it across beyond the adapter's tie (sliding along
/// itself is no move). Its shape `before` the write and `now`; an edge the object no longer has moved.
pub(crate) fn moved(before: &Shape, now: &Shape, part: u32, ring: u32, index: u32) -> bool {
    match (
        boundary_line(before, part, ring, index),
        boundary_line(now, part, ring, index),
    ) {
        (Some((a0, u0)), Some((a1, u1))) => {
            let turned = (u0.x * u1.y - u0.y * u1.x).abs() > PARALLEL;
            let across = -u0.y * (a1.x - a0.x) + u0.x * (a1.y - a0.y);
            turned || across.abs() > TIE
        }
        _ => true,
    }
}

/// A boundary edge of `entity` moved `by` along its left normal (the bond adapter's shift) by Paralel kaydır's rule:
/// its neighbours lengthen or shorten to meet it. The object as it would be written, or why not in Turkish (Paralel
/// kaydır's words: a neighbour passed, a neighbour parallel to it).
pub(crate) fn shifted(entity: &Entity, part: u32, ring: u32, index: u32, by: f64) -> Result<Entity, String> {
    let s = shape(entity);
    let (_, dir) = boundary_line(&s, part, ring, index)
        .ok_or("Bağlı kenar artık bir alanın ya da çoklu çizginin düz kenarı değil")?;
    let normal = edge_shift::normal(&s, ring as usize, index as usize).map_err(str::to_owned)?;
    // Paralel kaydır's distance is along its own normal: the left normal's shift with the sign of their dot.
    let sense = if -dir.y * normal.x + dir.x * normal.y >= 0.0 { 1.0 } else { -1.0 };
    let moved = edge_shift::shifted(&s, ring as usize, index as usize, by * sense)?;
    let mut next = with_shape(entity, moved).ok_or("Bağlı kenarın nesnesi yazılamıyor")?;
    // The core's path carries no widths; Paralel kaydır keeps the vertices, so the widths stay as they were.
    if let (Entity::Polyline(n), Entity::Polyline(o)) = (&mut next, entity)
        && n.widths.is_none()
    {
        n.widths.clone_from(&o.widths);
    }
    Ok(next)
}

fn entity_of(doc: &Document, uid: EntityId) -> Option<&Entity> {
    doc.get(doc.slot_of(Uuid::from_bytes(uid.0))?)
}

/// Why a map bond's parts do not fit the drawing (docs/adr/2580): a boundary edge on another kind than an area or a
/// path, on an arc, a several-part object or an edge it has not; a reference that is not a straight segment; the edge
/// its own reference; edges that are not parallel within 0.001°. The contract checked the rule's kinds already.
pub(crate) fn rule_problem(doc: &Document, rule: &ConstraintRule) -> Option<String> {
    let ConstraintRule::Distance { a, b, .. } = rule else {
        return None;
    };
    let line_of = |e: &ElementRef| -> Result<(P, P), String> {
        let Some(entity) = entity_of(doc, e.uid) else {
            return Err("bağın nesnesi çizimde yok".into());
        };
        match e.element {
            CadElement::Boundary { part, ring, index } => {
                if !matches!(entity, Entity::Polygon(_) | Entity::Polyline(_)) {
                    return Err("sınır kenarı yalnız bir alanın ya da çoklu çizginin kenarı olur".into());
                }
                boundary_line(&shape(entity), part, ring, index).ok_or_else(|| {
                    format!(
                        "sınır kenarı ({}. kenar) bulunamadı: kenar yay ya da yok, ya da nesne çok parçalı",
                        index + 1
                    )
                })
            }
            CadElement::Segment { part, ring, index } => segment_line(&shape(entity), part, ring, index)
                .ok_or_else(|| "referans düz bir kenar olmalı: bir çizgi ya da bir yolun veya alanın düz kenarı".into()),
            _ => Err("harita bağı bir düz kenardan sınır kenarına uzaklıktır".into()),
        }
    };
    let (ua, ub) = match (line_of(a), line_of(b)) {
        (Ok((_, ua)), Ok((_, ub))) => (ua, ub),
        (Err(why), _) | (_, Err(why)) => return Some(why),
    };
    let same = |e: &ElementRef| match e.element {
        CadElement::Boundary { part, ring, index } | CadElement::Segment { part, ring, index } => {
            Some((e.uid, part, ring, index))
        }
        _ => None,
    };
    if same(a).is_some() && same(a) == same(b) {
        return Some("kenar kendisinin referansı olamaz".into());
    }
    let sine = (ua.x * ub.y - ua.y * ub.x).abs();
    (sine > PARALLEL).then(|| {
        format!(
            "bağlanan kenarlar paralel olmalı (0,001° içinde); aralarında {:.4}° var",
            sine.min(1.0).asin().to_degrees()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::geom::arrangement::Ring;

    fn square() -> Shape {
        Shape::Polygon {
            pts: vec![P::new(0.0, 0.0), P::new(10.0, 0.0), P::new(10.0, 10.0), P::new(0.0, 10.0)],
            bulges: None,
            holes: Some(vec![Ring {
                pts: vec![P::new(2.0, 2.0), P::new(2.0, 4.0), P::new(4.0, 4.0)],
                bulges: None,
            }]),
            parts: None,
        }
    }

    #[test]
    fn a_ring_s_edges_run_round_and_a_path_s_stop_at_its_end() {
        let s = square();
        assert_eq!(ends(&s, 0, 1), Some((1, 2)));
        assert_eq!(ends(&s, 0, 3), Some((3, 0)));
        assert_eq!(ends(&s, 1, 2), Some((2, 0)));
        assert_eq!(ends(&s, 2, 0), None);
        let path = Shape::Polyline {
            pts: vec![P::new(0.0, 0.0), P::new(5.0, 0.0), P::new(5.0, 5.0)],
            bulges: None,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        };
        assert_eq!(ends(&path, 0, 1), Some((1, 2)));
        assert_eq!(ends(&path, 0, 2), None);
    }

    #[test]
    fn sliding_along_itself_is_no_move_but_across_or_turned_is() {
        let s = square();
        let slid = Shape::Polygon {
            pts: vec![P::new(-3.0, 0.0), P::new(10.0, 0.0), P::new(10.0, 10.0), P::new(0.0, 10.0)],
            bulges: None,
            holes: None,
            parts: None,
        };
        assert!(!moved(&s, &slid, 0, 0, 0));
        let across = Shape::Polygon {
            pts: vec![P::new(0.0, -1.0), P::new(10.0, -1.0), P::new(10.0, 10.0), P::new(0.0, 10.0)],
            bulges: None,
            holes: None,
            parts: None,
        };
        assert!(moved(&s, &across, 0, 0, 0));
        assert!(!moved(&s, &across, 0, 0, 2), "the far edge stayed");
        let turned = Shape::Polygon {
            pts: vec![P::new(0.0, 0.0), P::new(10.0, 0.5), P::new(10.0, 10.0), P::new(0.0, 10.0)],
            bulges: None,
            holes: None,
            parts: None,
        };
        assert!(moved(&s, &turned, 0, 0, 0));
    }
}
