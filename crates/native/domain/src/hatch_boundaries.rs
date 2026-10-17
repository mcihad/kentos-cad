//! A hatch's exact boundary and the ring it is drawn by (docs/adr/0328 §1).
//! Before a step is recorded, first of what follows it (`follow_into`), each
//! hatch the step added or changed is settled:
//!
//! - a boundary new or changed by the step draws the ring and holes (arcs
//!   and curves as chords); a boundary whose curves do not meet their loop's
//!   points is dropped and the ring stays as written;
//! - a ring or holes the step changed itself, the boundary as it was (a
//!   grip, the vertex table), drops the boundary: the drawn ring is all the
//!   hatch has now.
//!
//! The changes join the step: one undo takes them back with the rest.

use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_contracts::{Entity, HatchEntity, HatchLoop, Vec2, boundary_problem};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::HatchLoop as CoreLoop;
use kentos_geometry_core::geom::hatch_boundary;

use crate::document::Document;
use crate::history::Op;
use crate::identity::Slot;
use crate::store::Stored;

/// The boundary as the geometry core reads it (the contract's own JSON).
fn core_loops(b: &[HatchLoop]) -> Option<Vec<CoreLoop>> {
    let text = serde_json::to_string(b).ok()?;
    Vec::<CoreLoop>::from_json(&Json::parse(&text).ok()?).ok()
}

/// A hatch's drawn ring and holes.
pub type Drawn = (Vec<Vec2>, Option<Vec<Vec<Vec2>>>);

/// The ring and holes `b` draws, or none when it does not hold.
pub fn drawn(b: &[HatchLoop]) -> Option<Drawn> {
    if boundary_problem(b).is_some() {
        return None;
    }
    let loops = core_loops(b)?;
    if hatch_boundary::problem(&loops).is_some() {
        return None;
    }
    let (ring, holes) = hatch_boundary::rings(&loops);
    let back = |pts: &[kentos_geometry_core::Vec2]| -> Vec<Vec2> {
        pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect()
    };
    Some((
        back(&ring),
        (!holes.is_empty()).then(|| holes.iter().map(|h| back(h)).collect()),
    ))
}

/// The hatch settled against what it was before the step (none: added in it).
fn settled(before: Option<&HatchEntity>, now: &HatchEntity) -> Option<HatchEntity> {
    let b = now.boundary.as_ref()?;
    let boundary_new = before.is_none_or(|w| w.boundary.as_ref() != Some(b));
    if boundary_new {
        return match drawn(b) {
            Some((ring, holes)) if ring != now.ring || holes != now.holes => Some(HatchEntity {
                ring,
                holes,
                ..now.clone()
            }),
            Some(_) => None,
            None => Some(HatchEntity {
                boundary: None,
                ..now.clone()
            }),
        };
    }
    let was = before?;
    (was.ring != now.ring || was.holes != now.holes).then(|| HatchEntity {
        boundary: None,
        ..now.clone()
    })
}

impl Document {
    /// What settles the hatches `ops` (a step about to be recorded) added or
    /// changed, applied: the step takes it in.
    pub(crate) fn settle_boundaries(&mut self, ops: &[Op]) -> Vec<Op> {
        // Each hatch the step touched, with what it was before the step.
        let mut first: BTreeMap<Slot, Option<Arc<Entity>>> = BTreeMap::new();
        for op in ops {
            match op {
                Op::Add(s) if matches!(*s.entity, Entity::Hatch(_)) => {
                    first.entry(s.slot()).or_insert(None);
                }
                Op::Update { before, after } if matches!(*after.entity, Entity::Hatch(_)) => {
                    first
                        .entry(after.slot())
                        .or_insert_with(|| Some(Arc::clone(&before.entity)));
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for (slot, before) in first {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Hatch(now) = &*current.entity else {
                continue;
            };
            let was = before.as_deref().and_then(|e| match e {
                Entity::Hatch(h) => Some(h),
                _ => None,
            });
            if let Some(next) = settled(was, now) {
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Hatch(next)),
                    },
                    before: current,
                });
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
