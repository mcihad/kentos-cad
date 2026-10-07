//! Topoloji kuralları (docs/adr/0202) against the independent reference in
//! `fixtures/topology-rules/v1/cases.json` (`scripts/fixtures/topology_rules_cases.py`,
//! no KentOS code): each rule's findings (problem, objects, place, measure,
//! exception, fixes) and what the fixes write, by name, as the web calls
//! them through WASM (`ops::topology_rules::calls`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/topology-rules/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.topology-rule-cases");
    file
}

fn call(name: &str, args: Value) -> Result<Value, String> {
    run_named(name, &args.to_string()).map(|s| serde_json::from_str(&s).expect("JSON"))
}

fn close(got: f64, want: f64, rel: f64) -> bool {
    (got - want).abs() <= rel * if want.abs() > 1.0 { want.abs() } else { 1.0 }
}

/// The case's objects as the web passes them: entities, their layers, their uids.
fn inputs(c: &Value) -> (Value, Value, Value) {
    let objects = c["objects"].as_array().expect("objects");
    let entities: Vec<Value> = objects.iter().map(|o| o["shape"].clone()).collect();
    let layers: Vec<Value> = objects.iter().map(|o| o["layer"].clone()).collect();
    let uids: Vec<Value> = objects.iter().map(|o| o["uid"].clone()).collect();
    (json!(entities), json!(layers), json!(uids))
}

fn measured(shape: &Value) -> Value {
    call("geoMeasure", json!([[shape]])).expect("geoMeasure")[0].clone()
}

fn same_change(name: &str, got: &Value, want: &Value) {
    assert_eq!(got["object"], want["object"], "{name}: nesne");
    if want["remove"] == json!(true) {
        assert!(got["shape"].is_null(), "{name}: silinmeli, {got}");
        return;
    }
    let shape = &got["shape"];
    assert!(!shape.is_null(), "{name}: şekil yok");
    if let Some(pts) = want["pts"].as_array() {
        let (gp, holes) = match shape["kind"].as_str() {
            Some("line") => (json!([shape["a"], shape["b"]]), json!([])),
            _ => (shape["pts"].clone(), shape["holes"].clone()),
        };
        let gp = gp.as_array().expect("pts").clone();
        assert_eq!(gp.len(), pts.len(), "{name}: köşe sayısı {shape}");
        for (g, w) in gp.iter().zip(pts) {
            for k in ["x", "y"] {
                assert!(
                    close(g[k].as_f64().unwrap(), w[k].as_f64().unwrap(), 1e-12),
                    "{name}: köşe {g} ≠ {w}"
                );
            }
        }
        if let Some(wh) = want["holes"].as_array() {
            let gh = holes.as_array().expect("holes");
            assert_eq!(gh.len(), wh.len(), "{name}: delik sayısı");
            for (g, w) in gh.iter().zip(wh) {
                let gpts = g["pts"].as_array().expect("hole pts");
                let w = w.as_array().expect("hole");
                assert_eq!(gpts.len(), w.len(), "{name}: deliğin köşe sayısı");
                for (a, b) in gpts.iter().zip(w) {
                    for k in ["x", "y"] {
                        assert!(
                            close(a[k].as_f64().unwrap(), b[k].as_f64().unwrap(), 1e-12),
                            "{name}: deliğin köşesi {a} ≠ {b}"
                        );
                    }
                }
            }
        }
        return;
    }
    if let Some(p) = want.get("p") {
        for k in ["x", "y"] {
            assert!(
                close(
                    shape["p"][k].as_f64().unwrap(),
                    p[k].as_f64().unwrap(),
                    1e-12
                ),
                "{name}: nokta {shape}"
            );
        }
        return;
    }
    let m = measured(shape);
    assert_eq!(shape["kind"], want["kind"], "{name}: tür");
    assert_eq!(m["parts"], want["parts"], "{name}: parça {m}");
    if !want["holes"].is_null() {
        assert_eq!(m["holes"], want["holes"], "{name}: delik {m}");
    }
    for key in ["area", "length"] {
        if let Some(w) = want[key].as_f64() {
            let g = m[key].as_f64().unwrap_or(f64::NAN);
            assert!(close(g, w, 1e-9), "{name}: {key} {g}, beklenen {w}");
        }
    }
}

#[test]
fn the_rules_find_what_the_reference_finds_and_the_fixes_write_it() {
    let file = cases();
    let all = file["cases"].as_array().expect("cases");
    assert!(all.len() >= 17);
    for c in all {
        let id = c["id"].as_str().expect("id");
        let (entities, layers, uids) = inputs(c);
        let exceptions = c.get("exceptions").cloned().unwrap_or(json!([]));
        let rules = c["rules"].clone();
        let got = call(
            "topologyCheck",
            json!([entities, layers, uids, rules, c["tolerance"], exceptions]),
        )
        .unwrap_or_else(|e| panic!("{id}: {e}"));
        let findings = got["findings"].as_array().expect("findings");
        let want = c["findings"].as_array().expect("findings");
        let brief = |f: &Value| format!("{} {} {}", f["problem"], f["objects"], f["at"]);
        assert_eq!(
            findings.len(),
            want.len(),
            "{id}: bulgu sayısı\n  bulunan: {:?}\n  beklenen: {:?}",
            findings.iter().map(brief).collect::<Vec<_>>(),
            want.iter().map(brief).collect::<Vec<_>>()
        );
        let rule_ids: Vec<&str> = c["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        for (k, (g, w)) in findings.iter().zip(want).enumerate() {
            let name = format!("{id} #{k}");
            assert_eq!(
                rule_ids[g["rule"].as_u64().unwrap() as usize],
                w["rule"],
                "{name}: kural"
            );
            assert_eq!(g["problem"], w["problem"], "{name}: sorun");
            assert_eq!(g["objects"], w["objects"], "{name}: nesneler");
            for axis in ["x", "y"] {
                let (ga, wa) = (
                    g["at"][axis].as_f64().unwrap(),
                    w["at"][axis].as_f64().unwrap(),
                );
                assert!(
                    (ga - wa).abs() <= 1e-6,
                    "{name}: yer {} ≠ {}",
                    g["at"],
                    w["at"]
                );
            }
            match w["measure"].as_f64() {
                Some(m) => {
                    let gm = g["measure"].as_f64().unwrap_or(f64::NAN);
                    assert!(
                        close(gm, m, 1e-6) || (gm - m).abs() <= 1e-9,
                        "{name}: ölçü {gm}, beklenen {m}"
                    );
                }
                None => assert!(
                    g["measure"].is_null(),
                    "{name}: ölçü olmamalı: {}",
                    g["measure"]
                ),
            }
            if !w["measureKind"].is_null() {
                assert_eq!(g["measureKind"], w["measureKind"], "{name}: ölçünün türü");
            }
            assert_eq!(g["exception"], w["exception"], "{name}: istisna");
            let keys: Vec<Value> = g["fixes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f["key"].clone())
                .collect();
            assert_eq!(json!(keys), w["fixes"], "{name}: düzeltmeler");
        }
        for x in c
            .get("fixed")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let k = x["finding"].as_u64().unwrap() as usize;
            let key = x["fix"].as_str().unwrap();
            let name = format!("{id} #{k} {key}");
            let changes = call(
                "topologyFix",
                json!([
                    c["objects"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|o| o["shape"].clone())
                        .collect::<Vec<_>>(),
                    layers,
                    c["rules"],
                    findings[k],
                    key
                ]),
            )
            .unwrap_or_else(|e| panic!("{name}: {e}"));
            let changes = changes.as_array().expect("changes");
            let want = x["changes"].as_array().expect("changes");
            assert_eq!(
                changes.len(),
                want.len(),
                "{name}: değişiklik sayısı {changes:?}"
            );
            for (g, w) in changes.iter().zip(want) {
                same_change(&name, g, w);
            }
        }
    }
}

#[test]
fn the_catalog_names_every_kind_problem_and_fix() {
    let c = call("topologyCatalog", json!([])).expect("catalog");
    let kinds: Vec<&str> = c["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["key"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.len(), 13);
    assert_eq!(kinds[0], "mustNotOverlap");
    assert_eq!(kinds[12], "mustBeOnEndOf");
    let slivers = &c["kinds"][2];
    assert_eq!(slivers["value"], "length");
    assert_eq!(slivers["defaultValue"], 0.1);
    assert_eq!(c["kinds"][9]["between"], true);
    assert_eq!(c["kinds"][9]["label"], "… ile çakışmamalı");
    assert_eq!(c["problems"].as_array().unwrap().len(), 18);
    assert_eq!(c["fixes"][0]["label"], "Birinci nesneden çıkar");
}

#[test]
fn a_fix_the_finding_does_not_offer_is_refused() {
    let file = cases();
    let c = &file["cases"][0];
    let (entities, layers, uids) = inputs(c);
    let got = call(
        "topologyCheck",
        json!([entities, layers, uids, c["rules"], c["tolerance"], []]),
    )
    .unwrap();
    let f = &got["findings"][0];
    let e = call(
        "topologyFix",
        json!([entities, layers, c["rules"], f, "repair"]),
    )
    .unwrap_err();
    assert!(e.contains("sunmuyor"), "{e}");
}

/// How long the rules take over a 100 × 100 block of parcels (10 000 areas, a
/// few overlapping, one missing, one with an extra vertex), release build:
/// `cargo test --release -p kentos-geometry-core --test all topology_rules::timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn timing() {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::ops::topology_rules::{Kind, Objects, Rule, check};
    use kentos_geometry_core::vec2::Vec2;
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut shapes = Vec::new();
    for i in 0..100 {
        for j in 0..100 {
            if (i, j) == (50, 50) {
                continue;
            }
            let (x, y) = (e0 + f64::from(i) * 20.0, n0 + f64::from(j) * 20.0);
            let grow = if (i + j) % 997 == 0 { 0.3 } else { 0.0 };
            let mut pts = vec![
                Vec2::new(x, y),
                Vec2::new(x + 20.0 + grow, y),
                Vec2::new(x + 20.0 + grow, y + 20.0),
                Vec2::new(x, y + 20.0),
            ];
            if (i, j) == (10, 10) {
                pts.insert(2, Vec2::new(x + 20.0, y + 7.0));
            }
            shapes.push(Shape::Polygon {
                pts,
                bulges: None,
                holes: None,
                parts: None,
            });
        }
    }
    let layers = vec!["parsel".to_owned(); shapes.len()];
    let uids: Vec<String> = (0..shapes.len()).map(|k| format!("u{k}")).collect();
    let objects = Objects {
        shapes: &shapes,
        layers: &layers,
        uids: &uids,
    };
    for kind in [
        Kind::MustNotOverlap,
        Kind::MustNotHaveGaps,
        Kind::MustNotHaveMissingVertices,
        Kind::MustNotHaveSlivers,
        Kind::MustNotHaveShortEdges,
        Kind::MustNotHaveSmallAngles,
        Kind::MustBeValid,
    ] {
        let rules = [Rule {
            id: "r".into(),
            kind,
            layer: "parsel".into(),
            other: None,
            value: None,
        }];
        let t0 = std::time::Instant::now();
        let found = check(&objects, &rules, 0.001, &[]);
        println!(
            "{:<28} {:>6} bulgu  {:.3} s",
            kind.key(),
            found.findings.len(),
            t0.elapsed().as_secs_f64()
        );
    }
}
