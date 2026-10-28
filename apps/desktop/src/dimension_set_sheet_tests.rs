//! A dimension recipe on a sheet's maps (docs/adr/0730 §1, §7): the acceptance room's two A3 sheets
//! (fixtures/acceptance/v1/room.json: a 1:50 and a 1:100 map over the same ground) printed through the sheet mode's
//! Dışa aktar › PDF, the members' values measured on each page by poppler's `pdftotext -bbox`. A wall's recipe drawn
//! for the drawing's 1:50 only is half as high and its rows half as far apart at 1:100; with the sheet scale 1:100 too,
//! its values are as high and its rows as far from the wall and from each other on both pages.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/room.json");

/// Each word `word` on page `page` of the PDF at `path`: its height and its top, paper mm.
fn words(path: &std::path::Path, page: usize, word: &str) -> Vec<(f64, f64)> {
    let n = page.to_string();
    let run = std::process::Command::new("pdftotext")
        .args(["-bbox", "-f", &n, "-l", &n])
        .arg(path)
        .arg("-")
        .output()
        .expect("pdftotext runs");
    let html = String::from_utf8_lossy(&run.stdout);
    let pt = 25.4 / 72.0;
    let mut out = Vec::new();
    for line in html.lines().filter(|l| l.contains("<word ")) {
        let text = line.split('>').nth(1).and_then(|t| t.split('<').next());
        if text != Some(word) {
            continue;
        }
        let number = |key: &str| -> f64 {
            let at = line.find(&format!("{key}=\"")).expect("a box") + key.len() + 2;
            line[at..]
                .split('"')
                .next()
                .and_then(|v| v.parse().ok())
                .expect("a number")
        };
        out.push(((number("yMax") - number("yMin")) * pt, number("yMin") * pt));
    }
    out
}

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

/// On each page: the overall value's height, and how far below the door's value it stands (the rows' spacing), paper
/// mm.
fn rows(path: &std::path::Path) -> [(f64, f64); 2] {
    [1, 2].map(|page| {
        let overall = words(path, page, "4000.000");
        let door = words(path, page, "900.000");
        assert_eq!((overall.len(), door.len()), (1, 1), "page {page}: each value once");
        (overall[0].0, overall[0].1 - door[0].1)
    })
}

#[cfg(unix)]
#[test]
fn a_recipes_rows_read_the_same_on_paper_at_1_50_and_1_100() {
    if std::process::Command::new("pdftotext")
        .arg("-v")
        .output()
        .is_err()
    {
        eprintln!("pdftotext yok: ölçü dizisinin paftası denetlenmedi (poppler kurun).");
        return;
    }
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let dir = scratch("olcu-dizisi-pafta");
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("oda.kcad");
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
    // A 4 m wall with a 90 cm door in its middle, its south face measured 90 cm below it at the project's 1:50.
    let wall = command(
        &mut app,
        "cad.walls.create",
        json!({ "layerId": layer, "walls": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 4.0, "y": 0.0 }],
            "width": 0.2, "justify": "center" }] }),
    )["output"]["uids"][0]
        .as_str()
        .expect("the wall")
        .to_owned();
    command(
        &mut app,
        "cad.openings.create",
        json!({ "layerId": layer, "openings": [{ "host": wall, "at": 2.0, "width": 0.9 }] }),
    );
    let set = command(
        &mut app,
        "cad.dimensionsets.create",
        json!({ "layerId": layer, "wall": wall, "face": "right", "at": { "x": 2.0, "y": -1.0 } }),
    )["output"]["set"]
        .as_str()
        .expect("the recipe")
        .to_owned();
    let sheets = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["command"] == "cad.sheets.edit")
        .expect("the sheets' step");
    command(&mut app, "cad.sheets.edit", sheets["input"].clone());

    // Drawn for 1:50 only: half as high and half as far apart at 1:100.
    let plain = dir.join("tek-olcek.pdf");
    print(&mut app, &plain);
    let [(h50, gap50), (h100, gap100)] = rows(&plain);
    assert!((h100 - h50 / 2.0).abs() < 0.05 * h50, "one height: {h50} at 1:50, {h100} at 1:100");
    assert!((gap100 - gap50 / 2.0).abs() < 0.3, "one spacing: {gap50} mm at 1:50, {gap100} mm at 1:100");
    // 7.5 mm between the rows' lines on the 1:50 page.
    assert!((gap50 - 7.5).abs() < 0.3, "{gap50} mm");

    // With the sheet scale 1:100: the same on both pages.
    command(
        &mut app,
        "cad.dimensionsets.edit",
        json!({ "id": set, "operation": { "kind": "set", "scales": [100.0] } }),
    );
    let scaled = dir.join("pafta-olcekleri.pdf");
    print(&mut app, &scaled);
    let [(s50, sgap50), (s100, sgap100)] = rows(&scaled);
    assert!((s50 - h50).abs() < 0.02 * h50, "1:50 as it was: {s50}, {h50}");
    assert!((sgap50 - gap50).abs() < 0.05, "1:50 as it was: {sgap50}, {gap50}");
    assert!((s100 - s50).abs() < 0.05 * s50, "as high on paper: {s50} at 1:50, {s100} at 1:100");
    assert!((sgap100 - sgap50).abs() < 0.3, "as far apart: {sgap50} mm at 1:50, {sgap100} mm at 1:100");
}
