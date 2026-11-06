//! Each curve read at its own design speed (docs/adr/0985, 0986): on the shared cases' winding alignment g-5 (three
//! curves), the alignment at 80 km/h and p2 at its own 50; checked against the same command run at one given speed.
use kentos_commands::{CommandResult, DocumentSnapshotV2, SuperelevationsCheck, SuperelevationsChecked};
use kentos_domain::Document;
use kentos_native_application::ExecutionContext;
use kentos_native_application::road_superelevations_check::plan;

fn winding(speeds: bool) -> Document {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/road.superelevations.transitions.json"
    ))
    .unwrap();
    let mut setup = cases["setup"].clone();
    if speeds {
        let road = &mut setup["settings"]["road"]["alignments"];
        let g5 = road.as_array_mut().unwrap().iter_mut().find(|a| a["id"] == "g-5").unwrap();
        g5["designSpeed"] = serde_json::json!(80.0);
        let p2 = g5["pis"].as_array_mut().unwrap().iter_mut().find(|p| p["id"] == "p2").unwrap();
        p2["speed"] = serde_json::json!(50.0);
    }
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(setup).unwrap()).unwrap()
}

fn checked(doc: &mut Document, speed: Option<f64>) -> SuperelevationsChecked {
    let q = SuperelevationsCheck {
        superelevation: "d-5".into(),
        template: "tk-1".into(),
        speed,
        most_friction: Some(1e-6),
        most_relative: None,
        step: None,
    };
    match plan(&ExecutionContext::new(doc), &q) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

#[test]
fn each_curve_is_read_at_its_own_speed() {
    let mut own = winding(true);
    let got = checked(&mut own, None);
    let pis: Vec<&str> = got.curves.iter().map(|c| c.pi.as_str()).collect();
    assert_eq!(pis, ["p1", "p2", "p3"], "in station order");
    // The same curves at one speed given, on the drawing without speeds.
    let mut plain = winding(false);
    let at80 = checked(&mut plain, Some(80.0));
    let at50 = checked(&mut plain, Some(50.0));
    let row = |c: &SuperelevationsChecked, pi: &str| c.curves.iter().find(|r| r.pi == pi).unwrap().clone();
    assert_eq!(row(&got, "p1"), row(&at80, "p1"));
    assert_eq!(row(&got, "p2"), row(&at50, "p2"), "p2 at its own 50 km/h");
    assert_eq!(row(&got, "p3"), row(&at80, "p3"));
    for (pi, v) in [("p1", "80"), ("p2", "50"), ("p3", "80")] {
        assert!(
            got.findings.iter().any(|f| f.message.starts_with(&format!("{pi} kurbu {v} km/sa'te"))),
            "{pi} at {v}: {:?}",
            got.findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
    // A speed given stands for every curve.
    let mut own = winding(true);
    assert_eq!(checked(&mut own, Some(80.0)).curves, at80.curves);
}
