//! Two blocks and a shared basement counted once (docs/adr/8072 §4) on `arch_ifc::two_blocks` with a car park in the
//! basement (counted in A Blok) and the parcel's zoning: Emsal ve TAKS's footprint is both blocks' ground storeys'
//! (PAİY 4/1-bbbb: the structures' projection on the parcel), each building's own construction area and footprint add up
//! to the project's, the basement in A's alone; the IFC writes each building's own areas; Metraj by building partitions
//! the project's rooms; Ruhsat takımı for B draws B's storey alone, coded from its own ±0.00, its code on every sheet.

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_ifc::building::Value;
use kentos_native_application::{ExecutionContext, permit_set, registry};
use kentos_sheet::model::VarValue;
use serde_json::{Value as Json, json};

fn run(doc: &mut Document, name: &str, input: Json) -> Json {
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

/// The two blocks with a car park of 11 × 8 m in the shared basement and a parcel of 2 000 m² (emsal 1.5, TAKS 0.40).
fn blocks() -> Document {
    let mut doc = crate::arch_ifc::two_blocks();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": "0", "spaces": [{
            "parts": [{"outer": {"pts": [p(0.5, 0.5), p(11.5, 0.5), p(11.5, 8.5), p(0.5, 8.5)]}}], "number": "B-01",
            "name": "Otopark", "arch": {"level": "bodrum"}
        }]}),
    );
    let mut settings = doc.settings().clone();
    settings.arch.as_mut().expect("its settings").zoning = Some(kentos_commands::Zoning {
        parcel_area: 2000.0,
        far: Some(1.5),
        coverage: Some(0.4),
        ..Default::default()
    });
    doc.set_settings(settings);
    doc
}

fn area(props: &[(String, Value)], name: &str) -> Option<f64> {
    props.iter().find_map(|(k, v)| match (k.as_str(), v) {
        (n, Value::Area(a)) if n == name => Some(*a),
        _ => None,
    })
}

#[test]
fn the_ratios_count_both_blocks_and_the_basement_once() {
    let doc = blocks();
    let r = doc.zoning_report().expect("the parcel's zoning");
    let arch = doc.settings().arch.as_ref().expect("its settings");
    // Both ground storeys stand on the parcel: A's ground floor and B's own at +1.20.
    let grounds: Vec<&str> = r.grounds.iter().map(|(g, _)| g.as_str()).collect();
    assert_eq!(grounds, ["zemin", "b-zemin"]);
    let (a_ground, b_ground) = (r.grounds[0].1, r.grounds[1].1);
    assert!(a_ground > 90.0 && b_ground > 25.0, "{:?}", r.grounds);
    // Apart on the parcel, the footprint is the two together (their contours' union, overlap none).
    assert!(
        (r.footprint - (a_ground + b_ground)).abs() < 1e-6,
        "{} {:?}",
        r.footprint,
        r.grounds
    );
    // Each building's share: the levels it counts (the basement A's), its ground storey; together the project's.
    let (a_all, a_foot) = r.of_building(&arch.program, &arch.levels, "a");
    let (b_all, b_foot) = r.of_building(&arch.program, &arch.levels, "b");
    assert!(
        (a_all + b_all - r.construction()).abs() < 1e-9,
        "{a_all} + {b_all} ≠ {}",
        r.construction()
    );
    assert_eq!((a_foot, b_foot), (a_ground, b_ground));
    let basement: f64 = r
        .levels
        .iter()
        .filter(|l| l.level.as_deref() == Some("bodrum"))
        .map(|l| l.all())
        .sum();
    assert!(basement > 80.0, "the car park counts: {basement}");
    let b_levels: f64 = r
        .levels
        .iter()
        .filter(|l| l.level.as_deref() == Some("b-zemin"))
        .map(|l| l.all())
        .sum();
    assert!(
        (b_all - b_levels).abs() < 1e-9,
        "B's own storey alone, not the basement: {b_all} {b_levels}"
    );
}

#[test]
fn each_block_s_model_carries_its_own_areas() {
    let doc = blocks();
    let r = doc.zoning_report().expect("the parcel's zoning");
    let stamp = "2026-11-16T12:00:00";
    let mut gross = 0.0;
    let mut foot = 0.0;
    for id in ["a", "b"] {
        let (b, _, _) =
            kentos_native_application::arch_ifc::building_of(&doc, id, stamp, "KentOS Mimari")
                .expect("a block");
        let common = &b
            .building
            .psets
            .iter()
            .find(|p| p.name == "Pset_BuildingCommon")
            .expect("its pset")
            .props;
        let qto = &b
            .building
            .qtos
            .iter()
            .find(|q| q.name == "Qto_BuildingBaseQuantities")
            .expect("its qto")
            .quantities;
        let g = area(common, "GrossPlannedArea").expect("its own gross area");
        assert_eq!(area(qto, "GrossFloorArea"), Some(g));
        gross += g;
        foot += area(qto, "FootPrintArea").expect("its own footprint");
    }
    assert!(
        (gross - r.construction()).abs() < 1e-9,
        "{gross} ≠ {}",
        r.construction()
    );
    assert!(
        (foot - r.footprint).abs() < 1e-6,
        "{foot} ≠ {}",
        r.footprint
    );
}

#[test]
fn metraj_by_building_partitions_the_project_s_rooms() {
    let mut doc = blocks();
    let rooms = |doc: &mut Document, input: Json| -> Vec<String> {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.spaces.quantities",
            CallOp::Plan,
            input,
        )
        .unwrap();
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"]["spaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["uid"].as_str().unwrap().to_owned())
            .collect()
    };
    let all = rooms(&mut doc, json!({}));
    let a = rooms(&mut doc, json!({"building": "a"}));
    let b = rooms(&mut doc, json!({"building": "b"}));
    let mut both: Vec<String> = a.iter().chain(&b).cloned().collect();
    both.sort();
    let mut every = all.clone();
    every.sort();
    assert_eq!(
        both, every,
        "every room in one building's quantities, none in two"
    );
    let names = |doc: &mut Document, input: Json| -> Vec<String> {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.spaces.quantities",
            CallOp::Plan,
            input,
        )
        .unwrap();
        answer["output"]["spaces"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|s| s["name"].as_str().map(str::to_owned))
            .collect()
    };
    assert!(names(&mut doc, json!({"building": "a"})).contains(&"Otopark".to_owned()));
    assert_eq!(names(&mut doc, json!({"building": "b"})), ["Dükkân"]);
    let refused = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.spaces.quantities",
        CallOp::Plan,
        json!({"building": "c"}),
    )
    .unwrap();
    assert_eq!(refused["error"]["code"], "building_not_found", "{refused}");
}

#[test]
fn a_block_s_permit_set_is_its_own_storeys_with_its_code() {
    let mut doc = blocks();
    let input = permit_set::PermitSet {
        building: Some("b".into()),
        ..Default::default()
    };
    let (_, made, _) =
        permit_set::execute(&mut ExecutionContext::new(&mut doc), &input).expect("B's set");
    let titles: Vec<&str> = made.iter().map(|m| m.title.as_str()).collect();
    assert!(titles.contains(&"B zemin kat planı"), "{titles:?}");
    assert!(
        !titles
            .iter()
            .any(|t| t.contains("Bodrum") || t.contains("1. kat") || *t == "Zemin kat planı"),
        "{titles:?}"
    );
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("its sheets"))
        .expect("the book");
    for m in &made {
        let sheet = book
            .sheets
            .iter()
            .find(|s| s.id == m.sheet)
            .expect("the sheet");
        let var = |name: &str| {
            sheet
                .variables
                .iter()
                .find(|v| v.name == name)
                .map(|v| v.value.clone())
        };
        assert_eq!(
            var("bina_kodu"),
            Some(VarValue::Text("B1".into())),
            "{}",
            m.title
        );
        assert_eq!(
            var("blok_no"),
            Some(VarValue::Text("B1".into())),
            "{}",
            m.title
        );
        if m.title == "B zemin kat planı" {
            // Its storey at +1.20 is B's ±0.00: K00 (EK-2 Tablo 2.14).
            assert_eq!(var("kat_konum"), Some(VarValue::Text("K00".into())));
        }
    }
    // A building the program does not have is refused.
    let none = permit_set::PermitSet {
        building: Some("c".into()),
        ..Default::default()
    };
    assert!(
        permit_set::execute(&mut ExecutionContext::new(&mut doc), &none)
            .unwrap_err()
            .contains("“c”")
    );
}
