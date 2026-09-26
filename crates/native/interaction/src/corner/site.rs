//! Where the fillet and the chamfer find their corner (the web's
//! `cornerAt`, `pathCorner`, `linesCorner`, `cornerOfPicks`): under the
//! cursor, by the shared core (`corner_near`), or from two picked lines or
//! neighbouring edges of one path.

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::bulge_at;
use kentos_geometry_core::ops::fillet::Seg;
use kentos_geometry_core::ops::vertex::nearest_segment;
use kentos_geometry_core::tools::editing::{
    CornerGeom, CornerSite, corner_near, lines_corner_at, vertex_corner,
};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge;
use crate::tool::{Context, Pointer};

/// How near the cursor a corner is found, logical pixels (the web's `HOVER_PX`).
const HOVER_PX: f64 = 12.0;

/// Where the corner is, in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Site {
    /// Vertex `vertex` of the path at `slot`.
    Vertex { slot: Slot, vertex: usize },
    /// Two lines; each keeps the side of its pick point.
    Lines {
        first: Slot,
        pick1: Vec2,
        second: Slot,
        pick2: Vec2,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Corner {
    pub site: Site,
    pub geom: CornerGeom,
}

impl Corner {
    pub fn slots(&self) -> Vec<Slot> {
        match self.site {
            Site::Vertex { slot, .. } => vec![slot],
            Site::Lines { first, second, .. } => vec![first, second],
        }
    }
}

/// The kinds the corner tools take, off locked layers.
pub(super) fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    ) && edge::unlocked(e, doc)
}

pub(super) fn line_of(e: &Entity) -> Option<Seg> {
    match e {
        Entity::Line(l) => Some(Seg {
            a: Vec2::new(l.a.x, l.a.y),
            b: Vec2::new(l.b.x, l.b.y),
        }),
        _ => None,
    }
}

/// A path's corner at vertex `i` (the web's `pathCorner`): none at an open
/// path's end, beside an arc or on a straight run.
pub(super) fn path_corner(slot: Slot, e: &Entity, i: usize) -> Option<Corner> {
    let (Entity::Polyline(path) | Entity::Polygon(path)) = e else {
        return None;
    };
    let closed = matches!(e, Entity::Polygon(_));
    let n = path.pts.len();
    if i >= n || (!closed && (i == 0 || i + 1 >= n)) {
        return None;
    }
    let prev = (i + n - 1) % n;
    let p = |k: usize| Vec2::new(path.pts[k].x, path.pts[k].y);
    let bulges = path.bulges.as_deref();
    let geom = vertex_corner(
        p(prev),
        p(i),
        p((i + 1) % n),
        bulge_at(bulges, prev),
        bulge_at(bulges, i),
    )?;
    Some(Corner {
        site: Site::Vertex { slot, vertex: i },
        geom,
    })
}

/// The nearest corner within reach of the cursor: path vertices and
/// shared line ends among the editable objects around it, as the core
/// finds it (`corner_near`, the web's `cornerAt`).
pub(super) fn corner_at(p: &Pointer, cx: &Context<'_>) -> Option<Corner> {
    let tol = cx.view.world_length(HOVER_PX);
    let (a, b) = (
        Vec2::new(p.raw.x - tol, p.raw.y - tol),
        Vec2::new(p.raw.x + tol, p.raw.y + tol),
    );
    let doc = &*cx.doc;
    let near: Vec<(Slot, &Entity)> = cx
        .spatial
        .in_rect(a, b, true)
        .into_iter()
        .filter_map(|slot| Some((slot, doc.get(slot)?)))
        .filter(|(_, e)| editable(e, doc))
        .collect();
    let shapes: Vec<Shape> = near.iter().map(|(_, e)| shape(e)).collect();
    let hit = corner_near(&shapes, p.raw, tol, cx.view.world_length(2.0))?;
    let site = match hit.site {
        CornerSite::Vertex { object, vertex } => Site::Vertex {
            slot: near.get(object)?.0,
            vertex,
        },
        CornerSite::Lines {
            first,
            pick1,
            second,
            pick2,
        } => Site::Lines {
            first: near.get(first)?.0,
            pick1,
            second: near.get(second)?.0,
            pick2,
        },
    };
    Some(Corner {
        site,
        geom: hit.corner,
    })
}

/// The corner two picks name (the web's `cornerOfPicks`): two lines, or
/// two neighbouring edges of one path; the reason when they do not.
pub(super) fn corner_of_picks(
    a: (Slot, Vec2),
    b: (Slot, Vec2),
    cx: &Context<'_>,
) -> Result<Corner, &'static str> {
    let (Some(ea), Some(eb)) = (cx.doc.get(a.0), cx.doc.get(b.0)) else {
        return Err("Çizgiler paralel ya da seçilen tarafta çizgi yok.");
    };
    if let (Some(l1), Some(l2)) = (line_of(ea), line_of(eb)) {
        if a.0 == b.0 {
            return Err("İkinci çizgi ilkinden farklı olmalı.");
        }
        let geom = lines_corner_at(l1.a, l1.b, a.1, l2.a, l2.b, b.1)
            .ok_or("Çizgiler paralel ya da seçilen tarafta çizgi yok.")?;
        return Ok(Corner {
            site: Site::Lines {
                first: a.0,
                pick1: a.1,
                second: b.0,
                pick2: b.1,
            },
            geom,
        });
    }
    if a.0 != b.0 || matches!(ea, Entity::Line(_)) {
        return Err(
            "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.",
        );
    }
    let (Entity::Polyline(path) | Entity::Polygon(path)) = ea else {
        return Err(
            "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.",
        );
    };
    let n = path.pts.len();
    let s = shape(ea);
    let (i, j) = (nearest_segment(&s, a.1), nearest_segment(&s, b.1));
    let closed = matches!(ea, Entity::Polygon(_));
    let v = if j == i + 1 {
        Some(j)
    } else if i == j + 1 {
        Some(i)
    } else if closed && n > 0 && ((i == n - 1 && j == 0) || (j == n - 1 && i == 0)) {
        Some(0)
    } else {
        None
    };
    let v = v.ok_or("Seçilen kenarlar komşu değil; ortak köşesi olan iki kenar seçin.")?;
    path_corner(a.0, ea, v).ok_or(
        "Bu köşenin kenarlarından biri yay; yalnızca düz kenarlar arasındaki köşe işlenebilir.",
    )
}
