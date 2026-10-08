//! The project's second coordinate system (docs/adr/0167 §1–§2, §5): a
//! point of the drawing in it, written as the user reads it, with how sure
//! the values are. The status bar and Koordinat oku read it; the web's
//! `model/secondCrs.ts` writes the same. Either system may be one of the
//! registry's or a definition of the project's, and the project's datum
//! choices are taken where they apply (docs/adr/0168; `kentos_project::systems`).
//!
//! A projected second system is written as the project's points are: east
//! first, named as the project's type names its axes, with its length
//! decimals (`Y=412379.977, X=4512531.676`). A geographic one is latitude
//! first, in the user's notation (`display.geographic`) with fixed digits:
//! `40°45′12.3456″K, 29°55′01.2345″D` or `40.7534293°K, 29.9170096°D`.

use kentos_contracts::ProjectSettings;
use kentos_geometry_core::crs::measure::{NoPlane, PlaneMeasures, Ring, plane_measures_in};
use kentos_geometry_core::crs::{
    self as core, Transformed, Unreached, format_dd, format_dms, transform_in,
};
use kentos_project::systems;

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
    /// Its name: “ED50 / TM30”, “Belediye sistemi”.
    pub name: String,
    /// As a sentence names it: “ED50 / TM30 (EPSG:2320)”, “Belediye sistemi
    /// (özel sistem)”.
    pub title: String,
    from: core::System,
    to: core::System,
    /// The project's datum choices (docs/adr/0168 §3).
    choices: Vec<core::Choice>,
}

impl Second {
    /// The second system of a project that has one; none for a project
    /// without one, a project without a system, and a system the transforms
    /// do not read.
    pub fn of(settings: &ProjectSettings) -> Option<Self> {
        let own = systems::own(settings)?;
        let second = systems::second(settings)?;
        Some(Self {
            name: second.name,
            title: second.title,
            from: own.system?,
            to: second.system?,
            choices: systems::choices(settings),
        })
    }

    /// Its name without the slash: “ED50 TM30”, “WGS 84 UTM 35N”, “TUREF”.
    pub fn short(&self) -> String {
        self.name.replace(" / ", " ")
    }

    /// Whether its values are a latitude and a longitude.
    pub fn geographic(&self) -> bool {
        matches!(self.to, core::System::Geographic { .. })
    }

    /// `p`, a point of the drawing, in the second system; or why it has no
    /// value there.
    pub fn point(&self, p: Vec2) -> Result<Transformed, Unreached> {
        transform_in(&self.from, &self.to, p, &self.choices)
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

    /// A path (`closed` false) or an area's rings (the outer first), given
    /// in the project's system, measured in the second system's plane
    /// (docs/adr/0167 §2): none in a geographic system or the
    /// Pseudo-Mercator, or where it does not reach.
    pub fn measure(&self, rings: &[Ring], closed: bool) -> Result<PlaneMeasures, NoPlane> {
        plane_measures_in(&self.from, &self.to, rings, closed, &self.choices)
    }

    /// The line Mesafe ölç and Alan hesapla say after their own: the length,
    /// or the area and the perimeter, in the second system's plane, or why
    /// there are none.
    pub fn measures_line(&self, rings: &[Ring], closed: bool, f: &Format) -> String {
        let name = self.short();
        match self.measure(rings, closed) {
            Ok(m) if closed => format!(
                "{name} düzleminde: Alan {}   Çevre {}",
                f.area(m.area),
                f.length(m.length)
            ),
            Ok(m) => format!("{name} düzleminde: Toplam uzunluk {}", f.length(m.length)),
            Err(NoPlane::Geographic) => {
                format!("{name} coğrafi bir sistem: uzunluk ve alan onun düzleminde verilmez.")
            }
            Err(NoPlane::Mercator) => format!(
                "{name}: uzunluk ve alan verilmez, Pseudo-Mercator'un ölçeği her enlemde başkadır."
            ),
            Err(NoPlane::Unreachable) => format!(
                "{name}: ölçülen yerin bir noktası bu sistemin ulaştığı yerin dışında; değer yazılmadı."
            ),
            Err(NoPlane::NoLink) => {
                format!("{name}: datumlardan birinin WGS 84'e dönüşümü yok; değer yazılmadı.")
            }
            Err(NoPlane::NoGrid) => {
                format!("{name}: datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.")
            }
        }
    }

    /// How sure the values are: “±2.1 m, EPSG:1783 + EPSG:5260; resmî
    /// dönüşüm değil”, “±1 m, EPSG:5261”, or “kesin, yalnız projeksiyon”
    /// within one datum.
    pub fn accuracy(&self, t: &Transformed) -> String {
        accuracy_text(t)
    }
}

/// Why the cursor has no value in the second system, as the status bar's
/// tip says it (the web's `cursorUnreached`).
pub fn cursor_unreached(why: Unreached) -> &'static str {
    match why {
        Unreached::Outside => "İmleç bu sistemin ulaştığı yerin dışında; değer yazılmadı.",
        Unreached::OutsideGrid => "İmleç datum dönüşümünün ızgarasının dışında; değer yazılmadı.",
        Unreached::NoLink => "Datumlardan birinin WGS 84'e dönüşümü yok; değer yazılmadı.",
        Unreached::NoGrid => "Datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.",
    }
}

/// Why a point has no value in the second system, as Koordinat oku says it
/// after the system's name (the web's `pointUnreached`).
pub fn point_unreached(why: Unreached) -> &'static str {
    match why {
        Unreached::Outside => "nokta bu sistemin ulaştığı yerin dışında; değeri yazılmadı.",
        Unreached::OutsideGrid => "nokta datum dönüşümünün ızgarasının dışında; değeri yazılmadı.",
        Unreached::NoLink => "datumlardan birinin WGS 84'e dönüşümü yok; değeri yazılmadı.",
        Unreached::NoGrid => "datum dönüşümünün ızgarası bu cihazda yok; değeri yazılmadı.",
    }
}

/// How sure a point moved between two systems is: “±2.1 m, EPSG:1783 +
/// EPSG:5260; resmî dönüşüm değil” (an EPSG operation of ED50 was used,
/// docs/adr/0167 §5), “±1 m, EPSG:5261”, “doğruluğu bilinmiyor, Bölge 7”
/// (a step's accuracy is not written, docs/adr/0168 §2), or “kesin, yalnız
/// projeksiyon” within one datum (the web's `accuracyText`).
pub fn accuracy_text(t: &Transformed) -> String {
    if t.via.is_empty() {
        return "kesin, yalnız projeksiyon".to_owned();
    }
    let mut text = match t.accuracy {
        Some(a) => format!("±{a} m, {}", t.via),
        None => format!("doğruluğu bilinmiyor, {}", t.via),
    };
    if t.unofficial {
        text.push_str("; resmî dönüşüm değil");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{
        AngleUnit, AreaUnit, Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem,
        DatumTransform, GridChoice, Helmert, LocalDefinition, RegistryDatum, Workspace,
    };
    use kentos_geometry_core::crs::Plane;

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
            custom_crs: None,
            second_custom_crs: None,
            datum_transforms: Vec::new(),
            layer_states: Vec::new(),
            survey: None,
            dimension_styles: Vec::new(),
            topology: None,
            annotation: None,
            text_styles: Vec::new(),
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
    fn measures_are_said_in_the_second_plane_or_why_not() {
        let s = settings(5254, Some(2320));
        let second = Second::of(&s).expect("ED50 TM30");
        let f = Format::of(&s);
        let ring = Ring {
            pts: vec![
                Vec2::new(414_000.0, 4_540_000.0),
                Vec2::new(414_040.0, 4_540_000.0),
                Vec2::new(414_040.0, 4_540_025.0),
                Vec2::new(414_000.0, 4_540_025.0),
            ],
            bulges: None,
        };
        // fixtures/geodesy/v1/measure.json: 1000.0104879 m², 130.0006813 m (PROJ).
        assert_eq!(
            second.measures_line(std::slice::from_ref(&ring), true, &f),
            "ED50 TM30 düzleminde: Alan 1000.01 m²   Çevre 130.001 m"
        );
        let geographic = Second::of(&settings(5254, Some(4326))).expect("WGS 84");
        assert!(
            geographic
                .measures_line(&[ring], false, &f)
                .starts_with("WGS 84 coğrafi bir sistem")
        );
    }

    /// A local system bound to TUREF TM33 (docs/adr/0168 §1).
    fn site() -> CrsDefinition {
        CrsDefinition {
            name: "Şantiye".to_owned(),
            system: CrsSystem::Local(LocalDefinition {
                base: CrsBase {
                    srid: Some(5255),
                    definition: None,
                },
                plane: CrsPlane::Similarity {
                    east: 492_345.678,
                    north: 4_422_345.678,
                    rotation: 12.5,
                    scale: 1.000_012,
                },
            }),
        }
    }

    /// The project's choice for ED50–TUREF: seven parameters, or a grid.
    fn region(helmert: bool) -> DatumTransform {
        DatumTransform {
            from: RegistryDatum::Ed50,
            to: RegistryDatum::Turef,
            name: "ED50 → TUREF: Bölge 7".to_owned(),
            helmert: helmert.then_some(Helmert {
                translation: [-158.785, -109.965, -50.768],
                rotation: [1.4275, -3.0873, 0.5505],
                scale: -5.1814,
                convention: Convention::CoordinateFrame,
                accuracy: Some(0.3),
            }),
            grid: (!helmert).then(|| GridChoice {
                id: "ab".repeat(32),
                file: "bolge7.gsb".to_owned(),
                size: 1024,
                accuracy: None,
            }),
        }
    }

    /// The project's own definition takes the drawing's points to the second
    /// system: its plane to its base, then the datum by the project's choice;
    /// a grid this process does not have leaves no value and says so
    /// (docs/adr/0168 §1, §3–§4).
    #[test]
    fn the_projects_own_system_and_choices_reach_the_second() {
        let own = ProjectSettings {
            custom_crs: Some(site()),
            ..settings(0, Some(5255))
        };
        let second = Second::of(&own).expect("TUREF TM33");
        assert_eq!(
            (second.name.as_str(), second.title.as_str()),
            ("TUREF / TM33", "TUREF / TM33 (EPSG:5255)")
        );
        // The second system is the base: only the plane moves the point.
        let p = Vec2::new(1_000.0, 2_000.0);
        let t = second.point(p).expect("in the zone");
        let want = Plane::Similarity {
            east: 492_345.678,
            north: 4_422_345.678,
            rotation: 12.5,
            scale: 1.000_012,
        }
        .forward(p);
        assert!(
            (t.point.x - want.x).abs() < 1e-6 && (t.point.y - want.y).abs() < 1e-6,
            "{:?}",
            t.point
        );
        assert_eq!(second.accuracy(&t), "kesin, yalnız projeksiyon");
        // ED50 TM33 by the project's seven parameters.
        let ed50 = ProjectSettings {
            custom_crs: Some(site()),
            datum_transforms: vec![region(true)],
            ..settings(0, Some(2321))
        };
        let second = Second::of(&ed50).expect("ED50 TM33");
        let t = second.point(p).expect("in the zone");
        assert_eq!(second.accuracy(&t), "±0.3 m, ED50 → TUREF: Bölge 7");
        // The same pair by a grid not loaded here.
        let grid = ProjectSettings {
            datum_transforms: vec![region(false)],
            ..ed50
        };
        let second = Second::of(&grid).expect("ED50 TM33");
        assert_eq!(second.point(p).map(|t| t.point), Err(Unreached::NoGrid));
        assert_eq!(
            second.measures_line(
                &[Ring {
                    pts: vec![p, Vec2::new(1_010.0, 2_000.0)],
                    bulges: None,
                }],
                false,
                &Format::of(&grid)
            ),
            "ED50 TM33: datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı."
        );
        assert_eq!(
            (
                cursor_unreached(Unreached::NoGrid),
                point_unreached(Unreached::OutsideGrid)
            ),
            (
                "Datum dönüşümünün ızgarası bu cihazda yok; değer yazılmadı.",
                "nokta datum dönüşümünün ızgarasının dışında; değeri yazılmadı."
            )
        );
    }

    /// A second system the project defines is named by its definition, and
    /// takes the base's points through its plane's inverse.
    #[test]
    fn a_second_definition_is_named_by_its_name() {
        let s = ProjectSettings {
            second_custom_crs: Some(site()),
            ..settings(5255, None)
        };
        let second = Second::of(&s).expect("a second system");
        assert_eq!(
            (second.short(), second.title.as_str()),
            ("Şantiye".to_owned(), "Şantiye (özel sistem)")
        );
        let t = second
            .point(Vec2::new(492_345.678, 4_422_345.678))
            .expect("the site's origin");
        assert!(
            t.point.x.abs() < 1e-6 && t.point.y.abs() < 1e-6,
            "{:?}",
            t.point
        );
        // A project without a system has no second, whatever it says.
        let local = ProjectSettings {
            second_custom_crs: Some(site()),
            ..settings(0, None)
        };
        assert!(Second::of(&local).is_none());
    }

    #[test]
    fn none_without_a_second_system_the_transforms_read() {
        assert!(Second::of(&settings(5254, None)).is_none());
        assert!(Second::of(&settings(0, Some(5254))).is_none());
        assert!(Second::of(&settings(5254, Some(5254))).is_none());
        assert!(Second::of(&settings(5254, Some(99_999))).is_none());
    }
}
