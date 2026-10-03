//! Where the paper is on the stage: the page fitted (the default) or a zoom
//! and the paper point at the stage's middle. 100 % is the paper's real size
//! on a 96 dpi screen, as on the web.

use iced::{Point, Size};
use kentos_sheet::units::{RectUm, SizeUm};

use crate::paint::Xf;

/// Pixels per millimetre at 100 %.
pub const REAL: f64 = 96.0 / 25.4;
/// The room around a fitted page, pixels.
const ROOM: f64 = 28.0;
pub const MIN_ZOOM: f64 = REAL * 0.05;
pub const MAX_ZOOM: f64 = REAL * 16.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct View {
    /// Pixels per millimetre; none: the page fitted to the stage.
    pub zoom: Option<f64>,
    /// The paper point at the stage's middle, millimetres (with a zoom).
    pub center: [f64; 2],
}

/// The zoom that fits the page in the stage with some room around it.
pub fn fit(stage: Size, paper: SizeUm) -> f64 {
    let (pw, ph) = (
        f64::from(paper.width) / 1000.0,
        f64::from(paper.height) / 1000.0,
    );
    if pw <= 0.0 || ph <= 0.0 {
        return REAL;
    }
    let w = (f64::from(stage.width) - 2.0 * ROOM).max(40.0);
    let h = (f64::from(stage.height) - 2.0 * ROOM).max(40.0);
    (w / pw).min(h / ph).clamp(MIN_ZOOM, MAX_ZOOM)
}

impl View {
    /// Pixels per millimetre on a stage of this size.
    pub fn zoom_on(&self, stage: Size, paper: SizeUm) -> f64 {
        self.zoom.unwrap_or_else(|| fit(stage, paper))
    }

    /// The paper's transform on a stage of this size.
    pub fn xf(&self, stage: Size, paper: SizeUm) -> Xf {
        let z = self.zoom_on(stage, paper);
        let c = match self.zoom {
            Some(_) => self.center,
            None => [
                f64::from(paper.width) / 2000.0,
                f64::from(paper.height) / 2000.0,
            ],
        };
        Xf::new(
            z / 1000.0,
            Point::new(
                (f64::from(stage.width) / 2.0 - c[0] * z) as f32,
                (f64::from(stage.height) / 2.0 - c[1] * z) as f32,
            ),
        )
    }

    /// The zoom in per cent of the real size.
    pub fn percent(&self, stage: Size, paper: SizeUm) -> u32 {
        (self.zoom_on(stage, paper) / REAL * 100.0).round() as u32
    }

    /// Zoomed by `factor` about the stage pixel `px`: the paper point under it stays under it.
    pub fn zoomed(&self, factor: f64, px: Point, stage: Size, paper: SizeUm) -> View {
        let xf = self.xf(stage, paper);
        let under = xf.paper(px);
        let z = (self.zoom_on(stage, paper) * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // The new centre puts `under` at `px` again.
        let (cx, cy) = (f64::from(stage.width) / 2.0, f64::from(stage.height) / 2.0);
        View {
            zoom: Some(z),
            center: [
                under[0] / 1000.0 - (f64::from(px.x) - cx) / z,
                under[1] / 1000.0 - (f64::from(px.y) - cy) / z,
            ],
        }
    }

    /// Moved by pixels.
    pub fn panned(&self, dx: f32, dy: f32, stage: Size, paper: SizeUm) -> View {
        let z = self.zoom_on(stage, paper);
        let c = match self.zoom {
            Some(_) => self.center,
            None => [
                f64::from(paper.width) / 2000.0,
                f64::from(paper.height) / 2000.0,
            ],
        };
        View {
            zoom: Some(z),
            center: [c[0] - f64::from(dx) / z, c[1] - f64::from(dy) / z],
        }
    }

    /// A box of the paper (the chosen items') filling the stage less `pad` pixels a side
    /// (the web's `zoomChoice`).
    pub fn around(stage: Size, rect: RectUm, pad: f64) -> View {
        let (w, h) = (
            (f64::from(rect.width) / 1000.0).max(0.001),
            (f64::from(rect.height) / 1000.0).max(0.001),
        );
        let room_w = (f64::from(stage.width) - 2.0 * pad).max(40.0);
        let room_h = (f64::from(stage.height) - 2.0 * pad).max(40.0);
        View {
            zoom: Some((room_w / w).min(room_h / h).clamp(MIN_ZOOM, MAX_ZOOM)),
            center: [
                (f64::from(rect.left) + f64::from(rect.width) / 2.0) / 1000.0,
                (f64::from(rect.top) + f64::from(rect.height) / 2.0) / 1000.0,
            ],
        }
    }

    /// The real size (100 %), the stage's middle kept.
    pub fn real(&self, stage: Size, paper: SizeUm) -> View {
        let px = Point::new(stage.width / 2.0, stage.height / 2.0);
        let f = REAL / self.zoom_on(stage, paper);
        self.zoomed(f, px, stage, paper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A3: SizeUm = SizeUm {
        width: 420_000,
        height: 297_000,
    };

    /// The chosen items' box in the stage's middle, as large as fits with the room around it.
    #[test]
    fn a_box_fills_the_stage_less_its_room() {
        let stage = Size::new(1000.0, 700.0);
        let rect = RectUm {
            left: 300_000,
            top: 20_000,
            width: 50_000,
            height: 20_000,
        };
        let v = View::around(stage, rect, 72.0);
        assert_eq!(v.zoom, Some((1000.0 - 144.0) / 50.0));
        let xf = v.xf(stage, A3);
        let mid = xf.p(325_000.0, 30_000.0);
        assert!(
            (mid.x - 500.0).abs() < 0.01 && (mid.y - 350.0).abs() < 0.01,
            "{mid:?}"
        );
        // A tiny box: no closer than the largest zoom.
        let dot = RectUm {
            width: 10,
            height: 10,
            ..rect
        };
        assert_eq!(View::around(stage, dot, 72.0).zoom, Some(MAX_ZOOM));
    }

    #[test]
    fn a_fitted_page_sits_in_the_middle_and_a_zoom_keeps_its_point() {
        let stage = Size::new(1000.0, 700.0);
        let v = View::default();
        let xf = v.xf(stage, A3);
        // The page's middle is the stage's.
        let mid = xf.p(210_000.0, 148_500.0);
        assert!(
            (mid.x - 500.0).abs() < 0.01 && (mid.y - 350.0).abs() < 0.01,
            "{mid:?}"
        );
        // Zoomed about a point: the paper point under it stays.
        let px = Point::new(300.0, 200.0);
        let under = xf.paper(px);
        let z = v.zoomed(2.0, px, stage, A3);
        let back = z.xf(stage, A3).p(under[0], under[1]);
        assert!(
            (back.x - px.x).abs() < 0.01 && (back.y - px.y).abs() < 0.01,
            "{back:?}"
        );
        assert_eq!(v.real(stage, A3).percent(stage, A3), 100);
        // A pan moves the paper with the pointer.
        let p = z.panned(10.0, -5.0, stage, A3);
        let moved = p.xf(stage, A3).p(under[0], under[1]);
        assert!(
            (moved.x - 310.0).abs() < 0.01 && (moved.y - 195.0).abs() < 0.01,
            "{moved:?}"
        );
    }
}
