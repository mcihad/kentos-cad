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
    /// The view's twist (docs/adr/0285 §2): the angle of the screen's x axis
    /// from the world's x (east), counter-clockwise, radians in [0, 2π); 0
    /// shows the north up. The world turns by minus it about the middle.
    pub twist: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vec2::new(0.0, 0.0),
            scale: 1.0,
            width: 1.0,
            height: 1.0,
            twist: 0.0,
        }
    }
}

/// `a` in [0, 2π): the remainder of a division by 2π, a negative one turned
/// once; −0 is 0 (the named views' rule, `kentos_contracts::normal_angle`).
pub fn normal_twist(a: f64) -> f64 {
    let tau = 2.0 * std::f64::consts::PI;
    if !a.is_finite() {
        return 0.0;
    }
    let mut r = a % tau;
    if r < 0.0 {
        r += tau;
    }
    if r >= tau {
        r = 0.0;
    }
    r + 0.0
}

impl Camera {
    /// The area's new size; the centre and scale stay, so the view grows around its middle.
    pub fn set_size(&mut self, width: f64, height: f64) {
        if width.is_finite() && height.is_finite() {
            self.width = width.max(1.0);
            self.height = height.max(1.0);
        }
    }

    /// The twist's cosine and sine; exactly (1, 0) with the north up.
    pub fn twist_cos_sin(&self) -> (f64, f64) {
        if self.twist == 0.0 {
            (1.0, 0.0)
        } else {
            (self.twist.cos(), self.twist.sin())
        }
    }

    /// A world offset from the middle along the screen's axes (x right, y up), world units.
    fn to_view(&self, dx: f64, dy: f64) -> (f64, f64) {
        if self.twist == 0.0 {
            return (dx, dy);
        }
        let (c, s) = self.twist_cos_sin();
        (dx * c + dy * s, dy * c - dx * s)
    }

    /// The reverse: an offset along the screen's axes (x right, y up) as a world offset.
    fn from_view(&self, u: f64, v: f64) -> (f64, f64) {
        if self.twist == 0.0 {
            return (u, v);
        }
        let (c, s) = self.twist_cos_sin();
        (u * c - v * s, u * s + v * c)
    }

    /// Screen position (logical px from the top-left, y down) of a world point.
    pub fn world_to_screen(&self, p: Vec2) -> [f64; 2] {
        let (u, v) = self.to_view(p.x - self.center.x, p.y - self.center.y);
        [
            u * self.scale + self.width / 2.0,
            self.height / 2.0 - v * self.scale,
        ]
    }

    /// World point under a screen position.
    pub fn screen_to_world(&self, x: f64, y: f64) -> Vec2 {
        let (dx, dy) = self.from_view(
            (x - self.width / 2.0) / self.scale,
            -(y - self.height / 2.0) / self.scale,
        );
        Vec2::new(self.center.x + dx, self.center.y + dy)
    }

    /// A world direction (an angle from east, counter-clockwise) as the screen shows it: the
    /// angle from the screen's x axis, counter-clockwise (y up).
    pub fn screen_angle(&self, world: f64) -> f64 {
        world - self.twist
    }

    /// Moves the view with the pointer: the world under it follows it by (dx, dy) pixels.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        if dx.is_finite() && dy.is_finite() {
            let (wx, wy) = self.from_view(dx / self.scale, -dy / self.scale);
            self.center = Vec2::new(self.center.x - wx, self.center.y - wy);
        }
    }

    /// Zooms by `factor` about a screen position: the world point there stays under it.
    pub fn zoom_at(&mut self, factor: f64, x: f64, y: f64) {
        if !(factor.is_finite() && factor > 0.0 && x.is_finite() && y.is_finite()) {
            return;
        }
        let anchor = self.screen_to_world(x, y);
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let (dx, dy) = self.from_view(
            (x - self.width / 2.0) / self.scale,
            -(y - self.height / 2.0) / self.scale,
        );
        self.center = Vec2::new(anchor.x - dx, anchor.y - dy);
    }

    /// Zooms about the area's middle to a screen scale of 1:`n`, a 96 dpi
    /// screen's ([`Camera::screen_scale`]): the status bar's scale selector
    /// (docs/adr/0165 §5).
    pub fn zoom_to_screen_scale(&mut self, n: f64) {
        if !(n.is_finite() && n > 0.0) {
            return;
        }
        let target = 1.0 / (n * METRES_PER_PX);
        self.zoom_at(target / self.scale, self.width / 2.0, self.height / 2.0);
    }

    /// Puts `b` in the middle of the area, as large as fits within `padding`
    /// pixels of margin; a twisted view fits the box as the screen shows it, turned.
    pub fn fit(&mut self, b: &Bounds, padding: f64) {
        let finite = [b.min_x, b.min_y, b.max_x, b.max_y]
            .iter()
            .all(|v| v.is_finite());
        if !finite || b.min_x > b.max_x || b.min_y > b.max_y {
            return;
        }
        let (bw, bh) = (b.max_x - b.min_x, b.max_y - b.min_y);
        let (c, s) = self.twist_cos_sin();
        let (c, s) = (c.abs(), s.abs());
        let (w, h) = if self.twist == 0.0 {
            (bw, bh)
        } else {
            (bw * c + bh * s, bw * s + bh * c)
        };
        let center = Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
        self.fit_size(center, w, h, padding);
    }

    /// Puts `center` in the middle of the area with `w` × `h` world units along the screen's
    /// axes as large as fits within `padding` pixels of margin (a named view, docs/adr/0285 §1).
    pub fn fit_size(&mut self, center: Vec2, w: f64, h: f64, padding: f64) {
        if !(center.x.is_finite() && center.y.is_finite() && w.is_finite() && h.is_finite()) {
            return;
        }
        let w = w.max(1e-6);
        let h = h.max(1e-6);
        // An area smaller than its margins keeps at least half of itself for the drawing.
        let sx = (self.width - padding * 2.0).max(self.width / 2.0) / w;
        let sy = (self.height - padding * 2.0).max(self.height / 2.0) / h;
        let scale = sx.min(sy);
        self.scale = if scale.is_finite() && scale > 0.0 {
            scale.clamp(MIN_SCALE, MAX_SCALE)
        } else {
            MIN_SCALE
        };
        self.center = center;
    }

    /// Centres the view on a world point.
    pub fn center_on(&mut self, p: Vec2) {
        if p.x.is_finite() && p.y.is_finite() {
            self.center = p;
        }
    }

    /// The world size the area shows along the screen's x and y.
    pub fn view_size(&self) -> (f64, f64) {
        (self.width / self.scale, self.height / self.scale)
    }

    /// The world box the area shows: with a twist, the box round the turned area.
    pub fn visible_bounds(&self) -> Bounds {
        if self.twist == 0.0 {
            let a = self.screen_to_world(0.0, self.height);
            let b = self.screen_to_world(self.width, 0.0);
            return Bounds {
                min_x: a.x,
                min_y: a.y,
                max_x: b.x,
                max_y: b.y,
            };
        }
        let corners = self.visible_corners();
        let mut out = Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for p in corners {
            out.min_x = out.min_x.min(p.x);
            out.min_y = out.min_y.min(p.y);
            out.max_x = out.max_x.max(p.x);
            out.max_y = out.max_y.max(p.y);
        }
        out
    }

    /// The area's corners in the world: bottom-left, bottom-right, top-right, top-left.
    pub fn visible_corners(&self) -> [Vec2; 4] {
        [
            self.screen_to_world(0.0, self.height),
            self.screen_to_world(self.width, self.height),
            self.screen_to_world(self.width, 0.0),
            self.screen_to_world(0.0, 0.0),
        ]
    }

    /// The same view with the north up over the box round the turned area:
    /// what is laid out upright and then turned as a picture (the drawing's
    /// text, docs/adr/0285 §2). Itself when there is no twist.
    pub fn upright(&self) -> Camera {
        if self.twist == 0.0 {
            return *self;
        }
        let (c, s) = self.twist_cos_sin();
        let (c, s) = (c.abs(), s.abs());
        Camera {
            center: self.center,
            scale: self.scale,
            width: (self.width * c + self.height * s).ceil(),
            height: (self.width * s + self.height * c).ceil(),
            twist: 0.0,
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
        let [twist_c, twist_s] = self.twist_uniform();
        FrameUniform {
            center_hi: [cx_hi, cy_hi],
            center_lo: [cx_lo, cy_lo],
            viewport: [size_px[0].max(1.0), size_px[1].max(1.0)],
            px_per_unit: (self.scale * scale_factor) as f32,
            dpi: scale_factor as f32,
            background: settings.background.to_f32(),
            line_width: settings.line_width,
            srgb_target: u32::from(srgb_target),
            twist_c,
            twist_s,
        }
    }

    /// The twist as the shaders take it: cos − 1 and sin, both zero with the north up.
    pub fn twist_uniform(&self) -> [f32; 2] {
        if self.twist == 0.0 {
            return [0.0, 0.0];
        }
        let (c, s) = self.twist_cos_sin();
        [(c - 1.0) as f32, s as f32]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turned(twist: f64) -> Camera {
        Camera {
            center: Vec2::new(487_000.0, 4_420_000.0),
            scale: 8.0,
            width: 800.0,
            height: 600.0,
            twist,
        }
    }

    #[test]
    fn a_twisted_view_shows_its_direction_to_the_right_and_reads_back() {
        // Twisted by 90°: the world's north points right on the screen.
        let c = turned(std::f64::consts::FRAC_PI_2);
        let [x, y] = c.world_to_screen(Vec2::new(487_000.0, 4_420_010.0));
        assert!((x - 480.0).abs() < 1e-9 && (y - 300.0).abs() < 1e-9, "{x} {y}");
        for (px, py) in [(10.0, 20.0), (400.0, 300.0), (799.0, 1.0)] {
            let w = c.screen_to_world(px, py);
            let [bx, by] = c.world_to_screen(w);
            assert!((bx - px).abs() < 1e-7 && (by - py).abs() < 1e-7);
        }
    }

    #[test]
    fn panning_and_zooming_keep_the_world_under_the_pointer_in_a_twisted_view() {
        let mut c = turned(0.7);
        let under = c.screen_to_world(100.0, 50.0);
        c.zoom_at(1.7, 100.0, 50.0);
        let [x, y] = c.world_to_screen(under);
        assert!((x - 100.0).abs() < 1e-7 && (y - 50.0).abs() < 1e-7);
        c.pan_by(30.0, -12.0);
        let [x, y] = c.world_to_screen(under);
        assert!((x - 130.0).abs() < 1e-7 && (y - 38.0).abs() < 1e-7);
    }

    #[test]
    fn the_visible_box_holds_the_turned_area_and_a_fit_shows_the_whole_box() {
        let c = turned(0.5);
        let b = c.visible_bounds();
        for p in c.visible_corners() {
            assert!(p.x >= b.min_x - 1e-9 && p.x <= b.max_x + 1e-9);
            assert!(p.y >= b.min_y - 1e-9 && p.y <= b.max_y + 1e-9);
        }
        let mut f = turned(0.5);
        let target = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 50.0,
        };
        f.fit(&target, 0.0);
        for p in [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)] {
            let [x, y] = f.world_to_screen(Vec2::new(p.0, p.1));
            assert!((-1e-6..=800.0 + 1e-6).contains(&x) && (-1e-6..=600.0 + 1e-6).contains(&y));
        }
    }

    #[test]
    fn no_twist_is_the_upright_arithmetic_bit_for_bit() {
        let c = turned(0.0);
        let p = Vec2::new(487_003.25, 4_419_998.5);
        let [x, y] = c.world_to_screen(p);
        assert_eq!(x, (p.x - c.center.x) * c.scale + c.width / 2.0);
        assert_eq!(y, c.height / 2.0 - (p.y - c.center.y) * c.scale);
        assert_eq!(c.twist_uniform(), [0.0, 0.0]);
        assert_eq!(c.upright(), c);
    }
}
