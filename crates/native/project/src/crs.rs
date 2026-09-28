//! The coordinate systems KentOS knows (the web's `geo/crs.ts`): the
//! registry the web also reads (fixtures/crs/v1/registry.json), in its
//! order, its systems grouped by datum as the web's list is. A project, a
//! new project and a file's statement choose from it; setting a system is
//! not a transformation (CLAUDE.md §5).

use std::sync::OnceLock;

/// A coordinate system of the registry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub srid: u32,
    pub name: String,
    /// `projected` or `geographic`.
    pub kind: String,
    /// `TUREF`, `ED50` or `WGS84`.
    pub datum: String,
    /// `Transverse Mercator`, `UTM` or `Pseudo-Mercator`; none for a geographic system.
    #[serde(default)]
    pub projection: Option<String>,
    #[serde(default)]
    pub false_easting: Option<f64>,
    #[serde(default)]
    pub false_northing: Option<f64>,
    pub ellipsoid: String,
    #[serde(default)]
    pub central_meridian: Option<f64>,
    #[serde(default)]
    pub scale_factor: Option<f64>,
    /// `metre` or `degree`.
    pub unit: String,
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
        other => other,
    }
}
