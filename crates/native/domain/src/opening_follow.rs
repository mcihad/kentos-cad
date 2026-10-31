//! Openings go with their walls (docs/adr/0341 §3). Before a step is recorded (`commit`, after the relations, before
//! the linked texts), every opening the step added or changed, and every opening of a wall it changed or removed or
//! of a wall an opening it changed or removed was on, is settled:
//!
//! - an opening whose jambs the step moved by itself (its place changed, its `at` and wall did not: Taşı, Döndür,
//!   Aynala, a grip) is read back from them: its `at` where their centre falls on its wall's path, its hand and side
//!   turned when the jambs run against the path or their left points fell on its right;
//! - its jambs are written from its wall and `at` (a wall moved, stretched or thickened takes its openings along);
//! - a wall that is gone or no wall breaks it (`hostGone`) where it stands; an interval past its wall's end or round
//!   a corner breaks it (`offWall`); a niche or flue its wall is no longer thick enough for breaks (`tooDeep`,
//!   docs/adr/0719 §3); one running into another opening of its wall breaks both (`overlap`); a break that no longer
//!   holds goes. It never moves to another wall by itself.
//!
//! The changes join the step: one undo takes them back with the rest. `scripts/fixtures/opening_command_cases.py`
//! writes the same without KentOS's code.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use kentos_contracts::{Entity, OpeningBreak, OpeningFields, Vec2};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::opening::{self, Anchor};
use kentos_geometry_core::geom::wall::Spec as WallSpec;
use kentos_geometry_core::ops::path::Path;

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// A wall's base line and spec as the core reads them.
pub struct WallLine {
    pub path: Path,
    pub spec: WallSpec,
}

/// The core's reading of a wall; none for another object or one it cannot draw.
pub fn wall_line(e: &Entity) -> Option<WallLine> {
    if !matches!(e, Entity::Wall(_)) {
        return None;
    }
    let json = serde_json::to_string(e).ok()?;
    let Shape::Wall { pts, bulges, wall } =
        CoreEntity::from_json(&Json::parse(&json).ok()?).ok()?.shape
    else {
        return None;
    };
    Some(WallLine {
        path: opening::wall_path(&pts, bulges.as_deref(), wall.closed)?,
        spec: wall,
    })
}

/// The core's anchor of an opening.
pub fn anchor(o: &OpeningFields) -> Anchor {
    match o.anchor {
        kentos_contracts::OpeningAnchor::Start => Anchor::Start,
        kentos_contracts::OpeningAnchor::Center => Anchor::Center,
        kentos_contracts::OpeningAnchor::End => Anchor::End,
    }
}

/// Its interval along its wall's path.
pub fn span(o: &OpeningFields) -> (f64, f64) {
    opening::span(o.at, anchor(o), o.width)
}

/// Its interval along its wall with a pocket door's pockets (docs/adr/0777 §2): what must lie on one edge of its wall and
/// what no other opening may run into.
pub fn reach(o: &OpeningFields) -> (f64, f64) {
    let (mut s0, mut s1) = span(o);
    for p in kentos_contracts::opening_sizes(o).pockets {
        if p.at_end {
            s1 += p.length;
        } else {
            s0 -= p.length;
        }
    }
    (s0, s1)
}

/// Whether `o` is a pocket door (docs/adr/0777 §2).
pub fn is_pocket(o: &OpeningFields) -> bool {
    o.form == kentos_contracts::OpeningForm::Door && o.operation == kentos_contracts::OpeningOperation::Pocket
}

fn contract(p: kentos_geometry_core::Vec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// `o` on `wall` (none: its wall is gone or no wall): its jambs written from its `at`, or first read back from its
/// jambs when `moved`; its break, short of an overlap, the other openings' business.
pub fn placed_on(o: &OpeningFields, wall: Option<&WallLine>, moved: bool) -> OpeningFields {
    let mut out = o.clone();
    let Some(w) = wall else {
        out.broken = Some(OpeningBreak::HostGone);
        return out;
    };
    if moved {
        let core: Vec<kentos_geometry_core::Vec2> = o
            .place
            .iter()
            .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
            .collect();
        if let Some(r) = opening::read_back(&w.path, &core, anchor(o), o.width) {
            out.at = r.at;
            if r.hand_turned {
                out.hand = o.hand.other();
            }
            if r.side_turned {
                out.side = o.side.other();
            }
        }
    }
    let (s0, s1) = span(&out);
    match opening::jambs(&w.path, &w.spec, s0, s1) {
        Some(j) => {
            out.place = j.into_iter().map(contract).collect();
            out.broken = (!out.fits(w.spec.width)).then_some(OpeningBreak::TooDeep);
            // A pocket door's pockets: its wall thick enough for them, and they on its edge (docs/adr/0777 §2).
            if out.broken.is_none() && is_pocket(&out) {
                let tol = 1e-9 * w.spec.width.abs().max(1.0);
                let (r0, r1) = reach(&out);
                if out.pocket_wall().is_some_and(|need| need > w.spec.width + tol) {
                    out.broken = Some(OpeningBreak::TooDeep);
                } else if opening::jambs(&w.path, &w.spec, r0, r1).is_none() {
                    out.broken = Some(OpeningBreak::OffWall);
                }
            }
        }
        None => out.broken = Some(OpeningBreak::OffWall),
    }
    out
}

/// Whether two intervals along a wall run into each other (touching is not).
pub fn overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    let tol = opening::SPAN_TOLERANCE * a.1.abs().max(b.1.abs()).max(1.0);
    a.0 < b.1 - tol && b.0 < a.1 - tol
}

/// An opening's slot, its interval along its wall and its fields (whose heights and cut say whether it clashes).
type Spanned = (Slot, (f64, f64), OpeningFields);

fn updated(before: Stored, after: Entity) -> Op {
    Op::Update {
        after: Stored {
            uid: before.uid,
            entity: Arc::new(after),
        },
        before,
    }
}

fn host_of(e: &Entity) -> Option<Uuid> {
    match e {
        Entity::Opening(o) => Uuid::parse_str(&o.opening.host).ok(),
        _ => None,
    }
}

impl Document {
    /// The wall with persistent id `host` as the core reads it; none when it is gone, no wall or cannot be drawn.
    pub fn wall_line_of(&self, host: &str) -> Option<WallLine> {
        let uid = Uuid::parse_str(host).ok()?;
        let s = self.store.get(self.store.slot_of(uid)?)?;
        wall_line(&s.entity)
    }

    /// The openings on the wall with persistent id `host`, in slot order.
    pub fn openings_of(&self, host: &str) -> Vec<(Slot, OpeningFields)> {
        let Ok(uid) = Uuid::parse_str(host) else {
            return Vec::new();
        };
        let mut out: Vec<(Slot, OpeningFields)> = self
            .store
            .tied_to(uid)
            .filter_map(|slot| match &*self.store.get(slot)?.entity {
                Entity::Opening(o) if Uuid::parse_str(&o.opening.host).ok() == Some(uid) => {
                    Some((slot, o.opening.clone()))
                }
                _ => None,
            })
            .collect();
        out.sort_by_key(|(s, _)| *s);
        out
    }

    /// What keeps the openings with their walls after `ops`, applied: the step takes them in.
    pub(crate) fn follow_openings(&mut self, ops: &[Op]) -> Vec<Op> {
        let mut openings: BTreeSet<Uuid> = BTreeSet::new();
        let mut moved: BTreeSet<Uuid> = BTreeSet::new();
        let mut hosts: BTreeSet<Uuid> = BTreeSet::new();
        for op in ops {
            match op {
                Op::Add(s) => {
                    if let Some(h) = host_of(&s.entity) {
                        openings.insert(s.uid);
                        hosts.insert(h);
                    }
                }
                Op::Update { before, after } => match (&*before.entity, &*after.entity) {
                    (Entity::Opening(b), Entity::Opening(a)) => {
                        openings.insert(after.uid);
                        hosts.extend(host_of(&before.entity));
                        hosts.extend(host_of(&after.entity));
                        if a.opening.place != b.opening.place
                            && a.opening.at == b.opening.at
                            && a.opening.host == b.opening.host
                        {
                            moved.insert(after.uid);
                        }
                    }
                    _ => {
                        hosts.insert(after.uid);
                        if let Some(h) = host_of(&after.entity) {
                            openings.insert(after.uid);
                            hosts.insert(h);
                        }
                    }
                },
                Op::Remove(s) => match host_of(&s.entity) {
                    Some(h) => {
                        hosts.insert(h);
                    }
                    None => {
                        hosts.insert(s.uid);
                    }
                },
                // Objects into or out of a design option (docs/adr/0788 §4): their openings may clash or not anew.
                Op::ArchOptions { before, after } => {
                    for uid in crate::arch_options::touched_members(before, after) {
                        let entity = self.store.slot_of(uid).and_then(|s| self.store.get(s)).map(|s| s.entity.clone());
                        match entity.as_deref().and_then(host_of) {
                            Some(h) => {
                                openings.insert(uid);
                                hosts.insert(h);
                            }
                            None => {
                                hosts.insert(uid);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for h in &hosts {
            for slot in self.store.tied_to(*h).collect::<Vec<_>>() {
                if let Some(s) = self.store.get(slot)
                    && matches!(*s.entity, Entity::Opening(_))
                {
                    openings.insert(s.uid);
                }
            }
        }
        if openings.is_empty() {
            return Vec::new();
        }
        // Each one placed on its wall; then the overlaps on each wall among those that are on it.
        let mut next: BTreeMap<Slot, (Stored, OpeningFields)> = BTreeMap::new();
        for uid in &openings {
            let Some(slot) = self.store.slot_of(*uid) else {
                continue;
            };
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Opening(o) = &*current.entity else {
                continue;
            };
            let wall = self.wall_line_of(&o.opening.host);
            let fields = placed_on(&o.opening, wall.as_ref(), moved.contains(uid));
            next.insert(slot, (current, fields));
        }
        let mut by_host: BTreeMap<String, Vec<Spanned>> = BTreeMap::new();
        for (slot, (_, f)) in &next {
            if f.broken.is_none() {
                by_host
                    .entry(f.host.clone())
                    .or_default()
                    .push((*slot, reach(f), f.clone()));
            }
        }
        // The other openings of those walls, as they stand.
        for (host, list) in by_host.iter_mut() {
            for (slot, o) in self.openings_of(host) {
                if !next.contains_key(&slot) && o.broken.is_none() {
                    list.push((slot, reach(&o), o));
                }
            }
        }
        // Two clash where they run into each other and their heights meet or plan would cut both (docs/adr/0719 §3).
        let cut = kentos_contracts::PLAN_CUT_HEIGHT;
        // Two options of one set never clash (docs/adr/0788 §4): only one of them is ever built.
        let groups = self.option_groups();
        for list in by_host.values() {
            for (i, (slot, a, fa)) in list.iter().enumerate() {
                if list.iter().enumerate().any(|(j, (other, b, fb))| {
                    i != j
                        && overlap(*a, *b)
                        && fa.clashes_with(fb, cut)
                        && crate::arch_options::options_meet(&groups, *slot, *other)
                })
                    && let Some((_, f)) = next.get_mut(slot)
                {
                    f.broken = Some(OpeningBreak::Overlap);
                }
            }
        }
        let mut out = Vec::new();
        for (current, fields) in next.into_values() {
            let Entity::Opening(o) = &*current.entity else {
                continue;
            };
            if o.opening != fields {
                let mut e = o.clone();
                e.opening = fields;
                let op = updated(current, Entity::Opening(e));
                self.apply_op(&op);
                out.push(op);
            }
        }
        out
    }
}
