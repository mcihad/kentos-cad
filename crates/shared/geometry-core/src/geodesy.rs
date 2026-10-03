//! Projection arithmetic (docs/adr/0165 §3): a latitude and longitude on an
//! ellipsoid to a transverse Mercator grid (the TM3 zones, UTM), for a new
//! project's start view on a province's centre. Karney (2011), “Transverse
//! Mercator with an accuracy of a few nanometers”, the Krüger series to the
//! sixth order in n, as PROJ's `tmerc`; the cases are PROJ's
//! (fixtures/geodesy/v1/tm-forward.json, scripts/fixtures/tm_cases.py).
//! `libm` keeps native and WASM bit for bit equal. The way back is the
//! sheet core's `geodesy::tm_inverse`.

use crate::api::Op;
use crate::jsmath::PI;
use crate::vec2::Vec2;

/// A transverse Mercator projection: what the coordinate system registry
/// holds of a TM3 zone or a UTM zone, with its ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tm {
    /// Degrees east of Greenwich.
    pub central_meridian: f64,
    pub scale_factor: f64,
    pub false_easting: f64,
    pub false_northing: f64,
    /// The ellipsoid's semi-major axis, metres.
    pub semi_major: f64,
    pub inverse_flattening: f64,
}

crate::json_struct!(Tm {
    central_meridian => "centralMeridian",
    scale_factor => "scaleFactor",
    false_easting => "falseEasting",
    false_northing => "falseNorthing",
    semi_major => "semiMajor",
    inverse_flattening => "inverseFlattening",
});

/// The point at `lat`, `lon` (degrees) on the projection's grid: east and
/// north, the core's x and y. None for parameters that make no projection,
/// or a point the projection cannot reach (on the equator 90° from the
/// central meridian).
pub fn tm_forward(p: &Tm, lat: f64, lon: f64) -> Option<Vec2> {
    if !(p.semi_major > 0.0
        && p.inverse_flattening > 1.0
        && p.scale_factor > 0.0
        && lat.is_finite()
        && lon.is_finite()
        && lat.abs() <= 90.0)
    {
        return None;
    }
    let f = 1.0 / p.inverse_flattening;
    let e = libm::sqrt(f * (2.0 - f));
    let n = f / (2.0 - f);
    let (n2, n3) = (n * n, n * n * n);
    let (n4, n5, n6) = (n2 * n2, n2 * n3, n3 * n3);
    // The rectifying radius and the Krüger coefficients (Karney 2011, eqs. 14 and 35).
    let a_big = p.semi_major / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
    let alpha = [
        n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0 - 127.0 * n5 / 288.0
            + 7891.0 * n6 / 37800.0,
        13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0 + 281.0 * n5 / 630.0
            - 1_983_433.0 * n6 / 1_935_360.0,
        61.0 * n3 / 240.0 - 103.0 * n4 / 140.0
            + 15061.0 * n5 / 26880.0
            + 167_603.0 * n6 / 181_440.0,
        49561.0 * n4 / 161_280.0 - 179.0 * n5 / 168.0 + 6_601_661.0 * n6 / 7_257_600.0,
        34729.0 * n5 / 80640.0 - 3_418_889.0 * n6 / 1_995_840.0,
        212_378_941.0 * n6 / 319_334_400.0,
    ];
    let rad = PI / 180.0;
    let phi = lat * rad;
    let lambda = (lon - p.central_meridian) * rad;
    // The conformal latitude's tangent, then the spherical transverse Mercator (ξ′, η′).
    let s = libm::sin(phi);
    let t = libm::sinh(libm::atanh(s) - e * libm::atanh(e * s));
    let xi_p = libm::atan2(t, libm::cos(lambda));
    let eta_p = libm::atanh(libm::sin(lambda) / libm::sqrt(1.0 + t * t));
    let (mut xi, mut eta) = (xi_p, eta_p);
    for (j, a) in alpha.iter().enumerate() {
        let k = 2.0 * (j as f64 + 1.0);
        xi += a * libm::sin(k * xi_p) * libm::cosh(k * eta_p);
        eta += a * libm::cos(k * xi_p) * libm::sinh(k * eta_p);
    }
    let k0a = p.scale_factor * a_big;
    let east = p.false_easting + k0a * eta;
    let north = p.false_northing + k0a * xi;
    (east.is_finite() && north.is_finite()).then(|| Vec2::new(east, north))
}

pub(crate) static OPS: &[Op] = &[crate::op!("tmForward", |p: Tm, lat: f64, lon: f64| {
    tm_forward(&p, lat, lon)
})];

#[cfg(test)]
mod tests {
    use super::*;

    fn tm36() -> Tm {
        Tm {
            central_meridian: 36.0,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            semi_major: 6_378_137.0,
            inverse_flattening: 298.257_222_101,
        }
    }

    /// On the central meridian east is the false easting; north of the
    /// equator north grows with the latitude; a point and its mirror in the
    /// central meridian are mirrored; what cannot be projected is none.
    #[test]
    fn the_grid_keeps_its_symmetries() {
        let p = tm36();
        let on = tm_forward(&p, 39.0, 36.0).expect("a point");
        assert_eq!(on.x, 500_000.0);
        assert_eq!(tm_forward(&p, 0.0, 36.0).expect("the origin").y, 0.0);
        let (e, w) = (
            tm_forward(&p, 39.0, 37.5).expect("east"),
            tm_forward(&p, 39.0, 34.5).expect("west"),
        );
        assert!((e.x - 500_000.0 + (w.x - 500_000.0)).abs() < 1e-6 && e.y == w.y);
        assert!(tm_forward(&p, 40.0, 36.0).expect("north").y > on.y);
        assert_eq!(tm_forward(&p, 0.0, 126.0), None);
        assert_eq!(tm_forward(&p, 91.0, 36.0), None);
        assert_eq!(
            tm_forward(
                &Tm {
                    scale_factor: 0.0,
                    ..p
                },
                39.0,
                36.0
            ),
            None
        );
    }
}
