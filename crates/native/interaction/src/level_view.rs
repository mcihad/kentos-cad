//! Kat görünümü (docs/adr/0715): which of the project's levels the plan shows. Bütün katlar draws them all (as before
//! levels were seen); Yalnız etkin kat draws the active level's objects and those on no level; Etkin kat ve alt kat
//! draws the level under the active one too, faint: snapped to, never picked (Revit's underlay with Select underlay
//! elements off, Archicad's Trace Reference). An object's level is `Entity::level`: its own, an opening's its wall's,
//! a tag's its object's, a room finish's its room's, a dimension's that of the objects it is bound to. One on no level
//! shows on every level; one on a level the project no longer has shows too (Denetim says so).

use kentos_commands::{ArchLevel, ProjectSettings};
use kentos_domain::{Document, Uuid};

/// Which levels the plan shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LevelView {
    /// Bütün katlar.
    #[default]
    All,
    /// Yalnız etkin kat.
    Active,
    /// Etkin kat ve alt kat: the level under the active one faint.
    Below,
}

impl LevelView {
    /// Its name, as the menus say it.
    pub fn name(self) -> &'static str {
        match self {
            LevelView::All => "Bütün katlar",
            LevelView::Active => "Yalnız etkin kat",
            LevelView::Below => "Etkin kat ve alt kat",
        }
    }
}

/// What a level view hides and draws faint, by the objects' numbers (slots), in the drawing's order; and, whatever
/// the view, the objects on a level of the project other than the active one: a click takes the active level's (and
/// those on no level) before them where they lie over each other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelLooks {
    pub hidden: Vec<u32>,
    pub faded: Vec<u32>,
    pub elsewhere: Vec<u32>,
}

impl LevelLooks {
    /// Whether nothing changes.
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty() && self.faded.is_empty()
    }
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
/// and what is elsewhere. Nothing hidden or faint while the view is Bütün katlar; nothing at all while the project has
/// no levels.
pub fn looks(doc: &Document, view: LevelView, active: Option<&str>) -> LevelLooks {
    let mut out = LevelLooks::default();
    let all = levels(doc.settings());
    if all.is_empty() {
        return out;
    }
    let known = |id: &str| all.iter().any(|l| l.id == id);
    let under = match view {
        LevelView::Below => active
            .and_then(|a| below(doc.settings(), a))
            .map(|l| l.id.as_str()),
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
            _ if Some(level) == under => out.faded.push(id),
            _ => out.hidden.push(id),
        }
    }
    out
}
