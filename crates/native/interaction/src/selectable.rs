//! Seçim süzgeci (docs/adr/0187 §5): while it is on, only the kinds it holds
//! are selected, by a click and its hover, a window or crossing box, the
//! selection tools, Tümünü seç, Ters çevir and a command's step that
//! selects objects. What it leaves out is said. The web's twin is
//! `apps/web/src/tools/selectable.ts`. Every selection asks the one query
//! of `cad.entities.select` (docs/adr/0275) with the filter's kinds, and with
//! Otomatik seçim off (docs/adr/0622 §5) the active layer alone.

use kentos_commands::{EntitiesSelect, Entity, SelectRegion, Vec2 as Point};
use kentos_domain::{Document, Slot};
use kentos_native_application::entities_select::{self, Selected};

use crate::Vec2;
use crate::data_search::kind_name;
use crate::log::Level;
use crate::tool::Context;

/// The kinds the filter can hold, in its menu's order (the web's `FILTER_KINDS`).
pub const KINDS: [&str; 27] = [
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
    "image",
    "wall",
    "opening",
    "space",
    "grid",
    "column",
    "stair",
    "ramp",
    "tag",
    "viewmark",
    // Last, so a filter saved before keeps its bits (docs/adr/0722).
    "slab",
];

/// Every kind: the filter's kinds at the start of a session.
pub const ALL: u32 = (1 << KINDS.len()) - 1;

/// A kind's bit in the filter's mask; none for a kind it does not know.
pub fn bit(kind: &str) -> u32 {
    // A NURBS curve is a curve: the filter's Eğri holds it too (docs/adr/0259 §6).
    let kind = if kind == "nurbs" { "spline" } else { kind };
    KINDS.iter().position(|k| *k == kind).map_or(0, |i| 1 << i)
}

/// Whether the filter (`Draft::select_kinds`: none while it is off) lets an object of this kind be selected.
pub fn kind_allowed(filter: Option<u32>, kind: &str) -> bool {
    filter.is_none_or(|mask| mask & bit(kind) != 0)
}

fn allowed(cx: &Context<'_>, slot: Slot) -> bool {
    cx.doc.get(slot).is_some_and(|e| {
        kind_allowed(cx.draft.select_kinds, e.kind()) && on_selected_layer(cx.doc, e)
    })
}

/// Otomatik seçim (docs/adr/0622 §5): while the project's is off, the one layer whose objects are selected, the
/// active one, by every step the selection filter holds; none while it is on (any shown layer's).
pub fn only_layer(doc: &Document) -> Option<&str> {
    (!doc.settings().auto_select()).then(|| doc.layers().active())
}

/// Whether Otomatik seçim lets this object be selected: any while it is on, the active layer's while it is off.
pub fn on_selected_layer(doc: &Document, e: &Entity) -> bool {
    only_layer(doc).is_none_or(|layer| e.base().layer_id == layer)
}

/// Says that Otomatik seçim, off, left `n` objects of other layers out of a selection; nothing when none.
pub fn say_layer_left(cx: &mut Context<'_>, n: usize) {
    if let Some(words) = layer_left_words(cx.doc, n) {
        cx.say(Level::Info, words);
    }
}

/// What Otomatik seçim, off, left out of a selection said: `n` objects of other layers; none when none.
pub fn layer_left_words(doc: &Document, n: usize) -> Option<String> {
    (n > 0).then(|| {
        let name = active_name(doc);
        format!(
            "Otomatik seçim kapalı: yalnız etkin katmanın (“{name}”) nesneleri seçilir; başka katmandaki {n} nesne dışarıda kaldı."
        )
    })
}

/// The active layer's name as a sentence says it.
fn active_name(doc: &Document) -> String {
    let id = doc.layers().active();
    doc.layers()
        .get(id)
        .map_or_else(|| id.to_owned(), |n| n.name.clone())
}

/// A point as the query takes it.
pub fn point(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

/// The selection query (`cad.entities.select`, docs/adr/0275) over the session's store with the filter's kinds
/// (`kinds`: `Draft::select_kinds`, none while the filter is off); none when the region is refused (a tool checks
/// its region before it asks).
pub fn query_in(
    doc: &Document,
    spatial: &crate::Spatial,
    kinds: Option<u32>,
    region: SelectRegion,
) -> Option<Selected> {
    let input = EntitiesSelect {
        model_scope: spatial.sheet_model_scope(),
        region,
        kinds: kinds.map(entities_select::kinds_of_mask),
        // Otomatik seçim off: the active layer's alone (docs/adr/0622 §5).
        layers: only_layer(doc).map(|l| vec![l.to_owned()]),
    };
    entities_select::select(doc, spatial.store(), &input).ok()
}

/// [`query_in`] for a tool: the objects, and what the filter left out said.
pub fn query(cx: &mut Context<'_>, region: SelectRegion) -> Vec<Slot> {
    let Some(found) = query_in(cx.doc, cx.spatial, cx.draft.select_kinds, region) else {
        return Vec::new();
    };
    say_filtered(cx, found.left_out.kinds as usize);
    say_layer_left(cx, found.left_out.layers as usize);
    found.slots
}

/// A box's objects through the query (docs/adr/0275, 0278): a window (`crossing` false) or a crossing box between
/// two world corners, the filter's kinds with it and what it left out said; every tool that selects with a box asks
/// here.
pub fn in_box(cx: &mut Context<'_>, a: Vec2, b: Vec2, crossing: bool) -> Vec<Slot> {
    if let Some(ring) = twisted_ring(cx, a, b, crossing) {
        return query(cx, ring);
    }
    let (a, b) = (point(a), point(b));
    let region = if crossing {
        SelectRegion::Crossing { a, b }
    } else {
        SelectRegion::Window { a, b }
    };
    query(cx, region)
}

/// The box between two world corners as a twisted view draws it (docs/adr/0285 §2): its four screen corners'
/// world points as a ring, wholly inside or touching as the box went; none with the north up, where the box is
/// the world's own.
pub fn twisted_ring(cx: &Context<'_>, a: Vec2, b: Vec2, crossing: bool) -> Option<SelectRegion> {
    if cx.view.twist() == 0.0 {
        return None;
    }
    let (sa, sb) = (cx.view.to_screen(a), cx.view.to_screen(b));
    let mut points = Vec::with_capacity(4);
    for c in [sa, [sb[0], sa[1]], sb, [sa[0], sb[1]]] {
        points.push(point(cx.view.to_world(c)?));
    }
    Some(SelectRegion::Polygon {
        points,
        mode: if crossing {
            kentos_commands::PolygonSelectMode::Crossing
        } else {
            kentos_commands::PolygonSelectMode::Inside
        },
    })
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
    if cx.draft.select_kinds.is_none() && only_layer(cx.doc).is_none() {
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
    let region = SelectRegion::Point {
        at: point(at),
        tolerance: cx.pick_tolerance(),
    };
    let (all, other_layers) = query_in(cx.doc, cx.spatial, None, region)
        .map_or((Vec::new(), 0), |f| (f.slots, f.left_out.layers));
    if all.is_empty() && other_layers > 0 {
        // The click fell on another layer's object while Otomatik seçim is off (docs/adr/0622 §5).
        let name = active_name(cx.doc);
        cx.say(
            Level::Info,
            format!(
                "Tıklanan nesne başka katmanda. Otomatik seçim kapalıyken yalnız etkin katmanın (“{name}”) nesneleri seçilir: katmanı etkin yapın ya da Otomatik seçim'i açın."
            ),
        );
    }
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
    if cx.draft.select_kinds.is_none() && only_layer(cx.doc).is_none() {
        return cx.spatial.pick(at, cx.pick_tolerance());
    }
    candidates(cx, at).first().copied()
}

/// The object the pointer resting at `at` would select (its hover): nothing is said.
pub fn hover(cx: &Context<'_>, at: Vec2) -> Option<Slot> {
    let tol = cx.pick_tolerance();
    if cx.draft.select_kinds.is_none() && only_layer(cx.doc).is_none() {
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

/// Whether a selecting step takes this entity: of a kind the filter lets through and on a layer Otomatik seçim lets
/// it be selected from (docs/adr/0622 §5).
pub fn entity_selectable(cx: &Context<'_>, e: &Entity) -> bool {
    entity_allowed(cx.draft.select_kinds, e) && on_selected_layer(cx.doc, e)
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
        // Every kind of object is one of the filter's: a picture too (docs/adr/0187 §5, 8 Ekim).
        assert!(kind_allowed(Some(ALL), "image"));
        assert!(!kind_allowed(Some(ALL & !bit("image")), "image"));
    }

    /// The filter's bits are the query's kinds in one order (docs/adr/0275).
    #[test]
    fn the_mask_and_the_query_s_kinds_are_one_order() {
        let names: Vec<&str> = kentos_commands::SelectKind::ALL
            .iter()
            .map(|k| k.name())
            .collect();
        assert_eq!(names, KINDS);
        let mask = bit("line") | bit("table");
        let kinds = entities_select::kinds_of_mask(mask);
        assert_eq!(
            kinds.iter().map(|k| k.name()).collect::<Vec<_>>(),
            ["line", "table"]
        );
    }
}
