//! Kat görünümü (docs/adr/0715): which of the project's levels the plan shows. Bütün katlar draws them all (as before
//! levels were seen); Yalnız etkin kat draws the active level's objects and those on no level; Etkin kat ve alt kat
//! draws the level under the active one too, faint: snapped to, never picked (Revit's underlay with Select underlay
//! elements off, Archicad's Trace Reference). An object's level is `Entity::level`: its own, an opening's its wall's,
//! a tag's its object's, a room finish's its room's, a dimension's that of the objects it is bound to. One on no level
//! shows on every level; one on a level the project no longer has shows too (Denetim says so). A stair of the level
//! under the active one that reaches its cut plane shows on the active level's plan as seen from above: its part past
//! its cut (docs/adr/0724 §4), drawn as the active level's are, never picked.

use kentos_commands::{ArchLevel, Entity, ProjectSettings};
use kentos_domain::{Document, Uuid};

/// Which levels the plan shows: the contract's, a drawing saves the one it was seen with (docs/adr/0720 §1).
pub use kentos_commands::LevelView;

/// What a level view hides and draws faint, by the objects' numbers (slots), in the drawing's order; the stairs it
/// draws as the plan of the level above theirs shows them (docs/adr/0724 §4); and, whatever the view, the objects on a
/// level of the project other than the active one: a click takes the active level's (and those on no level) before
/// them where they lie over each other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelLooks {
    pub hidden: Vec<u32>,
    pub faded: Vec<u32>,
    pub upper: Vec<u32>,
    pub elsewhere: Vec<u32>,
}

impl LevelLooks {
    /// Whether nothing changes.
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty() && self.faded.is_empty() && self.upper.is_empty()
    }
}

/// Whether `e` shows on the plan of the level above its own (docs/adr/0724 §4): a stair with a cut line (one without
/// does not reach the plan's cut plane).
pub fn shows_above(e: &Entity) -> bool {
    matches!(e, Entity::Stair(s) if s.stair.cut.is_some())
}

/// The project's levels.
fn levels(settings: &ProjectSettings) -> &[ArchLevel] {
    settings.arch.as_ref().map_or(&[], |a| a.levels.as_slice())
}

/// The level under `active`: the highest of those whose floor is lower than its; none on the lowest.
pub fn below<'a>(settings: &'a ProjectSettings, active: &str) -> Option<&'a ArchLevel> {
    let levels = levels(settings);
    let at = levels.iter().find(|l| l.id == active)?.elevation;
    levels
        .iter()
        .filter(|l| l.elevation < at)
        .max_by(|a, b| a.elevation.total_cmp(&b.elevation))
}

/// What `view` hides and draws faint in `doc` with `active` the active level (none: Katsız, the objects on no level),
/// the stairs under it it draws from above, and what is elsewhere. Nothing hidden, faint or from above while the view
/// is Bütün katlar; nothing at all while the project has no levels.
pub fn looks(doc: &Document, view: LevelView, active: Option<&str>) -> LevelLooks {
    let mut out = LevelLooks::default();
    let all = levels(doc.settings());
    if all.is_empty() {
        return out;
    }
    let known = |id: &str| all.iter().any(|l| l.id == id);
    // The level under the active one: its stairs seen from above (any view but Bütün katlar), the rest faint
    // (Etkin kat ve alt kat).
    let lower = match view {
        LevelView::All => None,
        _ => active
            .and_then(|a| below(doc.settings(), a))
            .map(|l| l.id.as_str()),
    };
    let under = match view {
        LevelView::Below => lower,
        _ => None,
    };
    let by_uid = |u: &str| {
        Uuid::parse_str(u)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
    };
    for e in doc.entities() {
        let Some(level) = e.level(by_uid) else {
            continue;
        };
        if Some(level) == active || !known(level) {
            continue;
        }
        let id = e.base().id;
        out.elsewhere.push(id);
        match view {
            LevelView::All => {}
            _ if Some(level) == lower && shows_above(e) => out.upper.push(id),
            _ if Some(level) == under => out.faded.push(id),
            _ => out.hidden.push(id),
        }
    }
    out
}
