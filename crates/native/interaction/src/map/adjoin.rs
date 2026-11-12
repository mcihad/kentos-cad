//! Bitişik alan (docs/adr/0162 §3): the path tool's shape that draws only
//! the new boundary; the region it closes with the neighbouring areas is the
//! new area. The neighbours are the visible areas in view on the overlap
//! layers (§1; the active layer while the mode is Serbest or no layer is
//! chosen), kept while the view, the drawing and the layers stand; the
//! region is the shared core's (`ops::adjoin::Neighbours::fill`). The web's
//! is `apps/web/src/tools/adjoinTool.ts`; both play
//! `fixtures/interaction/v1/adjoin.json`.

use kentos_commands::{AdjoinWindow, EntitiesAdjoin};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::ops::adjoin::Neighbours;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, adjoin_cmd};

use crate::Vec2;
use crate::log::Level;
use crate::overlap;
use crate::points;
use crate::tool::Context;

/// The tool's id: its command is `tool.adjoin`.
pub const ID: &str = "adjoin";
pub const LABEL: &str = "Bitişik alan";

/// Neighbours with more edges than this are cut through at clicks and Enter
/// only, not at every pointer move (§5: 2.7 ms at 1 596 edges).
pub const PREVIEW_EDGES: usize = 2_000;

/// Said when the path closes no region with the neighbours (§3).
pub const NO_REGION: &str = adjoin_cmd::NO_REGION;

/// The layers whose areas the path closes against: the overlap mode's, the
/// active layer while it is Serbest or no layer is chosen.
fn layers(cx: &Context<'_>) -> Vec<String> {
    let active = cx.doc.layers().active().to_owned();
    let layers = overlap::overlap_layers(cx, &active);
    if layers.is_empty() {
        vec![active]
    } else {
        layers
    }
}

/// The neighbours and what they were taken from: the view, the drawing's
/// revision and the layers.
struct Kept {
    key: ([f64; 4], u64, Vec<String>),
    neighbours: Neighbours,
}

/// The neighbours of what the view shows, taken again when it is stale.
/// They are neither `Clone` nor `Debug`: a copy of the tool starts without.
#[derive(Default)]
pub(crate) struct NeighbourCache(Option<Kept>);

impl Clone for NeighbourCache {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for NeighbourCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NeighbourCache")
    }
}

impl NeighbourCache {
    pub(crate) fn get(&mut self, cx: &Context<'_>) -> &Neighbours {
        let b = cx.view.visible();
        let key = (
            [b.min_x, b.min_y, b.max_x, b.max_y],
            cx.doc.revision(),
            layers(cx),
        );
        let kept = match self.0.take() {
            Some(kept) if kept.key == key => kept,
            _ => {
                let sets: Vec<Vec<Area>> = cx
                    .spatial
                    .in_rect(
                        Vec2::new(b.min_x, b.min_y),
                        Vec2::new(b.max_x, b.max_y),
                        true,
                    )
                    .into_iter()
                    .filter_map(|s| cx.doc.get(s))
                    .filter(|e| key.2.iter().any(|l| *l == e.base().layer_id))
                    .map(|e| areas_of_entity(&shape(e)))
                    .filter(|set| !set.is_empty())
                    .collect();
                Kept {
                    key,
                    neighbours: Neighbours::new(sets),
                }
            }
        };
        &self.0.insert(kept).neighbours
    }
}

/// The region the path (one bulge per segment) closes with the neighbours;
/// none for fewer than two points.
pub(crate) fn fill(neighbours: &Neighbours, pts: &[Vec2], bulges: &[f64]) -> Vec<Area> {
    if pts.len() < 2 {
        return Vec::new();
    }
    neighbours.fill(pts, has_bulges(Some(bulges)).then_some(bulges))
}

/// Writes the region the path closes through `cad.entities.adjoin` (docs/adr/0622 §9): the active layer, the
/// neighbour layers and the view's box, the current look, the session's Çakışma (beyond the neighbours) and
/// Topoloji as the command's rules; one undo step “Bitişik alan”. Whether it was written: when not, the command's
/// reason is said and the path stays.
pub(crate) fn write(pts: &[Vec2], bulges: &[f64], cx: &mut Context<'_>) -> bool {
    let layer = cx.doc.layers().active().to_owned();
    let b = cx.view.visible();
    let rules = points::area_rules(cx, &layer);
    let input = EntitiesAdjoin {
        layer_id: layer,
        pts: points::wire_all(pts),
        bulges: has_bulges(Some(bulges)).then(|| bulges.to_vec()),
        neighbours: layers(cx),
        within: Some(AdjoinWindow {
            min: points::wire(Vec2::new(b.min_x, b.min_y)),
            max: points::wire(Vec2::new(b.max_x, b.max_y)),
        }),
        color: cx.draft.color_text(),
        line_weight: cx.draft.line_weight,
        line_type: cx.draft.line_type,
        attrs: cx.template_attrs(None),
        label: cx.template_label(),
        symbol: cx.template_symbol(),
        rules,
        expected_revision: None,
    };
    let result = adjoin_cmd::execute(
        &mut ExecutionContext::with_store(cx.doc, cx.spatial.store()),
        input,
    );
    let Some(out) = points::written(result, cx) else {
        return false;
    };
    let parts = cx
        .doc
        .get(kentos_domain::Slot(out.id))
        .map_or(1, |e| areas_of_entity(&shape(e)).len());
    let area = cx.format().area(points::area_of_slot(cx, out.id));
    let text = if parts > 1 {
        format!("{LABEL} eklendi: {area} ({parts} parça)")
    } else {
        format!("{LABEL} eklendi: {area}")
    };
    cx.say(Level::Success, text);
    true
}
