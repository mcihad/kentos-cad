//! A room's floor, ceiling and inside height (docs/adr/3040 §3; M40, ADV-18). PAİY 4(1)(ıı): the inside height is
//! "taban döşeme kaplaması üzerinden tavan kaplamasına kadar olan net mesafe", from the floor's finish to the
//! ceiling's.
//!
//! - **Its floor:** its own offset from its storey's floor (a wet room 2 cm lower); else the top of the floor slab under
//!   it (its finish, docs/adr/3040 §2), the slab whose top is half a metre under its storey's floor to 10 cm over it,
//!   the highest; else its storey's floor (0 without a storey).
//! - **Its ceiling:** its suspended ceiling (asma tavan: its height over the floor); else its own inside height, typed;
//!   else the ceiling of the lowest slab over it (its structure's underside less its plaster), within its storey; else
//!   its storey's top, the next floor: floor to floor is no ceiling, only a bound over it.
//! - **Where:** a room from the walls at its seed, a drawn one at its label's point (inside its largest part).

use kentos_contracts::{ArchInfo, SpaceFields, Vec2};

use crate::document::Document;

/// How far over a room's floor a slab's ceiling must be to stand over it, metres (a wall's rule, docs/adr/3040 §1).
const OVER: f64 = 0.1;

/// Where a room's floor comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloorFrom {
    /// Its own offset from its storey's floor.
    Own,
    /// The top of the floor slab under it.
    Slab,
    /// Its storey's floor.
    Storey,
}

/// Where a room's ceiling comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CeilingFrom {
    /// Its suspended ceiling (asma tavan).
    Suspended,
    /// Its own inside height, typed.
    Own,
    /// The slab over it: its structure's underside less its plaster.
    Slab,
    /// Its storey's top, the next floor: no ceiling known.
    Storey,
}

/// A room's floor and ceiling, metres over the project's ±0.00, where each comes from, and its inside height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomHeight {
    pub floor: f64,
    pub floor_from: FloorFrom,
    pub ceiling: f64,
    pub ceiling_from: CeilingFrom,
    /// Its asma tavan's or its typed height as written; else its ceiling less its floor.
    inside: f64,
}

impl RoomHeight {
    /// Its inside height (PAİY 4(1)(ıı)), metres: from its floor to its ceiling.
    pub fn inside(&self) -> f64 {
        self.inside
    }

    /// Whether its ceiling is one: an asma tavan, a typed height or a slab; not its storey's top.
    pub fn ceiling_known(&self) -> bool {
        self.ceiling_from != CeilingFrom::Storey
    }
}

impl Document {
    /// The room's floor, metres over ±0.00, and where it comes from.
    pub fn room_floor(&self, s: &SpaceFields) -> (f64, FloorFrom) {
        let (floor, _) = self.storey_of(&s.arch);
        if let Some(offset) = s.floor_offset {
            return (floor + offset, FloorFrom::Own);
        }
        match room_point(s).and_then(|at| self.floor_slab_under(at, floor)) {
            Some((slab, f)) => (f + slab.offset, FloorFrom::Slab),
            None => (floor, FloorFrom::Storey),
        }
    }

    /// The room's floor and ceiling; none without a ceiling: no asma tavan, no height of its own, no slab over it
    /// and no storey height.
    pub fn room_height(&self, s: &SpaceFields) -> Option<RoomHeight> {
        let (storey_floor, storey) = self.storey_of(&s.arch);
        let (floor, floor_from) = self.room_floor(s);
        // A height written (its asma tavan's, its own) is its inside height as written.
        let (ceiling, ceiling_from, inside) = if let Some(c) = s.suspended_ceiling {
            (floor + c, CeilingFrom::Suspended, c)
        } else if let Some(h) = s.height {
            (floor + h, CeilingFrom::Own, h)
        } else {
            // Never past its storey's top: the next floor.
            let top = storey.map(|h| storey_floor + h);
            let slab = room_point(s)
                .and_then(|at| self.slab_ceiling_over(at, floor))
                .filter(|c| top.is_none_or(|t| *c <= t + 1e-9));
            match (slab, top) {
                (Some(c), _) => (c, CeilingFrom::Slab, c - floor),
                (None, Some(t)) => (t, CeilingFrom::Storey, t - floor),
                (None, None) => return None,
            }
        };
        Some(RoomHeight {
            floor,
            floor_from,
            ceiling,
            ceiling_from,
            inside,
        })
    }

    /// The lowest ceiling, of the slabs whose outline holds `at`, more than [`OVER`] above `floor`.
    fn slab_ceiling_over(&self, at: Vec2, floor: f64) -> Option<f64> {
        self.slabs_holding(at)
            .map(|(s, f)| s.ceiling(f))
            .filter(|c| *c > floor + OVER)
            .min_by(f64::total_cmp)
    }

    /// Its storey's floor and height (floor to floor), metres; 0 and none without a storey.
    pub(crate) fn storey_of(&self, arch: &ArchInfo) -> (f64, Option<f64>) {
        let level = arch.level.as_deref().and_then(|id| {
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
}

/// Where the room's slabs are looked for: its seed (a room from the walls), else its label's point.
fn room_point(s: &SpaceFields) -> Option<Vec2> {
    s.seed.or_else(|| {
        let p =
            kentos_geometry_core::geom::space::label_at(&crate::arch_values::core_areas(&s.parts))?;
        Some(Vec2 { x: p.x, y: p.y })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wall_height::tests::{document, slab};

    /// A room drawn 0–4 × 1–4 on `level`; `extra` its other fields.
    fn room(level: &str, extra: serde_json::Value) -> SpaceFields {
        let mut v = serde_json::json!({
            "parts": [{"outer": {"pts": [
                {"x": 0.0, "y": 1.0}, {"x": 4.0, "y": 1.0}, {"x": 4.0, "y": 4.0}, {"x": 0.0, "y": 4.0}
            ]}}],
            "arch": {"level": level}
        });
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        serde_json::from_value(v).unwrap()
    }

    /// Ara kat döşemesi: kaplama 2, şap 5 over the concrete, sıva 2 under it.
    fn layered(level: &str, offset: f64) -> serde_json::Value {
        let mut s = slab(level, -1.0, 10.0, 0.15, offset);
        s["layers"] = serde_json::json!({
            "over": [{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05}],
            "under": [{"kind": "plaster", "thickness": 0.02}]
        });
        s
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_room_s_inside_height_runs_from_its_floor_s_finish_to_its_ceiling_s() {
        // No slab: its storey's floor to its storey's top, no ceiling known.
        let bare = document(serde_json::json!([]));
        let h = bare
            .room_height(&room("zemin", serde_json::json!({})))
            .unwrap();
        assert_eq!(
            (h.floor_from, h.ceiling_from),
            (FloorFrom::Storey, CeilingFrom::Storey)
        );
        assert!(near(h.inside(), 3.0) && !h.ceiling_known(), "{h:?}");
        // The storey above's floor slab, Ara kat: 3.00 − 0.07 − 0.15 − 0.02 = 2.76 to its plaster.
        let doc = document(serde_json::json!([
            layered("zemin", 0.0),
            layered("k1", 0.0)
        ]));
        let h = doc
            .room_height(&room("zemin", serde_json::json!({})))
            .unwrap();
        assert_eq!(
            (h.floor_from, h.ceiling_from),
            (FloorFrom::Slab, CeilingFrom::Slab)
        );
        assert!(near(h.floor, 0.0) && near(h.ceiling, 2.76), "{h:?}");
        // A wet room 2 cm lower: 2.78.
        let wet = doc
            .room_height(&room("zemin", serde_json::json!({"floorOffset": -0.02})))
            .unwrap();
        assert_eq!(wet.floor_from, FloorFrom::Own);
        assert!(
            near(wet.floor, -0.02) && near(wet.inside(), 2.78),
            "{wet:?}"
        );
        // An asma tavan at 2.50 over the floor: its inside height, over a height typed too.
        for extra in [
            serde_json::json!({"suspendedCeiling": 2.5}),
            serde_json::json!({"suspendedCeiling": 2.5, "height": 2.9}),
        ] {
            let s = doc.room_height(&room("zemin", extra)).unwrap();
            assert_eq!(s.ceiling_from, CeilingFrom::Suspended);
            assert!(near(s.inside(), 2.5), "{s:?}");
        }
        // A height typed without one: its own.
        let own = doc
            .room_height(&room("zemin", serde_json::json!({"height": 2.9})))
            .unwrap();
        assert_eq!(own.ceiling_from, CeilingFrom::Own);
        assert!(near(own.inside(), 2.9), "{own:?}");
        // The storey above: on its slab's finish (3.00), no slab over it: its storey's top.
        let up = doc.room_height(&room("k1", serde_json::json!({}))).unwrap();
        assert_eq!(
            (up.floor_from, up.ceiling_from),
            (FloorFrom::Slab, CeilingFrom::Storey)
        );
        assert!(near(up.floor, 3.0) && near(up.inside(), 3.0), "{up:?}");
    }

    #[test]
    fn a_lowered_floor_slab_lowers_the_room_and_a_gallery_s_slab_is_not_its_ceiling() {
        // The wet room's slab 5 cm lower: its floor −0.05; 2.76 + 0.05 = 2.81 to the ceiling.
        let doc = document(serde_json::json!([
            layered("zemin", -0.05),
            layered("k1", 0.0)
        ]));
        let h = doc
            .room_height(&room("zemin", serde_json::json!({})))
            .unwrap();
        assert!(near(h.floor, -0.05) && near(h.inside(), 2.81), "{h:?}");
        // A slab two storeys up only: its storey's top, not the slab.
        let gallery = document(serde_json::json!([slab("k1", -1.0, 10.0, 0.15, 3.0)]));
        let g = gallery
            .room_height(&room("zemin", serde_json::json!({})))
            .unwrap();
        assert_eq!(g.ceiling_from, CeilingFrom::Storey);
        assert!(near(g.inside(), 3.0), "{g:?}");
        // A room out of every slab: its storey's.
        let mut away = room("zemin", serde_json::json!({}));
        for p in &mut away.parts[0].outer.pts {
            p.x += 20.0;
        }
        let a = doc.room_height(&away).unwrap();
        assert_eq!(
            (a.floor_from, a.ceiling_from),
            (FloorFrom::Storey, CeilingFrom::Storey)
        );
        // No storey: no height.
        assert_eq!(bare_room_without_level(&doc), None);
    }

    fn bare_room_without_level(doc: &Document) -> Option<RoomHeight> {
        let mut r = room("zemin", serde_json::json!({}));
        r.arch.level = None;
        for p in &mut r.parts[0].outer.pts {
            p.x += 20.0;
        }
        doc.room_height(&r)
    }
}
