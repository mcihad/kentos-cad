//! A web map's zoom and a drawing's scale: the constants of the 512-pixel Web Mercator pyramid and the two ways
//! between a zoom level and a scale's denominator. A zoom is a power of two, and the powers are the numeric base's
//! (the same double on every platform).

use kentos_num::{log2, pow};

/// The metres one CSS pixel covers at zoom 0 on the equator (Web Mercator, 512-pixel tiles).
pub(crate) const METRES_PER_PX_Z0: f64 = 78_271.516_964_020_48;
/// The scale denominator at zoom 0 at the OGC standard rendering pixel (0.28 mm).
const SCALE_Z0: f64 = 279_541_132.014_358_9;
/// The highest zoom a metre width is interpolated to.
pub(crate) const TOP_ZOOM: f64 = 24.0;

/// The zoom at which the map is at a scale's denominator.
pub(crate) fn zoom_of_scale(denominator: f64) -> f64 {
    log2(SCALE_Z0 / denominator)
}

/// The scale's denominator at a zoom, to the whole number.
pub(crate) fn scale_of_zoom(z: f64) -> f64 {
    (SCALE_Z0 / pow(2.0, z)).round()
}
