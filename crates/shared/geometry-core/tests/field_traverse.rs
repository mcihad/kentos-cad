//! A field book's traverse (docs/adr/0169 §3) against
//! fixtures/field/v1/traverse.json (scripts/fixtures/field_traverse_cases.py:
//! the reduction's mpmath reference, from the rules alone): the stations in
//! order, each angle from the back target to the fore target, each leg's
//! distances from both ends with their mean and difference, the targets a
//! station has no row for. The web runs the same file through WASM
//! (`apps/web/src/wasm/fieldReduce.wasm.test.ts`).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::survey::Unit;
use kentos_geometry_core::survey::fieldbook::{Station, Tolerances, reduce, traverse_transfer};
use serde_json::Value;

fn near(got: Option<f64>, want: &Value, tol: f64, what: &str) {
    match (got, want.as_f64()) {
        (Some(g), Some(w)) => assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w}"),
        (None, None) => {}
        (g, _) => panic!("{what}: {g:?} ≠ {want}"),
    }
}

#[test]
fn a_field_books_traverse_is_the_references() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/traverse.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-traverse");
    let (metres, angle) = (
        file["tolerance"]["metres"].as_f64().expect("metres"),
        file["tolerance"]["angle"].as_f64().expect("angle"),
    );
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 6);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let unit = Unit::parse(case["unit"].as_str().expect("a unit")).expect("a unit");
        let to = Unit::parse(case["to"].as_str().expect("a unit")).expect("a unit");
        let k = case["k"].as_f64().expect("k");
        let reduced: Vec<(String, _)> = case["book"]
            .as_array()
            .expect("a book")
            .iter()
            .map(|s| {
                let station = Station::from_json(&Json::parse(&s.to_string()).expect("JSON"))
                    .expect("a station");
                let r = reduce(&station, unit, k, &Tolerances::default());
                (station.station, r)
            })
            .collect();
        let got = traverse_transfer(
            &reduced,
            case["back"].as_str().expect("a back sight"),
            case["fore"].as_str(),
            unit,
            to,
            case["twoWay"].as_f64(),
        );
        let want = &case["expect"];
        let stations: Vec<&str> = want["stations"]
            .as_array()
            .expect("stations")
            .iter()
            .map(|v| v.as_str().expect("a name"))
            .collect();
        assert_eq!(got.stations, stations, "{name}");
        let angles = want["angles"].as_array().expect("angles");
        assert_eq!(got.angles.len(), angles.len(), "{name}");
        for (i, (g, w)) in got.angles.iter().zip(angles).enumerate() {
            near(*g, w, angle, &format!("{name}: angle {i}"));
        }
        let legs = want["legs"].as_array().expect("legs");
        assert_eq!(got.legs.len(), legs.len(), "{name}");
        for (g, w) in got.legs.iter().zip(legs) {
            assert_eq!(
                (g.from.as_str(), g.to.as_str()),
                (
                    w["from"].as_str().expect("from"),
                    w["to"].as_str().expect("to")
                ),
                "{name}"
            );
            for (key, value) in [
                ("forward", g.forward),
                ("backward", g.backward),
                ("mean", g.mean),
                ("diff", g.diff),
            ] {
                near(value, &w[key], metres, &format!("{name}: {key}"));
            }
            assert_eq!(g.over, w["over"].as_bool().expect("over"), "{name}: over");
        }
        let missing: Vec<(&str, &str)> = want["missing"]
            .as_array()
            .expect("missing")
            .iter()
            .map(|m| {
                (
                    m["station"].as_str().expect("a station"),
                    m["target"].as_str().expect("a target"),
                )
            })
            .collect();
        assert_eq!(
            got.missing
                .iter()
                .map(|m| (m.station.as_str(), m.target.as_str()))
                .collect::<Vec<_>>(),
            missing,
            "{name}"
        );
    }
}

/// Poligon hesabı's misclosures against the project's tolerances, as the
/// reference compares them (the angle turned into the unit, an equal one
/// within its tolerance).
#[test]
fn a_traverses_closure_is_checked_as_the_reference_checks_it() {
    use kentos_geometry_core::survey::traverse::closure;
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/traverse.json"))
            .expect("the cases read");
    let cases = file["closures"].as_array().expect("closures");
    assert!(cases.len() >= 5);
    for c in cases {
        let unit = Unit::parse(c["unit"].as_str().expect("a unit")).expect("a unit");
        let got = closure(
            unit,
            c["angleMisclosure"].as_f64(),
            c["linearMisclosure"].as_f64(),
            c["angle"].as_f64(),
            c["coord"].as_f64(),
        );
        assert_eq!(
            (got.angle_over, got.coord_over),
            (c["angleOver"].as_bool(), c["coordOver"].as_bool()),
            "{c}"
        );
    }
}
