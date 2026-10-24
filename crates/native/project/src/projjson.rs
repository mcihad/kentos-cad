//! The registry's systems as PROJJSON (PROJ's JSON of ISO 19111; GeoParquet's `crs`, docs/adr/0628 §3): EPSG's
//! definition written from the registry's numbers as the WKT2 writer writes it (`wkt2`), with the code's `id`; and
//! the code a PROJJSON names. pyproj reads what is written as the code's system (scripts/fixtures/parquet_cases.py).

use serde_json::{Value, json};

use crate::crs::System;

fn datum(name: &str) -> Option<(&'static str, &'static str, &'static str, f64, f64)> {
    Some(match name {
        "TUREF" => (
            "TUREF",
            "Turkish National Reference Frame",
            "GRS 1980",
            6_378_137.0,
            298.257222101,
        ),
        "ED50" => (
            "ED50",
            "European Datum 1950",
            "International 1924",
            6_378_388.0,
            297.0,
        ),
        "WGS84" => (
            "WGS 84",
            "World Geodetic System 1984",
            "WGS 84",
            6_378_137.0,
            298.257223563,
        ),
        _ => return None,
    })
}

fn id(code: u32) -> Value {
    json!({ "authority": "EPSG", "code": code })
}

fn ellipsoidal() -> Value {
    json!({
        "subtype": "ellipsoidal",
        "axis": [
            { "name": "Geodetic latitude", "abbreviation": "Lat", "direction": "north", "unit": "degree" },
            { "name": "Geodetic longitude", "abbreviation": "Lon", "direction": "east", "unit": "degree" }
        ]
    })
}

fn parameter(name: &str, value: f64, unit: &str, code: u32) -> Value {
    json!({ "name": name, "value": value, "unit": unit, "id": id(code) })
}

/// A system of the registry as PROJJSON; none for the local system and a datum not known.
pub fn projjson(s: &System) -> Option<Value> {
    if s.is_local() {
        return None;
    }
    let (base, datum_name, ellipsoid, a, rf) = datum(&s.datum)?;
    let datum = json!({
        "type": "GeodeticReferenceFrame",
        "name": datum_name,
        "ellipsoid": { "name": ellipsoid, "semi_major_axis": a, "inverse_flattening": rf }
    });
    let schema = "https://proj.org/schemas/v0.7/projjson.schema.json";
    if s.kind == "geographic" {
        return Some(json!({
            "$schema": schema,
            "type": "GeographicCRS",
            "name": s.name,
            "datum": datum,
            "coordinate_system": ellipsoidal(),
            "id": id(s.srid)
        }));
    }
    let (method, params) = match s.projection.as_deref()? {
        "Transverse Mercator" | "UTM" => (
            json!({ "name": "Transverse Mercator", "id": id(9807) }),
            vec![
                parameter("Latitude of natural origin", 0.0, "degree", 8801),
                parameter(
                    "Longitude of natural origin",
                    s.central_meridian?,
                    "degree",
                    8802,
                ),
                parameter(
                    "Scale factor at natural origin",
                    s.scale_factor?,
                    "unity",
                    8805,
                ),
                parameter(
                    "False easting",
                    s.false_easting.unwrap_or(0.0),
                    "metre",
                    8806,
                ),
                parameter(
                    "False northing",
                    s.false_northing.unwrap_or(0.0),
                    "metre",
                    8807,
                ),
            ],
        ),
        "Pseudo-Mercator" => (
            json!({ "name": "Popular Visualisation Pseudo Mercator", "id": id(1024) }),
            vec![
                parameter("Latitude of natural origin", 0.0, "degree", 8801),
                parameter("Longitude of natural origin", 0.0, "degree", 8802),
                parameter("False easting", 0.0, "metre", 8806),
                parameter("False northing", 0.0, "metre", 8807),
            ],
        ),
        _ => return None,
    };
    let east = json!({ "name": "Easting", "abbreviation": if s.axis_order == "ne" { "Y" } else { "X" }, "direction": "east", "unit": "metre" });
    let north = json!({ "name": "Northing", "abbreviation": if s.axis_order == "ne" { "X" } else { "Y" }, "direction": "north", "unit": "metre" });
    let axes = if s.axis_order == "ne" {
        vec![north, east]
    } else {
        vec![east, north]
    };
    let conversion = s
        .name
        .rsplit_once(" / ")
        .map_or(s.name.as_str(), |(_, c)| c);
    Some(json!({
        "$schema": schema,
        "type": "ProjectedCRS",
        "name": s.name,
        "base_crs": { "name": base, "datum": datum, "coordinate_system": ellipsoidal() },
        "conversion": { "name": conversion, "method": method, "parameters": params },
        "coordinate_system": { "subtype": "Cartesian", "axis": axes },
        "id": id(s.srid)
    }))
}

/// The EPSG code an authority and a code name: EPSG's own, OGC's CRS84 as 4326 (longitude first, as KentOS takes
/// geographic coordinates).
fn code_of(authority: &str, code: &Value) -> Option<u32> {
    let text = match code {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return None,
    };
    match authority.to_ascii_uppercase().as_str() {
        "EPSG" => text.parse().ok(),
        "OGC" if matches!(text.as_str(), "CRS84" | "CRS84h") => Some(4326),
        _ => None,
    }
}

/// The system a GeoParquet column's `crs` names: `Ok(None)` for `null` (not known); for a PROJJSON its `id` (or the
/// first of its `ids`); for a text `EPSG:5256` or `OGC:CRS84`. A system without a code is refused with its name.
pub fn srid_of(crs: &Value) -> Result<Option<u32>, String> {
    match crs {
        Value::Null => Ok(None),
        Value::String(s) => {
            let (authority, code) = s
                .split_once(':')
                .ok_or_else(|| format!("“{s}” bir sistem adı değil"))?;
            code_of(
                authority,
                &Value::String(code.trim_start_matches(':').to_owned()),
            )
            .map(Some)
            .ok_or_else(|| format!("“{s}” KentOS'un tanıdığı bir sistem kodu değil"))
        }
        Value::Object(o) => {
            let ids = o
                .get("id")
                .into_iter()
                .chain(o.get("ids").and_then(Value::as_array).into_iter().flatten());
            for i in ids {
                if let (Some(a), Some(c)) =
                    (i.get("authority").and_then(Value::as_str), i.get("code"))
                    && let Some(code) = code_of(a, c)
                {
                    return Ok(Some(code));
                }
            }
            let name = o
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("adsız sistem");
            Err(format!("“{name}” sisteminin bir EPSG kodu yok"))
        }
        _ => Err("sistemin tanımı okunamadı".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registry_system_but_the_local_one_has_its_json_and_reads_back() {
        for s in crate::crs::systems() {
            match projjson(s) {
                Some(v) => assert_eq!(srid_of(&v), Ok(Some(s.srid)), "{}", s.name),
                None => assert!(s.is_local(), "{}", s.name),
            }
        }
        assert_eq!(srid_of(&Value::Null), Ok(None));
        assert_eq!(
            srid_of(&json!({ "id": { "authority": "OGC", "code": "CRS84" } })),
            Ok(Some(4326))
        );
        assert_eq!(srid_of(&json!("EPSG:2320")), Ok(Some(2320)));
        assert!(srid_of(&json!({ "name": "Yerel" })).is_err());
    }
}
