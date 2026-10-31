//! The first end-to-end acceptance drawing on the desktop (TODOS.md CADP-12,
//! plan/cad.md §23.1 K01–K04, K11, K12): the shared scenario
//! (fixtures/acceptance/v1/room.json, worked out by
//! scripts/fixtures/acceptance_room.py without KentOS code) replayed in the
//! app, its commands through the agent link an MCP client attaches to
//! (`desktop.attach`), undo as Ctrl+Z does it, a save and an open as the
//! app's own, the DXF through Dışa aktar › DXF's window and the PDF through
//! the sheet mode's Dışa aktar › PDF; every step's objects, measures and
//! sheets held to it, the files read back without KentOS code (the
//! plotter's tests/support/files.rs). Python and the MCP server replay the
//! same file; the mouse-and-keyboard trace draws the same room
//! (fixtures/interaction/v1/acceptance-room.json).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/room.json");

#[path = "../../../crates/native/plot/tests/support/files.rs"]
mod files;

/// Where `actual` differs from `expected`: only the fields `expected` names;
/// a point `[x, y]` is `{"x", "y"}` in a drawing; numbers to `tolerance`.
pub(crate) fn differs(actual: &Value, expected: &Value, tolerance: f64, at: &str, out: &mut Vec<String>) {
    if let (Some(e), Some(a)) = (expected.as_array(), actual.as_object())
        && e.len() == 2
        && e.iter().all(Value::is_number)
        && a.contains_key("x")
    {
        let point = json!([a["x"], a["y"]]);
        return differs(&point, expected, tolerance, at, out);
    }
    match expected {
        Value::Object(fields) => {
            for (k, v) in fields {
                differs(&actual[k.as_str()], v, tolerance, &format!("{at}.{k}"), out);
            }
        }
        Value::Array(items) => match actual.as_array() {
            Some(a) if a.len() == items.len() => {
                for (i, (x, e)) in a.iter().zip(items).enumerate() {
                    differs(x, e, tolerance, &format!("{at}[{i}]"), out);
                }
            }
            _ => out.push(format!("{at}: {actual}, beklenen {expected}")),
        },
        Value::Number(n) => {
            let ok = actual
                .as_f64()
                .zip(n.as_f64())
                .is_some_and(|(a, e)| (a - e).abs() <= tolerance);
            if !ok {
                out.push(format!("{at}: {actual}, beklenen {expected}"));
            }
        }
        _ => {
            if actual != expected {
                out.push(format!("{at}: {actual}, beklenen {expected}"));
            }
        }
    }
}

pub(crate) fn bind(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(s) if s.starts_with('$') => json!(names[&s[1..]]),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), bind(v, names)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names)).collect()),
        other => other.clone(),
    }
}

/// An agent's request to the open drawing, answered as the link answers it.
fn ask(app: &mut App, method: &str, params: Value) -> Value {
    app.agents_answer(&json!({ "id": 1, "method": method, "params": params }))
        .unwrap_or_else(|(code, message)| panic!("{method}: {code}: {message}"))
}

fn open(app: &mut App, path: &std::path::Path) {
    let doc = Document::read(path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(app, task);
}

#[cfg(unix)]
#[test]
fn the_room_is_drawn_corrected_dimensioned_and_hatched_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let dir = scratch("kabul-oda");
    // The project as the wizard makes it (the headless session's own rules, docs/adr/0245).
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("oda.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let (mut app, _) = App::boot(None);
    open(&mut app, &path);
    let (link, _task) = crate::python::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the agent link");
    app.python.link = Some(link);
    // The journal is the session's (docs/adr/0253 §3): taken before every reopening.
    let mut rows: Vec<Value> = Vec::new();
    let mut names = BTreeMap::new();
    names.insert(
        "layer".to_owned(),
        app.document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .expect("a layer"),
    );
    for (n, step) in scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .enumerate()
    {
        let name = step["name"].as_str().unwrap_or("");
        if let Some(command) = step["command"].as_str() {
            let answer = ask(
                &mut app,
                "run",
                json!({ "command": command, "op": "execute", "input": bind(&step["input"], &names) }),
            );
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            // As after any message: the sheet mode shows the drawing's book (sheet_book.rs).
            app.follow_sheets();
            match &step["as"] {
                Value::String(key) => {
                    names.insert(
                        key.clone(),
                        answer["output"]["uid"].as_str().expect("uid").to_owned(),
                    );
                }
                Value::Array(keys) => {
                    for (key, uid) in keys
                        .iter()
                        .zip(answer["output"]["created"].as_array().expect("created"))
                    {
                        names.insert(
                            key.as_str().expect("a name").to_owned(),
                            uid.as_str().expect("uid").to_owned(),
                        );
                    }
                }
                _ => {}
            }
        } else if let Some(label) = step["undo"].as_str() {
            // As Ctrl+Z: the app's own undo, which says the step it took back.
            app.undo();
            assert!(
                app.log.said(kentos_interaction::Level::Info, label)
                    || app.log.lines().any(|l| l.text.contains(label)),
                "{name}: the step “{label}” is said"
            );
        } else if step["reopen"] == true {
            rows.extend(
                ask(&mut app, "journal", json!({}))["calls"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
            let file = dir.join(format!("oda-{n}.kcad"));
            let snapshot = app
                .document
                .as_ref()
                .expect("a drawing")
                .model
                .to_snapshot_v2();
            crate::document::write(&snapshot, &file).expect("saved");
            open(&mut app, &file);
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        match step["export"].as_str() {
            // Dışa aktar › DXF: the window, Tümü, Dışa aktar; the file where the picker says.
            Some("dxf") => {
                let path = dir.join(format!("oda-{n}.dxf"));
                app.picker = crate::app::Picker::File(path.clone());
                let task = app.run("file.export.dxf");
                drive(&mut app, task);
                for event in [
                    crate::exchange::dxf_export::Event::Scope(
                        crate::exchange::dxf_export::Scope::All,
                    ),
                    crate::exchange::dxf_export::Event::Run,
                ] {
                    let task = app.update(Message::Exchange(Box::new(
                        crate::exchange::Event::DxfExport(event),
                    )));
                    drive(&mut app, task);
                }
                let bytes = std::fs::read(&path).expect("the DXF");
                problems.extend(files::dxf_problems(&bytes, &expect["dxf"]));
            }
            // The sheet mode's Dışa aktar › PDF: Hepsi, Kaydet, the file where the user chose.
            Some("pdf") => {
                use kentos_sheet_ui::message::{PdfMessage, PdfScope};
                let path = dir.join(format!("oda-{n}.pdf"));
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(
                    kentos_sheet_ui::ExportKind::Pdf,
                )));
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(
                    PdfMessage::Scope(PdfScope::All),
                )));
                app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(path.clone()));
                let bytes = std::fs::read(&path).expect("the PDF");
                problems.extend(files::pdf_problems(&bytes, &expect["pdf"]));
                // The desktop writes its maps' texts too: page 1 (1:50) shows the dimensions' 4000.
                let mut with_values = expect["pdf"].clone();
                if let Some(texts) = with_values["pages"][0]["texts"].as_array_mut() {
                    texts.push(json!("4000"));
                }
                match files::pdf_text_problems(&path, &with_values) {
                    Some(texts) => problems.extend(texts),
                    None => {
                        eprintln!("pdftotext yok: PDF'in yazıları denetlenmedi (poppler kurun).")
                    }
                }
            }
            _ => {}
        }
        if let Some(sheets) = expect.get("sheets") {
            let summary = ask(&mut app, "summary", json!({}));
            differs(
                &summary["sheets"],
                sheets,
                tolerance,
                "paftalar",
                &mut problems,
            );
            // The sheet mode shows the same book.
            let shown: Vec<&str> = app
                .sheets
                .book()
                .sheets
                .iter()
                .map(|s| s.id.as_str())
                .collect();
            let listed: Vec<&str> = sheets
                .as_array()
                .expect("sheets")
                .iter()
                .filter_map(|s| s["id"].as_str())
                .collect();
            if shown != listed {
                problems.push(format!(
                    "Pafta kipi {shown:?} gösteriyor, beklenen {listed:?}"
                ));
            }
        }
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "entity", json!({ "uid": names[key] }));
            differs(&got["entity"], want, tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "measure", json!({ "uid": names[key] }));
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(count) = expect.get("count") {
            let summary = ask(&mut app, "summary", json!({}));
            differs(
                &summary["objects"],
                count,
                0.0,
                "nesne sayısı",
                &mut problems,
            );
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    // Every command the agent ran is a line of the history.
    let runs = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter(|s| s.get("command").is_some())
        .count();
    let said = app
        .log
        .lines()
        .filter(|l| l.text.starts_with("Ajan: "))
        .count();
    assert!(said >= runs, "{said} of {runs} commands said");
    // The record says what ran, in order, and replays (docs/adr/0253 §3): on a new drawing of
    // the same project every command row done is the same and the drawing is the same.
    rows.extend(
        ask(&mut app, "journal", json!({}))["calls"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    );
    let ran: Vec<&str> = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["command"].as_str())
        .collect();
    let recorded: Vec<&str> = rows
        .iter()
        .filter(|r| r["kind"] == "command")
        .filter_map(|r| r["command"].as_str())
        .collect();
    assert_eq!(
        recorded, ran,
        "the record's commands are the scenario's, in order"
    );
    let edits: Vec<&Value> = rows.iter().filter(|r| r["kind"] == "edit").collect();
    assert!(
        edits.is_empty(),
        "every write went through a command: {edits:#?}"
    );
    let done = rows.iter().filter(|r| r["status"] == "done").count();
    let mut journal = ask(&mut app, "journal", json!({}));
    journal["calls"] = Value::Array(rows);
    let mut fresh = kentos_headless::Session::new(&choices).expect("a new project");
    let report = fresh.replay(journal).expect("replays");
    assert_eq!(report["same"], json!(done), "{report}");
    assert_eq!(
        (report["failed"].clone(), report["differs"].clone()),
        (json!(0), json!(0)),
        "{report}"
    );
    let objects = ask(&mut app, "summary", json!({}))["objects"].clone();
    assert_eq!(fresh.summary()["objects"], objects);
    // KENTOS_ACCEPTANCE_OUT=folder keeps the files the app wrote, to look at (CLAUDE.md §2).
    if let Ok(out) = std::env::var("KENTOS_ACCEPTANCE_OUT") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).expect("the folder");
        for entry in std::fs::read_dir(&dir).expect("the files").flatten() {
            let path = entry.path();
            if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("pdf" | "dxf" | "kcad")
            ) {
                std::fs::copy(
                    &path,
                    out.join(format!("masaustu-{}", entry.file_name().to_string_lossy())),
                )
                .expect("copied");
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
