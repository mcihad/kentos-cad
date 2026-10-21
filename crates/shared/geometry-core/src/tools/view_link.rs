//! Bağlı görünümlerin bağı ve ölçeği (docs/adr/0576 §2): how the other view of a split area follows the active one
//! (nothing, the centre, the centre with the scale and the twist), the screen scale a view shows as 1:N, and the
//! time position a view's own moment stands at (§1). The desktop keeps these rules; the cases are
//! `fixtures/views/v1/links.json` (scripts/fixtures/view_link_cases.py).

use crate::time::{Step, Window, nearest, position};
use crate::vec2::Vec2;

/// Metres of paper a CSS pixel is at 96 dpi, the status bar's “Ekran 1:N” (the web's `METRES_PER_PX`).
pub const METRES_PER_PX: f64 = 0.00026458;

/// What the other view follows of the active one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    /// Nothing: each view goes its own way.
    Off,
    /// The centre alone: the other keeps its own scale and twist.
    Centre,
    /// The centre, the scale and the twist.
    CentreScale,
}

impl Link {
    pub fn from_key(key: &str) -> Option<Link> {
        Some(match key {
            "off" => Link::Off,
            "centre" => Link::Centre,
            "centreScale" => Link::CentreScale,
            _ => return None,
        })
    }

    pub fn key(self) -> &'static str {
        match self {
            Link::Off => "off",
            Link::Centre => "centre",
            Link::CentreScale => "centreScale",
        }
    }
}

/// A view's frame: the world point at its middle, its scale (pixels per metre) and its twist (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub centre: Vec2,
    pub scale: f64,
    pub twist: f64,
}

/// The other view's frame once the active one stands at `active`: unchanged without a link, the active centre
/// with the centre's, the active centre, scale and twist with the centre-and-scale's.
pub fn follow(link: Link, active: Frame, other: Frame) -> Frame {
    match link {
        Link::Off => other,
        Link::Centre => Frame {
            centre: active.centre,
            ..other
        },
        Link::CentreScale => active,
    }
}

/// A scale in pixels per metre as 1:N on a 96 dpi screen.
pub fn screen_scale(scale: f64) -> f64 {
    1.0 / (scale * METRES_PER_PX)
}

/// The window a view with a time of its own shows (docs/adr/0576 §1): the position of the slider's grid nearest
/// `moment`, a moment or up to the next position.
pub fn own_window(anchor: f64, step: Step, last: i64, ranged: bool, moment: f64) -> Window {
    Window::at(anchor, step, nearest(anchor, step, last, moment), ranged)
}

/// The position `by` steps from the one nearest `moment`, within the grid: where ◀ and ▶ of a view's own time
/// take it; its moment.
pub fn own_step(anchor: f64, step: Step, last: i64, moment: f64, by: i64) -> f64 {
    let k = (nearest(anchor, step, last, moment) + by).clamp(0, last);
    position(anchor, step, k)
}
