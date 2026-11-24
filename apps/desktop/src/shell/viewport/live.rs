//! The drawing area's live view (plan/render.md RM-05, docs/adr/9205): the camera the area draws with while a gesture
//! runs, apart from the app.
//!
//! The wheel and a middle-button pan change this camera in the area's own widget, which then asks only for a frame.
//! Such a frame redraws the interface without the app's update or view; the ribbon, the panels and the drawing's
//! overlays are not built again. The scene reads this camera when it draws. The overlays built for the app's camera
//! are moved and scaled onto it (`follow.rs`), so nothing on the drawing stays where the view left it.
//!
//! Once the view has been still for [`SETTLE`], the area tells the app one change, from the camera the gesture
//! started at to the one it stopped at (`Event::ViewChanged`). The app takes it as its camera, as it took each turn
//! of the wheel before, and builds its view once. The heavy work of a view change happens then, once per gesture:
//! the labels placed, the curves' zoom band, screen-sized symbols. Before, it ran on every turn: 100 to 200 ms a
//! notch on a million objects (docs/adr/9201 §3).
//!
//! The app's camera stays the one the app reads; a change the app makes itself (a fit, Önceki görünüm, a resize) is
//! pushed here the next time its view is built (`Viewport::view`). An event the app takes while a gesture runs (the
//! pointer moved, a click) first takes the gesture's camera (`Viewport::adopt_live`), so a world point is never read
//! through a camera the screen no longer shows.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use iced::time::Instant;

use iced::{Point, Vector};
use kentos_render_wgpu::Camera;

/// How long the view stays still before the app is told: as long as the labels' settle (docs/adr/0597).
pub const SETTLE: Duration = Duration::from_millis(150);

/// The live camera, shared between the app's view state and the area's widget.
#[derive(Clone, Debug)]
pub struct LiveView(Arc<Mutex<Live>>);

#[derive(Debug)]
struct Live {
    camera: Camera,
    /// The camera a running gesture started at, and its last move; none when the app has the camera the area shows.
    gesture: Option<(Camera, Instant)>,
}

impl LiveView {
    pub fn new(camera: Camera) -> Self {
        Self(Arc::new(Mutex::new(Live { camera, gesture: None })))
    }

    fn with<T>(&self, f: impl FnOnce(&mut Live) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The camera the area shows now.
    pub fn camera(&self) -> Camera {
        self.with(|l| l.camera)
    }

    /// Whether a gesture moved the view since the app last took its camera.
    pub fn moving(&self) -> bool {
        self.with(|l| l.gesture.is_some())
    }

    /// The wheel turned: `factor` about `at` (logical pixels in the area), as `Camera::zoom_at`.
    pub fn zoom(&self, factor: f64, at: Point, now: Instant) {
        self.moved(now, |c| c.zoom_at(factor, f64::from(at.x), f64::from(at.y)));
    }

    /// A middle-button drag moved the view by `by` pixels, as `Camera::pan_by`.
    pub fn pan(&self, by: Vector, now: Instant) {
        self.moved(now, |c| c.pan_by(f64::from(by.x), f64::from(by.y)));
    }

    fn moved(&self, now: Instant, change: impl FnOnce(&mut Camera)) {
        self.with(|l| {
            let from = l.gesture.map_or(l.camera, |(from, _)| from);
            change(&mut l.camera);
            l.gesture = Some((from, now));
        });
    }

    /// When the running gesture settles: its last move and [`SETTLE`]; none without one.
    pub fn settles_at(&self) -> Option<Instant> {
        self.with(|l| l.gesture.map(|(_, at)| at + SETTLE))
    }

    /// The running gesture handed to the app: the camera it started at and the one the area shows. None without one.
    pub fn take(&self) -> Option<(Camera, Camera)> {
        self.with(|l| l.gesture.take().map(|(from, _)| (from, l.camera)))
    }

    /// The app's own camera: a fit, a step back in the history, a resize. It ends any gesture: the app's change wins.
    pub fn set(&self, camera: Camera) {
        self.with(|l| {
            l.camera = camera;
            l.gesture = None;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        let mut c = Camera::default();
        c.set_size(800.0, 600.0);
        c
    }

    #[test]
    fn a_gesture_moves_the_live_camera_and_hands_the_app_where_it_started() {
        let live = LiveView::new(camera());
        let start = live.camera();
        let t0 = Instant::now();
        live.zoom(1.15, Point::new(100.0, 100.0), t0);
        live.pan(Vector::new(30.0, -10.0), t0 + Duration::from_millis(40));
        assert!(live.moving());
        assert_eq!(live.settles_at(), Some(t0 + Duration::from_millis(40) + SETTLE), "the last move counts");
        let mut expected = start;
        expected.zoom_at(1.15, 100.0, 100.0);
        expected.pan_by(30.0, -10.0);
        let (from, to) = live.take().expect("a gesture");
        assert_eq!((from, to), (start, expected), "the camera's own math, from where the gesture started");
        assert!(!live.moving() && live.take().is_none(), "told once");
    }

    #[test]
    fn the_apps_own_change_ends_a_gesture() {
        let live = LiveView::new(camera());
        live.zoom(2.0, Point::new(10.0, 10.0), Instant::now());
        let mut fitted = camera();
        fitted.center_on(kentos_render_wgpu::Vec2::new(500.0, 500.0));
        live.set(fitted);
        assert!(!live.moving());
        assert_eq!(live.camera(), fitted);
        assert!(live.take().is_none());
    }
}
