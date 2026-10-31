//! The phase view (docs/adr/0346 §2): how a project's Mevcut durum, Yıkım planı or Yeni durum draws its architectural
//! objects, by `kentos_contracts::PhaseView::look`. A hidden object is not built; a dashed one is drawn with the dashed
//! line type, its pieces too; a faded one 60 % transparent, its fills too. The canvas's store hides the hidden ones
//! from picking and snapping on its own.

use std::borrow::Cow;
use std::collections::HashMap;

use kentos_contracts::{Entity, LineType, PHASE_FADED, PenCategory, PhaseLook, PhaseView, RecipePen};

/// The looks of a drawing's objects in a phase view, by their numbers; an object not listed is drawn as it is. A map's
/// recipe adds its pens and black and white (docs/adr/0783 §1), by the objects' kinds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Phasing {
    looks: HashMap<u32, PhaseLook>,
    pens: Vec<RecipePen>,
    mono: bool,
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
            return Self { looks, ..Self::default() };
        }
        for e in entities {
            let look = view.look(e.phase(&by_uid));
            if look != PhaseLook::Shown {
                looks.insert(e.base().id, look);
            }
        }
        Self { looks, ..Self::default() }
    }

    /// Whether nothing changes.
    pub fn is_empty(&self) -> bool {
        self.looks.is_empty() && self.pens.is_empty() && !self.mono
    }

    /// The recipe's pens and black and white drawn too (docs/adr/0783 §1).
    pub fn with_pens(mut self, pens: &[RecipePen], mono: bool) -> Self {
        self.pens = pens.to_vec();
        self.mono = mono;
        self
    }

    /// Whether the objects' weights or colours change.
    pub fn has_pens(&self) -> bool {
        !self.pens.is_empty() || self.mono
    }

    /// `e` as the phasing draws it: its look, then its category's pen; none: not drawn.
    pub fn draw<'a>(&self, e: &'a Entity) -> Option<Cow<'a, Entity>> {
        let drawn = phased_one(e, self.look(e.base().id))?;
        if !self.has_pens() {
            return Some(drawn);
        }
        Some(Cow::Owned(penned(drawn.into_owned(), &self.pens, self.mono)))
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
    entities.iter().filter_map(|e| phasing.draw(e)).collect()
}

/// `e` with its category's pen (docs/adr/0783 §1): the pen's weight and colour; black and white makes the rest black, a
/// hatch grey. A pen's colour wins over black and white.
pub fn penned(mut e: Entity, pens: &[RecipePen], mono: bool) -> Entity {
    let category = PenCategory::of_kind(e.kind());
    let pen = pens.iter().find(|p| p.of == category);
    let b = e.base_mut();
    if let Some(w) = pen.and_then(|p| p.weight) {
        b.line_weight = Some(w);
    }
    match pen.and_then(|p| p.color.clone()) {
        Some(c) => b.color = Some(c),
        None if mono => {
            b.color = Some(
                if category == PenCategory::Hatches {
                    kentos_contracts::MONO_HATCH
                } else {
                    kentos_contracts::MONO_LINE
                }
                .to_owned(),
            )
        }
        None => {}
    }
    e
}
