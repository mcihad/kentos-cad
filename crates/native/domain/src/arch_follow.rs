//! Columns go with their grids (docs/adr/0343 §3). Before a step is recorded (`commit`, after the spaces), every column
//! tied to two grids that the step added or changed, or whose grid the step added, changed or removed, is settled: its
//! centre where its grids cross (lines unbounded, arcs as whole circles; the crossing nearest where it was), its place
//! from their labels (“B-3”: a lettered grid first, else the order given). A grid gone or no grid, or grids that do not
//! cross, break it (`gridGone`) where it stands; a break that no longer holds goes. A column with no grids has neither
//! place nor break.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/arch_command_cases.py` writes
//! the same without KentOS's code.

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_contracts::{ColumnBreak, ColumnFields, Entity, GridFields, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::arch;
use kentos_geometry_core::geom::intersect::Edge;

use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;
use crate::store::Stored;

/// A grid's path as the core reads it; none for a grid it cannot draw.
pub fn grid_edge(g: &GridFields) -> Option<Edge> {
    if g.problem().is_some() {
        return None;
    }
    let pts: Vec<CoreVec2> = g.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect();
    arch::grid_edge(&pts, g.bulges.as_deref())
}

/// Where two grids cross nearest `near` (docs/adr/0343 §3); none: they do not.
pub fn grid_cross(a: &GridFields, b: &GridFields, near: Vec2) -> Option<Vec2> {
    let p = arch::grid_cross(
        &grid_edge(a)?,
        &grid_edge(b)?,
        CoreVec2::new(near.x, near.y),
    )?;
    Some(Vec2 { x: p.x, y: p.y })
}

/// A column's place from its grids' labels: a lettered one first, else the order given.
pub fn place_of(a: &str, b: &str) -> String {
    let lettered = |t: &str| t.chars().next().is_some_and(char::is_alphabetic);
    if !lettered(a) && lettered(b) {
        format!("{b}-{a}")
    } else {
        format!("{a}-{b}")
    }
}

fn updated(before: Stored, after: Entity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(after),
        },
        before,
    }
}

impl Document {
    /// The grid with persistent id `uid`; none: no such object or no grid.
    pub fn grid_of(&self, uid: &str) -> Option<GridFields> {
        let slot = self.store.slot_of(Uuid::parse_str(uid).ok()?)?;
        match &*self.store.get(slot)?.entity {
            Entity::Grid(g) => Some(g.grid.clone()),
            _ => None,
        }
    }

    /// A column settled on its grids as the document holds them.
    pub fn column_settled(&self, c: &ColumnFields) -> ColumnFields {
        let mut out = c.clone();
        let Some(ids) = c.grids.as_ref().filter(|g| g.len() == 2) else {
            out.place = None;
            out.broken = None;
            return out;
        };
        let found = self.grid_of(&ids[0]).zip(self.grid_of(&ids[1]));
        match found.and_then(|(a, b)| grid_cross(&a, &b, c.at).map(|p| (p, a, b))) {
            Some((p, a, b)) => {
                out.at = p;
                out.place = Some(place_of(&a.label, &b.label));
                out.broken = None;
            }
            None => out.broken = Some(ColumnBreak::GridGone),
        }
        out
    }

    /// What keeps the columns with their grids after `ops`, applied: the step takes them in.
    pub(crate) fn follow_columns(&mut self, ops: &[Op]) -> Vec<Op> {
        let mut columns: BTreeSet<Uuid> = BTreeSet::new();
        let mut grids: BTreeSet<Uuid> = BTreeSet::new();
        for op in ops {
            let (Op::Add(s) | Op::Remove(s) | Op::Update { after: s, .. }) = op else {
                continue;
            };
            match &*s.entity {
                Entity::Column(c) if !matches!(op, Op::Remove(_)) => {
                    if c.column.grids.is_some()
                        || c.column.place.is_some()
                        || c.column.broken.is_some()
                    {
                        columns.insert(s.uid);
                    }
                }
                Entity::Grid(_) => {
                    grids.insert(s.uid);
                }
                _ => {}
            }
            if let Op::Update { before, .. } = op
                && matches!(*before.entity, Entity::Grid(_))
            {
                grids.insert(before.uid);
            }
        }
        for g in &grids {
            for slot in self.store.tied_to(*g).collect::<Vec<_>>() {
                if let Some(s) = self.store.get(slot)
                    && matches!(*s.entity, Entity::Column(_))
                {
                    columns.insert(s.uid);
                }
            }
        }
        let mut out = Vec::new();
        for uid in columns {
            let Some(slot) = self.store.slot_of(uid) else {
                continue;
            };
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Column(c) = &*current.entity else {
                continue;
            };
            let settled = self.column_settled(&c.column);
            if settled != c.column {
                let mut e = c.clone();
                e.column = settled;
                let op = updated(current, Entity::Column(e));
                self.apply_op(&op);
                out.push(op);
            }
        }
        out
    }
}
