//! The 2D CAD camera (TODOS.md REN-01, REN-07): which world point sits in the
//! middle of the drawing area and how many logical pixels a world unit spans.
//! Everything is float64; screen positions are logical pixels from the area's
//! top-left corner, y down, as the host's pointer events give them. The
//! behaviour follows the web's camera (`apps/web/src/viewport/Camera.ts`):
//! the same zoom limits, zoom about the cursor, fit with 48 px of margin.

use crate::layout::FrameUniform;
use crate::precision::split;
use crate::settings::RenderSettings;
use crate::{Bounds, Vec2};

/// Farthest zoom out: logical pixels per metre (about a whole country on screen).
pub const MIN_SCALE: f64 = 1e-4;
/// Deepest zoom in: about 0.2 mm per logical pixel.
pub const MAX_SCALE: f64 = 5e3;
/// Margin a fit leaves around the box, logical pixels.
pub const FIT_PADDING: f64 = 48.0;
/// One CSS pixel at 96 dpi in metres: the screen scale 1:N the status bar shows.
const METRES_PER_PX: f64 = 0.000_264_58;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The world point in the middle of the area.
    pub center: Vec2,
    /// Logical pixels per world unit.
    pub scale: f64,
    /// The area's size in logical pixels.
    pub width: f64,
    pub height: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vec2::new(0.0, 0.0),
            scale: 1.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl Camera {
    /// The area's new size; the centre and scale stay, so the view grows around its middle.
    pub fn set_size(&mut self, width: f64, height: f64) {
        if width.is_finite() && height.is_finite() {
            self.width = width.max(1.0);
            self.height = height.max(1.0);
        }
    }

    /// Screen position (logical px from the top-left, y down) of a world point.
    pub fn world_to_screen(&self, p: Vec2) -> [f64; 2] {
        [
            (p.x - self.center.x) * self.scale + self.width / 2.0,
            self.height / 2.0 - (p.y - self.center.y) * self.scale,
        ]
    }

    /// World point under a screen position.
    pub fn screen_to_world(&self, x: f64, y: f64) -> Vec2 {
        Vec2::new(
            self.center.x + (x - self.width / 2.0) / self.scale,
            self.center.y - (y - self.height / 2.0) / self.scale,
        )
    }

    /// Moves the view with the pointer: the world under it follows it by (dx, dy) pixels.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        if dx.is_finite() && dy.is_finite() {
            self.center = Vec2::new(
                self.center.x - dx / self.scale,
                self.center.y + dy / self.scale,
            );
        }
    }

    /// Zooms by `factor` about a screen position: the world point there stays under it.
    pub fn zoom_at(&mut self, factor: f64, x: f64, y: f64) {
        if !(factor.is_finite() && factor > 0.0 && x.is_finite() && y.is_finite()) {
            return;
        }
        let anchor = self.screen_to_world(x, y);
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        self.center = Vec2::new(
            anchor.x - (x - self.width / 2.0) / self.scale,
            anchor.y + (y - self.height / 2.0) / self.scale,
        );
    }

    /// Puts `b` in the middle of the area, as large as fits within `padding` pixels of margin.
    pub fn fit(&mut self, b: &Bounds, padding: f64) {
        let finite = [b.min_x, b.min_y, b.max_x, b.max_y]
            .iter()
            .all(|v| v.is_finite());
        if !finite || b.min_x > b.max_x || b.min_y > b.max_y {
            return;
        }
        let w = (b.max_x - b.min_x).max(1e-6);
        let h = (b.max_y - b.min_y).max(1e-6);
        // An area smaller than its margins keeps at least half of itself for the drawing.
        let sx = (self.width - padding * 2.0).max(self.width / 2.0) / w;
        let sy = (self.height - padding * 2.0).max(self.height / 2.0) / h;
        let scale = sx.min(sy);
        self.scale = if scale.is_finite() && scale > 0.0 {
            scale.clamp(MIN_SCALE, MAX_SCALE)
        } else {
            MIN_SCALE
        };
        self.center = Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
    }

    /// Centres the view on a world point.
    pub fn center_on(&mut self, p: Vec2) {
        if p.x.is_finite() && p.y.is_finite() {
            self.center = p;
        }
    }

    /// The world box the area shows.
    pub fn visible_bounds(&self) -> Bounds {
        let a = self.screen_to_world(0.0, self.height);
        let b = self.screen_to_world(self.width, 0.0);
        Bounds {
            min_x: a.x,
            min_y: a.y,
            max_x: b.x,
            max_y: b.y,
        }
    }

    /// The view's scale as 1:N on a 96 dpi screen (the status bar's “Ekran 1:N”).
    pub fn screen_scale(&self) -> f64 {
        1.0 / (self.scale * METRES_PER_PX)
    }

    /// The frame uniform of this view for a scene whose GPU offsets are from
    /// `origin`: the camera centre split into float32 parts of its float64
    /// offset from the origin, and the scale in device pixels. `size_px` is
    /// the area in device pixels, `scale_factor` device pixels per logical
    /// pixel; `srgb_target` whether the target format encodes sRGB.
    pub fn frame_uniform(
        &self,
        origin: Vec2,
        size_px: [f32; 2],
        scale_factor: f64,
        settings: &RenderSettings,
        srgb_target: bool,
    ) -> FrameUniform {
        let (cx_hi, cx_lo) = split(self.center.x - origin.x);
        let (cy_hi, cy_lo) = split(self.center.y - origin.y);
        FrameUniform {
            center_hi: [cx_hi, cy_hi],
            center_lo: [cx_lo, cy_lo],
            viewport: [size_px[0].max(1.0), size_px[1].max(1.0)],
            px_per_unit: (self.scale * scale_factor) as f32,
            dpi: scale_factor as f32,
            background: settings.background.to_f32(),
            line_width: settings.line_width,
            srgb_target: u32::from(srgb_target),
            _pad: [0; 2],
        }
    }
}
