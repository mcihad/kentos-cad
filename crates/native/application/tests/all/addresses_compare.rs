//! `cad.addresses.compare` (docs/adr/0736) beyond the shared cases: a register longer than is read says so and is
//! compared up to its cut, and the release timing of 100 000 doors against a register of 250 000 rows:
//!
//! `cargo test --release -p kentos-native-application --test all addresses_compare::compare_timing -- --ignored --nocapture`

use kentos_commands::{AddressRegister, AddressesCompare, CommandResult};
use kentos_domain::Document;
use kentos_native_application::addresses_compare;
use serde_json::{Value, json};

/// `n` doors along streets of 100 doors each, every door with its code.
fn city(n: usize) -> Document {
    let doors: Vec<Value> = (0..n)
        .map(|i| {
            json!({
                "kind": "point", "id": i + 1, "layerId": "kapi",
                "attrs": { "YOL_ADI": format!("Atatürk {}. Sokak", i / 100), "KAPI_NO": (i % 100 + 1).to_string(),
                           "MAHALLE": "Kızılay", "ILCE": "Çankaya", "UAVT": format!("{:010}", 1_000_000_000 + i) },
                "p": { "x": 500_000.0 + (i % 1000) as f64, "y": 4_400_000.0 + (i / 1000) as f64 } })
        })
        .collect();
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Şehir",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis",
                      "addressLocators": [{ "id": "adres-1", "name": "Numarataj",
                          "doors": { "layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO",
                                     "neighbourhood": "MAHALLE", "district": "ILCE", "code": "UAVT" },
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

/// A register of `rows` rows: the city's codes first (every tenth street written another way, every hundredth door a
/// metre off), then codes no door has.
fn register(rows: usize, doors: usize, path: &std::path::Path) {
    let mut text = String::from("ADRES_NO;YOL;DIS_KAPI;MAHALLE;ILCE;X;Y\n");
    for i in 0..rows {
        let street = i / 100;
        let road = if street % 10 == 0 {
            format!("ATATÜRK {street}. SK.")
        } else {
            format!("Atatürk {street}. Sokak")
        };
        let off = if i % 100 == 0 { 1.5 } else { 0.0 };
        let x = 500_000.0 + (i % 1000) as f64 + off;
        let y = 4_400_000.0 + (i / 1000) as f64;
        let code = if i < doors { 1_000_000_000 + i } else { 2_000_000_000 + i };
        text.push_str(&format!(
            "{code:010};{road};{};Kızılay;Çankaya;{x};{y}\n",
            i % 100 + 1
        ));
    }
    std::fs::write(path, text).expect("the register");
}

fn input(path: &std::path::Path, tolerance: f64) -> AddressesCompare {
    AddressesCompare {
        register: AddressRegister {
            path: Some(path.display().to_string()),
            code: "ADRES_NO".into(),
            street: Some("YOL".into()),
            door: Some("DIS_KAPI".into()),
            neighbourhood: Some("MAHALLE".into()),
            district: Some("ILCE".into()),
            x: Some("X".into()),
            y: Some("Y".into()),
            ..AddressRegister::default()
        },
        tolerance: Some(tolerance),
        ..AddressesCompare::default()
    }
}

#[test]
fn a_register_longer_than_is_read_says_so() {
    let d = city(10);
    let dir = std::env::temp_dir().join(format!("kentos-uavt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the folder");
    let path = dir.join("uzun.csv");
    register(kentos_formats::table_file::PROCESSING_ROWS + 5, 10, &path);
    let CommandResult::Completed { output, warnings } =
        addresses_compare::plan_in(&d, &input(&path, 10.0))
    else {
        panic!("not compared");
    };
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "register_cut");
    assert_eq!(output.rows as usize, kentos_formats::table_file::PROCESSING_ROWS);
    assert_eq!(output.matched, 10);
}

#[test]
#[ignore = "a timing, run on its own in release"]
fn compare_timing() {
    let n: usize = std::env::var("KENTOS_PERF_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let rows = kentos_formats::table_file::PROCESSING_ROWS;
    let d = city(n);
    let dir = std::env::temp_dir().join(format!("kentos-uavt-perf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the folder");
    let path = dir.join("uavt.csv");
    register(rows, n, &path);
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut times = Vec::new();
    let mut last = None;
    for _ in 0..3 {
        let t = std::time::Instant::now();
        let CommandResult::Completed { output, .. } =
            addresses_compare::plan_in(&d, &input(&path, 1.0))
        else {
            panic!("not compared");
        };
        times.push(t.elapsed());
        last = Some(output);
    }
    let _ = std::fs::remove_dir_all(&dir);
    times.sort();
    let out = last.expect("compared");
    println!(
        "{n} kapı, {rows} satırlık kayıt ({size} bayt): orta {:?} (en kısa {:?}); {} kayıtta, {} farklı, {} uzak, {} çizimde yok",
        times[1], times[0], out.matched, out.differing, out.distant, out.undrawn
    );
    assert_eq!(out.matched as usize, n);
    assert!(times[1].as_millis() < 10_000);
}
