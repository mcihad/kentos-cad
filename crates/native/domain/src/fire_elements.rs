//! The building elements Binaların Yangından Korunması Hakkında Yönetmelik Ek-3/B gives fire resistance times for
//! (docs/adr/8082), with what the drawing says of each: the frame's columns and beams (row 1), the walls and whether
//! they are exterior (row 5), the floors (row 3). Whether a wall is a compartment's or a protected stair's is read from
//! the rooms it separates (`Document::separations`); the declared classes and the times asked are the standards
//! package's.
//!
//! - **Frame:** a column that is no post (docs/adr/8070 §8), a beam whose role is Kiriş (a lintel or a bond beam is not
//!   the frame's).
//! - **Exterior wall:** a wall with a corner on the outline of its storey's walls' and columns' bodies together, the rule
//!   the IFC model's `IsExternal` reads (docs/adr/8047), on the walls' own bodies (their faces' band, unjoined).
//! - **Floor:** a slab whose role is a floor (not a balcony's or a terrace's), no covering.
//! - **Roof** (Ek-3/B 4, docs/adr/8087): a roof (`roof`, 4-c: exposed to fire from outside) and a terrace's slab
//!   (`terrace`, 4-b: a roof serving as a floor), no covering.
//! - **Suspended ceiling** (Ek-3/B 11, docs/adr/8089): a covering of IFC type `CEILING` (asma tavan, `ceiling`).

use kentos_contracts::{BeamRole, Entity, SlabRole};
use kentos_geometry_core::vec2::Vec2;

use crate::Document;

/// An element Ek-3/B reads.
#[derive(Clone, Debug, PartialEq)]
pub struct FireElement {
    pub uid: String,
    /// `column`, `beam`, `wall`, `slab`, `roof`, `terrace` or `ceiling`.
    pub kind: &'static str,
    pub level: Option<String>,
    /// A column or a beam of the frame.
    pub frame: bool,
    /// A wall on its storey's outline.
    pub exterior: bool,
}

impl Document {
    /// The frame's columns and beams, the walls and the floors, in the drawing's order (docs/adr/8082).
    pub fn fire_elements(&self) -> Vec<FireElement> {
        let mut out: Vec<FireElement> = Vec::new();
        // Each storey's bodies for the outline: the walls' (and which element each is) and the columns'.
        let mut bodies: Vec<(Option<String>, Vec<Vec2>, Option<usize>)> = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let level = e.own_level().map(str::to_owned);
            match e {
                Entity::Column(c) => {
                    if let kentos_geometry_core::entity::Shape::Column { at, column } =
                        kentos_formats::arch_elements::core_column(&c.column)
                    {
                        bodies.push((level.clone(), kentos_geometry_core::geom::arch::column_box(at, &column), None));
                    }
                    if c.column.member.is_none() {
                        out.push(FireElement { uid: uid.to_string(), kind: "column", level, frame: true, exterior: false });
                    }
                }
                Entity::Beam(b) if b.beam.role == BeamRole::Beam => {
                    out.push(FireElement { uid: uid.to_string(), kind: "beam", level, frame: true, exterior: false });
                }
                Entity::Wall(w) => {
                    let k = out.len();
                    out.push(FireElement { uid: uid.to_string(), kind: "wall", level: level.clone(), frame: false, exterior: false });
                    if let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } =
                        kentos_formats::walls::core_wall(&w.wall)
                        && let Ok((outer, _)) = kentos_geometry_core::geom::wall::body(&pts, bulges.as_deref(), &wall)
                    {
                        bodies.push((level, outer, Some(k)));
                    }
                }
                Entity::Slab(s) if s.slab.covering.is_none() && s.slab.role == SlabRole::Floor => {
                    out.push(FireElement { uid: uid.to_string(), kind: "slab", level, frame: false, exterior: false });
                }
                Entity::Slab(s) if s.slab.covering.is_none() && s.slab.role == SlabRole::Terrace => {
                    out.push(FireElement { uid: uid.to_string(), kind: "terrace", level, frame: false, exterior: false });
                }
                Entity::Roof(_) => {
                    out.push(FireElement { uid: uid.to_string(), kind: "roof", level, frame: false, exterior: false });
                }
                Entity::Slab(s) if s.slab.covering.as_ref().is_some_and(|c| c.predefined == "CEILING") => {
                    out.push(FireElement { uid: uid.to_string(), kind: "ceiling", level, frame: false, exterior: false });
                }
                _ => {}
            }
        }
        let levels: std::collections::BTreeSet<Option<String>> = bodies.iter().map(|(l, ..)| l.clone()).collect();
        for level in levels {
            let here: Vec<&(Option<String>, Vec<Vec2>, Option<usize>)> =
                bodies.iter().filter(|(l, ..)| *l == level).collect();
            let rings: Vec<Vec<Vec2>> = here.iter().map(|(_, r, _)| r.clone()).collect();
            for i in kentos_geometry_core::geom::wall::exterior(&rings) {
                if let Some(k) = here[i].2 {
                    out[k].exterior = true;
                }
            }
        }
        out
    }
}
