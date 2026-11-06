//! A road project's alignments in a plan map's copy of the drawing (docs/adr/0982), checked against the sample
//! drawing's PIs by hand: the deflection from the PIs' bearings, the hectometres from the km range, the drawing itself
//! untouched.
use kentos_commands::{DocumentSnapshotV2, Entity};
use kentos_domain::Document;
use kentos_native_application::road_plan_axes::{LAYER, with_axes};

fn sample() -> (Document, serde_json::Value) {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/road.sheets.plan.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "KGM bantları ve dever çizelgesiyle")
        .unwrap();
    let setup = case["setup"].clone();
    let doc = Document::from_snapshot_v2(
        serde_json::from_value::<DocumentSnapshotV2>(setup.clone()).unwrap(),
    )
    .unwrap();
    (doc, setup)
}

fn texts(doc: &Document) -> Vec<(String, f64)> {
    doc.entities()
        .filter(|e| e.base().layer_id == LAYER)
        .filter_map(|e| match e {
            Entity::Text(t) => Some((t.text.clone(), t.rotation)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_map_draws_the_alignments_and_the_drawing_stays() {
    let (doc, setup) = sample();
    let revision = doc.revision();
    let count = doc.entities().count();
    let copy = with_axes(&doc, 1000.0, 29.847).expect("a road project with alignments");
    assert_eq!(
        (doc.revision(), doc.entities().count()),
        (revision, count),
        "the drawing is not touched"
    );
    assert!(doc.layers().get(LAYER).is_none());
    let words = texts(&copy);
    let has = |w: &str| words.iter().any(|(t, _)| t == w);

    // S1's deflection from its PIs' bearings: |atan2(300, 260) − atan2(10, 280)| in grad, five decimals.
    let pis = &setup["settings"]["road"]["alignments"][0]["pis"];
    let p = |i: usize| [pis[i]["x"].as_f64().unwrap(), pis[i]["y"].as_f64().unwrap()];
    let (a, b, c) = (p(0), p(1), p(2));
    let delta = (c[1] - b[1]).atan2(c[0] - b[0]) - (b[1] - a[1]).atan2(b[0] - a[0]);
    let grad = delta.abs() * 200.0 / std::f64::consts::PI;
    assert!(has(&format!("A={grad:.5}")), "A={grad:.5} in {words:?}");
    assert!(has("S1 (SOL)"), "a left turn: the bearing grows");
    assert!(has("R=150.00") && has("Lr=40.00-40.00"));
    assert!(
        has(&format!("Y={:.3}", b[0])) && has(&format!("X={:.3}", b[1])),
        "KGM: Y east, X north"
    );

    // The main points: TS, SC, CS, ST with the curve's name and a km of three decimals.
    for code in ["TS", "SC", "CS", "ST"] {
        assert!(
            words
                .iter()
                .any(|(t, _)| t.starts_with(&format!("{code} S1 1+"))
                    && t.split('.').nth(1).is_some_and(|d| d.len() == 3)),
            "{code} in {words:?}"
        );
    }
    // Hectometres: Ana aks runs km 1+000 → 1+669, Yan aks 0+000 → 0+657 (hypot(550, 360)): a full km each, the
    // hectometres' digits 1 to 6 each.
    assert!(has("1+000") && has("0+000"));
    for d in 1..=6 {
        assert_eq!(
            words.iter().filter(|(t, _)| *t == d.to_string()).count(),
            2,
            "digit {d}"
        );
    }
    assert!(has("PROJE BAŞI KM: 1+000.000"));
    // Every text reads left to right on the paper turned 29.847°.
    for (t, r) in &words {
        let on = (r - 29.847).rem_euclid(360.0);
        let on = if on > 180.0 { on - 360.0 } else { on };
        assert!(on > -90.0 - 1e-9 && on <= 90.0 + 1e-9, "{t} at {r}");
    }
}

#[test]
fn a_drawing_without_alignments_has_no_copy() {
    let (_, mut setup) = sample();
    setup["settings"]["road"]["alignments"] = serde_json::json!([]);
    let plain =
        Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(setup).unwrap())
            .unwrap();
    assert!(!kentos_native_application::road_plan_axes::has_axes(&plain));
    assert!(with_axes(&plain, 1000.0, 0.0).is_none());
}
