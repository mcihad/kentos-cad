//! Güneş in 3B görünüm (docs/adr/8177 §4): the building lit by the sun of a day and an hour, its faces casting their
//! shadows. The place and the time zone are the Gölge tool's: the project's coordinate system's, else the latitude and
//! longitude typed there (`Memory::sun_shadow`); the day starts as the tool's and the hour as its (12:00 for its
//! day-long diagram). The hour steps an hour at a time while the sun is up. A picture placed on a sheet (Paftaya koy)
//! is lit as the window is.

use kentos_geometry_core::geom::sun;
use kentos_interaction::shadow_tool::{DAYS, Kept};
use kentos_native_application::shadows;

use crate::app::App;

/// The sun of 3B görünüm: its day's place in Gölge's days and its hour (clock time, the place's time zone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunAt {
    pub day: usize,
    pub hour: f64,
}

/// What Güneş's buttons ask.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SunChange {
    /// On, or off.
    Toggle,
    Day(usize),
    /// An hour earlier (−1) or later (+1).
    Hour(i8),
}

/// The sun's place over the horizon under which it lights nothing.
const UP: f64 = 0.5;

/// The sun first asked: the Gölge tool's day and hour.
fn first(kept: &Kept) -> SunAt {
    let hour = match kept.hours {
        0 => 9.0,
        2 => 15.0,
        _ => 12.0,
    };
    SunAt {
        day: kept.day.min(DAYS.len() - 1),
        hour,
    }
}

impl App {
    /// The sun at `at` over the drawing's building: towards it in the model's space, and what it is in words; or why
    /// none (no place known, no drawing).
    fn sun_towards(&self, at: SunAt) -> Result<(sun::Sun, [f64; 3], String), String> {
        let doc = &self
            .document
            .as_ref()
            .ok_or_else(|| "Açık çizim yok.".to_owned())?
            .model;
        let k = self.memory.sun_shadow;
        let place = shadows::place(doc, k.lat.zip(k.lon), k.zone).map_err(|why| {
            format!("{why} Konumu Gölge (GL) aracında enlem (E) ve boylamla (B) verin.")
        })?;
        let (_, name, d, m) = DAYS[at.day];
        let (s, towards) = shadows::sun_at(&place, sun::day_of_year(d, m).unwrap_or(1), at.hour);
        let said = format!(
            "Güneş: {name} {}, yüksekliği {}°, azimut {}°",
            shadows::clock(at.hour),
            (s.elevation * 10.0).round() / 10.0,
            s.azimuth.round()
        )
        .replace('.', ",");
        Ok((s, towards, said))
    }

    /// Güneş turned on or off, its day or its hour changed: the viewer lit by the sun, or why not.
    pub(crate) fn building3d_sun(&mut self, change: SunChange) {
        let Some(now) = self
            .scene3d_window
            .as_ref()
            .and_then(|w| w.building.as_ref())
            .map(|b| b.sun)
        else {
            return;
        };
        let mut want = match (change, now) {
            (SunChange::Toggle, Some(_)) => {
                self.set_building3d_sun(None, None);
                return;
            }
            (SunChange::Toggle, None) => first(&self.memory.sun_shadow),
            (_, None) => return,
            (SunChange::Day(day), Some(at)) => SunAt {
                day: day.min(DAYS.len() - 1),
                ..at
            },
            (SunChange::Hour(step), Some(at)) => SunAt {
                hour: at.hour + f64::from(step),
                ..at
            },
        };
        let mut made = self.sun_towards(want);
        // Another day's sun not up at this hour (21 Aralık at 08:00): that day's noon.
        if matches!(change, SunChange::Day(_))
            && made.as_ref().is_ok_and(|(s, ..)| s.elevation <= UP)
        {
            want.hour = 12.0;
            made = self.sun_towards(want);
        }
        match made {
            Ok((s, ..)) if s.elevation <= UP => {
                self.warn(format!(
                    "{} {}: güneş ufkun altında.",
                    DAYS[want.day].1,
                    shadows::clock(want.hour)
                ));
            }
            Ok((_, towards, said)) => {
                self.set_building3d_sun(Some(want), Some(towards));
                self.output(said);
            }
            Err(why) => self.warn(why),
        }
    }

    fn set_building3d_sun(&mut self, at: Option<SunAt>, towards: Option<[f64; 3]>) {
        if let Some(b) = self
            .scene3d_window
            .as_mut()
            .and_then(|w| w.building.as_mut())
        {
            b.sun = at;
            b.viewer.sun = towards;
        }
    }
}
