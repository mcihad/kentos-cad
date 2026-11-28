//! The sun's place (docs/adr/8177 §3.1): its azimuth and elevation at a latitude and longitude, on a day of the year
//! at a clock time of a time zone, by NOAA's general solar position formulas (the fractional year's Fourier series for
//! the equation of time and the declination, the hour angle from the true solar time); good to a tenth of a degree,
//! with no refraction. And its direction in a plan whose north is turned from true north.

use libm::{asin, atan2, cos, sin};

const RAD: f64 = core::f64::consts::PI / 180.0;

/// The sun at a place and time: its azimuth (degrees clockwise from true north) and elevation (degrees over the
/// horizon; below it: negative), and the direction towards it (east, north, up; length one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    pub azimuth: f64,
    pub elevation: f64,
    pub towards: [f64; 3],
}

/// The sun at latitude `lat` and longitude `lon` (degrees; north and east positive) on day `day` of the year (1–366)
/// at clock time `hours` (0–24, local) of the time zone UTC+`zone` hours.
pub fn at(lat: f64, lon: f64, zone: f64, day: u32, hours: f64) -> Sun {
    let gamma = 2.0 * core::f64::consts::PI / 365.0 * (f64::from(day.max(1)) - 1.0 + (hours - zone - 12.0) / 24.0);
    // Minutes.
    let eot = 229.18
        * (0.000075 + 0.001868 * cos(gamma)
            - 0.032077 * sin(gamma)
            - 0.014615 * cos(2.0 * gamma)
            - 0.040849 * sin(2.0 * gamma));
    let decl = 0.006918 - 0.399912 * cos(gamma) + 0.070257 * sin(gamma) - 0.006758 * cos(2.0 * gamma)
        + 0.000907 * sin(2.0 * gamma)
        - 0.002697 * cos(3.0 * gamma)
        + 0.00148 * sin(3.0 * gamma);
    let true_solar = hours * 60.0 + eot + 4.0 * lon - 60.0 * zone;
    let ha = (true_solar / 4.0 - 180.0) * RAD;
    let (sp, cp) = (sin(lat * RAD), cos(lat * RAD));
    let (sd, cd) = (sin(decl), cos(decl));
    let east = -cd * sin(ha);
    let north = sd * cp - cd * sp * cos(ha);
    let up = sp * sd + cp * cd * cos(ha);
    let azimuth = (atan2(east, north) / RAD).rem_euclid(360.0);
    Sun { azimuth, elevation: asin(up.clamp(-1.0, 1.0)) / RAD, towards: [east, north, up] }
}

impl Sun {
    /// Towards the sun in a plan whose true north is `north` radians counter-clockwise from its north (+y): (x, y, up).
    pub fn in_plan(&self, north: f64) -> [f64; 3] {
        let [e, n, up] = self.towards;
        // True north's direction in the plan, and true east's (clockwise from it).
        let (nx, ny) = (-sin(north), cos(north));
        let (ex, ey) = (ny, -nx);
        [e * ex + n * nx, e * ey + n * ny, up]
    }
}

/// The day of the year (1–366) of `day` `month` in a year of 365 days (February's 29th the 60th).
pub fn day_of_year(day: u32, month: u32) -> Option<u32> {
    const DAYS: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let m = month.checked_sub(1).filter(|m| *m < 12)? as usize;
    (day >= 1 && day <= DAYS[m]).then(|| {
        let before: u32 = DAYS[..m].iter().sum();
        // A year of 365 days: from March on, February's 29th is not one.
        if m >= 2 { before + day - 1 } else { before + day }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noon_s_elevation_is_ninety_less_the_latitude_and_the_declination() {
        // Greenwich on 21 June at 12:01 UTC (its solar noon): 90 − (51.4779 − 23.44).
        let s = at(51.4779, 0.0, 0.0, 172, 12.0 + 1.7 / 60.0);
        assert!((s.elevation - 61.96).abs() < 0.1, "{s:?}");
        assert!((s.azimuth - 180.0).abs() < 0.5, "due south: {s:?}");
        // Sivas (39.75 N, 37.02 E) on 21 December at its solar noon: Türkiye's clock (UTC+3) runs half an hour ahead
        // of its sun, 720 − 4 · 37.02 + 180 − EoT (≈ 2 min) = 12:30. 90 − (39.75 + 23.44).
        let s = at(39.75, 37.02, 3.0, 355, 12.5);
        assert!((s.elevation - 26.81).abs() < 0.15 && (s.azimuth - 180.0).abs() < 0.5, "{s:?}");
    }

    #[test]
    fn the_sun_rises_east_and_sets_west_at_the_equinox_and_is_under_the_horizon_at_night() {
        let morning = at(0.0, 0.0, 0.0, 80, 6.2);
        assert!(morning.elevation.abs() < 2.0 && (morning.azimuth - 90.0).abs() < 2.0, "{morning:?}");
        let evening = at(0.0, 0.0, 0.0, 80, 18.1);
        assert!(evening.elevation.abs() < 2.0 && (evening.azimuth - 270.0).abs() < 2.0, "{evening:?}");
        assert!(at(39.75, 37.02, 3.0, 172, 1.0).elevation < 0.0, "night");
        let [e, n, up] = morning.towards;
        assert!((e * e + n * n + up * up - 1.0).abs() < 1e-9, "a unit vector");
    }

    #[test]
    fn the_plan_s_turned_north_turns_the_sun_with_it() {
        let s = Sun { azimuth: 180.0, elevation: 45.0, towards: [0.0, -0.5_f64.sqrt(), 0.5_f64.sqrt()] };
        let [x, y, _] = s.in_plan(0.0);
        assert!(x.abs() < 1e-12 && y < 0.0, "south is down");
        // True north 90° counter-clockwise from the plan's (to the plan's west): south is to the plan's east.
        let [x, y, _] = s.in_plan(core::f64::consts::FRAC_PI_2);
        assert!(x > 0.0 && y.abs() < 1e-12, "{x} {y}");
    }

    #[test]
    fn a_day_and_month_read_as_the_year_s_day() {
        assert_eq!(day_of_year(1, 1), Some(1));
        assert_eq!(day_of_year(21, 6), Some(172));
        assert_eq!(day_of_year(21, 12), Some(355));
        assert_eq!(day_of_year(31, 12), Some(365));
        assert_eq!(day_of_year(30, 2), None);
        assert_eq!(day_of_year(1, 13), None);
    }
}
