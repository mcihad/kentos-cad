//! Katmanları bul and Katmanları çevir over a large drawing (docs/adr/0311 §7): 500 layers holding 100 000 objects,
//! the layers found by a pattern and a property, then 150 of them translated by five rules (plan and
//! write). The shared cases are `fixtures/commands/v1/cad.layers.{find,translate}.json`.

use std::time::Instant;

use kentos_domain::Document;
use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, LayersFind, LayersTranslate, TranslateRule,
};
use kentos_native_application::{ExecutionContext, layers_find, layers_translate};
use serde_json::json;

fn drawing() -> Document {
    let layers: Vec<_> = (0..500u32)
        .map(|k| {
            json!({"id": format!("l{k}"), "name": format!("{}-{k}", if k % 2 == 0 { "eski" } else { "A" }),
                   "type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
        })
        .collect();
    let entities: Vec<_> = (0..100_000u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
            json!({"kind":"line","id": i + 1,"layerId": format!("l{}", i % 500),"attrs":{},
                   "a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}})
        })
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Katmanlar","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":layers,"activeLayer":"l1","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

/// `cargo test --release -p kentos-native-application --test all layers_find_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    let mut doc = drawing();
    let find = LayersFind {
        name: Some("eski-1*".into()),
        empty: Some(false),
        ..Default::default()
    };
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        layers_find::execute(&mut ExecutionContext::new(&mut doc), find)
    else {
        panic!("not found")
    };
    println!(
        "Katmanları bul (örüntü ve nesnesiz değil): {} katman, {:.1} ms",
        output.layers.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let input = LayersTranslate {
        rules: (0..5)
            .map(|k| TranslateRule {
                from: format!("eski-*{k}"),
                to: format!("STANDART-{k}"),
                color: None,
                line_type: None,
                line_weight: Some(0.35),
            })
            .collect(),
        parent: None,
        expected_revision: None,
    };
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        layers_translate::plan(&ExecutionContext::new(&mut doc), &input)
    else {
        panic!("not planned")
    };
    println!(
        "Katmanları çevir, plan: {} katman, {} nesne, {:.1} ms",
        output.translations.len(),
        output.objects,
        t.elapsed().as_secs_f64() * 1e3
    );
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        layers_translate::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("not written")
    };
    println!(
        "Katmanları çevir, yazma: {} katman, {} nesne, {:.1} ms",
        output.translations.len(),
        output.objects,
        t.elapsed().as_secs_f64() * 1e3
    );
    assert_eq!(output.translations.len(), 150);
    assert_eq!(output.objects, 30_000);
}
