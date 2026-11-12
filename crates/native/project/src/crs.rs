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

/// EPSG's area of use of a registry system (docs/adr/2500 §3): its text and bounding box, degrees. IOGP's Guidance
/// Note 7-1 (§5.7) gives the box as approximate, meant for selecting records, in WGS 84 terms: a point outside it is
/// a note to look at, not an error. Written from PROJ's database (fixtures/crs/v1/areas.json,
/// scripts/fixtures/crs_areas.py).
///
/// Since docs/adr/4900 a system whose EPSG extent is not just its box carries the extent's polygon too
/// (fixtures/crs/v1/area-polygons.json, from the EPSG Dataset unchanged): a point is then read against the polygon.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct AreaOfUse {
    pub srid: u32,
    pub name: String,
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
    /// The EPSG extent's polygon: its rings (a multipolygon's parts one after another), longitude and latitude in
    /// degrees, read even-odd; empty: the box is read.
    #[serde(skip)]
    pub polygon: Vec<Vec<[f64; 2]>>,
    /// The EPSG extent's code, when its polygon is carried.
    #[serde(skip)]
    pub extent: Option<u32>,
}

impl AreaOfUse {
    /// Whether the box holds a longitude and latitude (degrees), its edges included; a box whose west lies east of
    /// its east crosses the antimeridian.
    pub fn holds(&self, lon: f64, lat: f64) -> bool {
        box_holds([self.west, self.south, self.east, self.north], lon, lat)
    }

    /// The box, degrees west, south, east, north.
    pub fn bbox(&self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }

    /// Whether the area holds a point as a note reads it (docs/adr/3700, 4900): inside its polygon or within
    /// [`EDGE`] of one of its edges when it has one, else in its box with every edge [`EDGE`] out.
    pub fn holds_near(&self, lon: f64, lat: f64) -> bool {
        if self.polygon.is_empty() {
            return box_holds_near(self.bbox(), lon, lat);
        }
        polygon_holds(&self.polygon, lon, lat) || near_edge(&self.polygon, lon, lat, EDGE)
    }
}

/// Whether rings (read even-odd) hold a point: a ray to the east crosses their edges an odd number of times.
pub fn polygon_holds(rings: &[Vec<[f64; 2]>], x: f64, y: f64) -> bool {
    let mut inside = false;
    for r in rings {
        for w in r.windows(2) {
            let ([x1, y1], [x2, y2]) = (w[0], w[1]);
            if (y1 > y) != (y2 > y) {
                let at = x1 + (y - y1) * (x2 - x1) / (y2 - y1);
                if x < at {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Whether a point is within `d` (degrees, read as plane) of one of the rings' edges.
fn near_edge(rings: &[Vec<[f64; 2]>], x: f64, y: f64, d: f64) -> bool {
    rings.iter().flat_map(|r| r.windows(2)).any(|w| {
        let ([x1, y1], [x2, y2]) = (w[0], w[1]);
        let (vx, vy) = (x2 - x1, y2 - y1);
        let l2 = vx * vx + vy * vy;
        let t = if l2 == 0.0 { 0.0 } else { (((x - x1) * vx + (y - y1) * vy) / l2).clamp(0.0, 1.0) };
        let (dx, dy) = (x - x1 - vx * t, y - y1 - vy * t);
        dx * dx + dy * dy <= d * d
    })
}

/// A place read against an area of use: a registry system's (its polygon where it has one) or an operation's box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    Area(&'static AreaOfUse),
    Box([f64; 4]),
}

impl Region {
    /// [`AreaOfUse::holds_near`], or for a box [`box_holds_near`].
    pub fn holds_near(&self, lon: f64, lat: f64) -> bool {
        match self {
            Region::Area(a) => a.holds_near(lon, lat),
            Region::Box(b) => box_holds_near(*b, lon, lat),
        }
    }
}

/// How near an edge (degrees) a point is read as on it when a point is noted against an area of use (docs/adr/3700):
/// 10⁻⁹° (about 0.1 mm), far below an EPSG box's own precision (two decimals) and the noise two engines' inverse
/// projections leave on a point exactly on an edge (a central meridian that is a zone's border), so the note does not
/// turn on it.
pub const EDGE: f64 = 1e-9;

/// [`box_holds`] with every edge [`EDGE`] out (docs/adr/3700): the box a point's note reads.
pub fn box_holds_near([west, south, east, north]: [f64; 4], lon: f64, lat: f64) -> bool {
    box_holds([west - EDGE, south - EDGE, east + EDGE, north + EDGE], lon, lat)
}

/// Whether a box (degrees west, south, east, north) holds a longitude and latitude, its edges included.
pub fn box_holds([west, south, east, north]: [f64; 4], lon: f64, lat: f64) -> bool {
    let along = if west <= east {
        (west..=east).contains(&lon)
    } else {
        lon >= west || lon <= east
    };
    along && (south..=north).contains(&lat)
}

/// EPSG's areas of use of the registry's systems, the local one has none.
pub fn areas() -> &'static [AreaOfUse] {
    static AREAS: OnceLock<Vec<AreaOfUse>> = OnceLock::new();
    AREAS.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct File {
            areas: Vec<AreaOfUse>,
        }
        #[derive(serde::Deserialize)]
        struct Extent {
            code: u32,
            srids: Vec<u32>,
            rings: Vec<Vec<[f64; 2]>>,
        }
        #[derive(serde::Deserialize)]
        struct Polygons {
            extents: Vec<Extent>,
        }
        let mut areas = serde_json::from_str::<File>(include_str!("../../../../fixtures/crs/v1/areas.json"))
            .map(|f| f.areas)
            .unwrap_or_default();
        // docs/adr/4900: the polygons of the extents that are not just their boxes.
        let polygons = serde_json::from_str::<Polygons>(include_str!("../../../../fixtures/crs/v1/area-polygons.json"))
            .map(|p| p.extents)
            .unwrap_or_default();
        for a in &mut areas {
            if let Some(x) = polygons.iter().find(|x| x.srids.contains(&a.srid)) {
                a.polygon = x.rings.clone();
                a.extent = Some(x.code);
            }
        }
        areas
    })
}

/// docs/adr/5300: the area of an EPSG extent whose polygon is carried (a registry system's area of that extent).
pub fn extent_area(code: u32) -> Option<&'static AreaOfUse> {
    areas().iter().find(|a| a.extent == Some(code))
}

/// docs/adr/5300: the region a datum operation's points are read against: its extent's polygon where carried, else
/// its box; none for an operation without an area (the datum stays).
pub fn operation_region(op: &kentos_geometry_core::crs::operation::Operation) -> Option<Region> {
    match (op.extent.and_then(extent_area), op.area) {
        (Some(a), _) => Some(Region::Area(a)),
        (None, Some((_, b))) => Some(Region::Box(b)),
        (None, None) => None,
    }
}

/// EPSG's area of use of a registry system; none for the local system and a code the registry does not hold.
pub fn area(srid: u32) -> Option<&'static AreaOfUse> {
    areas().iter().find(|a| a.srid == srid)
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

    #[test]
    fn the_turkish_zones_read_their_polygons() {
        // docs/adr/4900: TUREF / TM30's extent leaves the Sea of Marmara out; its box holds it.
        let tm30 = area(5254).expect("TM30");
        assert_eq!(tm30.extent, Some(1525));
        let (lon, lat) = (28.9, 40.75);
        assert!(tm30.holds(lon, lat), "in the box");
        assert!(!tm30.holds_near(lon, lat), "out of the polygon");
        assert!(tm30.holds_near(29.5, 39.5), "Kütahya's plain");
        // A band whose polygon is its box carries none.
        assert!(area(32636).expect("UTM 36N").polygon.is_empty());
    }

    /// docs/adr/5300: the datum operations' area is extent 1237's polygon: Burgas is in their box, not in it.
    #[test]
    fn a_datum_operation_reads_its_extent_s_polygon() {
        let ed50 = system(23035).and_then(System::transform_system).expect("ED50 / UTM 35N");
        let turef = system(5252).and_then(System::transform_system).expect("TUREF");
        let op = kentos_geometry_core::crs::operation::operation(&ed50, &turef, &[], false).expect("a way");
        let region = operation_region(&op).expect("an area");
        assert!(matches!(region, Region::Area(a) if a.extent == Some(1237)));
        assert!(box_holds(op.area.expect("a box").1, 27.47, 42.5), "Burgas is in the box");
        assert!(!region.holds_near(27.47, 42.5), "not in the polygon");
        assert!(region.holds_near(32.86, 39.93), "Ankara");
        // The same datum: no operation's area.
        let tm30 = system(5254).and_then(System::transform_system).expect("TM30");
        let same = kentos_geometry_core::crs::operation::operation(&tm30, &turef, &[], false).expect("a way");
        assert!(operation_region(&same).is_none());
    }

    /// Every registry system but the local one has EPSG's area of use (docs/adr/2500 §3); the boxes hold their own
    /// zone's middle and not a neighbour's, and one crossing the antimeridian holds both its ends.
    #[test]
    fn every_registry_system_but_the_local_one_has_its_area_of_use() {
        for s in systems() {
            assert_eq!(area(s.srid).is_some(), !s.is_local(), "{}", s.name);
        }
        assert_eq!(areas().len(), systems().len() - 1);
        let tm36 = area(5256).expect("TUREF / TM36");
        assert!(tm36.holds(36.0, 39.0));
        assert!(!tm36.holds(33.0, 39.0) && !tm36.holds(36.0, 44.0));
        assert!(tm36.holds(tm36.west, tm36.south) && tm36.holds(tm36.east, tm36.north));
        assert!(area(4326).expect("WGS 84").holds(-179.9, -89.0));
        assert!(box_holds([170.0, -50.0, -170.0, -30.0], 179.0, -40.0));
        assert!(box_holds([170.0, -50.0, -170.0, -30.0], -175.0, -40.0));
        assert!(!box_holds([170.0, -50.0, -170.0, -30.0], 0.0, -40.0));
    }

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
