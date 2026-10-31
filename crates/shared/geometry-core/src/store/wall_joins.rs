//! Walls joined in the store (docs/adr/0340 §3): a wall put or removed marks where it was and where it is; before the
//! next query is answered (`maybe_rebuild`) the walls there are joined again (`geom::wall_join::solve`) among those
//! touching them, and only they are expanded anew. Each wall's drawing as its joins left it is kept, so a change of the
//! view pieces it again without joining. Many changes at once (opening a drawing) join every wall in one pass. The
//! walls go to the solver in the document's order, so the result is the same however they arrived.

use std::sync::Arc;

use super::{Expanded, IdMap, IdSet, Store};
use crate::entity::{Shape, entity_bounds_in};
use crate::geom::wall;
use crate::geom::wall_join::{self, IssueCode, JOIN_TOLERANCE, WallIn};
use crate::geom::opening;
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

    /// The walls the openings named are in (docs/adr/0341 §2), by their ids; none: the opening is in no wall (its wall
    /// gone). The host sends them after it puts the openings; the walls an opening leaves and goes into are joined
    /// again, the opening's gap with them.
    pub fn set_hosts(&mut self, entries: impl IntoIterator<Item = (f64, Option<f64>)>) {
        for (id, host) in entries {
            let key = id.to_bits();
            let old = match host {
                Some(h) => {
                    self.opening_loose.remove(&key);
                    self.opening_hosts.insert(key, h)
                }
                None => {
                    self.opening_loose.insert(key);
                    self.opening_hosts.remove(&key)
                }
            };
            for wall in old.into_iter().chain(host) {
                self.touch_wall(wall);
            }
        }
        self.maybe_rebuild();
    }

    /// An opening gone: its wall is joined again without its gap.
    pub(super) fn forget_opening(&mut self, id: f64) {
        self.opening_loose.remove(&id.to_bits());
        if let Some(wall) = self.opening_hosts.remove(&id.to_bits()) {
            self.touch_wall(wall);
        }
    }

    /// A wall to draw again, whatever changed about its openings.
    fn touch_wall(&mut self, wall: f64) {
        let key = wall.to_bits();
        if !self.wall_ids.contains(&key) {
            return;
        }
        if let Some(it) = self.by_id.get(&key).and_then(|&s| self.slots[s as usize].as_ref()) {
            self.wall_dirty.push(it.bounds);
        }
    }

    /// Each wall's openings' holes (docs/adr/0719 §3: none for one plan does not cut), for the walls of `keys` in
    /// their order.
    fn intervals(&self, keys: &[u64]) -> Vec<Vec<opening::Hole>> {
        let mut by_wall: IdMap<Vec<opening::Hole>> = IdMap::default();
        for (&o, &w) in &self.opening_hosts {
            let Some(it) = self.by_id.get(&o).and_then(|&s| self.slots[s as usize].as_ref()) else {
                continue;
            };
            if let Shape::Opening {
                at, width, opening, ..
            } = &it.shape
            {
                // Its hole and a pocket door's cavities (docs/adr/0777 §2).
                by_wall
                    .entry(w.to_bits())
                    .or_default()
                    .extend(opening::holes(*at, *width, opening));
            }
        }
        keys.iter()
            .map(|k| {
                let mut v = by_wall.remove(k).unwrap_or_default();
                v.sort_by(|a, b| a.s0.total_cmp(&b.s0));
                v
            })
            .collect()
    }

    /// A wall's pieces at the view from its joined drawing; one not yet joined (or that cannot be drawn) as on its own.
    pub(super) fn wall_expansion(&self, id: f64, shape: &Shape) -> (Option<Arc<Expanded>>, Bounds) {
        self.wall_expansion_in(id, shape, self.wall_view)
    }

    /// `wall_expansion` at `view`: the same joined drawing pieced at another detail (docs/adr/0696's 29 October
    /// addendum).
    pub(super) fn wall_expansion_in(
        &self,
        id: f64,
        shape: &Shape,
        view: wall::View,
    ) -> (Option<Arc<Expanded>>, Bounds) {
        match (shape, self.wall_drawn.get(&id.to_bits())) {
            (Shape::Wall { pts, wall, .. }, Some(d)) => {
                let b = entity_bounds_in(shape, self.font);
                self.expanded_of(wall::pieces_of(d, pts, wall, view), b)
            }
            _ => self.expansion_of_in(id, shape, view),
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
            let keys: Vec<u64> = context.iter().map(|&i| walls[i].0).collect();
            wall_join::solve_with(&ins, &self.intervals(&keys), Some(&only))
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

    /// Where the openings in no wall are (their jambs' centres): each shows a warning (docs/adr/0341 §1).
    pub fn loose_openings(&self) -> Vec<Vec2> {
        self.opening_loose
            .iter()
            .filter_map(|k| self.by_id.get(k).and_then(|&s| self.slots[s as usize].as_ref()))
            .filter_map(|it| match &it.shape {
                Shape::Opening { place, .. } => opening::centre(place),
                _ => None,
            })
            .collect()
    }
}
