//! Walls joined in the store (docs/adr/0340 §3): a wall put or removed marks where it was and where it is; before the
//! next query is answered (`maybe_rebuild`) the walls there are joined again (`geom::wall_join::solve`) among those
//! touching them, and only they are expanded anew. Each wall's drawing as its joins left it is kept, so a change of the
//! view pieces it again without joining. Many changes at once (opening a drawing) join every wall in one pass. The
//! walls go to the solver in the document's order, so the result is the same however they arrived.

use std::sync::Arc;

use super::{Expanded, IdSet, Store};
use crate::entity::{Shape, entity_bounds_in};
use crate::geom::wall;
use crate::geom::wall_join::{self, IssueCode, JOIN_TOLERANCE, WallIn};
use crate::geometry::Bounds;
use crate::vec2::Vec2;

/// Walls that meet and are not joined: why, where, and the walls by their ids.
#[derive(Clone, Debug, PartialEq)]
pub struct WallIssue {
    pub code: IssueCode,
    pub at: Vec2,
    pub walls: Vec<f64>,
}

/// More changed places than this, or more walls they touch: every wall is joined again.
const FULL_AFTER: usize = 64;

fn touches(a: &Bounds, b: &Bounds, grow: f64) -> bool {
    a.min_x <= b.max_x + grow
        && b.min_x <= a.max_x + grow
        && a.min_y <= b.max_y + grow
        && b.min_y <= a.max_y + grow
}

impl Store {
    /// Wall `id` put with its new box, or (`now` none) put as another kind or removed: where it was and where it is
    /// are to be joined again.
    pub(super) fn note_wall(&mut self, id: f64, now: Option<Bounds>) {
        let key = id.to_bits();
        let was = self
            .by_id
            .get(&key)
            .and_then(|&s| self.slots[s as usize].as_ref())
            .filter(|it| matches!(it.shape, Shape::Wall { .. }))
            .map(|it| it.bounds);
        if was.is_none() && now.is_none() {
            return;
        }
        self.wall_dirty.extend(was);
        self.wall_dirty.extend(now);
        if now.is_some() {
            self.wall_ids.insert(key);
        } else {
            self.wall_ids.remove(&key);
            self.wall_drawn.remove(&key);
        }
    }

    /// A wall's pieces at the view from its joined drawing; one not yet joined (or that cannot be drawn) as on its own.
    pub(super) fn wall_expansion(&self, id: f64, shape: &Shape) -> (Option<Arc<Expanded>>, Bounds) {
        match (shape, self.wall_drawn.get(&id.to_bits())) {
            (Shape::Wall { pts, wall, .. }, Some(d)) => {
                let b = entity_bounds_in(shape, self.font);
                self.expanded_of(wall::pieces_of(d, pts, wall, self.wall_view), b)
            }
            _ => self.expansion(shape),
        }
    }

    /// The walls where walls changed joined again.
    pub(super) fn settle_walls(&mut self) {
        if self.wall_dirty.is_empty() {
            return;
        }
        let dirty = std::mem::take(&mut self.wall_dirty);
        let mut walls: Vec<(u64, u32, Bounds, u64)> = self
            .wall_ids
            .iter()
            .filter_map(|k| {
                let s = *self.by_id.get(k)?;
                let it = self.slots[s as usize].as_ref()?;
                Some((*k, s, it.bounds, it.order))
            })
            .collect();
        walls.sort_by_key(|w| w.3);
        let grow = JOIN_TOLERANCE;
        let n = walls.len();
        let mut affected = vec![false; n];
        let mut full = dirty.len() > FULL_AFTER;
        if !full {
            let mut count = 0;
            for (i, w) in walls.iter().enumerate() {
                if dirty.iter().any(|d| touches(&w.2, d, grow)) {
                    affected[i] = true;
                    count += 1;
                }
            }
            full = count > FULL_AFTER;
        }
        let context: Vec<usize> = if full {
            affected.fill(true);
            (0..n).collect()
        } else {
            let hit: Vec<usize> = (0..n).filter(|&i| affected[i]).collect();
            (0..n)
                .filter(|&i| {
                    affected[i] || hit.iter().any(|&a| touches(&walls[i].2, &walls[a].2, grow))
                })
                .collect()
        };
        let solved = {
            let ins: Vec<WallIn<'_>> = context
                .iter()
                .filter_map(
                    |&i| match &self.slots[walls[i].1 as usize].as_ref()?.shape {
                        Shape::Wall { pts, bulges, wall } => Some(WallIn {
                            pts,
                            bulges: bulges.as_deref(),
                            spec: wall,
                        }),
                        _ => None,
                    },
                )
                .collect();
            if ins.len() != context.len() {
                return;
            }
            let only: Vec<usize> = (0..context.len())
                .filter(|&p| affected[context[p]])
                .collect();
            wall_join::solve(&ins, Some(&only))
        };
        let affected_keys: IdSet = (0..n)
            .filter(|&i| affected[i])
            .map(|i| walls[i].0)
            .collect();
        let ids = &self.wall_ids;
        self.wall_issues.retain(|x| {
            x.walls
                .iter()
                .all(|w| ids.contains(&w.to_bits()) && !affected_keys.contains(&w.to_bits()))
        });
        for issue in solved.issues {
            let ws: Vec<f64> = issue
                .walls
                .iter()
                .map(|&p| f64::from_bits(walls[context[p]].0))
                .collect();
            let mine = ws.iter().any(|w| affected_keys.contains(&w.to_bits()));
            let seen = self
                .wall_issues
                .iter()
                .any(|x| x.code == issue.code && x.at == issue.at);
            if mine && !seen {
                self.wall_issues.push(WallIssue {
                    code: issue.code,
                    at: issue.at,
                    walls: ws,
                });
            }
        }
        for (p, d) in solved.drawn.into_iter().enumerate() {
            let i = context[p];
            if !affected[i] {
                continue;
            }
            let (key, slot, _, _) = walls[i];
            match d {
                Some(d) => {
                    self.wall_drawn.insert(key, Arc::new(d));
                }
                None => {
                    self.wall_drawn.remove(&key);
                }
            }
            let fresh = self.slots[slot as usize]
                .as_ref()
                .map(|it| self.wall_expansion(it.id, &it.shape));
            if let (Some(it), Some((expanded, bounds))) =
                (self.slots[slot as usize].as_mut(), fresh)
            {
                it.bounds = bounds;
                it.expanded = expanded;
                it.veiled = None;
            }
            self.loosen(slot);
        }
    }

    /// Walls that meet and are not joined (docs/adr/0340 §2), as of the last change.
    pub fn wall_issues(&self) -> &[WallIssue] {
        &self.wall_issues
    }
}
