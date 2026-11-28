//! A region's least width (docs/adr/8110) against the independent reference in `fixtures/geometry/v1/least-width.json`
//! (`scripts/fixtures/least_width_cases.py`: Shapely, no KentOS code): corridors, an L, a T and a cross, door reveals and
//! niches, a pier, waists, a dead end tapering, a lobby, a curved corridor between halls, far-out coordinates, a ring
//! drawn either way round, free-standing piers.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::neck::least_width;
use serde_json::Value;

/// The circles are found to within this, metres; the reference to within a micrometre.
const PRECISION: f64 = 1e-4;

#[test]
fn every_case_is_what_the_reference_finds() {
    let path = format!(
        "{}/../../../fixtures/geometry/v1/least-width.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    let mut wrong = Vec::new();
    for c in cases {
        let id = c["id"].as_str().expect("an id");
        let rings: Vec<Vec<Vec2>> = c["rings"]
            .as_array()
            .expect("rings")
            .iter()
            .map(|r| {
                r.as_array()
                    .expect("a ring")
                    .iter()
                    .map(|p| Vec2::new(p[0].as_f64().expect("east"), p[1].as_f64().expect("north")))
                    .collect()
            })
            .collect();
        let want = c["expect"]["width"].as_f64().expect("a width");
        let neck = c["expect"]["neck"].as_bool().expect("a neck");
        let got = least_width(&rings, PRECISION).expect("a region");
        // A neck's length is exact; a largest circle's diameter within twice the precision.
        if (got.width - want).abs() > 2.0 * PRECISION + 1e-6 || got.neck != neck {
            wrong.push(format!(
                "{id}: {} {:?} ≠ reference {want} (neck {neck})",
                got.width, got.neck
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
