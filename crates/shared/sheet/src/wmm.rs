//! The World Magnetic Model (design §8a): NOAA/NCEI's WMM2025, its
//! coefficients in `data/wmm2025.json` (made from the official coefficient
//! file by `scripts/geodesy/wmm_coefficients.py`; the file names its source,
//! release, validity and public-domain status). The field at a geodetic place
//! and a decimal year as NOAA's own `geomag` computes it: the place turned
//! into geocentric spherical terms on WGS84, the coefficients carried to the
//! year by their secular variation, Schmidt semi-normalised spherical
//! harmonics to degree 12, the field turned back into the geodetic frame.
//! `libm` everywhere, so the browser and the desktop agree bit for bit. No
//! grid variation: the core asks the declination only away from the poles.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The table as the script writes it (the fields the core reads).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Table {
    model: String,
    released: String,
    valid_from: f64,
    valid_until: f64,
    epoch: f64,
    max_degree: usize,
    /// n, m, g, h (nT), ġ, ḣ (nT a year).
    coefficients: Vec<[f64; 6]>,
}

fn table() -> Option<&'static Table> {
    static T: OnceLock<Option<Table>> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("../data/wmm2025.json")).ok())
        .as_ref()
}

/// The model a host names (`wmmInfo`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct WmmInfo {
    /// “WMM2025”.
    pub model: String,
    /// ISO date of its release.
    pub released: String,
    /// The decimal years it is valid for, both ends included.
    pub valid_from: f64,
    pub valid_until: f64,
}

/// The magnetic field at a place and time.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MagneticField {
    /// Declination: from true north to magnetic north, degrees, east positive.
    pub declination: f64,
    /// Inclination: below the horizontal, degrees, down positive.
    pub inclination: f64,
    /// North, east and down components, nT.
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Horizontal and total intensity, nT.
    pub h: f64,
    pub f: f64,
}

pub fn info() -> Option<WmmInfo> {
    let t = table()?;
    Some(WmmInfo {
        model: t.model.clone(),
        released: t.released.clone(),
        valid_from: t.valid_from,
        valid_until: t.valid_until,
    })
}

/// Whether a decimal year is inside the model's validity (2025.0 … 2030.0).
pub fn valid(year: f64) -> bool {
    table().is_some_and(|t| year >= t.valid_from && year <= t.valid_until)
}

/// An ISO date (`2026-10-03`, a longer ISO text read to its day) as a decimal year, as NOAA's
/// software counts it: the year and the days before the date over the year's days.
pub fn decimal_year(iso: &str) -> Option<f64> {
    let d = iso.trim().get(..10)?;
    let mut it = d.split('-');
    let year: i32 = it.next()?.parse().ok()?;
    let month: usize = it.next()?.parse().ok()?;
    let day: u32 = it.next()?.parse().ok()?;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month) || day == 0 || day > days[month - 1] {
        return None;
    }
    let before: u32 = days[..month - 1].iter().sum::<u32>() + day - 1;
    Some(f64::from(year) + f64::from(before) / if leap { 366.0 } else { 365.0 })
}

/// The field at geodetic latitude and longitude (degrees, WGS84), height above the ellipsoid
/// (km) and decimal year; none when the table cannot be read or the place is not one.
pub fn field(lat: f64, lon: f64, height_km: f64, year: f64) -> Option<MagneticField> {
    let t = table()?;
    if !(lat.is_finite() && lon.is_finite() && height_km.is_finite() && year.is_finite())
        || lat.abs() > 90.0
    {
        return None;
    }
    let max = t.max_degree.clamp(1, 12);
    // WGS84 and the geomagnetic reference radius, km.
    let a = 6378.137_f64;
    let b = a * (1.0 - 1.0 / 298.257_223_563);
    let re = 6371.2_f64;
    let (a2, b2) = (a * a, b * b);
    let c2 = a2 - b2;
    let (a4, b4) = (a2 * a2, b2 * b2);
    let c4 = a4 - b4;
    // The coefficients as `geomag` keeps them: g at [m][n], h at [n][m − 1], Schmidt normalised.
    let mut c = [[0.0_f64; 13]; 13];
    let mut cd = [[0.0_f64; 13]; 13];
    for row in &t.coefficients {
        let (n, m) = (row[0] as usize, row[1] as usize);
        if n == 0 || n > max || m > n {
            continue;
        }
        c[m][n] = row[2];
        cd[m][n] = row[4];
        if m != 0 {
            c[n][m - 1] = row[3];
            cd[n][m - 1] = row[5];
        }
    }
    let mut snorm = [[0.0_f64; 13]; 13];
    let mut k = [[0.0_f64; 13]; 13];
    snorm[0][0] = 1.0;
    for n in 1..=max {
        let nf = n as f64;
        snorm[n][0] = snorm[n - 1][0] * (2.0 * nf - 1.0) / nf;
        let mut j = 2.0;
        for m in 0..=n {
            let mf = m as f64;
            k[m][n] = ((nf - 1.0) * (nf - 1.0) - mf * mf) / ((2.0 * nf - 1.0) * (2.0 * nf - 3.0));
            if m > 0 {
                let flnmj = ((nf - mf + 1.0) * j) / (nf + mf);
                snorm[n][m] = snorm[n][m - 1] * libm::sqrt(flnmj);
                j = 1.0;
                c[n][m - 1] *= snorm[n][m];
                cd[n][m - 1] *= snorm[n][m];
            }
            c[m][n] *= snorm[n][m];
            cd[m][n] *= snorm[n][m];
        }
    }
    let dt = year - t.epoch;
    let (rlat, rlon) = (lat.to_radians(), lon.to_radians());
    let (srlon, crlon) = (libm::sin(rlon), libm::cos(rlon));
    let (srlat, crlat) = (libm::sin(rlat), libm::cos(rlat));
    let (srlat2, crlat2) = (srlat * srlat, crlat * crlat);
    let mut sp = [0.0_f64; 13];
    let mut cp = [0.0_f64; 13];
    cp[0] = 1.0;
    sp[1] = srlon;
    cp[1] = crlon;
    for m in 2..=max {
        sp[m] = sp[1] * cp[m - 1] + cp[1] * sp[m - 1];
        cp[m] = cp[1] * cp[m - 1] - sp[1] * sp[m - 1];
    }
    // The geodetic place in geocentric spherical terms: the colatitude's cosine and sine, the
    // radius, and the turn back to the geodetic frame.
    let alt = height_km;
    let q = libm::sqrt(a2 - c2 * srlat2);
    let q1 = alt * q;
    let q2 = ((q1 + a2) / (q1 + b2)) * ((q1 + a2) / (q1 + b2));
    let ct = srlat / libm::sqrt(q2 * crlat2 + srlat2);
    let st = libm::sqrt((1.0 - ct * ct).max(0.0));
    let r2 = alt * alt + 2.0 * q1 + (a4 - c4 * srlat2) / (q * q);
    let r = libm::sqrt(r2);
    let d = libm::sqrt(a2 * crlat2 + b2 * srlat2);
    let ca = (alt + d) / r;
    let sa = c2 * crlat * srlat / (r * d);
    let aor = re / r;
    let mut ar = aor * aor;
    // Gauss-normalised Legendre functions p[n][m] and their colatitude derivatives dp[m][n].
    let mut p = [[0.0_f64; 13]; 13];
    let mut dp = [[0.0_f64; 13]; 13];
    let mut pp = [0.0_f64; 13];
    p[0][0] = 1.0;
    pp[0] = 1.0;
    let (mut br, mut bt, mut bp, mut bpp) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for n in 1..=max {
        ar *= aor;
        for m in 0..=n {
            if n == m {
                p[n][m] = st * p[n - 1][m - 1];
                dp[m][n] = st * dp[m - 1][n - 1] + ct * p[n - 1][m - 1];
            } else if n == 1 && m == 0 {
                p[n][m] = ct * p[n - 1][m];
                dp[m][n] = ct * dp[m][n - 1] - st * p[n - 1][m];
            } else if n > 1 {
                if m + 2 > n {
                    p[n - 2][m] = 0.0;
                    dp[m][n - 2] = 0.0;
                }
                p[n][m] = ct * p[n - 1][m] - k[m][n] * p[n - 2][m];
                dp[m][n] = ct * dp[m][n - 1] - st * p[n - 1][m] - k[m][n] * dp[m][n - 2];
            }
            // The coefficients at the year.
            let g = c[m][n] + dt * cd[m][n];
            let h = if m == 0 {
                0.0
            } else {
                c[n][m - 1] + dt * cd[n][m - 1]
            };
            let par = ar * p[n][m];
            let (temp1, temp2) = if m == 0 {
                (g * cp[m], g * sp[m])
            } else {
                (g * cp[m] + h * sp[m], g * sp[m] - h * cp[m])
            };
            bt -= ar * temp1 * dp[m][n];
            bp += m as f64 * temp2 * par;
            br += (n as f64 + 1.0) * temp1 * par;
            // At a geographic pole the east component comes from its own series.
            if st == 0.0 && m == 1 {
                pp[n] = if n == 1 {
                    pp[n - 1]
                } else {
                    ct * pp[n - 1] - k[m][n] * pp[n - 2]
                };
                bpp += m as f64 * temp2 * ar * pp[n];
            }
        }
    }
    let bp = if st == 0.0 { bpp } else { bp / st };
    // From the spherical frame to the geodetic: north, east, down.
    let x = -bt * ca - br * sa;
    let y = bp;
    let z = bt * sa - br * ca;
    let h = libm::sqrt(x * x + y * y);
    let f = libm::sqrt(h * h + z * z);
    Some(MagneticField {
        declination: libm::atan2(y, x).to_degrees(),
        inclination: libm::atan2(z, h).to_degrees(),
        x,
        y,
        z,
        h,
        f,
    })
}

/// The declination on the ellipsoid (height 0), degrees east positive.
pub fn declination(lat: f64, lon: f64, year: f64) -> Option<f64> {
    field(lat, lon, 0.0, year).map(|f| f.declination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_a_decimal_year_as_noaa_counts_it() {
        assert_eq!(decimal_year("2025-01-01"), Some(2025.0));
        assert_eq!(decimal_year("2026-10-03"), Some(2026.0 + 275.0 / 365.0));
        assert_eq!(decimal_year("2028-12-31"), Some(2028.0 + 365.0 / 366.0));
        assert_eq!(decimal_year("2026-02-30"), None);
        assert_eq!(decimal_year("2026-10"), None);
        assert_eq!(
            decimal_year("2026-10-03T12:00:00Z"),
            decimal_year("2026-10-03")
        );
    }

    #[test]
    fn the_model_is_wmm2025_valid_for_five_years() {
        let i = info().expect("the table reads");
        assert_eq!(
            (i.model.as_str(), i.valid_from, i.valid_until),
            ("WMM2025", 2025.0, 2030.0)
        );
        assert!(valid(2026.75) && !valid(2024.99) && !valid(2030.01));
    }
}
