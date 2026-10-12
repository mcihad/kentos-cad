//! Where the fillet and the chamfer find their corner (the web's
//! `cornerAt`, `pathCorner`, `linesCorner`, `cornerOfPicks`): under the
//! cursor, by the shared core (`corner_near`), or from two picked lines or
//! neighbouring edges of one path.

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::tools::editing::{
    CornerSite, corner_near, lines_corner_at, shared_corner,
};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge;
use crate::tool::{Context, Pointer};

/// How near the cursor a corner is found, logical pixels (the web's `HOVER_PX`).
const HOVER_PX: f64 = 12.0;

pub(super) use kentos_native_application::corner::{Corner, Site, line_of, path_corner};

/// The kinds the corner tools take, off locked layers.
pub(super) fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    ) && edge::unlocked(e, doc)
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
    if a.0 != b.0 || !matches!(ea, Entity::Polyline(_) | Entity::Polygon(_)) {
        return Err(
            "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.",
        );
    }
    let v = shared_corner(&shape(ea), a.1, b.1)
        .ok_or("Seçilen kenarlar komşu değil; ortak köşesi olan iki kenar seçin.")?;
    path_corner(a.0, ea, v).ok_or(
        "Bu köşenin kenarlarından biri yay; yalnızca düz kenarlar arasındaki köşe işlenebilir.",
    )
}
