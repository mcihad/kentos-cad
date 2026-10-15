//! Standardı denetle and Standardı uygula over a large drawing (docs/adr/0312 Doğrulama): 500 layers holding
//! 100 000 objects, a third named the old way (`duvar-1`, `kapi-2`, `aks-3`), the architectural office's standard
//! checked, then applied (plan and write). The shared cases are `fixtures/commands/v1/cad.standards.{check,apply}.json`.

use std::time::Instant;

use kentos_domain::Document;
use kentos_domain::contracts::{CommandResult, DocumentSnapshotV1, StandardsApply, StandardsCheck};
use kentos_native_application::{ExecutionContext, standards};
use serde_json::json;

fn drawing() -> Document {
    let old = ["duvar", "kapi", "aks", "mobilya", "A-PLAN"];
    let layers: Vec<_> = (0..500u32)
        .map(|k| {
            json!({"id": format!("l{k}"), "name": format!("{}-{k}", old[(k % 5) as usize]),
                   "type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
        })
        .collect();
    let entities: Vec<_> = (0..100_000u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
            let mut e = json!({"kind":"line","id": i + 1,"layerId": format!("l{}", i % 500),"attrs":{},
                   "a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}});
            if i % 7 == 0 {
                e["color"] = json!("#FF0000");
            }
            e
        })
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":layers,"activeLayer":"l4","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

/// `cargo test --release -p kentos-native-application --test all standards_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    let mut doc = drawing();
    let t = Instant::now();
    let CommandResult::Completed { output, .. } = standards::execute_check(
        &mut ExecutionContext::new(&mut doc),
        StandardsCheck {
            name: Some("mimari-ofis".into()),
            standard: None,
        },
    ) else {
        panic!("not checked")
    };
    println!(
        "Standardı denetle: {} bulgu, {:.1} ms",
        output.issues.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let input = StandardsApply {
        name: Some("mimari-ofis".into()),
        ..StandardsApply::default()
    };
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        standards::plan_apply(&ExecutionContext::new(&mut doc), &input)
    else {
        panic!("not planned")
    };
    println!(
        "Standardı uygula, plan: {} katman birleşir, {} nesne, {} katman eklenir, {:.1} ms",
        output.translations.len(),
        output.objects,
        output.created.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        standards::execute_apply(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("not applied")
    };
    println!(
        "Standardı uygula, yazma: {} katman birleşti, {:.1} ms",
        output.translations.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    assert!(output.changed);
}
