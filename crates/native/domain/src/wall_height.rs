//! A wall's height (docs/adr/3040 §1; M40, ADV-18). Its own when it has one; else up to the underside of the slab over
//! it, never past its storey's height (floor to floor); without a slab over it, its storey's height, as before.
//!
//! - A masonry wall is built from the floor to the slab over it (the unit price books pay "the wall as built"). Drawn
//!   up to the next floor, it ran through the slab: Metraj counted the slab's thickness × its length as wall too, and
//!   the interior elevations drew it into the slab.
//! - The slab over it: a slab whose outline holds the middle of the wall's path, its underside more than 10 cm above the
//!   wall's foot (its own floor's slab is under it); the lowest of them. A slab of the storey above (offset 0) is the
//!   usual one; a roof or terrace slab of its own storey (offset above its foot) too.
//! - Never past its storey's height: a wall under a gallery whose slab is two storeys up stops at its storey.
//! - Its foot is its level's floor (the level's elevation; none without a level: 0).

use kentos_contracts::{Entity, SlabFields, Vec2, WallFields};

use crate::document::Document;

/// How far over its foot a slab's underside must be to stand over the wall, metres.
const OVER: f64 = 0.1;

impl Document {
    /// The wall's height, metres: its own; else up to the underside of the slab over it within its storey's height;
    /// else its storey's height. None: no height of its own, no slab over it and no storey height.
    pub fn wall_height(&self, w: &WallFields) -> Option<f64> {
        if let Some(h) = w.height {
            return Some(h);
        }
        let (foot, storey) = self.wall_storey(w);
        let under = middle(w).and_then(|at| self.slab_underside_over(at, foot));
        match (under.map(|u| u - foot), storey) {
            (Some(u), Some(h)) => Some(u.min(h)),
            (Some(u), None) => Some(u),
            (None, h) => h,
        }
    }

    /// Whether the wall's height comes from the slab over it (no height of its own, a slab over it lower than its
    /// storey's top): Öznitelikler says so.
    pub fn wall_under_slab(&self, w: &WallFields) -> bool {
        if w.height.is_some() {
            return false;
        }
        let (foot, storey) = self.wall_storey(w);
        let Some(under) = middle(w).and_then(|at| self.slab_underside_over(at, foot)) else {
            return false;
        };
        storey.is_none_or(|h| under - foot < h - 1e-9)
    }

    /// Its level's floor and height (floor to floor), metres.
    fn wall_storey(&self, w: &WallFields) -> (f64, Option<f64>) {
        let level = w.arch.level.as_deref().and_then(|id| {
            self.settings()
                .arch
                .as_ref()
                .and_then(|a| a.levels.iter().find(|l| l.id == id))
        });
        (
            level.map_or(0.0, |l| l.elevation),
            level.and_then(|l| l.height),
        )
    }

    /// The lowest underside, of the slabs whose outline holds `at`, more than [`OVER`] above `foot`.
    fn slab_underside_over(&self, at: Vec2, foot: f64) -> Option<f64> {
        let levels = self
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.as_slice())
            .unwrap_or_default();
        self.entities()
            .filter_map(|e| match e {
                Entity::Slab(s) => Some(&s.slab),
                _ => None,
            })
            .filter(|s| holds(s, at))
            .map(|s| {
                let floor = s
                    .arch
                    .level
                    .as_deref()
                    .and_then(|id| levels.iter().find(|l| l.id == id))
                    .map_or(0.0, |l| l.elevation);
                floor + s.offset - s.thickness
            })
            .filter(|u| *u > foot + OVER)
            .min_by(f64::total_cmp)
    }
}

/// The middle of the wall's path by its length (its straight stretches; a bulge's chord), none for no path.
fn middle(w: &WallFields) -> Option<Vec2> {
    let pts = &w.pts;
    let first = *pts.first()?;
    let mut stretches: Vec<(Vec2, Vec2)> = pts.windows(2).map(|p| (p[0], p[1])).collect();
    if w.closed && pts.len() > 2 {
        stretches.push((pts[pts.len() - 1], first));
    }
    let length = |a: Vec2, b: Vec2| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
    let total: f64 = stretches.iter().map(|(a, b)| length(*a, *b)).sum();
    if total <= 0.0 {
        return Some(first);
    }
    let mut left = total / 2.0;
    for (a, b) in stretches {
        let l = length(a, b);
        if left <= l {
            let k = if l > 0.0 { left / l } else { 0.0 };
            return Some(Vec2 {
                x: a.x + (b.x - a.x) * k,
                y: a.y + (b.y - a.y) * k,
            });
        }
        left -= l;
    }
    Some(first)
}

/// Whether one of the slab's parts holds `at`: inside its outer ring and none of its holes.
fn holds(s: &SlabFields, at: Vec2) -> bool {
    let p = kentos_geometry_core::Vec2::new(at.x, at.y);
    let ring = |pts: &[Vec2]| -> Vec<kentos_geometry_core::Vec2> {
        pts.iter()
            .map(|q| kentos_geometry_core::Vec2::new(q.x, q.y))
            .collect()
    };
    s.parts.iter().any(|part| {
        kentos_geometry_core::geometry::point_in_polygon(p, &ring(&part.outer.pts))
            && !part
                .holes
                .iter()
                .any(|h| kentos_geometry_core::geometry::point_in_polygon(p, &ring(&h.pts)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(level: &str, x0: f64, x1: f64, height: Option<f64>) -> WallFields {
        serde_json::from_value(serde_json::json!({
            "pts": [{"x": x0, "y": 2.0}, {"x": x1, "y": 2.0}],
            "width": 0.2, "justify": "center", "arch": {"level": level}
        }))
        .map(|mut w: WallFields| {
            w.height = height;
            w
        })
        .unwrap()
    }

    fn document(slabs: serde_json::Value) -> Document {
        let mut snapshot: serde_json::Value = serde_json::json!({
            "format": "kentos.document", "version": 2, "name": "Kat", "origin": {"x": 0.0, "y": 0.0},
            "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 100.0, "workspace": "cad",
                "arch": {"levels": [
                    {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                    {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
                ]}},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
            "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
        });
        for (i, s) in slabs.as_array().unwrap().iter().enumerate() {
            let mut e = s.clone();
            e["kind"] = serde_json::json!("slab");
            e["id"] = serde_json::json!(i + 1);
            e["layerId"] = serde_json::json!("0");
            e["attrs"] = serde_json::json!({});
            snapshot["entities"].as_array_mut().unwrap().push(e);
            snapshot["uids"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!(format!(
                    "0192f5a1-2590-7000-8000-0000000000{:02}",
                    i + 10
                )));
        }
        Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
    }

    fn slab(level: &str, x0: f64, x1: f64, thickness: f64, offset: f64) -> serde_json::Value {
        serde_json::json!({
            "parts": [{"outer": {"pts": [
                {"x": x0, "y": 0.0}, {"x": x1, "y": 0.0}, {"x": x1, "y": 5.0}, {"x": x0, "y": 5.0}
            ]}}],
            "thickness": thickness, "offset": offset, "arch": {"level": level}
        })
    }

    #[test]
    fn a_wall_stops_under_the_slab_over_it_within_its_storey() {
        // No slab: its storey's height, as before.
        let bare = document(serde_json::json!([]));
        assert_eq!(bare.wall_height(&wall("zemin", 0.0, 4.0, None)), Some(3.0));
        assert!(!bare.wall_under_slab(&wall("zemin", 0.0, 4.0, None)));
        // The storey above's floor slab, 15 cm: 2.85; its own floor's slab is under it.
        let doc = document(serde_json::json!([
            slab("zemin", -1.0, 10.0, 0.15, 0.0),
            slab("k1", -1.0, 10.0, 0.15, 0.0)
        ]));
        let h = doc.wall_height(&wall("zemin", 0.0, 4.0, None)).unwrap();
        assert!((h - 2.85).abs() < 1e-9, "{h}");
        assert!(doc.wall_under_slab(&wall("zemin", 0.0, 4.0, None)));
        // Its own height wins.
        assert_eq!(
            doc.wall_height(&wall("zemin", 0.0, 4.0, Some(1.1))),
            Some(1.1)
        );
        // The storey above's wall: no slab over it, its storey's height.
        assert_eq!(doc.wall_height(&wall("k1", 0.0, 4.0, None)), Some(3.0));
        // A wall whose middle is out of the slab: its storey's.
        assert_eq!(doc.wall_height(&wall("zemin", 12.0, 16.0, None)), Some(3.0));
    }

    #[test]
    fn a_wall_under_a_gallery_stops_at_its_storey_and_takes_the_lowest_slab() {
        // A slab two storeys up only: the storey's height, not 5.85.
        let gallery = document(serde_json::json!([slab("k1", -1.0, 10.0, 0.15, 3.0)]));
        assert_eq!(
            gallery.wall_height(&wall("zemin", 0.0, 4.0, None)),
            Some(3.0)
        );
        // A lowered slab (offset −0.30) and the storey's own: the lowest underside, 2.55.
        let lowered = document(serde_json::json!([
            slab("k1", -1.0, 10.0, 0.15, 0.0),
            slab("k1", -1.0, 10.0, 0.15, -0.30)
        ]));
        let h = lowered.wall_height(&wall("zemin", 0.0, 4.0, None)).unwrap();
        assert!((h - 2.55).abs() < 1e-9, "{h}");
    }
}
