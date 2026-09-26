//! What the host decides about how the drawing is drawn (TODOS.md REN-01).
//! These are display choices; none of them changes the document. The desktop
//! app fills them from its theme and its typed settings (docs/adr/0023):
//! `graphics.msaa` and `graphics.hiDpi` as the settings resolve them.

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
    /// Samples per pixel (1: none). A count the device does not take is
    /// drawn with the nearest one below it (TODOS.md AA-01); above 1 the view
    /// draws into its own multisampled target (`targets`).
    pub samples: u32,
    /// Draw at the host's full resolution; off, one pixel per logical pixel,
    /// scaled up when composed (a quarter of the pixels on a 2× screen).
    pub hi_dpi: bool,
}

impl RenderSettings {
    pub const DEFAULT_LINE_WIDTH: f32 = 1.0;
    pub const DEFAULT_CURVE_TOLERANCE_PX: f64 = 0.25;
    pub const DEFAULT_CURVE_SEGMENT_BUDGET: usize = 1_000_000;

    /// The defaults on the given background: drawn straight into the host's
    /// pass, single-sampled, at full resolution.
    pub fn new(background: Rgba8) -> Self {
        Self {
            background,
            line_width: Self::DEFAULT_LINE_WIDTH,
            curve_tolerance_px: Self::DEFAULT_CURVE_TOLERANCE_PX,
            curve_segment_budget: Self::DEFAULT_CURVE_SEGMENT_BUDGET,
            samples: 1,
            hi_dpi: true,
        }
    }
}
