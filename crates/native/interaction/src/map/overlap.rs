//! The overlap control (docs/adr/0162 §1–§2): while it is on, a new area
//! drawn by its outline loses what overlaps the visible areas of the overlap
//! layers (its own layer, or the chosen ones). The session gives the layers;
//! the commands cut (`rules.avoid`, docs/adr/0622 §9; `points::area_rules`).
//! The web's is `apps/web/src/tools/overlap.ts`; both play
//! `fixtures/interaction/v1/overlap.json`.

use crate::tool::{Context, Overlap};

/// The layers whose areas a new area going on `layer` must not overlap;
/// none while the mode is Serbest.
pub(crate) fn overlap_layers(cx: &Context<'_>, layer: &str) -> Vec<String> {
    match cx.draft.overlap {
        Overlap::Allow => Vec::new(),
        Overlap::Layer => vec![layer.to_owned()],
        Overlap::Layers => cx.overlap_layers.to_vec(),
    }
}
