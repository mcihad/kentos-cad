//! The field book's reduction (docs/adr/0169 §3) against
//! fixtures/field/v1/reduce.json (scripts/fixtures/field_reduce_cases.py:
//! mpmath, 50 digits, from the rules alone): every row's faces, the
//! observations it came from, its reading and zenith in face I, the faces'
//! differences, the horizontal distance and the height difference, within
//! the file's tolerances; the observations left out. The web runs the same
//! file through WASM (`apps/web/src/wasm/fieldReduce.wasm.test.ts`).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::survey::Unit;
use kentos_geometry_core::survey::fieldbook::{Station, reduce};
use serde_json::Value;

fn near(got: Option<f64>, want: &Value, tol: f64, what: &str) {
    match (got, want.as_f64()) {
        (Some(g), Some(w)) => assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w}"),
        (None, None) => {}
        (g, _) => panic!("{what}: {g:?} ≠ {want}"),
    }
}

#[test]
fn the_field_book_reduces_as_the_reference_does() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/reduce.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-reduce");
    let (metres, angle) = (
        file["tolerance"]["metres"].as_f64().expect("metres"),
        file["tolerance"]["angle"].as_f64().expect("angle"),
    );
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let unit = Unit::parse(case["unit"].as_str().expect("a unit")).expect("a unit");
        let k = case["k"].as_f64().expect("k");
        let station = Station::from_json(&Json::parse(&case["setup"].to_string()).expect("JSON"))
            .expect("a station");
        let got = reduce(&station, unit, k);
        let want = &case["expect"];
        let rows = want["rows"].as_array().expect("rows");
        assert_eq!(got.rows.len(), rows.len(), "{name}: rows");
        for (g, w) in got.rows.iter().zip(rows) {
            assert_eq!(g.target, w["target"].as_str().expect("a target"), "{name}");
            assert_eq!(
                g.faces as u64,
                w["faces"].as_u64().expect("faces"),
                "{name}"
            );
            let obs: Vec<u64> = w["observations"]
                .as_array()
                .expect("observations")
                .iter()
                .map(|v| v.as_u64().expect("an index"))
                .collect();
            assert_eq!(
                g.observations.iter().map(|&i| i as u64).collect::<Vec<_>>(),
                obs,
                "{name}"
            );
            near(Some(g.hz), &w["hz"], angle, &format!("{name}: hz"));
            near(g.zenith, &w["zenith"], angle, &format!("{name}: zenith"));
            near(g.hz_diff, &w["hzDiff"], angle, &format!("{name}: hzDiff"));
            near(g.index, &w["index"], angle, &format!("{name}: index"));
            near(g.slope, &w["slope"], metres, &format!("{name}: slope"));
            near(
                g.slope_diff,
                &w["slopeDiff"],
                metres,
                &format!("{name}: slopeDiff"),
            );
            near(
                g.target_height,
                &w["targetHeight"],
                metres,
                &format!("{name}: targetHeight"),
            );
            near(
                g.horizontal,
                &w["horizontal"],
                metres,
                &format!("{name}: horizontal"),
            );
            near(g.dh, &w["dh"], metres, &format!("{name}: dh"));
        }
        let problems: Vec<u64> = want["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .map(|p| p["observation"].as_u64().expect("an index"))
            .collect();
        assert_eq!(
            got.problems
                .iter()
                .map(|p| p.observation as u64)
                .collect::<Vec<_>>(),
            problems,
            "{name}: problems"
        );
    }
}
