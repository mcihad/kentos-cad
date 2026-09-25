//! What the host decides about how the drawing is drawn (TODOS.md REN-01).
//! These are display choices; none of them changes the document. The typed,
//! persisted settings service (SET-*) and the quality presets (AA-*) will
//! fill them later; today the desktop app sets them from its theme.

use crate::color::Rgba8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    /// The drawing area's colour.
    pub background: Rgba8,
    /// Stroke width of every line, logical pixels (per-layer line weights: REN-11).
    pub line_width: f32,
    /// Largest distance between a curve and its chords on screen, logical pixels.
    pub curve_tolerance_px: f64,
    /// Most chords the curves of one scene may get. A deep zoom into a drawing
    /// full of curves coarsens the tolerance instead of exhausting memory
    /// (culling and per-view LOD: REN-10).
    pub curve_segment_budget: usize,
}

impl RenderSettings {
    pub const DEFAULT_LINE_WIDTH: f32 = 1.0;
    pub const DEFAULT_CURVE_TOLERANCE_PX: f64 = 0.25;
    pub const DEFAULT_CURVE_SEGMENT_BUDGET: usize = 1_000_000;

    /// The defaults on the given background.
    pub fn new(background: Rgba8) -> Self {
        Self {
            background,
            line_width: Self::DEFAULT_LINE_WIDTH,
            curve_tolerance_px: Self::DEFAULT_CURVE_TOLERANCE_PX,
            curve_segment_budget: Self::DEFAULT_CURVE_SEGMENT_BUDGET,
        }
    }
}
