//! The drawing's straight grids as the temporary distances' second reference (docs/adr/0728 §1): their lines and levels
//! for the core's `ops::grid_ref`, and the grid a distance names back to its persistent id.

use kentos_commands::{Entity, GridGap, Vec2};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::grid_ref::{self, GridIn};

/// The drawing's straight grids in its order, with their slots; an arc grid is none.
pub(crate) struct Grids {
    slots: Vec<Slot>,
    lines: Vec<(kentos_geometry_core::Vec2, kentos_geometry_core::Vec2, Option<String>)>,
}

impl Grids {
    pub(crate) fn of(doc: &Document) -> Grids {
        let mut out = Grids {
            slots: Vec::new(),
            lines: Vec::new(),
        };
        for e in doc.entities() {
            let Entity::Grid(g) = e else { continue };
            let pts = &g.grid.pts;
            let straight = g.grid.bulges.as_ref().is_none_or(|b| b.iter().all(|x| *x == 0.0));
            if pts.len() != 2 || !straight {
                continue;
            }
            out.slots.push(Slot(e.base().id));
            out.lines.push((
                kentos_geometry_core::Vec2::new(pts[0].x, pts[0].y),
                kentos_geometry_core::Vec2::new(pts[1].x, pts[1].y),
                g.grid.arch.level.clone(),
            ));
        }
        out
    }

    pub(crate) fn ins(&self) -> Vec<GridIn<'_>> {
        self.lines
            .iter()
            .map(|(a, b, level)| GridIn {
                a: *a,
                b: *b,
                level: level.as_deref(),
            })
            .collect()
    }

    /// The persistent id of the grid at `index` among them.
    pub(crate) fn uid_of(&self, doc: &Document, index: usize) -> Option<kentos_domain::Uuid> {
        doc.uid(*self.slots.get(index)?)
    }

    /// The core's gap as the contract says it: the grid by its persistent id.
    pub(crate) fn described(&self, doc: &Document, g: &grid_ref::GridGap) -> Option<GridGap> {
        Some(GridGap {
            distance: g.distance,
            uid: doc.uid(self.slots[g.grid])?.to_string(),
            from: Vec2 {
                x: g.from.x,
                y: g.from.y,
            },
            to: Vec2 { x: g.to.x, y: g.to.y },
        })
    }
}
