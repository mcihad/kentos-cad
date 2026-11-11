//! Dijital proje denetimi (docs/adr/8047 §9): Mimari denetim › Mevzuat says the model quality control form's (EK-9)
//! questions the section building fails, from the IFC model `cad.arch.ifc` writes for it: the künye's project words
//! (Md. 13, EK-7), the project's code (Md. 6), the rooms' numbers and names (Md. 10), its coordinate system (EK-7
//! Tablo 7.6–7.7), its rooms, and what the model does not hold (Md. 9).

use kentos_commands::{CallOp, DocumentSnapshotV2};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn snapshot() -> DocumentSnapshotV2 {
    kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap()
}

/// The section building's künye as an office fills it, in TUREF / TM33 with ±0.00 at 1285.40 m.
fn whole(mut snap: DocumentSnapshotV2) -> DocumentSnapshotV2 {
    for (name, value) in [
        ("proje_kodu", "260123"),
        ("il", "Sivas"),
        ("ilce", "Merkez"),
        ("mahalle", "Esentepe"),
        ("ada", "1234"),
        ("parsel", "5"),
        ("mimar", "Ayşe Yılmaz"),
        ("sicil_no", "41234"),
        ("buro_adi", "Örnek Mimarlık"),
        ("vergi_no", "1234567890"),
        ("buro_adres", "Atatürk Cad. No:1 Sivas"),
    ] {
        snap.settings
            .variables
            .push(kentos_commands::ProjectVariable {
                name: name.into(),
                label: name.into(),
                value: kentos_commands::VariableValue::Text(value.into()),
                ..Default::default()
            });
    }
    snap.settings.srid = 5255;
    if let Some(arch) = snap.settings.arch.as_mut() {
        arch.site = Some(kentos_commands::SitePlan {
            datum: Some(1285.4),
            ..Default::default()
        });
    }
    snap
}

/// Mevzuat's digital project findings: (code, message, its objects' count).
fn digital(doc: &mut Document) -> Vec<(String, String, usize)> {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        "cad.arch.audit",
        CallOp::Execute,
        json!({"checks": ["regulation"]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"].as_str().is_some_and(|c| c.starts_with("digital")))
        .map(|f: &Value| {
            (
                f["code"].as_str().unwrap().to_owned(),
                f["message"].as_str().unwrap().to_owned(),
                f["uids"].as_array().map_or(0, Vec::len),
            )
        })
        .collect()
}

#[test]
fn a_drawing_without_its_kunye_and_system_is_said_to_fail_the_form() {
    let mut doc = Document::from_snapshot_v2(snapshot()).unwrap();
    let found = digital(&mut doc);
    let codes: Vec<&str> = found.iter().map(|(c, ..)| c.as_str()).collect();
    assert_eq!(
        codes,
        [
            "digital_project_info_missing",
            "digital_project_code_missing",
            "digital_system_wrong"
        ],
        "{found:#?}"
    );
    assert!(
        found[0].1.contains("mimar, Mimarlar Odası sicil no"),
        "{}",
        found[0].1
    );
    assert!(
        found[0].1.ends_with("— Dijital Proje Yön. 13"),
        "{}",
        found[0].1
    );
    assert!(
        found[2].1.contains("yerel koordinatlarda"),
        "{}",
        found[2].1
    );
}

#[test]
fn a_whole_kunye_in_turef_fails_nothing_the_drawing_can_hold() {
    let mut doc = Document::from_snapshot_v2(whole(snapshot())).unwrap();
    assert_eq!(digital(&mut doc), Vec::new());
}

#[test]
fn a_ramp_is_said_with_its_object_and_a_room_without_a_name_with_its() {
    let mut snap = whole(snapshot());
    let n = snap.entities.len() as u64 + 1;
    let ramp: kentos_commands::Entity = serde_json::from_value(json!({
        "kind": "ramp", "id": n, "layerId": "merdiven", "attrs": {},
        "pts": [{"x": 5.3, "y": 12.0}, {"x": 5.3, "y": 9.25}], "width": 1.2, "rise": 0.15, "slope": 5.0,
        "arch": {"level": "zemin"}
    }))
    .unwrap();
    snap.entities.push(ramp);
    snap.uids.push(kentos_commands::EntityId(
        (0x0192f5a1_4242_7000_8000_000000000000_u128 + u128::from(n)).to_be_bytes(),
    ));
    // The first room (Yatak odası, 001) loses its name.
    for e in &mut snap.entities {
        if let kentos_commands::Entity::Space(s) = e {
            s.space.name = None;
            break;
        }
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let found = digital(&mut doc);
    // Mimari denetim orders a group's findings by their place: the building's corner (0, 0), then the room's (0.2, 0.2).
    let codes: Vec<&str> = found.iter().map(|(c, ..)| c.as_str()).collect();
    assert_eq!(
        codes,
        ["digital_model_loss", "digital_space_unnamed"],
        "{found:#?}"
    );
    assert_eq!(found[1].2, 1, "the room without a name");
    assert!(
        found[1].1.starts_with("1 mahalin numarası ya da adı yok"),
        "{}",
        found[1].1
    );
    assert_eq!(found[0].2, 1, "the ramp");
    assert!(found[0].1.contains("1 rampa"), "{}", found[0].1);
    assert!(
        found[0].1.ends_with("— Dijital Proje Yön. 9"),
        "{}",
        found[0].1
    );
}

#[test]
fn a_drawing_without_walls_is_no_building() {
    let mut snap = snapshot();
    let keep: Vec<usize> = snap
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, kentos_commands::Entity::Grid(_)))
        .map(|(i, _)| i)
        .collect();
    snap.entities = keep.iter().map(|&i| snap.entities[i].clone()).collect();
    snap.uids = keep.iter().map(|&i| snap.uids[i]).collect();
    for (k, e) in snap.entities.iter_mut().enumerate() {
        e.base_mut().id = k as u32 + 1;
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    assert_eq!(digital(&mut doc), Vec::new());
}
