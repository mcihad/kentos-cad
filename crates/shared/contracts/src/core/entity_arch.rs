//! An object's architectural state (docs/adr/0346, 0715, 8039): its phase and how a phase view draws and marks it, its
//! level and its architectural information, read from the kinds that keep them.

use crate::Entity;

impl Entity {
    /// Its own phase (docs/adr/0346 §2): a wall's, a space's, a grid's, a column's, a stair's, a ramp's or a block
    /// insert's (docs/adr/0733 §1).
    pub fn own_phase(&self) -> Option<crate::ArchPhase> {
        match self {
            Entity::Wall(x) => x.wall.arch.phase,
            Entity::Space(x) => x.space.arch.phase,
            Entity::Grid(x) => x.grid.arch.phase,
            Entity::Column(x) => x.column.arch.phase,
            Entity::Beam(x) => x.beam.arch.phase,
            Entity::Roof(x) => x.roof.arch.phase,
            Entity::Railing(x) => x.railing.arch.phase,
            Entity::Stair(x) => x.stair.arch.phase,
            Entity::Ramp(x) => x.ramp.arch.phase,
            Entity::Insert(x) => x.arch.phase,
            Entity::Opening(x) => x.opening.phase,
            // A slab's and a room separator's (docs/adr/8039 §4: they were left out, a slab İptal was drawn in Yeni durum).
            Entity::Slab(x) => x.slab.arch.phase,
            Entity::Mass(x) => x.mass.arch.phase,
            Entity::Polyline(p) => p.space_separator.as_ref().and_then(|a| a.phase),
            _ => None,
        }
    }

    /// Its phase as a phase view reads it (docs/adr/0346 §2): its own, an opening without one its wall's
    /// (docs/adr/8039 §1), a tag's its object's (`by_uid` finds an object by its persistent id).
    pub fn phase<'a>(
        &'a self,
        by_uid: impl Fn(&str) -> Option<&'a Entity>,
    ) -> Option<crate::ArchPhase> {
        let opening = |o: &crate::OpeningEntity| {
            o.opening
                .phase
                .or_else(|| by_uid(&o.opening.host).and_then(|w| w.own_phase()))
        };
        match self {
            Entity::Opening(o) => opening(o),
            Entity::Tag(t) => by_uid(&t.tag.of).and_then(|e| match e {
                Entity::Opening(o) => opening(o),
                other => other.own_phase(),
            }),
            other => other.own_phase(),
        }
    }

    /// How `view` draws it (docs/adr/0346 §2): its tadilat durumu's look, except that Tadilat projesi leaves out a room
    /// pulled down (İptal) and its label (docs/adr/8039 §6): the plan names the rooms as they will be, its walls and
    /// openings carry the Mimarlar Odası's three techniques; drawn too, the old rooms' names and areas lay over the new
    /// ones' in the same place.
    pub fn look_in<'a>(
        &'a self,
        view: crate::PhaseView,
        by_uid: impl Fn(&str) -> Option<&'a Entity>,
    ) -> crate::PhaseLook {
        let phase = self.phase(&by_uid);
        if view == crate::PhaseView::Renovation && phase == Some(crate::ArchPhase::Demolish) {
            let room = match self {
                Entity::Space(_) => true,
                Entity::Tag(t) => matches!(by_uid(&t.tag.of), Some(Entity::Space(_))),
                _ => false,
            };
            if room {
                return crate::PhaseLook::Hidden;
            }
        }
        view.look(phase)
    }

    /// How the Tadilat projesi view marks it (docs/adr/8039 §5): a door or window of its own state other than its wall's
    /// is that wall's infill (closed up, iptal) or cut into it (ilave); otherwise by its state, none a plain outline.
    pub fn renovation_mark<'a>(
        &'a self,
        by_uid: impl Fn(&str) -> Option<&'a Entity>,
    ) -> crate::RenovationMark {
        use crate::{ArchPhase as P, RenovationMark as M};
        if let Entity::Opening(o) = self {
            let wall = by_uid(&o.opening.host).and_then(|w| w.own_phase());
            match o.opening.phase {
                Some(P::Demolish) if wall != Some(P::Demolish) => return M::Infill,
                Some(P::New) if wall != Some(P::New) => return M::CutIn,
                _ => {}
            }
        }
        match self.phase(&by_uid) {
            None | Some(P::Existing) => M::Plain,
            Some(P::New) => M::Added,
            Some(P::Demolish | P::Temporary) => M::Removed,
        }
    }

    /// Its own level (docs/adr/0715): a wall's, a space's, a grid's, a column's, a stair's, a ramp's, a room
    /// separator's or a block insert's (docs/adr/0733 §1).
    pub fn own_level(&self) -> Option<&str> {
        match self {
            Entity::Insert(x) => x.arch.level.as_deref(),
            Entity::Wall(x) => x.wall.arch.level.as_deref(),
            Entity::Space(x) => x.space.arch.level.as_deref(),
            Entity::Slab(x) => x.slab.arch.level.as_deref(),
            Entity::Mass(x) => x.mass.arch.level.as_deref(),
            Entity::Grid(x) => x.grid.arch.level.as_deref(),
            Entity::Column(x) => x.column.arch.level.as_deref(),
            Entity::Beam(x) => x.beam.arch.level.as_deref(),
            Entity::Roof(x) => x.roof.arch.level.as_deref(),
            Entity::Railing(x) => x.railing.arch.level.as_deref(),
            Entity::Stair(x) => x.stair.arch.level.as_deref(),
            Entity::Ramp(x) => x.ramp.arch.level.as_deref(),
            Entity::Polyline(p) => p.space_separator.as_ref().and_then(|a| a.level.as_deref()),
            _ => None,
        }
    }

    /// Its architectural information (level, phase, status, discipline, fire resistance, exit role): a wall's, a
    /// space's, a slab's, a grid's, a column's, a beam's, a roof's, a railing's, a stair's, a ramp's, a room
    /// separator's or a block insert's. An opening keeps its own fields (`OpeningFields`).
    pub fn arch_info(&self) -> Option<&crate::ArchInfo> {
        match self {
            Entity::Insert(x) => Some(&x.arch),
            Entity::Wall(x) => Some(&x.wall.arch),
            Entity::Space(x) => Some(&x.space.arch),
            Entity::Slab(x) => Some(&x.slab.arch),
            Entity::Mass(x) => Some(&x.mass.arch),
            Entity::Grid(x) => Some(&x.grid.arch),
            Entity::Column(x) => Some(&x.column.arch),
            Entity::Beam(x) => Some(&x.beam.arch),
            Entity::Roof(x) => Some(&x.roof.arch),
            Entity::Railing(x) => Some(&x.railing.arch),
            Entity::Stair(x) => Some(&x.stair.arch),
            Entity::Ramp(x) => Some(&x.ramp.arch),
            Entity::Polyline(p) => p.space_separator.as_deref(),
            _ => None,
        }
    }

    /// [`Self::arch_info`], to be written.
    pub fn arch_info_mut(&mut self) -> Option<&mut crate::ArchInfo> {
        match self {
            Entity::Insert(x) => Some(&mut x.arch),
            Entity::Wall(x) => Some(&mut x.wall.arch),
            Entity::Space(x) => Some(&mut x.space.arch),
            Entity::Slab(x) => Some(&mut x.slab.arch),
            Entity::Mass(x) => Some(&mut x.mass.arch),
            Entity::Grid(x) => Some(&mut x.grid.arch),
            Entity::Column(x) => Some(&mut x.column.arch),
            Entity::Beam(x) => Some(&mut x.beam.arch),
            Entity::Roof(x) => Some(&mut x.roof.arch),
            Entity::Railing(x) => Some(&mut x.railing.arch),
            Entity::Stair(x) => Some(&mut x.stair.arch),
            Entity::Ramp(x) => Some(&mut x.ramp.arch),
            Entity::Polyline(p) => p.space_separator.as_deref_mut(),
            _ => None,
        }
    }

    /// Its level as a level view reads it (docs/adr/0715 §1): its own; an opening's its wall's; a tag's its object's;
    /// a hatch following a room (a room's finish, docs/adr/0345) the room's; a dimension bound to objects all of one
    /// level that level (`by_uid` finds an object by its persistent id). None: on no level, shown on every one.
    pub fn level<'a>(&'a self, by_uid: impl Fn(&str) -> Option<&'a Entity>) -> Option<&'a str> {
        let followed = |e: &'a Entity| match e {
            Entity::Opening(o) => by_uid(&o.opening.host).and_then(Entity::own_level),
            other => other.own_level(),
        };
        match self {
            Entity::Opening(_) => followed(self),
            Entity::Tag(t) => by_uid(&t.tag.of).and_then(followed),
            Entity::Hatch(h) => {
                let outer = h.assoc.as_ref()?.outer.as_ref()?;
                by_uid(&outer.to_text()).and_then(followed)
            }
            Entity::Dimension(d) => {
                let refs = d.refs.as_deref()?;
                let mut level = None;
                for r in [&refs.a, &refs.b, &refs.c].into_iter().flatten() {
                    let l = by_uid(&r.uid.to_text()).and_then(followed)?;
                    match level {
                        Some(m) if m != l => return None,
                        _ => level = Some(l),
                    }
                }
                level
            }
            other => other.own_level(),
        }
    }
}
