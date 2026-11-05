//! The clothoid integral against the Fresnel integrals (fixtures/road/v1/clothoid-integrals.json, docs/adr/0969,
//! ROAD-ALIGN-04): written at 60 digits by scripts/fixtures/road_clothoid_integral_cases.py, no KentOS code. The
//! truncation of 10-point Gauss–Legendre on pieces turning at most 0.2 rad is bounded by 10⁻²⁰·s (the ADR's
//! derivation); what is measured here is the rounding, held under 10⁻¹²·max(1, s) m.

use kentos_road::alignment::element::integral;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/clothoid-integrals.json");

#[test]
fn the_clothoid_integral_is_the_fresnel_one() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let mut worst = (0.0f64, String::new());
    for c in all["cases"].as_array().expect("cases") {
        let f = |k: &str| c[k].as_f64().expect("a number");
        for at in c["at"].as_array().expect("rows") {
            let s = at["s"].as_f64().unwrap();
            let [x, y] = integral(f("theta0"), f("k0"), f("c"), s);
            let err = (x - at["x"].as_f64().unwrap()).hypot(y - at["y"].as_f64().unwrap());
            let name = c["name"].as_str().unwrap();
            assert!(err <= 1e-12 * s.max(1.0), "{name} s {s}: {err:e} m");
            if err > worst.0 {
                worst = (err, format!("{name} s {s}"));
            }
        }
    }
    println!("en büyük fark {:e} m ({})", worst.0, worst.1);
}
