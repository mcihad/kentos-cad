//! Bitişik alan (docs/adr/0162 §3): the path tool's shape that draws only
//! the new boundary; the region it closes with the neighbouring areas is the
//! new area. The neighbours are the visible areas in view on the overlap
//! layers (§1; the active layer while the mode is Serbest or no layer is
//! chosen), kept while the view, the drawing and the layers stand; the
//! region is the shared core's (`ops::adjoin::Neighbours::fill`). The web's
//! is `apps/web/src/tools/adjoinTool.ts`; both play
//! `fixtures/interaction/v1/adjoin.json`.

use kentos_contracts::CreateOperation;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::ops::adjoin::Neighbours;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::geometry::shape;

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
pub const NO_REGION: &str = "Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun.";

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

/// Writes the region as one area (its parts and holes as they are) on the
/// active layer through `cad.entities.create` (`adjoin`, one undo step
/// “Bitişik alan”), in the current colour and weight. The overlap control
/// (§2) cuts what overlaps areas the view does not show. Whether it was
/// written: when not, the reason is said and the path stays.
pub(crate) fn write(region: Vec<Area>, cx: &mut Context<'_>) -> bool {
    if region.is_empty() {
        cx.say(Level::Warn, NO_REGION);
        return false;
    }
    let layer = cx.doc.layers().active().to_owned();
    let mut areas = region;
    if let Some(clipped) = overlap::clip_new_areas(cx, &areas, &layer) {
        overlap::say_clipped(cx, &clipped);
        if clipped.areas.is_empty() {
            return false;
        }
        areas = clipped.areas;
    }
    let Some(geometry) = overlap::clipped_geometry(&areas) else {
        return false;
    };
    if points::write_objects(vec![geometry], Some(CreateOperation::Adjoin), cx).is_none() {
        return false;
    }
    let area = cx.format().area(overlap::written_area(&areas));
    let text = if areas.len() > 1 {
        format!("{LABEL} eklendi: {area} ({} parça)", areas.len())
    } else {
        format!("{LABEL} eklendi: {area}")
    };
    cx.say(Level::Success, text);
    true
}
