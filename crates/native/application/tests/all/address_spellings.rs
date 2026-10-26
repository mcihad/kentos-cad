//! `cargo test --release -p kentos-native-application --test all address_spellings -- --ignored --nocapture`
//! (docs/adr/0647 Ölçümler): `cad.addresses.normalize`'s plan and write, street, neighbourhood and district, on 100 000
//! doors along 1 000 streets that write their names in three ways.

use kentos_commands::{AddressName, AddressesNormalize, CommandResult};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, addresses_normalize};
use serde_json::{Value, json};

fn city(n: usize) -> Document {
    let doors: Vec<Value> = (0..n)
        .map(|i| {
            let street = i / 100;
            let road = match i % 7 {
                0 => format!("ATATÜRK {street}. SOKAK"),
                1 => format!("Atatürk {street}. Sk."),
                _ => format!("Atatürk {street}. Sokak"),
            };
            let hood = if i % 11 == 0 { "KIZILAY" } else { "Kızılay" };
            let district = if i % 13 == 0 { "ÇANKAYA" } else { "Çankaya" };
            json!({
                "kind": "point", "id": i + 1, "layerId": "kapi",
                "attrs": { "YOL_ADI": road, "KAPI_NO": (i % 100 + 1).to_string(), "MAHALLE": hood, "ILCE": district },
                "p": { "x": 500_000.0 + (i % 1000) as f64, "y": 4_400_000.0 + (i / 1000) as f64 } })
        })
        .collect();
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Şehir",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis",
                      "addressLocators": [{ "id": "adres-1", "name": "Numarataj",
                          "doors": { "layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO",
                                     "neighbourhood": "MAHALLE", "district": "ILCE" },
                          "minScore": 85, "reach": 100.0 }] },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "kapi", "name": "Kapılar", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "kapi",
        "entities": doors,
        "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

#[test]
#[ignore = "a timing, run on its own in release"]
fn address_spellings_timing() {
    let n: usize = std::env::var("KENTOS_PERF_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let mut d = city(n);
    for what in [
        AddressName::Street,
        AddressName::Neighbourhood,
        AddressName::District,
    ] {
        let input = AddressesNormalize {
            what,
            ..AddressesNormalize::default()
        };
        let t = std::time::Instant::now();
        let CommandResult::Completed { output, .. } =
            addresses_normalize::plan(&ExecutionContext::new(&mut d), &input)
        else {
            panic!("not planned");
        };
        let planned = t.elapsed();
        let t = std::time::Instant::now();
        let CommandResult::Completed { output: done, .. } =
            addresses_normalize::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("not written");
        };
        let written = t.elapsed();
        assert_eq!(output.written, done.written);
        println!(
            "{n} kapı, {what:?}: {} grup, {} kapı; plan {planned:?}, yazma {written:?}",
            done.groups.len(),
            done.written
        );
        assert!(planned.as_millis() < 5000 && written.as_millis() < 5000);
    }
}
