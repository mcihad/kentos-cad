//! Obje üzerinde nokta and Km ve sapma (docs/adr/0188 §1–§2): the object
//! the calculator walks, picked on the drawing, and what is typed against
//! it. The arithmetic is the shared core's (`tools::point_calc`).

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::tools::point_calc::{Station, km_text, reading, route_of, station};
use kentos_geometry_core::tools::point_text::{is_js_space, js_trim};

use super::decimal;
use crate::format::Format;
use crate::log::Level;
use crate::tool::Context;
use crate::{Vec2, dist};

/// Why a click gave no object to walk.
pub const NO_OBJECT_HERE: &str = "Tıklanan yerde nesne yok; çizginin, yayın, dairenin, elipsin, eğrinin ya da alanın üzerine tıklayın.";
/// Why the object clicked cannot be walked.
pub const NO_ROUTE: &str = "Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin.";

/// The object walked: its route, which end it is walked from and where that is.
#[derive(Clone, Debug)]
pub struct Route {
    pub shape: Shape,
    pub from_end: bool,
    pub start: Vec2,
    pub length: f64,
}

impl Route {
    /// The object under the click that has a route, the most specific
    /// first; Obje üzerinde nokta walks it from the end nearer the click
    /// (`nearer_end`), Km from its first vertex.
    pub fn pick(at: Vec2, nearer_end: bool, cx: &mut Context<'_>) -> Option<Route> {
        let hits = cx.spatial.hits(at, cx.pick_tolerance());
        if hits.is_empty() {
            cx.say(Level::Warn, NO_OBJECT_HERE);
            return None;
        }
        let found = hits.into_iter().find_map(|slot| {
            let e = cx.doc.get(slot)?;
            route_of(&kentos_native_application::geometry::shape(e))
        });
        let Some(shape) = found else {
            cx.say(Level::Warn, NO_ROUTE);
            return None;
        };
        let first = station(&shape, false, 0.0, 0.0);
        let (Some(start), length) = (first.point, first.length) else {
            cx.say(Level::Warn, NO_ROUTE);
            return None;
        };
        let end = station(&shape, true, 0.0, 0.0).point.unwrap_or(start);
        // A closed path starts at its first vertex; an open one at the end nearer the click.
        let closed = dist(start, end) < 1e-12;
        let from_end = nearer_end && !closed && dist(at, end) < dist(at, start);
        Some(Route {
            start: if from_end { end } else { start },
            shape,
            from_end,
            length,
        })
    }

    /// The point `s` along it and `offset` square to it (metres). The
    /// length as written is the end: typing what the length is said to be
    /// reaches it.
    pub fn at(&self, s: f64, offset: f64, f: &Format) -> Station {
        let s = if s > self.length && f.length(s) == f.length(self.length) {
            self.length
        } else {
            s
        };
        station(&self.shape, self.from_end, s, offset)
    }

    /// The distance along it of the point nearest `p` and how far `p` is
    /// from it, the right positive.
    pub fn read(&self, p: Vec2) -> Option<(f64, f64)> {
        reading(&self.shape, self.from_end, p).map(|r| (r.s, r.offset))
    }
}

/// `-?[0-9]+(\.[0-9]+)?` alone (the calculator's numbers).
pub fn number(t: &str) -> Option<f64> {
    let (v, rest) = decimal(js_trim(t), true)?;
    rest.is_empty().then_some(v)
}

/// A km and an offset typed for Km ve sapma: `km`, or `km,sapma` (a `;`
/// or spaces part them too). The km is read by the core's rule
/// (`km_value`), the offset as a number.
pub fn km_and_offset(t: &str) -> Option<(f64, f64)> {
    let t = js_trim(t);
    let (km, offset) = match t.split_once([',', ';']) {
        Some((km, offset)) => (km, Some(offset)),
        None => match t.split_once(is_js_space) {
            Some((km, offset)) => (km, Some(offset)),
            None => (t, None),
        },
    };
    let km = kentos_geometry_core::tools::point_calc::km_value(km)?;
    let offset = match offset {
        Some(o) => number(o)?,
        None => 0.0,
    };
    Some((km, offset))
}

/// A km as the calculator writes it: the project's length decimals.
pub fn km(value: f64, f: &Format) -> String {
    km_text(value, f.length_decimals)
}
