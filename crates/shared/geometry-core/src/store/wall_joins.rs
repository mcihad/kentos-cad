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
            self.wall_whole.remove(&key);
        }
    }

    /// Object `id` put as `now`, or removed (none): a column where it was and where it is cuts the walls again and
    /// takes its look again (docs/adr/8041 §2); any other kind changes nothing here.
    pub(super) fn note_column(&mut self, id: f64, now: Option<&Shape>) {
        let key = id.to_bits();
        let foot = match now {
            Some(Shape::Column { at, column }) => crate::geom::column_join::Foot::of(*at, column),
            _ => None,
        };
        let was = self.column_feet.get(&key).map(|f| f.bounds);
        if was.is_none() && foot.is_none() {
            return;
        }
        self.wall_dirty.extend(was);
        match foot {
            Some(f) => {
                self.wall_dirty.push(f.bounds);
                self.column_feet.insert(key, Arc::new(f));
            }
            None => {
                self.column_feet.remove(&key);
                self.column_looks.remove(&key);
            }
        }
    }

    /// Wall `item`'s drawing `d` cut by the columns on its level that reach it (docs/adr/8041 §2); none when none does.
    fn cut_by_columns(&self, item: &super::Item, d: &crate::geom::wall::Drawn) -> Option<crate::geom::wall::Drawn> {
        let Shape::Wall { wall, .. } = &item.shape else {
            return None;
        };
        let level = wall_join::level(wall);
        let feet: Vec<&crate::geom::column_join::Foot> = self
            .column_feet
            .values()
            .filter(|f| f.level.as_deref() == level && touches(&f.bounds, &item.bounds, 0.0))
            .map(|f| &**f)
            .collect();
        crate::geom::column_join::cut(d, &feet)
    }

    /// The columns standing where the walls changed (`dirty`; `all`: every column) take their look again from the walls
    /// on their level they meet, and are expanded again (docs/adr/8041 §2).
    fn settle_columns(&mut self, dirty: &[Bounds], all: bool) {
        let grow = JOIN_TOLERANCE;
        let keys: Vec<u64> = self
            .column_feet
            .iter()
            .filter(|(_, f)| all || dirty.iter().any(|d| touches(&f.bounds, d, grow)))
            .map(|(k, _)| *k)
            .collect();
        if keys.is_empty() {
            return;
        }
        let mut looks = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(foot) = self.column_feet.get(&key).cloned() else {
                continue;
            };
            // The walls on its level it reaches, in the document's order: the first layer of its material looks.
            let mut walls: Vec<(u64, crate::geom::column_join::WallAt<'_>)> = self
                .wall_ids
                .iter()
                .filter_map(|w| {
                    let s = *self.by_id.get(w)?;
                    let it = self.slots[s as usize].as_ref()?;
                    let Shape::Wall { pts, wall, .. } = &it.shape else {
                        return None;
                    };
                    if wall_join::level(wall) != foot.level.as_deref()
                        || !touches(&it.bounds, &foot.bounds, 0.0)
                        || pts.len() < 2
                    {
                        return None;
                    }
                    let drawn = self.wall_whole.get(w)?;
                    Some((
                        it.order,
                        crate::geom::column_join::WallAt {
                            drawn,
                            spec: wall,
                            turn: crate::geom::wall::first_direction(pts),
                        },
                    ))
                })
                .collect();
            walls.sort_by_key(|w| w.0);
            let at: Vec<crate::geom::column_join::WallAt<'_>> = walls.into_iter().map(|w| w.1).collect();
            looks.push((key, crate::geom::column_join::joined(&foot, &at)));
        }
        for (key, look) in looks {
            match look {
                Some(l) => {
                    self.column_looks.insert(key, Arc::new(l));
                }
                None => {
                    self.column_looks.remove(&key);
                }
            }
            let Some(&slot) = self.by_id.get(&key) else {
                continue;
            };
            let fresh = self.slots[slot as usize]
                .as_ref()
                .map(|it| self.expansion_of(it.id, &it.shape));
            if let (Some(it), Some((expanded, bounds))) = (self.slots[slot as usize].as_mut(), fresh) {
                it.bounds = bounds;
                it.expanded = expanded;
                it.veiled = None;
            }
            self.loosen(slot);
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

    /// The objects of the design options this store does not show (docs/adr/0788 §4): their walls join no other
    /// wall and their openings cut no wall, so the options shown draw as if the others were not there. The walls
    /// whose state changed, and the walls of the openings whose state changed, are joined again.
    pub fn set_option_out(&mut self, ids: impl IntoIterator<Item = f64>) {
        let next: IdSet = ids.into_iter().map(f64::to_bits).collect();
        if next == self.option_out {
            return;
        }
        let changed: Vec<u64> = next
            .symmetric_difference(&self.option_out)
            .copied()
            .collect();
        self.option_out = next;
        for key in changed {
            let id = f64::from_bits(key);
            self.touch_wall(id);
            if let Some(&w) = self.opening_hosts.get(&key) {
                self.touch_wall(w);
            }
            // A wall left out is drawn on its own: joined again it would take its old joints.
            if self.option_out.contains(&key) {
                self.wall_drawn.remove(&key);
            }
        }
        self.maybe_rebuild();
    }

    /// Whether the store shows no option's objects as out.
    pub fn no_option_out(&self) -> bool {
        self.option_out.is_empty()
    }

    /// The Tadilat projesi view's marks (docs/adr/8039 §5); none: another view. Every wall and opening is pieced again.
    pub fn set_renovation(&mut self, marks: Option<super::Renovation>) {
        if marks == self.renovation {
            return;
        }
        self.renovation = marks;
        let slots: Vec<u32> = (0..self.slots.len() as u32)
            .filter(|&s| {
                matches!(&self.slots[s as usize], Some(it)
                    if matches!(it.shape, Shape::Wall { .. } | Shape::Opening { .. }))
            })
            .collect();
        for s in slots {
            let fresh = self.slots[s as usize]
                .as_ref()
                .map(|it| self.wall_expansion(it.id, &it.shape));
            if let (Some(it), Some((expanded, bounds))) = (self.slots[s as usize].as_mut(), fresh) {
                it.bounds = bounds;
                it.expanded = expanded;
                it.veiled = None;
            }
            self.loosen(s);
        }
        self.maybe_rebuild();
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
            if self.option_out.contains(&o) {
                continue;
            }
            let Some(it) = self.by_id.get(&o).and_then(|&s| self.slots[s as usize].as_ref()) else {
                continue;
            };
            if let Shape::Opening {
                at, width, opening, ..
            } = &it.shape
            {
                // Its hole and a pocket door's cavities (docs/adr/0777 §2), cut at the view's height (docs/adr/8012).
                by_wall
                    .entry(w.to_bits())
                    .or_default()
                    .extend(opening::holes_at(*at, *width, opening, self.wall_view.cut));
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
                // Tadilat projesi (docs/adr/8039 §5): mevcut a plain outline (no solid fill), ilave its body hatched.
                let pieces = match &self.renovation {
                    Some(r) => {
                        let plain = wall::Spec {
                            fill: None,
                            ..wall.clone()
                        };
                        let mut pieces = Vec::new();
                        if r.added(id)
                            && let Some(pattern) =
                                wall::component_hatch(super::RENOVATION_HATCH, 1.0, 0.0, view.scale)
                        {
                            for (ring, holes) in &d.fill {
                                pieces.push(super::renovation_piece(
                                    Shape::Hatch {
                                        ring: ring.clone(),
                                        holes: holes.clone(),
                                        pattern: pattern.clone(),
                                        assoc: None,
                                        boundary: None,
                                    },
                                    None,
                                ));
                            }
                        }
                        pieces.extend(wall::pieces_of(d, pts, &plain, view));
                        pieces
                    }
                    None => wall::pieces_of(d, pts, wall, view),
                };
                // At the view's line group (docs/adr/8013).
                let pieces = self.with_line_group(shape, pieces, view);
                self.expanded_of(pieces, b)
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
        // The walls of options not shown join nothing (docs/adr/0788 §4): drawn on their own.
        let (context, apart): (Vec<usize>, Vec<usize>) = if self.option_out.is_empty() {
            (context, Vec::new())
        } else {
            context
                .into_iter()
                .partition(|&i| !self.option_out.contains(&walls[i].0))
        };
        for &i in apart.iter().filter(|&&i| affected[i]) {
            let (key, slot, _, _) = walls[i];
            self.wall_drawn.remove(&key);
            self.wall_whole.remove(&key);
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
                    // Cut where the columns on its level stand (docs/adr/8041 §2); the whole kept for their looks.
                    let cut = self.slots[slot as usize]
                        .as_ref()
                        .and_then(|it| self.cut_by_columns(it, &d));
                    let whole = Arc::new(d);
                    self.wall_drawn.insert(key, cut.map_or_else(|| whole.clone(), Arc::new));
                    self.wall_whole.insert(key, whole);
                }
                None => {
                    self.wall_drawn.remove(&key);
                    self.wall_whole.remove(&key);
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
        self.settle_columns(&dirty, full);
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
