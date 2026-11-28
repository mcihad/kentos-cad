//! Ölçü kutusu (docs/adr/9328): the last measurement of Mesafe ölç, Alan hesapla, Açı ölç and Koordinat oku as the host
//! shows it in a box of its own, beside the log where the tools said it before:
//!
//! - what was measured and its values, a row each with its unit: every part of a path and the total, an area and its
//!   perimeter, an angle and its explement, a point's east and north;
//! - the units by name (metre, metrekare, gon or derece), how the values were reckoned (on the projection's plane, on
//!   the ellipsoid in a geographic project, on the drawing's own plane without a system) and in which system;
//! - the same as text, for Kopyala.
//!
//! The values are the tools' own, reckoned as before; the rows only say them.

use kentos_commands::ProjectSettings;
use kentos_geometry_core::crs::measure::Ring;
use kentos_geometry_core::crs::{Datum, System};
use kentos_geometry_core::geom::bulge::bulge_path_length;
use kentos_project::systems;

use crate::Vec2;
use crate::format::Format;

/// A measurement as its box shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// The tool that measured it, by id: its box shows while that tool runs.
    pub tool: &'static str,
    /// What was measured: Mesafe, Alan, Açı, Koordinat.
    pub title: &'static str,
    /// Its values, a row each: what and how much, with the unit.
    pub rows: Vec<(String, String)>,
    /// The units by name.
    pub units: String,
    /// How the values were reckoned.
    pub method: String,
    /// The coordinate system, as a sentence names it; none for a drawing without one.
    pub system: Option<String>,
}

impl Measurement {
    /// As Kopyala writes it: the title, a line a row (what, a tab, how much), the units, the way and the system.
    pub fn text(&self) -> String {
        let mut lines = vec![self.title.to_owned()];
        lines.extend(
            self.rows
                .iter()
                .map(|(what, value)| format!("{what}\t{value}")),
        );
        lines.push(format!("Birim\t{}", self.units));
        lines.push(format!("Hesap\t{}", self.method));
        if let Some(system) = &self.system {
            lines.push(format!("Koordinat sistemi\t{system}"));
        }
        lines.join("\n")
    }
}

/// The ellipsoid a datum's lengths are reckoned on, by name.
fn ellipsoid_name(datum: &Datum) -> String {
    match datum {
        Datum::Turef => "GRS 1980".to_owned(),
        Datum::Ed50 => "International 1924".to_owned(),
        Datum::Wgs84 => "WGS 84".to_owned(),
        Datum::Custom(d) => d.ellipsoid.name.clone(),
    }
}

/// The project's system as a sentence names it (“TUREF / TM36 (EPSG:5256)”); none without one.
fn system_of(settings: &ProjectSettings) -> Option<String> {
    systems::own(settings).map(|n| n.title)
}

/// How a project's lengths and areas are reckoned: on the ellipsoid in a geographic system, on the projection's plane
/// in a projected one, on the drawing's own plane without a system.
fn method_of(settings: &ProjectSettings) -> String {
    match systems::own(settings).and_then(|n| n.system) {
        Some(s) if s.is_geographic() => format!(
            "Elipsoit üstünde (jeodezik), {} elipsoidi",
            ellipsoid_name(&s.datum())
        ),
        Some(System::Local { .. }) => "Düzlemde: yerel sistemin koordinatlarıyla".to_owned(),
        Some(_) => "Düzlemde: projeksiyon koordinatlarıyla, ölçek düzeltmesi yok".to_owned(),
        None => "Düzlemde: çizimin kendi koordinatlarıyla".to_owned(),
    }
}

/// The length and area units by name.
fn length_units(f: &Format, area: bool) -> String {
    let length = f.length_unit_name();
    if !area {
        return length.to_owned();
    }
    let area = match f.area_unit_label() {
        "m²" => "metrekare",
        "dönüm" => "dönüm",
        "ha" => "hektar",
        other => other,
    };
    format!("{length}, {area}")
}

/// A geographic project's length of one ring on its ellipsoid; none in a projected one.
fn on_ellipsoid(
    settings: &ProjectSettings,
    ring: &Ring,
    closed: bool,
) -> Option<Result<(f64, Option<f64>), ()>> {
    crate::ground::on_geographic_ellipsoid(settings, std::slice::from_ref(ring), closed)
}

/// Mesafe ölç's path: every part and the total, on the ellipsoid in a geographic project.
pub fn path(
    settings: &ProjectSettings,
    f: &Format,
    pts: &[Vec2],
    bulges: Option<&[f64]>,
) -> Measurement {
    let mut rows = Vec::with_capacity(pts.len() + 1);
    let mut total = 0.0;
    let mut unreached = false;
    for (i, pair) in pts.windows(2).enumerate() {
        let bulge = bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
        let part = Ring {
            pts: pair.to_vec(),
            bulges: (bulge != 0.0).then(|| vec![bulge]),
        };
        let length = match on_ellipsoid(settings, &part, false) {
            Some(Ok((length, _))) => length,
            Some(Err(())) => {
                unreached = true;
                continue;
            }
            None => bulge_path_length(&part.pts, part.bulges.as_deref(), false),
        };
        total += length;
        rows.push((format!("{}. parça", i + 1), f.length(length)));
    }
    rows.push((
        "Toplam".to_owned(),
        if unreached {
            crate::ground::UNREACHED.to_owned()
        } else {
            f.length(total)
        },
    ));
    Measurement {
        tool: crate::path::MEASURE_ID,
        title: "Mesafe",
        rows,
        units: length_units(f, false),
        method: method_of(settings),
        system: system_of(settings),
    }
}

/// Sabit ilk nokta's rays: each from the first point, by its number, with its direction.
pub fn rays(settings: &ProjectSettings, f: &Format, rays: &[(String, String)]) -> Measurement {
    Measurement {
        tool: crate::path::MEASURE_ID,
        title: "Mesafe",
        rows: rays.to_vec(),
        units: format!("{}; yön {}", length_units(f, false), angle_unit_name(f)),
        method: method_of(settings),
        system: system_of(settings),
    }
}

/// Alan hesapla's region: its area (the holes' left out) and its perimeter, on the ellipsoid in a geographic project.
pub fn area(settings: &ProjectSettings, f: &Format, area: f64, perimeter: f64) -> Measurement {
    Measurement {
        tool: crate::path::AREA_ID,
        title: "Alan",
        rows: vec![
            ("Alan".to_owned(), f.area(area)),
            ("Çevre".to_owned(), f.length(perimeter)),
        ],
        units: length_units(f, true),
        method: method_of(settings),
        system: system_of(settings),
    }
}

/// The project's angle unit by name.
fn angle_unit_name(f: &Format) -> &'static str {
    match f.angle_unit_label() {
        "°" => "derece (bir tur 360°)",
        _ => "gon (bir tur 400 g)",
    }
}

/// Açı ölç's angle at its vertex and its explement.
pub fn angle(settings: &ProjectSettings, f: &Format, inner: f64, outer: f64) -> Measurement {
    Measurement {
        tool: crate::angle::ID,
        title: "Açı",
        rows: vec![
            ("Açı".to_owned(), f.angle(inner)),
            ("Dış açı".to_owned(), f.angle(outer)),
        ],
        units: angle_unit_name(f).to_owned(),
        method: "Düzlemde, iki kolun arasında".to_owned(),
        system: system_of(settings),
    }
}

/// Koordinat oku's point: its east and north by their names (and its elevation), then its place in the second
/// system.
pub fn point(
    settings: &ProjectSettings,
    f: &Format,
    at: Vec2,
    z: Option<f64>,
    second: Vec<(String, String)>,
) -> Measurement {
    let mut rows = vec![
        (format!("{} (doğu)", f.east_label()), f.coord(at.x)),
        (format!("{} (kuzey)", f.north_label()), f.coord(at.y)),
    ];
    if let Some(z) = z {
        rows.push(("Z (kot)".to_owned(), f.length_bare(z)));
    }
    rows.extend(second);
    Measurement {
        tool: crate::coordinate::ID,
        title: "Koordinat",
        rows,
        units: f.length_unit_name().to_owned(),
        method: "Çizimin koordinatları, kenetlenen noktada".to_owned(),
        system: system_of(settings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(json: &str) -> ProjectSettings {
        serde_json::from_str(json).expect("settings")
    }

    fn tm36() -> ProjectSettings {
        settings(
            r#"{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000}"#,
        )
    }

    /// The review's triangle: A (500000, 4320000), C (500030, 4320000), B (500030, 4320040); AC 30 m, CB 40 m, AB 50 m.
    #[test]
    fn the_triangle_reads_30_40_70_and_600_with_its_units_and_way() {
        let s = tm36();
        let f = Format::of(&s);
        let (a, c, b) = (
            Vec2::new(500_000.0, 4_320_000.0),
            Vec2::new(500_030.0, 4_320_000.0),
            Vec2::new(500_030.0, 4_320_040.0),
        );
        let m = path(&s, &f, &[a, c, b], None);
        assert_eq!(
            m.rows,
            [
                ("1. parça".to_owned(), f.length(30.0)),
                ("2. parça".to_owned(), f.length(40.0)),
                ("Toplam".to_owned(), f.length(70.0)),
            ]
        );
        assert_eq!(m.system.as_deref(), Some("TUREF / TM36 (EPSG:5256)"));
        assert!(
            m.method.starts_with("Düzlemde: projeksiyon"),
            "{}",
            m.method
        );
        assert_eq!(
            path(&s, &f, &[a, b], None).rows.last().map(|r| r.1.clone()),
            Some(f.length(50.0))
        );
        let m = area(&s, &f, 600.0, 120.0);
        assert_eq!(
            m.rows,
            [
                ("Alan".to_owned(), f.area(600.0)),
                ("Çevre".to_owned(), f.length(120.0))
            ]
        );
        assert_eq!(m.units, "metre, metrekare");
        let text = m.text();
        assert!(
            text.starts_with("Alan\nAlan\t")
                && text.contains("Koordinat sistemi\tTUREF / TM36 (EPSG:5256)"),
            "{text}"
        );
    }

    #[test]
    fn an_angle_says_its_unit_by_name_and_a_point_its_axes() {
        let s = tm36();
        let f = Format::of(&s);
        let m = angle(
            &s,
            &f,
            std::f64::consts::FRAC_PI_2,
            1.5 * std::f64::consts::PI,
        );
        assert_eq!(
            m.rows[0],
            ("Açı".to_owned(), f.angle(std::f64::consts::FRAC_PI_2))
        );
        assert!(m.rows[0].1.starts_with("100"), "{}", m.rows[0].1);
        assert_eq!(m.units, "gon (bir tur 400 g)");
        let p = point(&s, &f, Vec2::new(500_000.0, 4_320_000.0), None, Vec::new());
        assert_eq!(p.rows[0].0, "Y (doğu)");
        assert_eq!(p.rows[1].0, "X (kuzey)");
    }

    /// A geographic project reckons on its ellipsoid and says so: a second of arc along the equator is about 30.92 m on
    /// GRS 1980 (a = 6 378 137 m: 2πa / 1 296 000 = 30.922 m).
    #[test]
    fn a_geographic_project_measures_on_its_ellipsoid() {
        let s = settings(
            r#"{"srid":4326,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000}"#,
        );
        let f = Format::of(&s);
        let m = path(
            &s,
            &f,
            &[Vec2::new(30.0, 0.0), Vec2::new(30.0 + 1.0 / 3600.0, 0.0)],
            None,
        );
        assert!(
            m.method.starts_with("Elipsoit üstünde (jeodezik), WGS 84"),
            "{}",
            m.method
        );
        let reference = 2.0 * std::f64::consts::PI * 6_378_137.0 / 1_296_000.0;
        assert_eq!(m.rows[0].1, f.length(reference), "{:?}", m.rows);
    }
}
