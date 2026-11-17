use kentos_commands::CallOp;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

use super::*;

/// The audit facts' architecture drawing with nothing drawn: its layers and settings, no object.
fn empty() -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/audit-facts/v1/mimari.kcad");
    let mut snapshot: kentos_commands::DocumentSnapshotV2 = kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    snapshot.entities.clear();
    snapshot.uids.clear();
    if let Some(arch) = snapshot.settings.arch.as_mut() {
        arch.levels.clear();
    }
    Document::from_snapshot_v2(snapshot).unwrap()
}

/// A building of three storeys over a basement (docs/adr/8078's acceptance drawing): the basement holds two storage
/// rooms, each a flat's annex, and a boiler room of the common area; each storey a flat's living room and bedroom and
/// the common stair hall; an elevator shaft (a void named Asansör) on the three storeys, not in the basement; on the
/// first storey an Asansör block standing in the shaft (one elevator, not two).
pub(crate) fn basement_with_annexes() -> Document {
    let mut doc = empty();
    let mut settings = doc.settings().clone();
    settings.arch = Some(
        serde_json::from_value(json!({
            "levels": [
                {"id": "b01", "name": "Bodrum kat", "elevation": -3.0, "height": 3.0},
                {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k01", "name": "1. kat", "elevation": 3.0, "height": 3.0},
                {"id": "k02", "name": "2. kat", "elevation": 6.0, "height": 3.0}
            ],
            "program": {"units": [
                {"id": "d1", "number": "1", "nature": "Daire"},
                {"id": "d2", "number": "2", "nature": "Daire"},
                {"id": "d3", "number": "3", "nature": "Daire"},
                {"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
                {"id": "e2", "number": "E2", "kind": "annex", "annexOf": "d2", "nature": "Depo"},
                {"id": "o", "number": "O", "kind": "common"}
            ]}
        }))
        .expect("the building's settings"),
    );
    doc.set_settings(settings);
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let rect = |x0: f64| vec![p(x0, 0.0), p(x0 + 4.0, 0.0), p(x0 + 4.0, 4.0), p(x0, 4.0)];
    let room = |x0: f64, name: &str, level: &str, unit: Option<&str>, void: bool| {
        let mut r = json!({"parts": [{"outer": {"pts": rect(x0)}}], "name": name, "arch": {"level": level}});
        if let Some(u) = unit {
            r["unit"] = json!(u);
        }
        if void {
            r["void"] = json!(true);
        }
        r
    };
    let mut spaces: Vec<Value> = vec![
        room(0.0, "Depo", "b01", Some("e1"), false),
        room(4.0, "Depo", "b01", Some("e2"), false),
        room(8.0, "Kazan dairesi", "b01", Some("o"), false),
    ];
    for (level, unit) in [("k00", "d1"), ("k01", "d2"), ("k02", "d3")] {
        spaces.push(room(0.0, "Salon", level, Some(unit), false));
        spaces.push(room(4.0, "Yatak odası", level, Some(unit), false));
        spaces.push(room(8.0, "Merdiven holü", level, Some("o"), false));
        spaces.push(room(12.0, "Asansör", level, Some("o"), true));
    }
    let layer = doc.layers().active().to_owned();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.spaces.create",
        CallOp::Execute,
        json!({"layerId": layer, "spaces": spaces}),
    )
    .expect("the rooms");
    assert_eq!(answer["status"], "completed", "{answer}");
    doc
}

#[test]
fn each_storey_says_its_rooms_by_group_and_its_elevators() {
    let facts = measure(&basement_with_annexes()).normalized();
    assert_eq!((facts.units, facts.dwellings, facts.local.as_deref()), (3, 3, None));
    let s: Vec<(&str, u32, u32, u32, u32, u32, u32, u32)> = facts
        .storeys
        .iter()
        .map(|s| (s.level.as_str(), s.units, s.common, s.annexes, s.other, s.technical, s.inhabited, s.elevators))
        .collect();
    assert_eq!(
        s,
        [
            ("b01", 0, 0, 2, 0, 1, 0, 0),
            ("k00", 2, 1, 0, 0, 0, 2, 1),
            ("k01", 2, 1, 0, 0, 0, 2, 1),
            ("k02", 2, 1, 0, 0, 0, 2, 1),
        ]
    );
    assert_eq!(facts.storeys[0].elevation, -3.0);
}

/// Without Mimari's storeys there is nothing to count: the section is left out of the facts.
#[test]
fn a_project_without_storeys_gives_no_section() {
    let facts = measure(&empty());
    assert!(facts.is_empty());
    let project = kentos_extensions::audit_facts::ProjectAuditFacts::new(0.0, Vec::new(), false).with_building(facts);
    assert!(!project.to_json().contains("building"));
}
