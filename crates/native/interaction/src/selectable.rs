//! Seçim süzgeci (docs/adr/0187 §5): while it is on, only the kinds it holds
//! are selected, by a click and its hover, a window or crossing box, the
//! selection tools, Tümünü seç, Ters çevir and a command's step that
//! selects objects. What it leaves out is said. The web's twin is
//! `apps/web/src/tools/selectable.ts`.

use kentos_contracts::Entity;
use kentos_domain::Slot;

use crate::Vec2;
use crate::data_search::kind_name;
use crate::log::Level;
use crate::tool::Context;

/// The kinds the filter can hold, in its menu's order (the web's `FILTER_KINDS`).
pub const KINDS: [&str; 16] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "xline",
    "ray",
    "text",
    "dimension",
    "hatch",
    "insert",
    "leader",
    "table",
];

/// Every kind: the filter's kinds at the start of a session.
pub const ALL: u32 = (1 << KINDS.len()) - 1;

/// A kind's bit in the filter's mask; none for a kind it does not know.
pub fn bit(kind: &str) -> u32 {
    KINDS.iter().position(|k| *k == kind).map_or(0, |i| 1 << i)
}

/// Whether the filter (`Draft::select_kinds`: none while it is off) lets an object of this kind be selected.
pub fn kind_allowed(filter: Option<u32>, kind: &str) -> bool {
    filter.is_none_or(|mask| mask & bit(kind) != 0)
}

fn allowed(cx: &Context<'_>, slot: Slot) -> bool {
    cx.doc
        .get(slot)
        .is_some_and(|e| kind_allowed(cx.draft.select_kinds, e.kind()))
}

/// Says how many objects the filter left out of a selection; nothing when none.
pub fn say_filtered(cx: &mut Context<'_>, n: usize) {
    if n > 0 {
        cx.say(
            Level::Info,
            format!("Seçim süzgeci {n} nesneyi dışarıda bıraktı."),
        );
    }
}

/// The objects the filter lets through, in their order; what it leaves out is said.
pub fn ids(cx: &mut Context<'_>, ids: Vec<Slot>) -> Vec<Slot> {
    if cx.draft.select_kinds.is_none() {
        return ids;
    }
    let before = ids.len();
    let out: Vec<Slot> = ids.into_iter().filter(|s| allowed(cx, *s)).collect();
    say_filtered(cx, before - out.len());
    out
}

/// What a click at `at` can select, the most specific first (Sıradakini
/// seç's candidates, docs/adr/0187 §1), the filter's leftovers dropped; when
/// it drops every one, said.
pub fn candidates(cx: &mut Context<'_>, at: Vec2) -> Vec<Slot> {
    let all = cx.spatial.hits(at, cx.pick_tolerance());
    if cx.draft.select_kinds.is_none() {
        return all;
    }
    let out: Vec<Slot> = all.iter().copied().filter(|s| allowed(cx, *s)).collect();
    if out.is_empty()
        && let Some(first) = all.first().and_then(|s| cx.doc.get(*s))
    {
        let kind = kind_name(first);
        cx.say(
            Level::Info,
            format!("Seçim süzgeci tıklanan nesneyi ({kind}) dışarıda bıraktı."),
        );
    }
    out
}

/// The object a click at `at` selects: the first candidate, the plain pick
/// while no filter is on (said when the filter leaves the click nothing).
pub fn pick(cx: &mut Context<'_>, at: Vec2) -> Option<Slot> {
    if cx.draft.select_kinds.is_none() {
        return cx.spatial.pick(at, cx.pick_tolerance());
    }
    candidates(cx, at).first().copied()
}

/// The object the pointer resting at `at` would select (its hover): nothing is said.
pub fn hover(cx: &Context<'_>, at: Vec2) -> Option<Slot> {
    let tol = cx.pick_tolerance();
    if cx.draft.select_kinds.is_none() {
        return cx.spatial.pick(at, tol);
    }
    cx.spatial
        .hits(at, tol)
        .into_iter()
        .find(|s| allowed(cx, *s))
}

/// Whether an entity is of a kind the filter lets through.
pub fn entity_allowed(filter: Option<u32>, e: &Entity) -> bool {
    kind_allowed(filter, e.kind())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mask_holds_the_kinds_ticked() {
        let mask = bit("line") | bit("polygon");
        assert!(kind_allowed(Some(mask), "line"));
        assert!(!kind_allowed(Some(mask), "point"));
        assert!(kind_allowed(None, "point"), "no filter: every kind");
        assert!(!kind_allowed(Some(0), "table"), "none ticked: nothing");
        assert_eq!(ALL.count_ones() as usize, KINDS.len());
        assert_eq!(bit("nothing"), 0);
    }
}
