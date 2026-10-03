//! Meridian convergence of a transverse Mercator projection (design §8): how
//! far grid north is turned from true north at a map's centre, so a north
//! arrow can show true and magnetic north. `kentos-geometry-core` has no
//! projection arithmetic yet, so the sheet core computes it from the
//! projection's parameters the host gives (Karney 2011, the Krüger series
//! to the third order in n: far below a second of arc inside a zone); the
//! host's own value is only the fallback (docs/sheet/tasks-rust.md
//! §Sapmalar). `libm` keeps native and WASM bit for bit equal.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A transverse Mercator projection (TM3 zones, UTM): what `geo/crs.ts` holds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TmParams {
    /// Degrees east of Greenwich.
    pub central_meridian: f64,
    pub scale_factor: f64,
    pub false_easting: f64,
    pub false_northing: f64,
    /// The ellipsoid: semi-major axis (m) and inverse flattening (GRS80: 6 378 137, 298.257 222 101).
    pub semi_major: f64,
    pub inverse_flattening: f64,
}

/// Latitude and longitude (degrees) and the convergence (degrees, positive where grid north is east of true north, east of the central meridian in the north).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geographic {
    pub lat: f64,
    pub lon: f64,
    pub convergence: f64,
}

/// The point (east, north) of a TM grid in geographic terms; none for parameters that make no projection.
pub fn tm_inverse(p: &TmParams, east: f64, north: f64) -> Option<Geographic> {
    if !(p.semi_major > 0.0
        && p.inverse_flattening > 1.0
        && p.scale_factor > 0.0
        && east.is_finite()
        && north.is_finite())
    {
        return None;
    }
    let f = 1.0 / p.inverse_flattening;
    let n = f / (2.0 - f);
    let (n2, n3) = (n * n, n * n * n);
    let a_big = p.semi_major / (1.0 + n) * (1.0 + n2 / 4.0 + n2 * n2 / 64.0);
    let beta = [
        n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0,
        n2 / 48.0 + n3 / 15.0,
        17.0 * n3 / 480.0,
    ];
    let delta = [
        2.0 * n - 2.0 * n2 / 3.0 - 2.0 * n3,
        7.0 * n2 / 3.0 - 8.0 * n3 / 5.0,
        56.0 * n3 / 15.0,
    ];
    let k0a = p.scale_factor * a_big;
    let xi = (north - p.false_northing) / k0a;
    let eta = (east - p.false_easting) / k0a;
    let (mut xi_p, mut eta_p, mut sigma_p, mut tau_p) = (xi, eta, 1.0, 0.0);
    for (j, b) in beta.iter().enumerate() {
        let k = 2.0 * (j as f64 + 1.0);
        let (s, c) = (libm::sin(k * xi), libm::cos(k * xi));
        let (sh, ch) = (libm::sinh(k * eta), libm::cosh(k * eta));
        xi_p -= b * s * ch;
        eta_p -= b * c * sh;
        sigma_p -= k * b * c * ch;
        tau_p += k * b * s * sh;
    }
    let chi = libm::asin(libm::sin(xi_p) / libm::cosh(eta_p));
    let mut phi = chi;
    for (j, d) in delta.iter().enumerate() {
        phi += d * libm::sin(2.0 * (j as f64 + 1.0) * chi);
    }
    let lambda = libm::atan2(libm::sinh(eta_p), libm::cos(xi_p));
    let tt = libm::tan(xi_p) * libm::tanh(eta_p);
    let gamma = libm::atan2(tau_p + sigma_p * tt, sigma_p - tau_p * tt);
    let deg = 180.0 / core::f64::consts::PI;
    Some(Geographic {
        lat: phi * deg,
        lon: p.central_meridian + lambda * deg,
        convergence: gamma * deg,
    })
}

/// An angle in degrees as degrees, minutes and seconds: “0°38'12"”; the sign is the caller's.
/// The minute and second marks are the apostrophe and the quotation mark, as Turkish
/// surveying writes them on a keyboard: every drawing face draws them as narrow ticks, where
/// Barlow's prime (U+2032) is 0.6 em wide, nearly half of it empty; and a PDF is searched for
/// what a user types.
pub fn dms(deg: f64, seconds_decimals: u8) -> String {
    let a = deg.abs();
    let scale = [1.0, 10.0, 100.0, 1000.0][usize::from(seconds_decimals.min(3))];
    // Whole units of the last place shown, so 59.99" never prints as 60".
    let total = libm::round(a * 3600.0 * scale) as i64;
    let per_deg = (3600.0 * scale) as i64;
    let per_min = (60.0 * scale) as i64;
    let d = total / per_deg;
    let m = (total % per_deg) / per_min;
    let s = total % per_min;
    let sec = if seconds_decimals == 0 {
        format!("{s:02}")
    } else {
        let whole = s / scale as i64;
        let frac = s % scale as i64;
        format!(
            "{whole:02}.{frac:0w$}",
            w = usize::from(seconds_decimals.min(3))
        )
    };
    format!("{d}°{m:02}'{sec}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tm36() -> TmParams {
        TmParams {
            central_meridian: 36.0,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            semi_major: 6_378_137.0,
            inverse_flattening: 298.257_222_101,
        }
    }

    #[test]
    fn convergence_matches_proj() {
        // PROJ 9 (pyproj 3.7.2) `+proj=tmerc +lon_0=36 +k=1 +x_0=500000 +ellps=GRS80`, inverse and get_factors.
        let cases = [
            (
                600_000.0,
                4_400_000.0,
                39.728_189_574,
                37.166_417_793,
                0.745_573_252,
            ),
            (
                400_000.0,
                4_400_000.0,
                39.728_189_574,
                34.833_582_207,
                -0.745_573_252,
            ),
            (500_000.0, 4_400_000.0, 39.734_049_559, 36.0, 0.0),
            (
                485_200.0,
                4_512_300.0,
                40.745_270_648,
                35.824_765_563,
                -0.114_375_237,
            ),
            (
                650_000.0,
                4_200_000.0,
                37.920_081_931,
                37.705_884_895,
                1.048_566_818,
            ),
        ];
        for (e, n, lat, lon, conv) in cases {
            let g = tm_inverse(&tm36(), e, n).unwrap();
            assert!((g.lat - lat).abs() < 1e-8, "{e} {n}: lat {}", g.lat);
            assert!((g.lon - lon).abs() < 1e-8, "{e} {n}: lon {}", g.lon);
            assert!(
                (g.convergence - conv).abs() < 1e-8,
                "{e} {n}: conv {}",
                g.convergence
            );
        }
        assert!(
            tm_inverse(
                &TmParams {
                    semi_major: 0.0,
                    ..tm36()
                },
                0.0,
                0.0
            )
            .is_none()
        );
    }

    #[test]
    fn angles_are_written_in_degrees_minutes_seconds() {
        assert_eq!(dms(0.745_573_252, 0), "0°44'44\"");
        assert_eq!(dms(-1.048_566_818, 1), "1°02'54.8\"");
        assert_eq!(dms(5.2, 0), "5°12'00\"");
        assert_eq!(dms(0.999_999_9, 0), "1°00'00\"");
    }
}
