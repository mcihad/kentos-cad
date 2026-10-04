//! The forward transverse Mercator projection against PROJ
//! (`fixtures/geodesy/v1/tm-forward.json`, written by
//! `scripts/fixtures/tm_cases.py` from PROJ's `tmerc`; docs/adr/0165 §3):
//! every case within a micrometre, through the function and through the
//! call table the web uses. The web reads the same file through WASM
//! (`apps/web/src/model/geom/geodesy.test.ts`).

use kentos_geometry_core::api;
use kentos_geometry_core::geodesy::{Tm, tm_forward};
use kentos_geometry_core::jsmath::js_max;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/geodesy/v1/tm-forward.json");

fn tm(v: &Value) -> Tm {
    let f = |k: &str| v[k].as_f64().unwrap_or(f64::NAN);
    Tm {
        central_meridian: f("centralMeridian"),
        scale_factor: f("scaleFactor"),
        false_easting: f("falseEasting"),
        false_northing: f("falseNorthing"),
        semi_major: f("semiMajor"),
        inverse_flattening: f("inverseFlattening"),
    }
}

#[test]
fn every_case_lands_where_proj_puts_it() {
    let file: Value = serde_json::from_str(CASES).expect("the cases are JSON");
    assert_eq!(file["format"], "kentos.tm-forward-cases");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 80);
    let mut off = Vec::new();
    for c in cases {
        let (lat, lon) = (
            c["lat"].as_f64().unwrap_or(0.0),
            c["lon"].as_f64().unwrap_or(0.0),
        );
        let want = (
            c["east"].as_f64().unwrap_or(0.0),
            c["north"].as_f64().unwrap_or(0.0),
        );
        let got = tm_forward(&tm(&c["tm"]), lat, lon).expect("a point");
        let far = js_max((got.x - want.0).abs(), (got.y - want.1).abs());
        if far > 1e-6 {
            off.push(format!("{} {lat},{lon}: {far:e} m", c["grid"]));
        }
        // The call table gives the same bits as the function.
        let args = format!("[{},{lat},{lon}]", c["tm"]);
        let through: Value =
            serde_json::from_str(&api::run_named("tmForward", &args).expect("runs")).expect("JSON");
        assert_eq!(
            (through["x"].as_f64(), through["y"].as_f64()),
            (Some(got.x), Some(got.y))
        );
    }
    assert!(off.is_empty(), "{off:#?}");
}
