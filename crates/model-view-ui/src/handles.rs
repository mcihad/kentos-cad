//! A section's handles on the scene (docs/adr/8132): a plane's one, a box's six ([`Handle`], the core's), each a dot on
//! its plane with an arrow towards the side the plane cuts away. A press on one drags it: the pointer's move along the
//! arrow's line on the screen is taken back to metres along the arrow ([`along`]), so the plane follows the pointer.

use iced::Point;
use iced::widget::canvas::{Frame, Path, Stroke};
use kentos_model_view::section::Handle;
use kentos_scene3d::math::{add, length, scale, sub};
use kentos_scene3d::{Camera, Viewport};
use kentos_ui::theme::Tokens;

/// How long an arrow is drawn and how near it (logical pixels) a press takes it.
pub const LENGTH: f32 = 34.0;
pub const REACH: f32 = 8.0;

/// A handle on the screen: its foot and its arrow's tip (the foot when the arrow points at the eye).
pub fn on_screen(h: &Handle, camera: &Camera, vp: &Viewport) -> Option<(Point, Point)> {
    let (x, y, _) = camera.project(h.at, vp)?;
    let foot = Point::new(x as f32, y as f32);
    let (dx, dy) = screen_way(h, camera, vp).map_or((0.0, 0.0), |(dx, dy)| (dx as f32, dy as f32));
    let l = dx.hypot(dy);
    let tip = if l > 0.0 { Point::new(foot.x + dx / l * LENGTH, foot.y + dy / l * LENGTH) } else { foot };
    Some((foot, tip))
}

/// The pixels a metre along the arrow takes on the screen at its foot, as a vector; none when the arrow points at the
/// eye or the foot is behind it.
fn screen_way(h: &Handle, camera: &Camera, vp: &Viewport) -> Option<(f64, f64)> {
    // A step a hundredth of the foot's distance, so the perspective is the foot's own.
    let step = (length(sub(h.at, camera.eye)) / 100.0).max(1e-6);
    let (x0, y0, _) = camera.project(h.at, vp)?;
    let (x1, y1, _) = camera.project(add(h.at, scale(h.dir, step)), vp)?;
    let (dx, dy) = ((x1 - x0) / step, (y1 - y0) / step);
    (dx.hypot(dy) > 1e-9).then_some((dx, dy))
}

/// The handle a press at `p` takes: the nearest whose foot or arrow lies within [`REACH`].
pub fn at(handles: &[Handle], camera: &Camera, vp: &Viewport, p: Point) -> Option<usize> {
    handles
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let (foot, tip) = on_screen(h, camera, vp)?;
            let d = to_segment(p, foot, tip);
            (d <= REACH).then_some((i, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The metres along the handle's arrow a pointer's move (`dx`, `dy` logical pixels) asks: its part along the arrow's
/// line on the screen over the pixels a metre takes there; 0 when the arrow points at the eye.
pub fn along(h: &Handle, camera: &Camera, vp: &Viewport, dx: f32, dy: f32) -> f64 {
    let Some((wx, wy)) = screen_way(h, camera, vp) else { return 0.0 };
    (f64::from(dx) * wx + f64::from(dy) * wy) / (wx * wx + wy * wy)
}

fn to_segment(p: Point, a: Point, b: Point) -> f32 {
    let (vx, vy) = (b.x - a.x, b.y - a.y);
    let l2 = vx * vx + vy * vy;
    let t = if l2 > 0.0 { (((p.x - a.x) * vx + (p.y - a.y) * vy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p.x - (a.x + vx * t)).hypot(p.y - (a.y + vy * t))
}

/// The handles drawn over the scene: each a white-edged accent arrow from a dot; the one under the pointer or being
/// dragged (`hot`) larger and filled.
pub(crate) fn draw(frame: &mut Frame, handles: &[Handle], camera: &Camera, vp: &Viewport, hot: Option<usize>, t: &Tokens) {
    for (i, h) in handles.iter().enumerate() {
        let Some((foot, tip)) = on_screen(h, camera, vp) else { continue };
        let lit = hot == Some(i);
        let (shaft, dot) = if lit { (3.0, 6.5) } else { (2.0, 5.0) };
        if tip != foot {
            let (dx, dy) = (tip.x - foot.x, tip.y - foot.y);
            let l = dx.hypot(dy);
            let (ux, uy) = (dx / l, dy / l);
            // The head: a triangle a third of the arrow long.
            let head = LENGTH / 3.0;
            let base = Point::new(tip.x - ux * head, tip.y - uy * head);
            let half = head * 0.55;
            let arrow = Path::new(|p| {
                p.move_to(tip);
                p.line_to(Point::new(base.x - uy * half, base.y + ux * half));
                p.line_to(Point::new(base.x + uy * half, base.y - ux * half));
                p.close();
            });
            let line = Path::line(foot, base);
            frame.stroke(&line, Stroke::default().with_color(iced::Color::WHITE.scale_alpha(0.9)).with_width(shaft + 2.5));
            frame.stroke(&arrow, Stroke::default().with_color(iced::Color::WHITE.scale_alpha(0.9)).with_width(2.5));
            frame.stroke(&line, Stroke::default().with_color(t.accent).with_width(shaft));
            frame.fill(&arrow, t.accent);
        }
        frame.fill(&Path::circle(foot, dot + 1.8), iced::Color::WHITE);
        frame.fill(&Path::circle(foot, dot), if lit { t.accent } else { t.field });
        frame.stroke(&Path::circle(foot, dot), Stroke::default().with_color(t.accent).with_width(2.0));
    }
}

#[cfg(test)]
mod tests {
    use kentos_scene3d::Projection;

    use super::*;

    /// Seen from straight above, 20 m across 200 pixels (10 a metre): a level cut's handle points at the eye (no
    /// arrow, a move along nothing); a north–south cut's handle at x 2 points east, its arrow 34 pixels to the right,
    /// a press on its arrow takes it, one beside it not; 25 pixels to the right is 2.5 m east, 25 up nothing.
    #[test]
    fn a_drag_along_a_handle_s_arrow_is_metres_along_it() {
        let camera = Camera {
            eye: [0.0, 0.0, 100.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            projection: Projection::Orthographic { height: 20.0, depth: 400.0 },
        };
        let vp = Viewport { width: 200.0, height: 200.0 };
        let level = Handle { at: [0.0, 0.0, 3.0], dir: [0.0, 0.0, 1.0] };
        let (foot, tip) = on_screen(&level, &camera, &vp).expect("in view");
        assert_eq!(foot, tip, "pointing at the eye: no arrow");
        assert_eq!(along(&level, &camera, &vp, 25.0, 0.0), 0.0);
        let east = Handle { at: [2.0, 0.0, 0.0], dir: [1.0, 0.0, 0.0] };
        let (foot, tip) = on_screen(&east, &camera, &vp).expect("in view");
        assert!((foot.x - 120.0).abs() < 1e-3 && (foot.y - 100.0).abs() < 1e-3, "{foot:?}");
        assert!((tip.x - foot.x - LENGTH).abs() < 1e-3 && (tip.y - foot.y).abs() < 1e-3, "{tip:?}");
        let handles = [level, east];
        assert_eq!(at(&handles, &camera, &vp, Point::new(140.0, 103.0)), Some(1), "on its arrow");
        assert_eq!(at(&handles, &camera, &vp, Point::new(140.0, 112.0)), None, "beside it");
        assert_eq!(at(&handles, &camera, &vp, Point::new(101.0, 101.0)), Some(0), "the level cut's dot");
        assert!((along(&east, &camera, &vp, 25.0, 0.0) - 2.5).abs() < 1e-9);
        assert!(along(&east, &camera, &vp, 0.0, -25.0).abs() < 1e-9);
    }
}
