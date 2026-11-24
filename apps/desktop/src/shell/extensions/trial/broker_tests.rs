//! A trial's step against a stand-in for its code: the calls a command makes, answered on the synthetic drawing.

use std::collections::BTreeMap;

use kentos_extensions::manifest::Capability;
use kentos_python_env::process::{Answer, Heard};
use serde_json::{Value, json};

use super::super::drawing::{Synthetic, synthetic};
use super::super::steps::{Origin, Rights, Spec};
use super::{Ground, run};

fn building(x: f64, no: &str) -> Value {
    json!({ "layer": "yapi", "geometry": { "type": "Polygon",
            "coordinates": [[[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 12.0], [x, 12.0], [x, 0.0]]] },
            "attrs": { "no": no } })
}

fn drawing() -> Synthetic {
    synthetic(&[building(0.0, ""), building(50.0, "Y-02")]).expect("a drawing")
}

fn rights(capabilities: &[Capability]) -> Rights {
    let panel = kentos_extensions::ui_panel::read(&json!({
        "format": kentos_extensions::ui_panel::FORMAT, "schemaVersion": kentos_extensions::ui_panel::VERSION,
        "title": "Yapı ara",
        "body": [
            { "type": "textField", "id": "sorgu", "label": "Numara", "required": true },
            { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara", "table": "sonuc" },
            { "type": "table", "id": "sonuc", "columns": ["no"] }
        ]
    }));
    Rights {
        ext: "tr.ornek.x".into(),
        capabilities: capabilities.to_vec(),
        depends: Vec::new(),
        panels: BTreeMap::from([("yapi-paneli".to_owned(), panel)]),
    }
}

fn spec(dialogs: Vec<Value>) -> Spec {
    Spec {
        id: "ext.tr.ornek.x.numarala".into(),
        map_tool: false,
        title: "Yapıları numarala".into(),
        params: json!({}),
        panel: None,
        selection: vec![0],
        dialogs,
    }
}

/// Runs `spec` on `made` with `code` standing in for the extension's code.
fn step(
    made: &mut Synthetic,
    rights: &Rights,
    panels: &mut BTreeMap<String, super::super::super::panels::OpenPanel>,
    spec: &Spec,
    code: &mut dyn FnMut(&Value, &mut Answer<'_>) -> Value,
) -> Value {
    let places: BTreeMap<String, usize> = made
        .uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.to_string(), i))
        .collect();
    let ground = Ground {
        doc: &mut made.doc,
        rights,
        panels,
        places: &places,
        uids: &made.uids,
    };
    run(
        ground,
        spec,
        &mut |invoke: Value,
              document: bool,
              answer: &mut Answer<'_>,
              heard: &mut dyn FnMut(Heard)| {
            assert_eq!(document, rights.read(), "the drawing given as granted");
            heard(Heard::Out {
                stream: "out".into(),
                text: "başladı\n".into(),
            });
            Ok(code(&invoke, answer))
        },
    )
}

fn set_no(uid: &str, no: &str) -> Value {
    json!({ "command": "cad.entities.set", "op": "execute",
            "input": { "uids": [uid], "operation": "attributes", "attrs": { "no": no } } })
}

/// A command reads and writes the synthetic drawing as granted, its writes one undo step named after it; the objects it
/// changed are said by their place in the agent's list, what it selected and printed too; a call the broker does not
/// answer is refused and listed. A command that fails after writing has its writes taken back.
#[test]
fn a_command_writes_one_step_and_a_failing_one_is_taken_back() {
    let mut made = drawing();
    let first = made.uids[0].to_string();
    let rights = rights(&[Capability::DocumentWrite]);
    let mut panels = BTreeMap::new();
    let report = step(
        &mut made,
        &rights,
        &mut panels,
        &spec(Vec::new()),
        &mut |invoke, answer| {
            assert_eq!(invoke["kind"], "command");
            let selected = answer("selection", &json!({})).expect("the selection");
            assert_eq!(selected.as_array().map(Vec::len), Some(1), "{selected}");
            answer("layers", &json!({})).expect("it reads");
            answer("run", &set_no(&first, "Y-01")).expect("it writes");
            let undo = answer("undo", &json!({})).unwrap_err();
            assert_eq!(undo.0, "not_for_extensions");
            json!({ "type": "done", "ok": true, "result": { "kind": "message", "message": "1 yapı numaralandı." } })
        },
    );
    assert_eq!(report["status"], "done", "{report}");
    assert_eq!(report["message"], "1 yapı numaralandı.");
    assert_eq!(report["changed"]["objects"], 1, "{report}");
    assert_eq!(report["changed"]["sample"][0]["feature"], 0);
    assert_eq!(report["changed"]["sample"][0]["attrs"]["no"], "Y-01");
    assert_eq!(report["refusedCalls"][0]["method"], "undo");
    assert_eq!(report["output"], "başladı\n");
    assert!(report.get("undone").is_none());
    assert_eq!(
        made.doc.undo_labels().first().map(String::as_str),
        Some("Yapıları numarala"),
        "one step, the last"
    );
    // It fails after writing: what it wrote is taken back, the error said with its traceback.
    let report = step(
        &mut made,
        &rights,
        &mut panels,
        &spec(Vec::new()),
        &mut |_, answer| {
            answer("run", &set_no(&first, "Y-99")).expect("it writes");
            json!({ "type": "done", "ok": false, "exception": "ValueError", "message": "olmadı",
                "error": "Traceback …" })
        },
    );
    assert_eq!(report["status"], "failed", "{report}");
    assert_eq!(report["error"], "ValueError: olmadı");
    assert_eq!(report["traceback"], "Traceback …");
    assert_eq!(report["undone"], true);
    assert!(report.get("changed").is_none(), "taken back: {report}");
    let no = made
        .doc
        .slot_of(made.uids[0])
        .and_then(|s| made.doc.get(s))
        .map(|e| e.base().attrs.get("no").cloned());
    assert_eq!(no, Some(Some("Y-01".to_owned())));
}

/// Without `document.write` a write is refused in the broker's words and listed; another extension's tool is refused
/// without a declared dependency; without `document.read` the drawing is not given.
#[test]
fn what_the_user_did_not_grant_is_refused() {
    let mut made = drawing();
    let first = made.uids[0].to_string();
    let reader = rights(&[Capability::DocumentRead]);
    let mut panels = BTreeMap::new();
    let report = step(
        &mut made,
        &reader,
        &mut panels,
        &spec(Vec::new()),
        &mut |_, answer| {
            let write = answer("run", &set_no(&first, "Y-01")).unwrap_err();
            assert_eq!(write.0, "capability");
            assert!(write.1.contains("document.write"), "{}", write.1);
            let other = answer(
                "run",
                &json!({ "command": "cad.processing.run", "op": "dryRun",
                     "input": { "tool": "ext.tr.ornek.baska.arac", "parameters": {} } }),
            )
            .unwrap_err();
            assert!(other.1.contains("dependencies.extensions"), "{}", other.1);
            json!({ "type": "done", "ok": false, "refused": true, "message": "Yazamadım." })
        },
    );
    assert_eq!(report["status"], "refused");
    assert_eq!(report["message"], "Yazamadım.");
    assert_eq!(report["refusedCalls"].as_array().map(Vec::len), Some(2));
    let blind = rights(&[]);
    let report = step(
        &mut made,
        &blind,
        &mut panels,
        &spec(Vec::new()),
        &mut |_, answer| {
            assert!(
                answer("layers", &json!({}))
                    .unwrap_err()
                    .1
                    .contains("document.read")
            );
            json!({ "type": "done", "ok": true, "result": { "kind": "none" } })
        },
    );
    assert_eq!(report["status"], "done");
}

/// A dialog is checked as the window checks it and answered with the agent's next answer, its values checked as
/// Tamam checks them; an answer that does not fit, one not given and a dialog with a file picker are İptal, each
/// said.
#[test]
fn a_dialog_is_answered_with_the_agents_answer_or_cancelled() {
    let mut made = drawing();
    let rights = rights(&[Capability::DocumentWrite, Capability::FilesUserSelected]);
    let mut panels = BTreeMap::new();
    let ask = json!({ "title": "Yapıları numarala", "body": [
        { "type": "textField", "id": "onek", "label": "Önek", "value": "Y-" },
        { "type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1, "min": 1, "required": true }
    ] });
    let picker = json!({ "title": "CSV seç", "body": [{ "type": "filePicker", "id": "dosya", "label": "Dosya" }] });
    let answers = vec![json!({ "onek": "B-", "ilk": 3 }), json!({ "ilk": "üç" })];
    let report = step(
        &mut made,
        &rights,
        &mut panels,
        &spec(answers),
        &mut |_, answer| {
            let given = answer("dialog", &ask).expect("answered");
            assert_eq!(given["onek"], "B-");
            assert_eq!(given["ilk"], 3.0);
            assert_eq!(
                answer("dialog", &ask).expect("answered"),
                Value::Null,
                "a wrong answer is İptal"
            );
            assert_eq!(
                answer("dialog", &ask).expect("answered"),
                Value::Null,
                "none given is İptal"
            );
            assert_eq!(answer("dialog", &picker).expect("answered"), Value::Null);
            let wrong = answer(
                "dialog",
                &json!({ "title": "Boş", "body": [{ "type": "button", "id": "b" }] }),
            );
            assert_eq!(
                wrong.unwrap_err().0,
                "invalid_input",
                "the code's mistake, its error"
            );
            json!({ "type": "done", "ok": true, "result": { "kind": "message", "message": "Tamam." } })
        },
    );
    let asked = report["dialogs"].as_array().expect("its dialogs");
    assert_eq!(asked.len(), 4, "{report}");
    assert_eq!(asked[0]["fields"], json!(["onek", "ilk"]));
    assert_eq!(asked[0]["answer"]["ilk"], 3.0);
    assert!(
        asked[1]["error"]
            .as_str()
            .is_some_and(|e| e.contains("sayı")),
        "{}",
        asked[1]
    );
    assert!(
        asked[2]["note"]
            .as_str()
            .is_some_and(|n| n.contains("Cevabı verilmedi"))
    );
    assert!(
        asked[3]["note"]
            .as_str()
            .is_some_and(|n| n.contains("dosya seçici"))
    );
}

/// A panel's button: the agent's values set in the panel and checked as the button checks them (a required one
/// left out is said, nothing run); the code changes the panel and opens it again; the answer's rows go to the button's
/// table, the code's words stay under the form, and the report gives the panel as the step left it.
#[test]
fn a_panels_button_runs_with_its_values_and_its_answer_fills_the_panel() {
    let mut made = drawing();
    let rights = rights(&[Capability::DocumentRead]);
    let mut panels = BTreeMap::new();
    let mut button = spec(Vec::new());
    button.panel = Some(Origin {
        panel: "yapi-paneli".into(),
        button: "ara".into(),
        table: Some("sonuc".into()),
    });
    button.selection.clear();
    let report = step(&mut made, &rights, &mut panels, &button, &mut |_, _| {
        panic!("not run: its required value is missing")
    });
    assert_eq!(report["status"], "invalid");
    assert!(
        report["error"]
            .as_str()
            .is_some_and(|e| e.contains("Numara")),
        "{report}"
    );
    button.params = json!({ "sorgu": "Y-" });
    let report = step(
        &mut made,
        &rights,
        &mut panels,
        &button,
        &mut |invoke, answer| {
            assert_eq!(invoke["panel"], "yapi-paneli");
            assert_eq!(invoke["params"]["sorgu"], "Y-");
            answer("panel.say", &json!({ "text": "Aranıyor", "kind": "info" })).expect("its panel");
            answer("panel.open", &json!({ "panel": "yapi-paneli" })).expect("open again");
            let missing = answer("panel.open", &json!({ "panel": "yok" })).unwrap_err();
            assert_eq!(missing.0, "unknown_panel");
            json!({ "type": "done", "ok": true, "result": { "kind": "report", "rows": [{ "no": "Y-02" }],
                "columns": ["no"] } })
        },
    );
    assert_eq!(report["status"], "done", "{report}");
    let panel = &report["panels"][0];
    assert_eq!(panel["panel"], "yapi-paneli");
    assert_eq!(panel["tables"]["sonuc"]["rows"], 1, "{panel}");
    // Its code's words stay when its answer says none of its own, as on the user's panel.
    assert_eq!(panel["said"]["text"], "Aranıyor");
    assert_eq!(panel["values"]["sorgu"], "Y-");
    // From the ribbon its code has no panel of its own.
    let report = step(
        &mut made,
        &rights,
        &mut panels,
        &spec(Vec::new()),
        &mut |_, answer| {
            assert_eq!(
                answer("panel.say", &json!({ "text": "x" })).unwrap_err().0,
                "no_panel"
            );
            json!({ "type": "done", "ok": true, "result": { "kind": "none" } })
        },
    );
    assert_eq!(report["status"], "done");
}

/// Parsel alan çizelgesi in an extension's code (docs/adr/9306): with `document.read` the report is planned and lists
/// the areas given, writing nothing, and the table's write is refused; with `document.write` the table is written as
/// one object in the command's one undo step.
#[test]
fn an_extension_reads_the_parcel_table_and_writes_it_only_as_granted() {
    let mut made = drawing();
    let uids: Vec<String> = made.uids.iter().map(ToString::to_string).collect();
    let table = json!({ "command": "cad.parcels.table", "op": "execute",
                        "input": { "uids": uids, "at": { "x": 100.0, "y": 0.0 } } });
    let mut panels = BTreeMap::new();
    let reader = rights(&[Capability::DocumentRead]);
    let report = step(&mut made, &reader, &mut panels, &spec(Vec::new()), &mut |_, answer| {
        let listed = answer("run", &json!({ "command": "cad.parcels.report", "op": "plan", "input": { "uids": uids } }))
            .expect("it reads");
        assert_eq!(listed["status"], "completed", "{listed}");
        assert_eq!(listed["output"]["parcels"].as_array().map(Vec::len), Some(2), "{listed}");
        let write = answer("run", &table).unwrap_err();
        assert_eq!(write.0, "capability");
        json!({ "type": "done", "ok": true, "result": { "kind": "message", "message": "Okundu." } })
    });
    assert_eq!(report["status"], "done", "{report}");
    assert!(report.get("changed").is_none(), "nothing written: {report}");
    let writer = rights(&[Capability::DocumentWrite]);
    let report = step(&mut made, &writer, &mut panels, &spec(Vec::new()), &mut |_, answer| {
        let written = answer("run", &table).expect("it writes");
        assert_eq!(written["status"], "completed", "{written}");
        assert_eq!(written["output"]["parcels"], 2, "{written}");
        json!({ "type": "done", "ok": true, "result": { "kind": "message", "message": "Yazıldı." } })
    });
    assert_eq!(report["status"], "done", "{report}");
    assert_eq!(made.doc.undo_labels().first().map(String::as_str), Some("Yapıları numarala"), "one step");
}
