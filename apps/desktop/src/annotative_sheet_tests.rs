//! Ölçekli açıklamalar on a sheet's maps (docs/adr/0323 §2; plan/cad.md CAD-ANNO-13): the acceptance room's two A3
//! sheets (fixtures/acceptance/v1/room.json: a 1:50 and a 1:100 map over the same ground) printed through the sheet
//! mode's Dışa aktar › PDF, the dimension's value measured on each page by poppler's `pdftotext -bbox`. One height
//! (the dimension made for 1:50) is half as high at 1:100, too small to be written there; annotative with 1:100 too,
//! it is as high on both, and
//! setting its 1:100 place leaves the 1:50 page as it was.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/room.json");

/// The height and the left of each word `word` on page `page` of the PDF at `path`, paper mm.
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
        out.push(((number("yMax") - number("yMin")) * pt, number("xMin") * pt));
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

/// The dimension value's height on the 1:50 and the 1:100 page, and its left on the 1:100 page.
fn heights(path: &std::path::Path) -> (f64, f64, f64) {
    let (a, b) = (words(path, 1, "4000.000"), words(path, 2, "4000.000"));
    assert_eq!((a.len(), b.len()), (1, 1), "the value once on each page");
    (a[0].0, b[0].0, b[0].1)
}

#[cfg(unix)]
#[test]
fn an_annotative_dimension_reads_the_same_on_paper_at_1_50_and_1_100() {
    if std::process::Command::new("pdftotext")
        .arg("-v")
        .output()
        .is_err()
    {
        eprintln!("pdftotext yok: ölçekli açıklamanın paftası denetlenmedi (poppler kurun).");
        return;
    }
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let dir = scratch("olcekli-pafta");
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
    // A 4000 mm dimension at 2.5 mm on paper at the project's 1:50.
    let made = command(
        &mut app,
        "cad.entities.create",
        json!({ "layerId": layer, "objects": [{ "geometry": {
            "kind": "dimension", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 4.0, "y": 0.0 },
            "offset": -0.6, "height": 0.125 } }] }),
    );
    let uid = made["output"]["created"][0]
        .as_str()
        .expect("its id")
        .to_owned();
    let sheets = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["command"] == "cad.sheets.edit")
        .expect("the sheets' step");
    command(&mut app, "cad.sheets.edit", sheets["input"].clone());

    // One height: 2.5 mm at 1:50, 1.25 mm at 1:100 (where the drawing's smallest texts are not written).
    let one = dir.join("tek.pdf");
    print(&mut app, &one);
    let h50 = words(&one, 1, "4000.000")
        .first()
        .expect("the value at 1:50")
        .0;
    let small = words(&one, 2, "4000.000");
    assert!(
        small.iter().all(|w| (w.0 - h50 / 2.0).abs() < 0.05 * h50),
        "one height at 1:100: {small:?}"
    );

    command(
        &mut app,
        "cad.annotations.scales",
        json!({ "operation": "make", "uids": [uid], "scales": [100.0] }),
    );
    let scaled = dir.join("olcekli.pdf");
    print(&mut app, &scaled);
    let (s50, s100, left) = heights(&scaled);
    assert!(
        (s50 - h50).abs() < 0.02 * h50,
        "1:50 as it was: {s50}, {h50}"
    );
    assert!(
        (s50 - s100).abs() < 0.05 * s50,
        "the same on paper: {s50} at 1:50, {s100} at 1:100"
    );

    // The 1:100 value moved 1 m along: only the 1:100 page changes.
    command(
        &mut app,
        "cad.annotations.scales",
        json!({ "operation": "set", "uids": [uid], "scale": 100.0, "textShift": { "along": 1.0, "across": 0.0 } }),
    );
    let moved = dir.join("yeri.pdf");
    print(&mut app, &moved);
    let (m50, m100, moved_left) = heights(&moved);
    assert!((m50 - s50).abs() < 1e-6 && (m100 - s100).abs() < 1e-6);
    assert_eq!(
        words(&moved, 1, "4000.000"),
        words(&scaled, 1, "4000.000"),
        "the 1:50 page as it was"
    );
    // 1 m at 1:100 is 10 mm on paper.
    assert!(
        (moved_left - left - 10.0).abs() < 0.5,
        "moved {} mm",
        moved_left - left
    );
}
