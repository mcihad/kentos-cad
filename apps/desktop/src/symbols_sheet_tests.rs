//! Mimari işaretler on paper (docs/adr/0721; plan/architecture.md ARCH-DOC-01's acceptance): two A3 sheets numbered
//! A-101 and A-102; a section mark on A-101's plan frame reads A-101, renumbered A-201 it reads that, and so does a copy
//! of it; a Kot bound to 1. kat, the north arrow, a revision with its Revizyon listesi and the material legend beside.
//! Printed through the sheet
//! mode's Dışa aktar › PDF, poppler's `pdftotext` finds the sheet's number in the mark, the revision's number and the
//! legend's words on the page and `pdffonts` says every font is embedded; A-101 taken away, both marks are broken and
//! Denetim names them.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const MARKS: &str = include_str!("../../../fixtures/commands/v1/cad.viewmarks.create.json");

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    app.follow_sheets();
    answer
}

fn print(app: &mut App, path: &std::path::Path) {
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(
        PdfMessage::Scope(PdfScope::All),
    )));
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(path.to_path_buf()));
}

/// The page's words, as poppler reads them.
fn text_of(path: &std::path::Path, page: usize) -> String {
    let n = page.to_string();
    let run = std::process::Command::new("pdftotext")
        .args(["-f", &n, "-l", &n])
        .arg(path)
        .arg("-")
        .output()
        .expect("pdftotext runs");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// Whether every font `pdffonts` lists is embedded: its `emb` column, found by the dashes under the header.
fn embedded(listed: &str) -> bool {
    let mut lines = listed.lines();
    let (Some(_), Some(dashes)) = (lines.next(), lines.next()) else {
        return false;
    };
    // The fourth run of dashes is `emb`'s.
    let mut starts = Vec::new();
    let mut prev = ' ';
    for (i, c) in dashes.char_indices() {
        if c == '-' && prev == ' ' {
            starts.push(i);
        }
        prev = c;
    }
    let Some(&at) = starts.get(3) else {
        return false;
    };
    let rows: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
    !rows.is_empty() && rows.iter().all(|r| r.get(at..at + 3) == Some("yes"))
}

/// The object `uid` as its JSON.
fn object(app: &App, uid: &str) -> Value {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slot = doc.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid")).expect("in the drawing");
    serde_json::to_value(doc.get(slot).expect("the object")).expect("its JSON")
}

#[cfg(unix)]
#[test]
fn marks_read_their_sheets_numbers_print_with_embedded_fonts_and_break_when_the_sheet_goes() {
    if std::process::Command::new("pdftotext").arg("-v").output().is_err() {
        eprintln!("pdftotext yok: işaretlerin paftası denetlenmedi (poppler kurun).");
        return;
    }
    let dir = scratch("isaretler-pafta");
    let choices = kentos_headless::new_drawing(&json!({
        "name": "İşaretler", "workspace": "cad", "coords": "local", "drawingUnit": "m", "plotScale": 100
    }))
    .expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("isaretler.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let (mut app, _) = App::boot(None);
    let doc = Document::read(&path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");

    // Two levels; an inner wall of brick and plaster (KentOS's İç duvar 15).
    command(&mut app, "cad.settings.set", json!({ "arch": { "levels": [
        { "id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.25 },
        { "id": "k1", "name": "1. kat", "elevation": 3.25, "height": 3.25 } ] } }));
    command(&mut app, "cad.walls.create", json!({ "layerId": layer, "walls": [{
        "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 8.0, "y": 0.0 }], "style": "İç duvar 15" }] }));
    // Two sheets: the shared cases' A3 with its plan and section frames (9 m by 6 m about (4, 1.5) at 1:50), and a
    // copy of it.
    let marks: Value = serde_json::from_str(MARKS).expect("the cases");
    let sheet = marks["cases"][0]["steps"][0]["input"]["ops"][0]["sheet"].clone();
    assert_eq!(sheet["id"], "pafta-1", "the shared cases' sheet");
    let mut second = sheet.clone();
    second["id"] = json!("pafta-2");
    second["name"] = json!("A-102");
    for (k, item) in second["items"].as_array_mut().expect("items").iter_mut().enumerate() {
        item["id"] = json!(format!("p2-harita-{k}"));
    }
    command(&mut app, "cad.sheets.edit", json!({ "ops": [
        { "op": "addSheet", "sheet": sheet }, { "op": "addSheet", "sheet": second } ] }));
    command(&mut app, "cad.sheets.edit", json!({ "ops": [
        { "op": "setDrawingInfo", "id": "pafta-1", "info": { "number": "A-101" } },
        { "op": "setDrawingInfo", "id": "pafta-2", "info": { "number": "A-102" } } ] }));

    // A section mark on A-101's section frame reads A-101.
    let made = command(&mut app, "cad.viewmarks.create", json!({ "layerId": layer, "marks": [{
        "form": "section", "points": [{ "x": 1.0, "y": 1.2 }, { "x": 7.0, "y": 1.2 }], "mark": "1",
        "target": { "sheet": "pafta-1", "item": "harita-2" } }] }));
    let mark = made["output"]["uids"][0].as_str().expect("the mark's id").to_owned();
    assert_eq!(object(&app, &mark)["words"]["bottom"], "A-101", "{}", object(&app, &mark));
    // Renumbered A-201: the mark reads it; a copy of the mark reads it too.
    command(&mut app, "cad.sheets.edit", json!({ "ops": [
        { "op": "setDrawingInfo", "id": "pafta-1", "info": { "number": "A-201" } } ] }));
    assert_eq!(object(&app, &mark)["words"]["bottom"], "A-201");
    let copied = command(&mut app, "cad.entities.transform", json!({ "uids": [mark],
        "transform": { "kind": "move", "dx": 0.0, "dy": 0.8 }, "copy": true }));
    let copy = copied["output"]["created"][0].as_str().expect("the copy's id").to_owned();
    assert_eq!(object(&app, &copy)["words"]["bottom"], "A-201", "{}", object(&app, &copy));

    // A Kot bound to 1. kat, the north arrow, a revision and the material legend.
    command(&mut app, "cad.dimensions.create", json!({ "layerId": layer, "at": { "x": 4.8, "y": 3.3 },
        "measure": { "kind": "elevation", "a": { "kind": "at", "at": { "x": 4.5, "y": 3.0 } }, "level": { "id": "k1" } } }));
    command(&mut app, "cad.north.create", json!({ "at": { "x": 7.6, "y": 3.6 }, "layerId": layer }));
    command(&mut app, "cad.revisions.mark", json!({ "layerId": layer,
        "frame": { "kind": "rectangle", "a": { "x": 3.0, "y": -1.2 }, "b": { "x": 5.0, "y": -0.3 } },
        "arc": 0.3, "number": "R7", "date": "2026-10-27", "description": "Duvar uzatıldı" }));
    // Revizyon listesi's table (as `arch.revisionList` places it): its rows the document's.
    let list = command(&mut app, "cad.entities.create", json!({ "layerId": layer, "operation": "table", "objects": [{
        "geometry": { "kind": "table", "p": { "x": 5.3, "y": -0.45 }, "rotation": 0.0, "height": 0.125,
            "rows": [0.5], "columns": [1.0], "cells": [["x"]], "header": true,
            "source": { "kind": "blocks", "names": ["Revizyon"], "by": "REV", "columns": ["Tarih", "Açıklama"] } } }] }));
    let list = list["output"]["created"][0].as_str().expect("the list's id").to_owned();
    assert_eq!(
        object(&app, &list)["cells"],
        json!([["REV", "Tarih", "Açıklama", "Adet"], ["R7", "2026-10-27", "Duvar uzatıldı", "1"], ["Toplam", "", "", "1"]])
    );
    let legend = command(&mut app, "cad.legends.materials", json!({ "at": { "x": -0.4, "y": 4.4 }, "layerId": layer }));
    let names: Vec<&str> = legend["output"]["materials"].as_array().expect("materials").iter()
        .filter_map(|m| m["name"].as_str()).collect();
    assert_eq!(names, ["Sıva", "Tuğla"]);

    // Printed: the marks' sheet number, the revision's, the Kot's value and the legend's words on A-101; every font
    // embedded.
    let pdf = dir.join("isaretler.pdf");
    print(&mut app, &pdf);
    let page = text_of(&pdf, 1);
    for word in ["A-201", "R7", "3.25", "Malzeme", "lejandı", "Tuğla", "Sıva", "2026-10-27", "uzatıldı"] {
        assert!(page.contains(word), "{word} on the page: {page}");
    }
    assert!(page.matches("A-201").count() >= 2, "both marks: {page}");
    let fonts = std::process::Command::new("pdffonts").arg(&pdf).output().expect("pdffonts runs");
    let listed = String::from_utf8_lossy(&fonts.stdout).into_owned();
    assert!(embedded(&listed), "every font embedded: {listed}");

    // A-101 taken away: both marks are broken, Denetim names them.
    command(&mut app, "cad.sheets.edit", json!({ "ops": [{ "op": "removeSheet", "id": "pafta-1" }] }));
    for uid in [&mark, &copy] {
        assert_eq!(object(&app, uid)["broken"], "targetGone", "{}", object(&app, uid));
    }
    let audit = command(&mut app, "cad.arch.audit", json!({}));
    let gone: Vec<&str> = audit["output"]["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|f| f["code"] == "arch_viewmark_target_gone")
        .flat_map(|f| f["uids"].as_array().into_iter().flatten().filter_map(Value::as_str))
        .collect();
    assert!(gone.contains(&mark.as_str()) && gone.contains(&copy.as_str()), "{audit}");
}
