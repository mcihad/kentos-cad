//! A headless audit's regulation group with the standards package's Python rules (docs/adr/8078; plan/bim-plan.md
//! BIM-13): `cad.arch.audit` over the protocol, as MCP and Python call it, lists the national regulation's findings
//! beside the Rust ones and says its rule set. The same drawing (three storeys of flats over a basement of storage
//! annexes and a boiler room, the elevator's shaft on the storeys only) applied for on 30.6.2026 passes 34/1 as of
//! 11.7.2021; applied for on 2.7.2026 it fails 34/8 as of 1.7.2026; with no date it is eksik veri.

use kentos_commands::{CallEnvelope, CallOp, CommandResult};
use kentos_domain::Document;
use serde_json::{Value, json};

fn run(doc: &mut Document, name: &str, input: Value) {
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// The acceptance drawing, built with KentOS's commands from the audit facts' empty drawing.
fn basement_with_annexes() -> Document {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/audit-facts/v1/mimari.kcad"
    );
    let mut snap: kentos_commands::DocumentSnapshotV2 =
        kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    snap.entities.clear();
    snap.uids.clear();
    if let Some(arch) = snap.settings.arch.as_mut() {
        arch.levels.clear();
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let level = |id: &str, name: &str, z: f64| json!({ "id": id, "name": name, "elevation": z, "height": 3.0 });
    run(
        &mut doc,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            level("b01", "Bodrum kat", -3.0), level("k00", "Zemin kat", 0.0), level("k01", "1. kat", 3.0),
            level("k02", "2. kat", 6.0)
        ] } }),
    );
    run(
        &mut doc,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"},
            {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire"},
            {"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
            {"id": "e2", "number": "E2", "kind": "annex", "annexOf": "d2", "nature": "Depo"},
            {"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let layer = doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let room = |x0: f64, name: &str, level: &str, unit: &str, void: bool| {
        json!({"parts": [{"outer": {"pts": [p(x0, 0.0), p(x0 + 4.0, 0.0), p(x0 + 4.0, 4.0), p(x0, 4.0)]}}],
               "name": name, "unit": unit, "void": void, "arch": {"level": level}})
    };
    let mut spaces = vec![
        room(0.0, "Depo", "b01", "e1", false),
        room(4.0, "Depo", "b01", "e2", false),
        room(8.0, "Kazan dairesi", "b01", "o", false),
    ];
    for (level, unit) in [("k00", "d1"), ("k01", "d2"), ("k02", "d3")] {
        spaces.push(room(0.0, "Salon", level, unit, false));
        spaces.push(room(4.0, "Yatak odası", level, unit, false));
        spaces.push(room(8.0, "Merdiven holü", level, "o", false));
        spaces.push(room(12.0, "Asansör", level, "o", true));
    }
    run(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": spaces}),
    );
    doc
}

fn dated(doc: &mut Document, date: Option<&str>) {
    let mut settings = doc.settings().clone();
    settings.variables.retain(|v| v.name != "basvuru_tarihi");
    if let Some(d) = date {
        settings.variables.push(kentos_commands::ProjectVariable {
            name: "basvuru_tarihi".into(),
            label: "Başvuru tarihi".into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(d.into()),
        });
    }
    doc.set_settings(settings);
}

/// The regulation group asked of the whole drawing over the protocol: its output.
fn audited(doc: &mut Document, input: Value) -> Value {
    let answer = kentos_headless::call_envelope(
        doc,
        None,
        CallEnvelope {
            command: "cad.arch.audit".into(),
            version: None,
            op: CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    );
    match answer.result {
        CommandResult::Completed { output, warnings } => {
            assert!(
                warnings
                    .iter()
                    .all(|w| w.code != "regulation_rule_unavailable"),
                "{warnings:?}"
            );
            output
        }
        other => panic!("{other:?}"),
    }
}

fn package(output: &Value) -> Vec<(String, Option<String>, String)> {
    output["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| {
            matches!(
                f["code"].as_str(),
                Some(
                    "paiy_elevator_missing"
                        | "paiy_elevator_not_serving"
                        | "regulation_date_needed"
                )
            )
        })
        .map(|f| {
            (
                f["code"].as_str().unwrap().to_owned(),
                f["state"].as_str().map(str::to_owned),
                f["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// The lift articles' outcomes (the door rule's rows are its own test's).
fn outcomes(output: &Value) -> Vec<(String, Value, String)> {
    output["ruleSet"]["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["rule"].as_str().is_some_and(|x| x.contains(" 34/")))
        .map(|r| {
            (
                r["rule"].as_str().unwrap().to_owned(),
                r["version"].clone(),
                r["state"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_headless_audit_reads_the_elevator_articles_by_the_application_date() {
    let mut doc = basement_with_annexes();
    let ask = json!({"checks": ["regulation"]});
    dated(&mut doc, Some("30.06.2026"));
    let out = audited(&mut doc, ask.clone());
    assert_eq!(package(&out), Vec::new());
    assert_eq!(
        outcomes(&out),
        [
            (
                "PAİY 34/1".to_owned(),
                json!("11.7.2021"),
                "passed".to_owned()
            ),
            (
                "PAİY 34/8".to_owned(),
                json!("11.7.2021"),
                "outOfScope".to_owned()
            ),
        ]
    );
    assert_eq!(out["ruleSet"]["date"], "2026-06-30");
    dated(&mut doc, Some("02.07.2026"));
    let out = audited(&mut doc, ask.clone());
    let found = package(&out);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "paiy_elevator_not_serving");
    assert!(
        found[0]
            .2
            .starts_with("Bodrum kat: asansör bu kata ulaşmıyor"),
        "{}",
        found[0].2
    );
    let finding = out["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "paiy_elevator_not_serving")
        .unwrap()
        .clone();
    assert_eq!(finding["check"], "regulation");
    assert_eq!(finding["rule"], "PAİY 34/8 · RG 1.7.2026/33297");
    // It names the basement's rooms: its key is theirs, a click shows them.
    assert_eq!(finding["uids"].as_array().map(Vec::len), Some(3));
    assert!(
        finding["key"]
            .as_str()
            .unwrap()
            .starts_with("paiy_elevator_not_serving:")
    );
    assert_eq!(
        outcomes(&out)[1],
        (
            "PAİY 34/8".to_owned(),
            json!("1.7.2026"),
            "failed".to_owned()
        )
    );
    // It follows the regulation group's own findings, before any other group's.
    let checks: Vec<&str> = out["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["check"].as_str().unwrap())
        .collect();
    assert!(checks.iter().all(|c| *c == "regulation"), "{checks:?}");
    // Left on purpose over the protocol: the finding given as the audit listed it, then hidden and counted.
    let excepted = out["excepted"].as_u64().unwrap();
    let marked = kentos_headless::call_envelope(
        &mut doc,
        None,
        CallEnvelope {
            command: "cad.arch.exceptions".into(),
            version: None,
            op: CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input: json!({"operation": "mark", "keys": [finding["key"]], "note": "İdare bodruma asansör istemedi.",
                          "findings": [finding]}),
        },
    );
    assert!(
        matches!(marked.result, CommandResult::Completed { .. }),
        "{:?}",
        marked.result
    );
    let out = audited(&mut doc, ask.clone());
    assert!(package(&out).is_empty(), "{out}");
    assert_eq!(out["excepted"].as_u64(), Some(excepted + 1));
    let shown = audited(
        &mut doc,
        json!({"checks": ["regulation"], "exceptions": "show"}),
    );
    let f = shown["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "paiy_elevator_not_serving")
        .unwrap();
    assert_eq!(
        (f["exception"].as_bool(), f["note"].as_str()),
        (Some(true), Some("İdare bodruma asansör istemedi."))
    );
    dated(&mut doc, None);
    let out = audited(&mut doc, ask);
    let found = package(&out);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        (found[0].0.as_str(), found[0].1.as_deref()),
        ("regulation_date_needed", Some("missingData"))
    );
    assert!(out["ruleSet"].get("date").is_none());
    // Of Etkin kat, or without the regulation group, the rule is not asked.
    let out = audited(
        &mut doc,
        json!({"checks": ["regulation"], "within": {"levels": ["k00"]}}),
    );
    assert!(
        out.get("ruleSet").is_none() && package(&out).is_empty(),
        "{out}"
    );
    let out = audited(&mut doc, json!({"checks": ["walls"]}));
    assert!(out.get("ruleSet").is_none(), "{out}");
}

/// An office's storey (docs/adr/8078 §8): a meeting room of 10 × 12 m, the corridor north of it, the escape stair's room
/// north of that; a door from the room to the corridor opening to `side`, a door from the corridor into the stair; the
/// stair door's declared class `rating`.
fn office_storey(side: &str, rating: &str) -> Document {
    let mut doc = basement_with_annexes();
    let mut settings = doc.settings().clone();
    let arch = settings.arch.as_mut().expect("Mimari");
    arch.levels = serde_json::from_value(
        json!([{"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]),
    )
    .expect("the storey");
    arch.program = serde_json::from_value(json!({"units": [
        {"id": "b1", "number": "1", "nature": "Büro"}, {"id": "o", "number": "O", "kind": "common"}
    ]}))
    .expect("the program");
    doc.set_settings(settings);
    let gone: Vec<Value> = doc
        .entities_with_uids()
        .map(|(u, _)| json!(u.to_string()))
        .collect();
    let call = |doc: &mut Document, name: &str, input: Value| -> Value {
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(doc),
            name,
            CallOp::Execute,
            input,
        )
        .unwrap();
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    call(&mut doc, "cad.entities.delete", json!({"uids": gone}));
    let layer = doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64), w: f64| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": "k00"}});
    let walls = call(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (10.0, 0.0), 0.2), wall((10.0, 0.0), (10.0, 16.0), 0.2), wall((10.0, 16.0), (0.0, 16.0), 0.2),
            wall((0.0, 16.0), (0.0, 0.0), 0.2), wall((0.0, 12.0), (10.0, 12.0), 0.1), wall((0.0, 14.0), (10.0, 14.0), 0.1)
        ]}),
    );
    let host = |i: usize| walls["output"]["uids"][i].as_str().unwrap().to_owned();
    let doors = call(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [
            {"host": host(4), "at": 5.0, "form": "door", "width": 1.0, "head": 2.1, "side": side},
            {"host": host(5), "at": 5.0, "form": "door", "width": 1.0, "head": 2.1}
        ]}),
    );
    let stair_door = doors["output"]["uids"][1].clone();
    call(
        &mut doc,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": [stair_door], "fireRating": {"value": rating, "basis": {"kind": "author"}}}}),
    );
    let rect =
        |x0: f64, y0: f64, x1: f64, y1: f64| vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)];
    call(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"parts": [{"outer": {"pts": rect(0.1, 0.1, 9.9, 11.95)}}], "name": "Toplantı salonu", "unit": "b1", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(0.1, 12.05, 9.9, 13.95)}}], "name": "Koridor", "unit": "b1", "arch": {"level": "k00"}},
            {"parts": [{"outer": {"pts": rect(0.1, 14.05, 9.9, 15.9)}}], "name": "Merdiven", "unit": "o", "arch": {"level": "k00"}}
        ]}),
    );
    dated(&mut doc, Some("02.07.2026"));
    doc
}

fn doors_said(output: &Value) -> Vec<(String, Option<String>, String)> {
    output["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| {
            f["code"].as_str().is_some_and(|c| {
                c.starts_with("bykhy_exit_door") || c.starts_with("bykhy_stair_door")
            })
        })
        .map(|f| {
            (
                f["code"].as_str().unwrap().to_owned(),
                f["state"].as_str().map(str::to_owned),
                f["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_crowded_room_s_door_and_the_stair_s_door_are_read_by_bykhy_47() {
    let ask = json!({"checks": ["regulation"]});
    let state = |out: &Value, rule: &str| {
        out["ruleSet"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule"] == rule)
            .map(|r| r["state"].as_str().unwrap().to_owned())
    };
    // The room's door opens to the corridor (the path's left, out of the room); the stair door is EI2 60-S200C5.
    let mut doc = office_storey("left", "EI2 60-S200C5");
    let out = audited(&mut doc, ask.clone());
    assert_eq!(doors_said(&out), Vec::new(), "{}", out["ruleSet"]);
    assert_eq!(state(&out, "BYKHY 47/2").as_deref(), Some("passed"));
    assert_eq!(state(&out, "BYKHY 47/3").as_deref(), Some("passed"));
    // Turned into the room: 47/2 says it; a stair door of 30 minutes: 47/3 says it.
    let mut doc = office_storey("right", "EI 30");
    let out = audited(&mut doc, ask);
    let said = doors_said(&out);
    assert_eq!(said.len(), 2, "{said:#?}");
    assert!(
        said[0].2.contains("Toplantı salonu") && said[0].2.contains("içine açılıyor"),
        "{said:#?}"
    );
    assert!(
        said[1].2.contains("30 dakika < 60 dakika") && said[1].2.contains("duman sızdırmaz değil"),
        "{said:#?}"
    );
    assert_eq!(state(&out, "BYKHY 47/2").as_deref(), Some("failed"));
    assert_eq!(state(&out, "BYKHY 47/3").as_deref(), Some("failed"));
}

/// `cad.arch.ifc` over the protocol with no `compartments` (docs/adr/8088): the package's fire compartments are the
/// file's zones. Every wall of the office's storey declared EI 60: the meeting room and the corridor stay one
/// compartment (their door declares nothing), the stair's room, behind EI 60 and its EI2 60 door, another; given an
/// empty list, the file has none.
#[test]
fn a_headless_building_ifc_writes_the_package_s_fire_compartments() {
    let mut doc = office_storey("left", "EI2 60-S200C5");
    let walls: Vec<Value> = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, kentos_commands::Entity::Wall(_)))
        .map(|(u, _)| json!(u.to_string()))
        .collect();
    run(
        &mut doc,
        "cad.arch.performance",
        json!({"operation": {"kind": "objects", "uids": walls, "fireRating": {"value": "EI 60", "basis": {"kind": "author"}}}}),
    );
    let file = std::env::temp_dir().join(format!(
        "kentos-headless-kompartiman-{}.ifc",
        std::process::id()
    ));
    let export = |doc: &mut Document, compartments: Option<Value>| -> (Value, String) {
        let mut input = json!({"path": file.display().to_string(), "overwrite": true, "stamp": "2026-11-17T12:00:00"});
        if let Some(c) = compartments {
            input["compartments"] = c;
        }
        let answer = kentos_headless::call_envelope(
            doc,
            None,
            CallEnvelope {
                command: "cad.arch.ifc".into(),
                version: None,
                op: CallOp::Execute,
                request_id: None,
                expected: None,
                relax_constraints: false,
                input,
            },
        );
        let output = match answer.result {
            CommandResult::Completed { output, warnings } => {
                assert!(
                    warnings
                        .iter()
                        .all(|w| w.code != "regulation_rule_unavailable"),
                    "{warnings:?}"
                );
                output
            }
            other => panic!("{other:?}"),
        };
        (output, std::fs::read_to_string(&file).expect("the file"))
    };
    let (output, text) = export(&mut doc, None);
    assert_eq!(output["compartments"], 2, "{output}");
    let zones = text
        .lines()
        .filter(|l| l.contains("IFCZONE(") && l.contains("'MM-ZON-Yang"))
        .count();
    assert_eq!(zones, 2);
    let (output, text) = export(&mut doc, Some(json!([])));
    assert_eq!(output["compartments"], 0, "{output}");
    assert!(!text.contains("'MM-ZON-Yang"));
    let _ = std::fs::remove_file(&file);
}
