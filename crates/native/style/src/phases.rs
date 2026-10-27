//! The phase view (docs/adr/0346 §2): how a project's Mevcut durum, Yıkım planı or Yeni durum draws its architectural
//! objects, by `kentos_contracts::PhaseView::look`. A hidden object is not built; a dashed one is drawn with the dashed
//! line type, its pieces too; a faded one 60 % transparent, its fills too. The canvas's store hides the hidden ones
//! from picking and snapping on its own.

use std::borrow::Cow;
use std::collections::HashMap;

use kentos_contracts::{Entity, LineType, PHASE_FADED, PhaseLook, PhaseView};

/// The looks of a drawing's objects in a phase view, by their numbers; an object not listed is drawn as it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Phasing {
    looks: HashMap<u32, PhaseLook>,
}

impl Phasing {
    /// The looks of `entities` in `view` (`by_uid` finds an object by its persistent id: an opening's wall, a tag's
    /// object); none to change in Tümü.
    pub fn of<'a>(
        entities: impl IntoIterator<Item = &'a Entity>,
        view: PhaseView,
        by_uid: impl Fn(&str) -> Option<&'a Entity>,
    ) -> Self {
        let mut looks = HashMap::new();
        if view == PhaseView::All {
            return Self { looks };
        }
        for e in entities {
            let look = view.look(e.phase(&by_uid));
            if look != PhaseLook::Shown {
                looks.insert(e.base().id, look);
            }
        }
        Self { looks }
    }

    /// Whether nothing changes.
    pub fn is_empty(&self) -> bool {
        self.looks.is_empty()
    }

    /// An object's look.
    pub fn look(&self, id: u32) -> PhaseLook {
        self.looks.get(&id).copied().unwrap_or(PhaseLook::Shown)
    }

    /// Object `id` left out (a PDF underlay that is not plotted, docs/adr/0351 §2).
    pub fn hide(&mut self, id: u32) {
        self.looks.insert(id, PhaseLook::Hidden);
    }

    /// Object `id` drawn faint (the level under the active one, docs/adr/0715 §2) unless it is left out.
    pub fn fade(&mut self, id: u32) {
        let look = self.looks.entry(id).or_insert(PhaseLook::Faded);
        if *look != PhaseLook::Hidden {
            *look = PhaseLook::Faded;
        }
    }

    /// The hidden objects' numbers.
    pub fn hidden(&self) -> impl Iterator<Item = u32> + '_ {
        self.looks
            .iter()
            .filter(|(_, l)| **l == PhaseLook::Hidden)
            .map(|(id, _)| *id)
    }
}

/// `e` as the look draws it; none: not drawn.
pub fn phased_one(e: &Entity, look: PhaseLook) -> Option<Cow<'_, Entity>> {
    match look {
        PhaseLook::Shown => Some(Cow::Borrowed(e)),
        PhaseLook::Hidden => None,
        PhaseLook::Dashed => {
            let mut x = e.clone();
            let b = x.base_mut();
            b.look.line_type = Some(LineType::Dashed);
            b.look.line_pattern = None;
            Some(Cow::Owned(x))
        }
        PhaseLook::Faded => {
            let mut x = e.clone();
            let b = x.base_mut();
            let own = b.look.transparency.unwrap_or(0.0);
            b.look.transparency = Some(if own > PHASE_FADED { own } else { PHASE_FADED });
            Some(Cow::Owned(x))
        }
    }
}

/// `entities` as the phasing draws them, in their order; the hidden left out.
pub fn phased<'a>(entities: &[&'a Entity], phasing: &Phasing) -> Vec<Cow<'a, Entity>> {
    if phasing.is_empty() {
        return entities.iter().map(|e| Cow::Borrowed(*e)).collect();
    }
    entities
        .iter()
        .filter_map(|e| phased_one(e, phasing.look(e.base().id)))
        .collect()
}
