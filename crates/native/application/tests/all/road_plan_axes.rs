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

    // The curve's data as KGM's June 2024 sample sheets write it (docs/adr/0984), worked out from the drawing's PIs
    // without KentOS. The deflection Δ from the PIs' bearings: |atan2(300, 260) − atan2(10, 280)|.
    let pis = &setup["settings"]["road"]["alignments"][0]["pis"];
    let p = |i: usize| [pis[i]["x"].as_f64().unwrap(), pis[i]["y"].as_f64().unwrap()];
    let (a, b, c) = (p(0), p(1), p(2));
    let delta = ((c[1] - b[1]).atan2(c[0] - b[0]) - (b[1] - a[1]).atan2(b[0] - a[0])).abs();
    let grad = |rad: f64| rad * 200.0 / std::f64::consts::PI;
    // R 150 and clothoids of 40 m: D, L and T are the circular part's, Dc = Δ − 2·Ls/(2R) (KGM 2019 EK-10: A and S-A).
    let (r, ls) = (150.0_f64, 40.0_f64);
    let dc = delta - ls / r;
    assert!(has("S - 1 (Sol)"), "a left turn: the bearing grows; {words:?}");
    assert!(has(&format!("D = {:.4}", grad(dc))), "D in {words:?}");
    assert!(has("R = 150.00"));
    assert!(has(&format!("L = {:.2}", r * dc)), "L in {words:?}");
    assert!(has(&format!("T = {:.2}", r * (0.5 * dc).tan())), "T in {words:?}");
    assert!(has(&format!("S-D = {:.4}", grad(delta))), "S-D in {words:?}");
    // The sample's only schedule, rows on the right (outside): −2.5 % at 150 to +6 % at 200 and back from 300 to 350.
    // Level at 150 + 50 · 2.5/8.5: Lt = 250/17, Lr = 600/17 each way. TS from the clothoids' series (p, k), ST = TS +
    // R·Δ + Ls: the rise starts 44.7 m before TS, and the way out is back to normal 7.9 m before ST (La 0).
    let shift = ls.powi(2) / (24.0 * r) - ls.powi(4) / (2688.0 * r.powi(3));
    let k = ls / 2.0 - ls.powi(3) / (240.0 * r * r);
    let ts = (b[0] - a[0]).hypot(b[1] - a[1]) - ((r + shift) * (0.5 * delta).tan() + k);
    let st = ts + r * delta + ls;
    assert!(has("Se = % 6.00"), "Se in {words:?}");
    assert!(has(&format!("Lr = {0:.2} - {0:.2}", 600.0 / 17.0)), "Lr in {words:?}");
    assert!(has(&format!("Lt = {0:.2} - {0:.2}", 250.0 / 17.0)), "Lt in {words:?}");
    assert!(has(&format!("La = {:.2} - {:.2}", ts - 150.0, (350.0 - st).max(0.0))), "La in {words:?}");
    assert!(has("Gn = 0.00"));
    // KGM 2024's order, then the 2019 sheets' whole curve.
    let at = |w: &str| words.iter().position(|(t, _)| t.starts_with(w)).unwrap_or_else(|| panic!("{w} in {words:?}"));
    let order = ["S - 1", "D =", "R =", "L =", "T =", "Se =", "Lr =", "Lt =", "La =", "Gn =", "X =", "Y =", "S-D =",
        "S-X =", "S-Y ="].map(at);
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    // X, Y: where the tangents at SC and CS meet, on the bisector inside the PI by the two external distances'
    // difference, (R + p)·sec(Δ/2) − R less R·sec(Dc/2) − R. S-X, S-Y: the PI. North first, in groups of three.
    let value = |w: &str| -> f64 {
        let (_, v) = words[at(w)].0.split_once('=').unwrap();
        v.replace(' ', "").parse().unwrap()
    };
    let unit = |v: [f64; 2]| {
        let l = v[0].hypot(v[1]);
        [v[0] / l, v[1] / l]
    };
    let (ua, uc) = (unit([a[0] - b[0], a[1] - b[1]]), unit([c[0] - b[0], c[1] - b[1]]));
    let w = unit([ua[0] + uc[0], ua[1] + uc[1]]);
    let inside = ((r + shift) / (0.5 * delta).cos() - r) - (r / (0.5 * dc).cos() - r);
    let inner = [b[0] + w[0] * inside, b[1] + w[1] * inside];
    assert!((value("X =") - inner[1]).abs() < 1e-3 && (value("Y =") - inner[0]).abs() < 1e-3, "{inner:?}");
    assert!((value("S-X =") - b[1]).abs() < 1e-4 && (value("S-Y =") - b[0]).abs() < 1e-4);

    // The main points: TS1, SC1, CS1, ST1 and a km of two decimals (KGM 2024: `TO1=0+100.42`).
    for code in ["TS", "SC", "CS", "ST"] {
        assert!(
            words
                .iter()
                .any(|(t, _)| t.starts_with(&format!("{code}1=1+"))
                    && t.split('.').nth(1).is_some_and(|d| d.len() == 2)),
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
    // The project's ends with their km and coordinates (KGM 2019 şartname; the 2024 sheets).
    assert!(has("PROJE BAŞI KM : 1+000.00") && has("X = 50.0000") && has("Y = 20.0000"), "{words:?}");
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

#[test]
fn the_curve_data_stays_inside_the_map() {
    // A map 80 m tall at 1:1000 round the PI: the box (16 lines of 4 m) would run 65 m below it and the map would cut
    // its last lines (docs/adr/0984); it moves up into the map, 2 m from the edge, and every line stays in.
    use kentos_native_application::road_plan_axes::{Room, with_axes_on};
    let (doc, setup) = sample();
    let pi = &setup["settings"]["road"]["alignments"][0]["pis"][1];
    let center = [pi["x"].as_f64().unwrap(), pi["y"].as_f64().unwrap()];
    let room = Room { center, half: [200.0, 40.0] };
    let copy = with_axes_on(doc.reading_copy(), 1000.0, 0.0, Some(room)).expect("a copy");
    let lines: Vec<(String, [f64; 2])> = copy
        .entities()
        .filter(|e| e.base().layer_id == LAYER)
        .filter_map(|e| match e {
            Entity::Text(t) => Some((t.text.clone(), [t.p.x, t.p.y])),
            _ => None,
        })
        .filter(|(t, _)| ["D =", "Lr =", "X =", "Y =", "S-D =", "S-Y ="].iter().any(|w| t.starts_with(w)))
        // The box's, not the project ends' X and Y (260 m and 280 m from this PI, outside the map).
        .filter(|(_, p)| (p[0] - center[0]).abs() < 200.0)
        .collect();
    assert!(lines.len() >= 6, "{lines:?}");
    for (t, p) in &lines {
        assert!((p[1] - center[1]).abs() <= 40.0 - 2.0 + 1e-9, "{t} at {p:?}");
    }
    // Without the map the box hangs below the PI as before.
    let free = with_axes_on(doc.reading_copy(), 1000.0, 0.0, None).expect("a copy");
    let lowest = free
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) if t.text.starts_with("S-Y =") => Some(t.p.y),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    assert!(lowest < center[1] - 40.0, "{lowest}");
}

#[test]
fn a_map_that_does_not_show_the_pi_has_no_box() {
    // The profile's map on the same sheet shows another part of the drawing: it neither pulls S - 1's box into itself
    // (docs/adr/0984; Y78's picture had a second box under the plan) nor leaves it by the PI, where it hung into the
    // profile map's rows (docs/adr/0992; Y85's picture); the project's ends' words likewise. The plan's map, which
    // shows them, keeps them.
    use kentos_native_application::road_plan_axes::{Room, with_axes_on};
    let (doc, setup) = sample();
    let pi = &setup["settings"]["road"]["alignments"][0]["pis"][1];
    let pi = [pi["x"].as_f64().unwrap(), pi["y"].as_f64().unwrap()];
    let far = Room { center: [pi[0], pi[1] - 1000.0], half: [300.0, 100.0] };
    let words = |room: Room| -> Vec<String> {
        let copy = with_axes_on(doc.reading_copy(), 1000.0, 0.0, Some(room)).expect("a copy");
        copy.entities()
            .filter_map(|e| match e {
                Entity::Text(t) if ["D =", "S-Y =", "PROJE"].iter().any(|w| t.text.starts_with(w)) => Some(t.text.clone()),
                _ => None,
            })
            .collect()
    };
    assert!(words(far).is_empty(), "the far map writes {:?}", words(far));
    let all = Room { center: pi, half: [5000.0, 5000.0] };
    let near = words(all);
    assert!(near.iter().any(|w| w.starts_with("S-Y =")) && near.iter().any(|w| w.starts_with("PROJE BAŞI")), "{near:?}");
}

#[test]
fn the_box_writes_the_curves_design_speed() {
    // Docs/adr/0985: V is the PI's own speed, else the alignment's; none, no V line (KGM 2024: `V = 80`).
    let (_, mut setup) = sample();
    let v_line = |setup: &serde_json::Value| -> Vec<String> {
        let doc = Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(setup.clone()).unwrap())
            .unwrap();
        texts(&with_axes(&doc, 1000.0, 0.0).expect("a copy")).into_iter().map(|(t, _)| t).filter(|t| t.starts_with("V =")).collect()
    };
    assert!(v_line(&setup).is_empty(), "no speed, no V");
    setup["settings"]["road"]["alignments"][0]["designSpeed"] = serde_json::json!(80.0);
    assert_eq!(v_line(&setup), vec!["V = 80".to_owned()]);
    setup["settings"]["road"]["alignments"][0]["pis"][1]["speed"] = serde_json::json!(62.5);
    assert_eq!(v_line(&setup), vec!["V = 62.5".to_owned()], "the curve's own speed stands for it");
}
