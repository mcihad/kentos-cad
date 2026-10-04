//! The project's second coordinate system (docs/adr/0167 §1–§2, §5): a
//! point of the drawing in it, written as the user reads it, with how sure
//! the values are. The status bar and Koordinat oku read it; the web's
//! `app/secondCrs.ts` writes the same.
//!
//! A projected second system is written as the project's points are: east
//! first, named as the project's type names its axes, with its length
//! decimals (`Y=412379.977, X=4512531.676`). A geographic one is latitude
//! first, in the user's notation (`display.geographic`) with fixed digits:
//! `40°45′12.3456″K, 29°55′01.2345″D` or `40.7534293°K, 29.9170096°D`.

use kentos_contracts::ProjectSettings;
use kentos_geometry_core::crs::{self as core, Transformed, format_dd, format_dms};
use kentos_project::crs;

use crate::Vec2;
use crate::format::Format;

/// How a geographic second system's latitude and longitude are written
/// (`display.geographic`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Notation {
    /// Degrees, minutes and seconds, the seconds' four decimals (≈3 mm).
    #[default]
    Dms,
    /// Decimal degrees, seven decimals (≈1 cm).
    Dd,
}

impl Notation {
    /// The setting's value; anything but `dd` is the default.
    pub fn parse(value: &str) -> Self {
        if value == "dd" { Self::Dd } else { Self::Dms }
    }

    fn write(self, deg: f64, latitude: bool) -> String {
        match self {
            Self::Dms => format_dms(deg, latitude, 4),
            Self::Dd => format_dd(deg, latitude, 7),
        }
    }
}

/// The project's second system, ready to take the drawing's points into.
#[derive(Clone, Debug)]
pub struct Second {
    /// Its registry entry.
    pub system: &'static crs::System,
    from: core::System,
    to: core::System,
    /// ED50 on either side: the values are not an official transformation's (§5).
    unofficial: bool,
}

impl Second {
    /// The second system of a project that has one the registry knows; none
    /// for a project without one, a local project, and a system the
    /// transforms do not read.
    pub fn of(settings: &ProjectSettings) -> Option<Self> {
        let system = crs::system(settings.second()?)?;
        let project = crs::system(settings.srid)?;
        Some(Self {
            system,
            from: project.transform_system()?,
            to: system.transform_system()?,
            unofficial: project.datum == "ED50" || system.datum == "ED50",
        })
    }

    /// Its name without the slash: “ED50 TM30”, “WGS 84 UTM 35N”, “TUREF”.
    pub fn short(&self) -> String {
        self.system.name.replace(" / ", " ")
    }

    /// Whether its values are a latitude and a longitude.
    pub fn geographic(&self) -> bool {
        matches!(self.to, core::System::Geographic { .. })
    }

    /// `p`, a point of the drawing, in the second system; none where the
    /// projection does not reach.
    pub fn point(&self, p: Vec2) -> Option<Transformed> {
        core::transform(&self.from, &self.to, p)
    }

    /// The two values of `q` (a point in the second system) with their
    /// names: east and north as the project's type names them, or the
    /// latitude and the longitude in `notation`.
    pub fn values(&self, q: Vec2, f: &Format, notation: Notation) -> [(&'static str, String); 2] {
        if self.geographic() {
            [
                ("Enlem", notation.write(q.y, true)),
                ("Boylam", notation.write(q.x, false)),
            ]
        } else {
            [
                (f.east_label(), f.coord(q.x)),
                (f.north_label(), f.coord(q.y)),
            ]
        }
    }

    /// The values on one line, as Koordinat oku says them: `Y=…, X=…`, or
    /// `40°45′12.3456″K, 29°55′01.2345″D`.
    pub fn reading(&self, q: Vec2, f: &Format, notation: Notation) -> String {
        let [a, b] = self.values(q, f, notation);
        if self.geographic() {
            format!("{}, {}", a.1, b.1)
        } else {
            format!("{}={}, {}={}", a.0, a.1, b.0, b.1)
        }
    }

    /// How sure the values are: “±2.1 m, EPSG:1783 + EPSG:5260; resmî
    /// dönüşüm değil”, “±1 m, EPSG:5261”, or “kesin, yalnız projeksiyon”
    /// within one datum.
    pub fn accuracy(&self, t: &Transformed) -> String {
        if t.via.is_empty() {
            return "kesin, yalnız projeksiyon".to_owned();
        }
        let mut text = format!("±{} m, {}", t.accuracy, t.via);
        if self.unofficial {
            text.push_str("; resmî dönüşüm değil");
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{AngleUnit, AreaUnit, Workspace};

    fn settings(srid: u32, second: Option<u32>) -> ProjectSettings {
        ProjectSettings {
            srid,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
            plot_scale: 1000.0,
            workspace: Some(Workspace::Gis),
            drawing_font: None,
            drawing_unit: None,
            second_srid: second,
        }
    }

    #[test]
    fn a_projected_second_system_is_written_as_the_projects_points() {
        let s = settings(5254, Some(2320));
        let second = Second::of(&s).expect("ED50 TM30");
        assert_eq!(second.short(), "ED50 TM30");
        let t = second
            .point(Vec2::new(412_500.0, 4_540_000.0))
            .expect("in the zone");
        let f = Format::of(&s);
        let reading = second.reading(t.point, &f, Notation::Dms);
        assert!(reading.starts_with("Y=412"), "{reading}");
        assert!(reading.contains(", X=4540"), "{reading}");
        assert_eq!(
            second.accuracy(&t),
            "±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil"
        );
    }

    #[test]
    fn a_geographic_second_system_is_latitude_first_in_the_notation() {
        let s = settings(5254, Some(4326));
        let second = Second::of(&s).expect("WGS 84");
        assert!(second.geographic());
        let t = second
            .point(Vec2::new(500_000.0, 4_400_000.0))
            .expect("in the zone");
        let f = Format::of(&s);
        let dms = second.reading(t.point, &f, Notation::Dms);
        assert!(dms.contains("″K, ") && dms.ends_with("″D"), "{dms}");
        let dd = second.reading(t.point, &f, Notation::Dd);
        assert!(dd.starts_with("39.") && dd.ends_with("°D"), "{dd}");
        assert_eq!(second.accuracy(&t), "±1 m, EPSG:5261");
        // Within one datum, only the projection changes.
        let utm = Second::of(&settings(5254, Some(5252))).expect("TUREF");
        let t = utm.point(Vec2::new(500_000.0, 4_400_000.0)).expect("TUREF");
        assert_eq!(utm.accuracy(&t), "kesin, yalnız projeksiyon");
    }

    #[test]
    fn none_without_a_second_system_the_transforms_read() {
        assert!(Second::of(&settings(5254, None)).is_none());
        assert!(Second::of(&settings(0, Some(5254))).is_none());
        assert!(Second::of(&settings(5254, Some(5254))).is_none());
        assert!(Second::of(&settings(5254, Some(99_999))).is_none());
    }
}
