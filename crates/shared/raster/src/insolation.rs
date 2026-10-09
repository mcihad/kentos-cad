//! Güneşlenme (docs/adr/0231 §8): the direct sunlight a DEM's sloped cells
//! take over a period under a clear sky, kWh/m². The sun's places (Spencer
//! 1971's declination and distance, the hour angle in solar time) and their
//! weight through the air (Kasten and Young 1989's air mass, one
//! transmissivity for the whole raster) are worked out once a row; a cell
//! only adds up its surface normal's dot products with them, the sun's
//! rays below the surface left out (no terrain shadow).

use libm::{cos, pow, sin};

/// The solar constant, W/m².
pub const SOLAR_CONSTANT: f64 = 1367.0;

/// The days of the months of a 365-day year.
pub const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// The months' names, as the tools' menus write them.
pub const MONTH_NAMES: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];

/// The day of the year (1–365) of `day` of `month` (1–12).
pub fn day_of_year(month: u32, day: u32) -> Result<u32, String> {
    let m = month
        .checked_sub(1)
        .and_then(|m| MONTH_DAYS.get(m as usize))
        .ok_or_else(|| format!("Ay 1 ile 12 arasında olmalı; {month} verildi."))?;
    if day == 0 || day > *m {
        return Err(format!(
            "{} ayının günü 1 ile {m} arasında olmalı; {day} verildi.",
            MONTH_NAMES[(month - 1) as usize]
        ));
    }
    Ok(MONTH_DAYS[..(month - 1) as usize].iter().sum::<u32>() + day)
}

/// The period's day blocks: each its middle day and its days (docs/adr/0231 §8).
pub fn blocks(first: u32, last: u32, step: u32) -> Result<Vec<(u32, f64)>, String> {
    if !(1..=365).contains(&first) || !(1..=365).contains(&last) {
        return Err("Dönemin günleri 1 ile 365 arasında olmalı.".into());
    }
    if first > last {
        return Err(
            "Dönemin başlangıcı bitişinden sonra; aynı yıl içinde bir aralık verin.".into(),
        );
    }
    if step == 0 {
        return Err("Gün aralığı en az 1 olmalı.".into());
    }
    let mut out = Vec::new();
    let mut start = first;
    while start <= last {
        let end = (start + step - 1).min(last);
        out.push((start + (end - start) / 2, f64::from(end - start + 1)));
        start = end + 1;
    }
    Ok(out)
}

/// A sun place: towards the sun (east, north, up) and what a unit of the
/// dot product with the surface's normal brings, kWh/m².
pub type Sun = [f64; 4];

/// The period's suns above the horizon at latitude `phi` (degrees), in the
/// order of the days, then the hours; `hour_step` hours, `tau` the
/// transmissivity.
pub fn suns(phi: f64, days: &[(u32, f64)], hour_step: f64, tau: f64, out: &mut Vec<Sun>) {
    out.clear();
    let rad = core::f64::consts::PI / 180.0;
    let (sp, cp) = (sin(phi * rad), cos(phi * rad));
    let slices = libm::round(24.0 / hour_step) as u32;
    for &(n, weight) in days {
        let g = 2.0 * core::f64::consts::PI * f64::from(n - 1) / 365.0;
        let decl = 0.006918 - 0.399912 * cos(g) + 0.070257 * sin(g) - 0.006758 * cos(2.0 * g)
            + 0.000907 * sin(2.0 * g)
            - 0.002697 * cos(3.0 * g)
            + 0.00148 * sin(3.0 * g);
        let e0 = 1.000110
            + 0.034221 * cos(g)
            + 0.001280 * sin(g)
            + 0.000719 * cos(2.0 * g)
            + 0.000077 * sin(2.0 * g);
        let (sd, cd) = (sin(decl), cos(decl));
        for k in 0..slices {
            let t = (f64::from(k) + 0.5) * hour_step;
            let w = 15.0 * (t - 12.0) * rad;
            let (sw, cw) = (sin(w), cos(w));
            let up = sp * sd + cp * cd * cw;
            if up <= 0.0 {
                continue;
            }
            let east = -cd * sw;
            let north = sd * cp - cd * sp * cw;
            let h = libm::asin(up) / rad;
            let m = 1.0 / (up + 0.50572 * pow(h + 6.07995, -1.6364));
            let k_sun = SOLAR_CONSTANT * e0 * pow(tau, m) * hour_step * weight / 1000.0;
            out.push([east, north, up, k_sun]);
        }
    }
}

/// The energy (kWh/m²) a surface of gradient (`gx`, `gy`) takes from `suns`.
#[inline]
pub fn energy(gx: f64, gy: f64, suns: &[Sun]) -> f64 {
    let mut s = 0.0;
    for sun in suns {
        let d = -gx * sun[0] - gy * sun[1] + sun[2];
        if d > 0.0 {
            s += sun[3] * d;
        }
    }
    s / libm::sqrt(1.0 + gx * gx + gy * gy)
}
