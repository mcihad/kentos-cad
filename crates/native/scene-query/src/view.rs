//! What a tool and the spatial index ask of the view they work through: screen and world, its zoom, the
//! box it shows, its twist.

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::vec2::Vec2;

/// The drawing area as a tool needs it: the camera's mapping, nothing else.
pub trait View {
    /// Physical viewport denominator when editing through paper, independent of screen zoom.
    fn paper_scale(&self) -> Option<f64> {
        None
    }
    /// Where a world point is on the area, in logical pixels.
    fn to_screen(&self, p: Vec2) -> [f64; 2];
    /// How much world `px` logical pixels span at the current zoom.
    fn world_length(&self, px: f64) -> f64;
    /// The world box the area shows: the edges a trim or an extend meets by
    /// default (the web's `camera.visibleBounds()`); with a twist, the box round the turned area.
    fn visible(&self) -> Bounds;
    /// The view's twist (docs/adr/0285 §2): the angle of the screen's x axis from the world's, radians; 0 with the
    /// north up.
    fn twist(&self) -> f64 {
        0.0
    }
    /// The world point under a place of the area, in logical pixels; none where the view cannot say.
    fn to_world(&self, _p: [f64; 2]) -> Option<Vec2> {
        None
    }
}
