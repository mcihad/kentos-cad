//! The upward plan's shared model and paper visibility, and a device's grid alignment.

use kentos_commands::{
    CEILING_DEVICE_ATTR, CEILING_SPACE_ATTR, CeilingRange, Entity, SpaceEntity, SpacePart,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::{
    Vec2,
    geom::{
        arrangement::{Area, Ring},
        region::{inside_area, net_area},
    },
};

fn by_uid<'a>(doc: &'a Document, uid: &str) -> Option<&'a Entity> {
    doc.get(doc.slot_of(Uuid::parse_str(uid).ok()?)?)
}

fn areas(parts: &[SpacePart]) -> impl Iterator<Item = Area> + '_ {
    let ring = |r: &kentos_commands::SpaceRing| Ring {
        pts: r.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
        bulges: r.bulges.clone(),
    };
    parts.iter().map(move |p| Area {
        outer: ring(&p.outer),
        holes: p.holes.iter().map(ring).collect(),
    })
}

pub fn room_at<'a>(
    doc: &'a Document,
    level: Option<&str>,
    p: Vec2,
) -> Option<(Slot, &'a SpaceEntity)> {
    doc.entities()
        .filter_map(|e| {
            let Entity::Space(s) = e else { return None };
            if s.space.arch.level.as_deref() != level {
                return None;
            }
            let area = areas(&s.space.parts).find(|a| inside_area(a, p))?;
            Some((net_area(&area), Slot(s.base.id), s))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, slot, s)| (slot, s))
}

fn level_elevation(doc: &Document, level: Option<&str>) -> f64 {
    level
        .and_then(|id| {
            doc.settings()
                .arch
                .as_ref()?
                .levels
                .iter()
                .find(|l| l.id == id)
        })
        .map_or(0.0, |l| l.elevation)
}

fn is_ceiling(s: &kentos_commands::SlabFields) -> bool {
    s.covering
        .as_ref()
        .is_some_and(|c| c.predefined == "CEILING")
}

fn room_plane(
    doc: &Document,
    s: &SpaceEntity,
    point: Vec2,
    coverings: &[&kentos_commands::SlabFields],
) -> Option<f64> {
    if !areas(&s.space.parts).any(|a| inside_area(&a, point)) {
        return None;
    }
    let level = s.space.arch.level.as_deref();
    let base = level_elevation(doc, level);
    coverings
        .iter()
        .filter(|c| {
            c.arch.level.as_deref() == level && areas(&c.parts).any(|a| inside_area(&a, point))
        })
        .map(|s| {
            s.structure_underside_at(
                0.0,
                kentos_commands::Vec2 {
                    x: point.x,
                    y: point.y,
                },
            )
        })
        .min_by(f64::total_cmp)
        .or_else(|| doc.room_height(&s.space).map(|h| h.ceiling - base))
}

fn grid(h: &kentos_commands::HatchEntity) -> bool {
    h.base.attrs.get("Kaplama").map(String::as_str) == Some("Tavan")
}

/// Centre of the existing ceiling grid cell at `p`. The original point stays if the centre falls in a shaft.
pub fn align(doc: &Document, room: Slot, p: Vec2) -> Option<(Vec2, f64)> {
    doc.entities().find_map(|e| {
        let Entity::Hatch(h) = e else { return None };
        if !grid(h) || h.assoc.as_ref()?.outer?.0 != doc.uid(room)?.into_bytes() {
            return None;
        }
        let as_ring = |pts: &[kentos_commands::Vec2]| Ring {
            pts: pts.iter().map(|q| Vec2::new(q.x, q.y)).collect(),
            bulges: None,
        };
        let area = Area {
            outer: as_ring(&h.ring),
            holes: h.holes.iter().flatten().map(|r| as_ring(r)).collect(),
        };
        if !inside_area(&area, p) {
            return None;
        }
        let origin = h.pattern.origin?;
        let [horizontal, vertical] = h.pattern.lines.as_deref()? else {
            return None;
        };
        let dx = vertical.offset[1].abs();
        let dy = horizontal.offset[1].abs();
        if dx <= 1e-8 || dy <= 1e-8 {
            return None;
        }
        let (sin, cos) = kentos_num::sin_cos(h.pattern.angle.to_radians());
        let (x, y) = (p.x - origin.x, p.y - origin.y);
        let (u, v) = (
            ((x * cos + y * sin) / dx).floor() + 0.5,
            ((y * cos - x * sin) / dy).floor() + 0.5,
        );
        let at = Vec2::new(
            origin.x + u * dx * cos - v * dy * sin,
            origin.y + u * dx * sin + v * dy * cos,
        );
        inside_area(&area, at).then_some((at, h.pattern.angle))
    })
}

/// IDs left out of the reflected plan, the same for pick/snap, viewport, labels and print.
pub fn hidden(doc: &Document, level: Option<&str>, range: CeilingRange) -> Vec<u32> {
    let coverings: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Slab(s) if is_ceiling(&s.slab) => Some(&*s.slab),
            _ => None,
        })
        .collect();
    doc.entities()
        .filter(|e| {
            if level.is_some_and(|id| e.level(|u| by_uid(doc, u)).is_some_and(|other| id != other))
            {
                return true;
            }
            !match e {
                Entity::Wall(w) => w.wall.height.is_none_or(|h| h >= range.low),
                Entity::Column(_)
                | Entity::Space(_)
                | Entity::Grid(_)
                | Entity::Text(_)
                | Entity::Table(_)
                | Entity::Leader(_) => true,
                Entity::Slab(s) if is_ceiling(&s.slab) => {
                    range.contains(s.slab.structure_underside(0.0))
                }
                Entity::Hatch(h) if grid(h) => h
                    .assoc
                    .as_ref()
                    .and_then(|a| a.outer)
                    .and_then(|id| doc.slot_of(Uuid::from_bytes(id.0)))
                    .and_then(|s| doc.get(s))
                    .and_then(|e| match e {
                        Entity::Space(s) => Some(s),
                        _ => None,
                    })
                    .and_then(|s| {
                        room_plane(
                            doc,
                            s,
                            Vec2::new(h.assoc.as_ref()?.seed.x, h.assoc.as_ref()?.seed.y),
                            &coverings,
                        )
                    })
                    .is_some_and(|z| range.contains(z)),
                Entity::Insert(i) if i.base.attrs.contains_key(CEILING_DEVICE_ATTR) => i
                    .base
                    .attrs
                    .get(CEILING_SPACE_ATTR)
                    .map(String::as_str)
                    .and_then(|u| by_uid(doc, u))
                    .and_then(|e| match e {
                        Entity::Space(s) => Some(s),
                        _ => None,
                    })
                    .and_then(|s| room_plane(doc, s, Vec2::new(i.p.x, i.p.y), &coverings))
                    .is_some_and(|z| range.contains(z)),
                _ => false,
            }
        })
        .map(|e| e.base().id)
        .collect()
}
