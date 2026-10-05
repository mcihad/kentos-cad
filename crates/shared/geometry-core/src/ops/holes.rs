//! Delikler (docs/adr/0173 §5), one for both platforms (the web through
//! WASM): a hole added to an area part, a hole removed, a hole's ring for
//! the area that fills it. A ring wholly inside a part's outer ring, meeting
//! none of its holes, is added as drawn; one that meets or covers holes
//! merges with them (the part less the ring, on the overlay engine). The
//! independent reference is `scripts/fixtures/reshape_cases.py`.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Shape, area_parts, replace_part};
use crate::geom::arrangement::{Area, Ring, winding};
use crate::geom::intersect::intersect_edges;
use crate::geom::region::{ring_area, ring_edges, subtract_areas};
use crate::op;
use crate::vec2::Vec2;

/// Why a hole is not added, removed or filled.
#[derive(Clone, Debug, PartialEq)]
pub enum HoleRefusal {
    /// The object is no area.
    NotArea,
    /// The ring has fewer than three vertices or no area.
    Degenerate,
    /// The ring is not wholly inside an area part (it crosses or touches
    /// the outer ring, lies outside, or lies inside a hole).
    Outside,
    /// The part less the ring would come apart.
    Splits,
    /// No hole holds the point.
    NotInHole,
}

crate::json_tagged!(HoleRefusal, "why",
    NotArea => "notArea" {},
    Degenerate => "degenerate" {},
    Outside => "outside" {},
    Splits => "splits" {},
    NotInHole => "notInHole" {},
);

/// A hole: its part (0: the area's own fields) and its place among the
/// part's holes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoleAt {
    pub part: usize,
    pub hole: usize,
}

crate::json_struct!(HoleAt { part, hole });

/// A part's rings as an area: its outer ring and its holes.
fn part_area(part: &Shape) -> Option<Area> {
    let Shape::Polygon {
        pts, bulges, holes, ..
    } = part
    else {
        return None;
    };
    Some(Area {
        outer: Ring {
            pts: pts.clone(),
            bulges: bulges.clone(),
        },
        holes: holes.clone().unwrap_or_default(),
    })
}

/// Whether `p` is inside the ring.
fn within(r: &Ring, p: Vec2) -> bool {
    winding(&ring_edges(r), p) != 0.0
}

/// Whether two rings cross or touch.
fn meet(a: &Ring, b: &Ring) -> bool {
    let (ea, eb) = (ring_edges(a), ring_edges(b));
    ea.iter()
        .any(|x| eb.iter().any(|y| !intersect_edges(x, y).is_empty()))
}

/// Whether `inner` lies wholly inside `outer`: its vertices inside, its
/// edges crossing and touching none of `outer`'s.
fn wholly_inside(inner: &Ring, outer: &Ring) -> bool {
    inner.pts.iter().all(|&p| within(outer, p)) && !meet(inner, outer)
}

/// The area with a hole `ring` added to the part whose outer ring holds it
/// (docs/adr/0173 §5): as drawn when it meets no hole of the part, else
/// merged with the holes it meets or covers.
pub fn hole_add(e: &Shape, ring: &Ring) -> Result<Shape, HoleRefusal> {
    if !matches!(e, Shape::Polygon { .. }) {
        return Err(HoleRefusal::NotArea);
    }
    if ring.pts.len() < 3 || ring_area(ring).abs() <= 1e-12 {
        return Err(HoleRefusal::Degenerate);
    }
    for (k, part) in area_parts(e).iter().enumerate() {
        let Some(area) = part_area(part) else {
            continue;
        };
        if !wholly_inside(ring, &area.outer) {
            continue;
        }
        // Inside a hole: nothing there to cut.
        if area.holes.iter().any(|h| wholly_inside(ring, h)) {
            return Err(HoleRefusal::Outside);
        }
        let touches = area.holes.iter().any(|h| {
            meet(ring, h)
                || h.pts.iter().any(|&q| within(ring, q))
                || ring.pts.iter().any(|&q| within(h, q))
        });
        let new_part = if !touches {
            let mut holes = area.holes.clone();
            holes.push(ring.clone());
            Shape::Polygon {
                pts: area.outer.pts,
                bulges: area.outer.bulges,
                holes: Some(holes),
                parts: None,
            }
        } else {
            let out = subtract_areas(
                std::slice::from_ref(&area),
                &[Area {
                    outer: ring.clone(),
                    holes: Vec::new(),
                }],
            );
            let [one] = out.as_slice() else {
                return Err(HoleRefusal::Splits);
            };
            Shape::Polygon {
                pts: one.outer.pts.clone(),
                bulges: one.outer.bulges.clone(),
                holes: (!one.holes.is_empty()).then(|| one.holes.clone()),
                parts: None,
            }
        };
        return replace_part(e, k, new_part).ok_or(HoleRefusal::NotArea);
    }
    Err(HoleRefusal::Outside)
}

/// The hole that holds `p`, the first so in the parts' order.
pub fn hole_at(e: &Shape, p: Vec2) -> Option<HoleAt> {
    if !matches!(e, Shape::Polygon { .. }) {
        return None;
    }
    for (k, part) in area_parts(e).iter().enumerate() {
        let Shape::Polygon {
            holes: Some(holes), ..
        } = part
        else {
            continue;
        };
        if let Some(h) = holes.iter().position(|h| within(h, p)) {
            return Some(HoleAt { part: k, hole: h });
        }
    }
    None
}

/// The ring of the hole that holds `p`: Deliği doldur's new area.
pub fn hole_ring(e: &Shape, p: Vec2) -> Result<Ring, HoleRefusal> {
    let at = hole_at(e, p).ok_or(HoleRefusal::NotInHole)?;
    match &area_parts(e)[at.part] {
        Shape::Polygon {
            holes: Some(holes), ..
        } => Ok(holes[at.hole].clone()),
        _ => Err(HoleRefusal::NotInHole),
    }
}

/// The area without the hole that holds `p` (Deliği sil).
pub fn hole_remove(e: &Shape, p: Vec2) -> Result<Shape, HoleRefusal> {
    let at = hole_at(e, p).ok_or(HoleRefusal::NotInHole)?;
    let Shape::Polygon {
        pts, bulges, holes, ..
    } = &area_parts(e)[at.part]
    else {
        return Err(HoleRefusal::NotArea);
    };
    let mut holes = holes.clone().unwrap_or_default();
    holes.remove(at.hole);
    let part = Shape::Polygon {
        pts: pts.clone(),
        bulges: bulges.clone(),
        holes: (!holes.is_empty()).then_some(holes),
        parts: None,
    };
    replace_part(e, at.part, part).ok_or(HoleRefusal::NotArea)
}

/// A write's answer for the web: the area (or the hole's ring), or why not.
pub enum Answer<T> {
    Done(T),
    Refused(HoleRefusal),
}

impl<T: ToJson> ToJson for Answer<T> {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match self {
            Answer::Done(v) => field(out, &mut first, "done", v),
            Answer::Refused(r) => field(out, &mut first, "refusal", r),
        }
        out.push('}');
    }
}

fn answer<T>(r: Result<T, HoleRefusal>) -> Answer<T> {
    match r {
        Ok(v) => Answer::Done(v),
        Err(e) => Answer::Refused(e),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("holeAdd", |e: crate::entity::Entity, ring: Ring| {
        answer(hole_add(&e.shape, &ring).map(crate::entity::Entity::new))
    }),
    op!("holeAt", |e: crate::entity::Entity, p: Vec2| hole_at(
        &e.shape, p
    )),
    op!("holeRing", |e: crate::entity::Entity, p: Vec2| {
        answer(hole_ring(&e.shape, p))
    }),
    op!("holeRemove", |e: crate::entity::Entity, p: Vec2| {
        answer(hole_remove(&e.shape, p).map(crate::entity::Entity::new))
    }),
];
