//! The CRS registry file (`fixtures/crs/v1/registry.json`, written from
//! `apps/web/src/geo/crs.ts`, the single source of CRS metadata) checked against EPSG
//! facts written out here independently: SRID ↔ zone, ellipsoid, scale
//! factor and false origin of every system, and the TUREF zone suggestion
//! (nearest central meridian; on a boundary the western zone).

use serde_json::Value;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/crs/v1/registry.json"
);

fn file() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FILE).expect("registry.json")).expect("JSON")
}

fn system(file: &Value, srid: u64) -> &Value {
    file["systems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["srid"] == srid)
        .unwrap_or_else(|| panic!("EPSG:{srid} kayıtta yok"))
}

/// Transverse Mercator parameters of a projected system.
fn assert_tm(s: &Value, datum: &str, ellipsoid: &str, cm: f64, k: f64) {
    let srid = &s["srid"];
    assert_eq!(s["kind"], "projected", "{srid}");
    assert_eq!(s["datum"], datum, "{srid}");
    assert_eq!(s["ellipsoid"], ellipsoid, "{srid}");
    assert_eq!(s["centralMeridian"].as_f64(), Some(cm), "{srid}");
    assert_eq!(s["scaleFactor"].as_f64(), Some(k), "{srid}");
    assert_eq!(s["falseEasting"].as_f64(), Some(500_000.0), "{srid}");
    assert_eq!(s["falseNorthing"].as_f64(), Some(0.0), "{srid}");
    assert_eq!(s["unit"], "metre", "{srid}");
}

#[test]
fn is_a_versioned_file_with_unique_srids() {
    let f = file();
    assert_eq!(f["format"], "kentos.crs-registry");
    assert_eq!(f["version"], 1);
    let srids: Vec<u64> = f["systems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["srid"].as_u64().unwrap())
        .collect();
    let mut unique = srids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), srids.len());
    assert_eq!(
        system(&f, f["defaultSrid"].as_u64().unwrap())["name"],
        "TUREF / TM36"
    );
}

#[test]
fn matches_epsg_for_every_zone() {
    let f = file();
    for (i, cm) in (27..=45).step_by(3).enumerate() {
        let cm = cm as f64;
        // EPSG:5253–5259 TUREF / TM27–TM45 and EPSG:2319–2325 ED50 / TM27–TM45: 3° zones, k = 1.
        assert_tm(system(&f, 5253 + i as u64), "TUREF", "GRS80", cm, 1.0);
        assert_tm(
            system(&f, 2319 + i as u64),
            "ED50",
            "International 1924",
            cm,
            1.0,
        );
    }
    for zone in 35..=38u64 {
        // UTM zone n: central meridian 6n − 183, k = 0.9996.
        let cm = (6 * zone) as f64 - 183.0;
        assert_tm(
            system(&f, 23000 + zone),
            "ED50",
            "International 1924",
            cm,
            0.9996,
        );
        assert_tm(system(&f, 32600 + zone), "WGS84", "WGS84", cm, 0.9996);
    }
    for (srid, datum) in [(5252, "TUREF"), (4326, "WGS84")] {
        let s = system(&f, srid);
        assert_eq!(
            (&s["kind"], &s["datum"], &s["unit"]),
            (&"geographic".into(), &datum.into(), &"degree".into())
        );
    }
    assert_eq!(system(&f, 3857)["projection"], "Pseudo-Mercator");
}

#[test]
fn suggests_the_nearest_turef_zone() {
    let f = file();
    let cases = f["zoneSuggestions"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for c in cases {
        let lon = c["lon"].as_f64().unwrap();
        // Nearest of 27, 30 … 45; a tie keeps the western (first) meridian.
        let mut best = 27.0f64;
        for cm in (30..=45).step_by(3).map(f64::from) {
            if (cm - lon).abs() < (best - lon).abs() {
                best = cm;
            }
        }
        let srid = 5253 + ((best - 27.0) / 3.0) as u64;
        assert_eq!(c["srid"].as_u64(), Some(srid), "boylam {lon}");
    }
}
