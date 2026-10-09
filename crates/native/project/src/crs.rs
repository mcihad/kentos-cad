//! The coordinate systems KentOS knows (the web's `geo/crs.ts`): the
//! registry the web also reads (fixtures/crs/v1/registry.json), in its
//! order, its systems grouped by datum as the web's list is. A project, a
//! new project and a file's statement choose from it; setting a system is
//! not a transformation (CLAUDE.md §5).

use std::f64::consts::PI;
use std::sync::OnceLock;

use kentos_contracts::Vec2;
use kentos_geometry_core::geodesy::{Tm, tm_forward};

/// A coordinate system of the registry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub srid: u32,
    pub name: String,
    /// `projected`, `geographic` or `local` (SRID 0: no coordinate system, docs/adr/0165 §2).
    pub kind: String,
    /// `TUREF`, `ED50`, `WGS84`, or `LOCAL` for the local system.
    pub datum: String,
    /// `Transverse Mercator`, `UTM` or `Pseudo-Mercator`; none for a geographic system.
    #[serde(default)]
    pub projection: Option<String>,
    #[serde(default)]
    pub false_easting: Option<f64>,
    #[serde(default)]
    pub false_northing: Option<f64>,
    /// None for the local system.
    #[serde(default)]
    pub ellipsoid: Option<String>,
    #[serde(default)]
    pub central_meridian: Option<f64>,
    #[serde(default)]
    pub scale_factor: Option<f64>,
    /// `metre` or `degree`.
    pub unit: String,
    /// The order EPSG gives its axes: `en`, `ne` (TUREF's and ED50's TM
    /// zones) or `latlon` (docs/adr/0208 §5).
    #[serde(default)]
    pub axis_order: String,
    /// Where the system is meant for (“25.5°–28.5° D (3° dilim)”).
    #[serde(default)]
    pub area: Option<String>,
}

/// The registry's systems, their datums in the order they first appear.
pub fn systems() -> &'static [System] {
    static SYSTEMS: OnceLock<Vec<System>> = OnceLock::new();
    SYSTEMS.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Registry {
            systems: Vec<System>,
        }
        let all = serde_json::from_str::<Registry>(include_str!(
            "../../../../fixtures/crs/v1/registry.json"
        ))
        .map(|r| r.systems)
        .unwrap_or_default();
        // The web's list: one group per datum, in the order the datums first come.
        let mut datums: Vec<&str> = Vec::new();
        for s in &all {
            if !datums.contains(&s.datum.as_str()) {
                datums.push(&s.datum);
            }
        }
        datums
            .iter()
            .flat_map(|d| all.iter().filter(move |s| s.datum == *d).cloned())
            .collect()
    })
}

pub fn system(srid: u32) -> Option<&'static System> {
    systems().iter().find(|s| s.srid == srid)
}

/// The web's `DATUM_LABEL`.
pub fn datum_label(datum: &str) -> &str {
    match datum {
        "TUREF" => "TUREF (ITRF96)",
        "ED50" => "ED50",
        "WGS84" => "WGS 84",
        "LOCAL" => "Yerel",
        other => other,
    }
}

/// The local system's SRID: not an EPSG code; PostGIS reads 0 as “unknown” (docs/adr/0165 §2).
pub const LOCAL_SRID: u32 = 0;

/// A system as a sentence names it (the web's `crsTitle`): “TUREF / TM36
/// (EPSG:5256)”, the local one “Yerel (koordinat sistemi yok)”.
pub fn title(s: &System) -> String {
    if s.is_local() {
        "Yerel (koordinat sistemi yok)".to_owned()
    } else {
        format!("{} (EPSG:{})", s.name, s.srid)
    }
}

/// A system's code as a value or a chip shows it (the web's `crsCode`):
/// “EPSG:5256”; the local one is no EPSG code: “SRID 0”.
pub fn code(s: &System) -> String {
    if s.is_local() {
        format!("SRID {}", s.srid)
    } else {
        format!("EPSG:{}", s.srid)
    }
}

impl System {
    /// Whether this is the local system: no coordinate system, coordinates bound to no place.
    pub fn is_local(&self) -> bool {
        self.kind == "local"
    }

    /// Its transverse Mercator projection with its ellipsoid (a TM3 or UTM
    /// zone); none for the others.
    pub fn tm(&self) -> Option<Tm> {
        if !matches!(
            self.projection.as_deref(),
            Some("Transverse Mercator" | "UTM")
        ) {
            return None;
        }
        let (semi_major, inverse_flattening) = ellipsoid(self.ellipsoid.as_deref()?)?;
        Some(Tm {
            central_meridian: self.central_meridian?,
            scale_factor: self.scale_factor.unwrap_or(1.0),
            false_easting: self.false_easting.unwrap_or(0.0),
            false_northing: self.false_northing.unwrap_or(0.0),
            semi_major,
            inverse_flattening,
        })
    }

    /// The system as the transforms read it (docs/adr/0167 §3; the web's
    /// `systemOf`); none for the local one and an unknown datum.
    pub fn transform_system(&self) -> Option<kentos_geometry_core::crs::System> {
        use kentos_geometry_core::crs::{Datum, System as Of};
        if self.projection.as_deref() == Some("Pseudo-Mercator") {
            return Some(Of::Mercator {});
        }
        let datum = match self.datum.as_str() {
            "TUREF" => Datum::Turef,
            "ED50" => Datum::Ed50,
            "WGS84" => Datum::Wgs84,
            _ => return None,
        };
        match self.kind.as_str() {
            "geographic" => Some(Of::Geographic { datum }),
            "projected" => {
                let tm = self.tm()?;
                Some(Of::Tm {
                    datum,
                    latitude_of_origin: None,
                    central_meridian: tm.central_meridian,
                    scale_factor: tm.scale_factor,
                    false_easting: tm.false_easting,
                    false_northing: tm.false_northing,
                })
            }
            _ => None,
        }
    }
}

/// An ellipsoid of the registry: semi-major axis (m) and inverse flattening
/// (the web's `ELLIPSOIDS`).
pub fn ellipsoid(name: &str) -> Option<(f64, f64)> {
    match name {
        "GRS80" => Some((6_378_137.0, 298.257_222_101)),
        "WGS84" => Some((6_378_137.0, 298.257_223_563)),
        "International 1924" => Some((6_378_388.0, 297.0)),
        _ => None,
    }
}

/// A latitude and longitude (degrees) in `crs`: a geographic system's
/// longitude and latitude, a projected one's grid point (east, north);
/// none for the local system and where the projection cannot reach (the
/// web's `projectLatLon`, docs/adr/0165 §3). A start view's, not a datum
/// transformation: the degrees are taken on the system's own ellipsoid.
pub fn project_lat_lon(crs: &System, lat: f64, lon: f64) -> Option<Vec2> {
    if crs.is_local() {
        return None;
    }
    if crs.kind == "geographic" {
        return Some(Vec2 { x: lon, y: lat });
    }
    if crs.projection.as_deref() == Some("Pseudo-Mercator") {
        let r = 6_378_137.0;
        let x = r * (lon * PI / 180.0);
        let y = r * libm::log(libm::tan(PI / 4.0 + lat * PI / 360.0));
        return (x.is_finite() && y.is_finite()).then_some(Vec2 { x, y });
    }
    let p = tm_forward(&crs.tm()?, lat, lon)?;
    Some(Vec2 { x: p.x, y: p.y })
}

/// The TM3 zones' central meridians, west to east.
const TM_MERIDIANS: [f64; 7] = [27.0, 30.0, 33.0, 36.0, 39.0, 42.0, 45.0];

/// The TUREF TM3 zone suggested for a longitude (degrees east): the nearest
/// central meridian, the western one on a boundary (the web's
/// `turefZoneFor`; the registry's `zoneSuggestions` are its cases).
pub fn turef_zone_for(lon: f64) -> Option<&'static System> {
    let mut best = 0;
    for (i, m) in TM_MERIDIANS.iter().enumerate() {
        if (m - lon).abs() < (TM_MERIDIANS[best] - lon).abs() {
            best = i;
        }
    }
    system(5253 + best as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every system of the transform reference is the registry's entry as
    /// the transforms read it (fixtures/geodesy/v1/transform.json; the web
    /// checks its `systemOf` against the same).
    #[test]
    fn the_registry_gives_the_references_systems() {
        use kentos_geometry_core::api::json::{FromJson, Json};
        let file: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/geodesy/v1/transform.json"
        ))
        .expect("the reference reads");
        for case in file["transform"].as_array().expect("cases") {
            for (srid, json) in [("fromSrid", "from"), ("toSrid", "to")] {
                let srid = case[srid].as_u64().expect("an SRID") as u32;
                let want = kentos_geometry_core::crs::System::from_json(
                    &Json::parse(&case[json].to_string()).expect("JSON"),
                )
                .expect("a system");
                let got = system(srid).and_then(System::transform_system);
                assert_eq!(got, Some(want), "{srid}");
            }
        }
        assert_eq!(system(0).and_then(System::transform_system), None);
    }

    /// The registry's own cases (`zoneSuggestions`, written by the web from its `turefZoneFor`).
    #[test]
    fn the_zone_suggested_is_the_webs() {
        #[derive(serde::Deserialize)]
        struct Case {
            lon: f64,
            srid: Option<u32>,
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Registry {
            zone_suggestions: Vec<Case>,
        }
        let r: Registry =
            serde_json::from_str(include_str!("../../../../fixtures/crs/v1/registry.json"))
                .expect("the registry reads");
        assert!(r.zone_suggestions.len() >= 10);
        for c in r.zone_suggestions {
            assert_eq!(turef_zone_for(c.lon).map(|s| s.srid), c.srid, "{}", c.lon);
        }
    }

    /// On its central meridian a TM point is at the false easting; a
    /// geographic system keeps the degrees; the local system has no place.
    #[test]
    fn a_latitude_and_longitude_land_in_the_system() {
        let tm36 = system(5256).expect("TM36");
        assert_eq!(
            project_lat_lon(tm36, 39.0, 36.0).map(|p| p.x),
            Some(500_000.0)
        );
        assert_eq!(
            project_lat_lon(system(4326).expect("WGS 84"), 39.0, 36.0),
            Some(Vec2 { x: 36.0, y: 39.0 })
        );
        assert!(project_lat_lon(system(LOCAL_SRID).expect("local"), 39.0, 36.0).is_none());
        assert!(system(4326).expect("WGS 84").tm().is_none());
        assert_eq!(
            system(32636).expect("UTM 36N").tm().map(|t| t.scale_factor),
            Some(0.9996)
        );
    }
}
