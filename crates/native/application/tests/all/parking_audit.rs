//! Otopark yerlerinin denetimi (docs/adr/8045 §6): the library's Otopark yeri blocks are the drawing's parking places.
//! Otopark Yönetmeliği 5/1-h-8, 9 measures each (2.40 × 4.90 m, 3.50 m for the disabled), 4/1-ç a garage's 20 m² a car,
//! 4/1-ı a public building's place for the disabled in every twenty, 5/1-j twenty required places' 5 % for electric
//! cars; Otopark hesabı counts them.

use kentos_commands::{CallOp, Entity};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn blank() -> Document {
    let snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Otopark", "origin": {"x": 0.0, "y": 0.0},
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 100.0, "workspace": "cad",
            "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
    });
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!({"outer": {"pts": [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]}})
}

/// The units (`nature` each, `size` m² each, a room of it on Zemin kat) and a common Otopark of `garage` (x0, y0, x1,
/// y1), with the library's Otopark yeri taken in alone.
fn building(units: usize, nature: &str, size: f64, garage: [f64; 4]) -> Document {
    let mut doc = blank();
    let mut list: Vec<Value> = (1..=units)
        .map(|i| json!({"id": format!("b{i}"), "number": i.to_string(), "nature": nature}))
        .collect();
    list.push(json!({"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"}));
    run(
        &mut doc,
        "cad.program.edit",
        json!({"program": {"units": list}}),
    );
    let space = |r: Value, name: &str, unit: &str| json!({"parts": [r], "method": "net", "name": name, "unit": unit, "arch": {"level": "zemin"}});
    let mut spaces: Vec<Value> = (0..units)
        .map(|i| {
            let x = 20.0 * i as f64;
            space(
                rect(x, 0.0, x + size / 10.0, 10.0),
                nature,
                &format!("b{}", i + 1),
            )
        })
        .collect();
    let [x0, y0, x1, y1] = garage;
    spaces.push(space(rect(x0, y0, x1, y1), "Otopark", "ortak"));
    run(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": "0", "spaces": spaces}),
    );
    let mut library: Value = serde_json::from_str(include_str!(
        "../../../../../catalog/v1/blocks/kentos-temel.kblok"
    ))
    .unwrap();
    library["blocks"]
        .as_array_mut()
        .unwrap()
        .retain(|b| b["name"] == "Otopark yeri");
    run(
        &mut doc,
        "cad.blocks.edit",
        json!({"operation": "import", "library": library.to_string()}),
    );
    doc
}

/// A place with its front left corner at (x, y) on Zemin kat: its uid.
fn place(doc: &mut Document, x: f64, y: f64, values: Value) -> String {
    let mut input =
        json!({"name": "Otopark yeri", "at": p(x, y), "layerId": "0", "arch": {"level": "zemin"}});
    if !values.is_null() {
        input["parameters"] = values;
    }
    let made = run(doc, "cad.blocks.insert", input);
    made["output"]["uid"]
        .as_str()
        .or_else(|| made["output"]["uids"][0].as_str())
        .unwrap_or_else(|| panic!("an insert's uid: {made}"))
        .to_owned()
}

/// Mevzuat's parking findings.
fn parking(doc: &mut Document) -> Vec<Value> {
    let answer = run(doc, "cad.arch.audit", json!({"checks": ["regulation"]}));
    answer["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"].as_str().is_some_and(|c| c.starts_with("parking")))
        .cloned()
        .collect()
}

fn said(found: &[Value]) -> Vec<String> {
    found
        .iter()
        .map(|f| f["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn apply(doc: &mut Document, f: &Value) {
    let step = &f["fix"]["step"];
    run(
        doc,
        step["command"].as_str().unwrap(),
        step["input"].clone(),
    );
}

#[test]
fn places_are_counted_measured_kept_for_the_disabled_and_fitted_to_their_garage() {
    // Two offices of 100 m² (Ek-1: 50 m² a place): four places; a public building (PAİY 4/jjjj). The garage of
    // 15 × 10 m holds seven by its area (4/1-ç); no place drawn, its area tells and nothing is short.
    let mut doc = building(2, "Büro", 100.0, [0.0, -12.0, 15.0, -2.0]);
    let report = doc.parking_report();
    assert_eq!((report.required, report.available), (4, 7));
    assert!(report.public);
    assert!(parking(&mut doc).is_empty());
    // Four places along its south side, the last 2.30 m wide: the places drawn are the drawing's now.
    let row: Vec<String> = [0.5, 2.9, 5.3]
        .into_iter()
        .map(|x| place(&mut doc, x, -11.5, Value::Null))
        .collect();
    let narrow = place(&mut doc, 7.7, -11.5, json!({"En": 2.3}));
    let report = doc.parking_report();
    assert_eq!(
        (report.required, report.available, report.disabled),
        (4, 4, 1)
    );
    let found = parking(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Otopark yerinin kısa kenarı 2.30 m < 2.40 m — Otopark Yön. 5/1-h-9",
            "Engelli otopark yeri 0 < 1 (4 yerde her 20'de 1, en az 1; umumi bina) — Otopark Yön. 4/1-ı",
        ]
    );
    assert_eq!(found[0]["code"], "parking_space_small");
    assert_eq!(
        found[0]["rule"],
        "Otopark Yön. 5/1-h-9 · RG 22.2.2018/30340"
    );
    assert_eq!(found[0]["uids"], json!([narrow]));
    assert_eq!(found[0]["fix"]["label"], "Yeri 2,40 × 4,90 m yap");
    assert_eq!(
        found[0]["fix"]["step"]["input"],
        json!({"operation": "set", "uids": [narrow], "values": {"En": 2.4}})
    );
    assert_eq!(found[1]["code"], "parking_disabled_short");
    assert!(found[1]["fix"].is_null(), "a wider place moves its row");
    apply(&mut doc, &found[0]);
    // A place for the disabled, of the table's row: 3.50 m.
    let wide = place(&mut doc, 10.1, -11.5, json!({"Ölçü": "Engelli 350 × 490"}));
    assert!(
        parking(&mut doc).is_empty(),
        "{:?}",
        said(&parking(&mut doc))
    );
    // Made 3.00 m wide, it is said by 5/1-h-9's 3.50 m.
    run(
        &mut doc,
        "cad.blocks.parameters",
        json!({"operation": "set", "uids": [wide], "values": {"En": 3.0}}),
    );
    let found = parking(&mut doc);
    assert_eq!(
        said(&found),
        ["Engelli otopark yerinin kısa kenarı 3.00 m < 3.50 m — Otopark Yön. 5/1-h-9"]
    );
    apply(&mut doc, &found[0]);
    // Three more on the north side: eight places, the garage's 150 m² holds seven (4/1-ç).
    for x in [0.5, 2.9, 5.3] {
        place(&mut doc, x, -6.95, Value::Null);
    }
    let found = parking(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Otopark: 8 otopark yeri, 150.00 m² ancak 7 yere yeter (araç başına manevra dahil 20 m²) — Otopark Yön. 4/1-ç"
        ]
    );
    assert_eq!(
        found[0]["uids"].as_array().map(Vec::len),
        Some(9),
        "the room and its places"
    );
    // The row's middle place made the disabled's: 2.40 m is said, but no fix grows it into its neighbours.
    run(
        &mut doc,
        "cad.blocks.parameters",
        json!({"operation": "set", "uids": [row[1]], "values": {"Tür": "Engelli"}}),
    );
    let found = parking(&mut doc);
    let small = found
        .iter()
        .find(|f| f["code"] == "parking_space_small")
        .expect("the disabled's place said");
    assert_eq!(
        small["message"],
        "Engelli otopark yerinin kısa kenarı 2.40 m < 3.50 m — Otopark Yön. 5/1-h-9"
    );
    assert!(small["fix"].is_null(), "{small}");
    // Otopark hesabı: the places drawn, the disabled's row (the place for the disabled and the row's middle one).
    let table = doc.parking_table();
    let rows: Vec<Vec<String>> = table.rows.clone();
    assert!(
        rows.iter()
            .any(|r| r[0] == "Çizilen otopark yeri" && r[3] == "8"),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|r| r[0] == "Engelli park yeri (çizimde 2)" && r[3] == "1"),
        "{rows:?}"
    );
}

#[test]
fn twenty_places_ask_one_for_electric_cars() {
    // Twenty flats of 150 m² (Ek-1: 120–180 m², a place each): twenty places, so 5 % for electric cars (5/1-j); twenty
    // places drawn in an open lot (no garage), none charging.
    let mut doc = building(20, "Daire", 150.0, [0.0, -40.0, 60.0, -30.0]);
    for i in 0..20 {
        place(&mut doc, 2.4 * i as f64, -60.0, Value::Null);
    }
    let report = doc.parking_report();
    assert_eq!(
        (
            report.required,
            report.available,
            report.charging,
            report.public
        ),
        (20, 20, 1, false)
    );
    let found = parking(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Elektrikli araç şarj yeri 0 < 1 (20 zorunlu yerin %5'i, en az 1) — Otopark Yön. 5/1-j (25.3.2021)"
        ]
    );
    assert_eq!(found[0]["code"], "parking_charging_short");
    assert_eq!(found[0]["fix"]["label"], "Bu yeri şarjlı yap");
    apply(&mut doc, &found[0]);
    assert!(
        parking(&mut doc).is_empty(),
        "{:?}",
        said(&parking(&mut doc))
    );
    let charged = doc
        .entities()
        .filter(|e| matches!(e, Entity::Insert(i) if i.parameters.contains_key("Tür")))
        .count();
    assert_eq!(charged, 1);
    let rows = doc.parking_table().rows;
    assert!(
        rows.iter()
            .any(|r| r[0] == "Elektrikli araç şarj yeri (çizimde 1)" && r[3] == "1"),
        "{rows:?}"
    );
    // One of them taken away: Ek-1's twenty are short.
    let first = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Insert(_)).then(|| u.to_string()))
        .unwrap();
    run(&mut doc, "cad.entities.delete", json!({"uids": [first]}));
    let found = parking(&mut doc);
    assert!(
        said(&found).contains(
            &"Otopark: gereken 20 yer, çizimde 19 otopark yeri — Otopark Yön. 8, Ek-1 (16.6.2022)"
                .to_owned()
        ),
        "{:?}",
        said(&found)
    );
}
