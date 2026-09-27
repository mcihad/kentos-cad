//! The canvas's view (the web's `SvgCanvas` zoom and offset): drawing units
//! to screen pixels and back, fitting the canvas into the stage (with room
//! for the rulers, three canvases across for the tile preview), and zooming
//! about a point, between 20 % and 20 000 %.

use kentos_svg_core::shape::Pt;
use kentos_ui::theme::typography;

use super::doc::Drawing;
use super::state::Options;

/// The rulers' breadth at the default text size (the web's `RULER`).
pub const RULER: f32 = 18.0;

/// The rulers' breadth at the current text size.
pub fn ruler() -> f64 {
    f64::from(typography::scaled(RULER))
}

/// Zoom limits (a factor on drawing units).
const MIN: f64 = 0.2;
const MAX: f64 = 200.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub zoom: f64,
    pub ox: f64,
    pub oy: f64,
    /// The stage's size once the canvas has told it.
    pub size: Option<(f64, f64)>,
    /// Fitted once the size was known (the web's first `ResizeObserver` call).
    pub fitted: bool,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            zoom: 4.0,
            ox: 20.0,
            oy: 20.0,
            size: None,
            fitted: false,
        }
    }
}

impl Camera {
    pub fn to_doc(&self, s: Pt) -> Pt {
        [(s[0] - self.ox) / self.zoom, (s[1] - self.oy) / self.zoom]
    }

    pub fn to_screen(&self, p: Pt) -> Pt {
        [p[0] * self.zoom + self.ox, p[1] * self.zoom + self.oy]
    }

    fn stage(&self) -> (f64, f64) {
        self.size.unwrap_or((600.0, 500.0))
    }

    /// The canvas (or its tile preview) in the middle of the stage (`fit`).
    pub fn fit(&mut self, doc: &Drawing, o: &Options) {
        let r = if o.rulers { ruler() } else { 0.0 };
        let (sw, sh) = self.stage();
        let w = sw - r;
        let h = sh - r;
        let k = if o.tile { 3.0 } else { 1.0 };
        self.zoom = ((w - 48.0) / (doc.width * k))
            .min((h - 48.0) / (doc.height * k))
            .clamp(MIN, MAX);
        self.ox = r + (w - doc.width * self.zoom) / 2.0;
        self.oy = r + (h - doc.height * self.zoom) / 2.0;
    }

    /// Zooms by `f` about a screen point (the stage's middle when none).
    pub fn zoom_by(&mut self, f: f64, at: Option<Pt>) {
        let (sw, sh) = self.stage();
        let [cx, cy] = at.unwrap_or([sw / 2.0, sh / 2.0]);
        let z = (self.zoom * f).clamp(MIN, MAX);
        self.ox = cx - ((cx - self.ox) * z) / self.zoom;
        self.oy = cy - ((cy - self.oy) * z) / self.zoom;
        self.zoom = z;
    }

    /// The zoom as the bar writes it: `%580`.
    pub fn percent(&self) -> String {
        format!(
            "%{}",
            kentos_native_style::classify::js_round(self.zoom * 100.0)
        )
    }
}
