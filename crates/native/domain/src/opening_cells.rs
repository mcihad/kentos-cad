//! A door, a window or a bare opening in a curtain wall fills its cells in place of their panels (docs/adr/8129): its
//! jambs at the faces of the mullions round the bays it covers, its head and a window's sill at the transoms'; a door,
//! and one without a sill, stands on the floor. The commands fit a new or an edited one, and every step fits again the
//! openings it settles (`follow_openings`: their wall's grid, line or height changed).

use kentos_contracts::{Entity, OpeningAnchor, OpeningFields, OpeningForm};
use kentos_geometry_core::Vec2;

use crate::document::Document;
use crate::identity::Uuid;

impl Document {
    /// `o` fitted to the cells of its curtain wall (`geom::curtain::fill`): its `at`, width, sill and head theirs. None
    /// when its wall is gone or no curtain wall, it is a niche or a flue, its wall has no height, or it fills no cell
    /// (its middle off the wall).
    pub fn fitted_to_cells(&self, o: &OpeningFields) -> Option<OpeningFields> {
        if !matches!(
            o.form,
            OpeningForm::Door | OpeningForm::Window | OpeningForm::Opening
        ) {
            return None;
        }
        let uid = Uuid::parse_str(&o.host).ok()?;
        let stored = self.store.get(self.store.slot_of(uid)?)?;
        let Entity::Wall(w) = &*stored.entity else {
            return None;
        };
        let grid = w.wall.curtain.as_ref()?;
        let height = self.wall_height(&w.wall)?;
        let (floor, _) = self.storey_of(&w.wall.arch);
        // Its sill and head are over its storey's floor, the grid's rows over the wall's foot.
        let lift = self.wall_foot(&w.wall) - floor;
        let on_floor = o.form == OpeningForm::Door || o.sill.is_none_or(|s| s <= 1e-9);
        let pts: Vec<Vec2> = w.wall.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let f = kentos_geometry_core::geom::curtain::fill(
            &pts,
            w.wall.closed,
            (grid.spacing, grid.rows, grid.frame()),
            height,
            crate::opening_follow::span(o),
            (
                o.sill.unwrap_or(0.0) - lift,
                o.head.unwrap_or(height + lift) - lift,
            ),
            on_floor,
        )?;
        let mut out = o.clone();
        out.width = f.s1 - f.s0;
        out.at = match o.anchor {
            OpeningAnchor::Start => f.s0,
            OpeningAnchor::Center => (f.s0 + f.s1) / 2.0,
            OpeningAnchor::End => f.s1,
        };
        // One on the floor keeps its sill as it was said (none, or the floor).
        if !on_floor {
            out.sill = Some(f.sill + lift);
        } else if o.sill.is_some() {
            out.sill = Some(0.0);
        }
        out.head = Some(f.head + lift);
        Some(out)
    }

    /// Whether the wall with persistent id `host` is a curtain wall.
    pub fn is_curtain_wall(&self, host: &str) -> bool {
        Uuid::parse_str(host)
            .ok()
            .and_then(|u| self.store.slot_of(u))
            .and_then(|s| self.store.get(s))
            .is_some_and(|s| matches!(&*s.entity, Entity::Wall(w) if w.wall.curtain.is_some()))
    }
}
