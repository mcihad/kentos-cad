//! Kapı ve pencere ölçeğe göre (docs/adr/3010): the same storey on 1:200, 1:100 and 1:50 sheets, written as PDF and read
//! back. A door is its leaf at 1:200, its leaf and swing at 1:100, with its frame's sections at 1:50; a window its glass
//! at 1:200, its frame's faces and glass at 1:100, with its frame's sections at 1:50. Before, every scale printed every
//! piece.
//!
//! `KENTOS_SCALE_PDF_DIR=dir` keeps the three PDFs there.

use kentos_commands::{CadWork, DrawingFont, DrawingUnit, Workspace};
use kentos_headless::{NewDrawing, Session};
use serde_json::{Value, json};

const PT_PER_MM: f64 = 72.0 / 25.4;

fn done(answer: Value) -> Value {
    assert!(answer["error"].is_null(), "{answer}");
    answer["output"].clone()
}

/// The stroked paths of `pdf`'s first page at `mm` (±0.01).
fn at_width(pdf: &[u8], mm: f64) -> usize {
    let page = kentos_formats::pdf::read_page(pdf, 1).expect("its first page");
    page.paths
        .iter()
        .filter_map(|p| p.stroke.as_ref())
        .filter(|s| (s.width / PT_PER_MM - mm).abs() < 0.01)
        .count()
}

#[test]
fn a_door_and_a_window_print_by_their_maps_scale() {
    let mut s = Session::new(&NewDrawing {
        name: "Ölçek".into(),
        srid: 0,
        plot_scale: 100.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(DrawingUnit::M),
        annotation: None,
        cad_work: Some(CadWork::Architecture),
        heights: None,
    })
    .expect("a new building");
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let walls = done(
        s.call(json!({ "command": "cad.walls.rectangle", "input": {
            "layerId": layer,
            "frame": { "kind": "corners", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 5.0, "y": 4.0 } },
            "width": 0.25, "lineWeight": 0.5,
            "arch": { "level": "zemin" }
        } }))
        .expect("answers"),
    );
    let host = walls["uids"][0].as_str().expect("a wall").to_owned();
    for (form, at) in [("door", 1.5), ("window", 3.5)] {
        done(
            s.call(json!({ "command": "cad.openings.create", "input": {
                "layerId": layer,
                "openings": [{ "host": host, "at": at, "form": form }],
                "lineWeight": 0.25
            } }))
            .expect("answers"),
        );
    }
    let dir = std::env::temp_dir().join(format!("kentos-olcek-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut pdfs = Vec::new();
    for (scale, first) in [(200, "A-101"), (100, "A-102"), (50, "A-103")] {
        let made = done(
            s.call(json!({ "command": "cad.sheets.levels", "input": {
                "options": { "scale": scale }, "first": first, "again": true
            } }))
            .expect("answers"),
        );
        // The book's sheets after the change; this scale's is the one numbered `first`.
        let id = made["sheets"]
            .as_array()
            .and_then(|all| {
                all.iter()
                    .find(|sheet| sheet["name"].as_str().is_some_and(|n| n.starts_with(first)))
            })
            .and_then(|sheet| sheet["id"].as_str())
            .expect("its sheet")
            .to_owned();
        let pdf = dir.join(format!("olcek-1-{scale}.pdf"));
        s.export_pdf(&pdf, &[id]).expect("the PDF");
        if let Ok(keep) = std::env::var("KENTOS_SCALE_PDF_DIR") {
            std::fs::copy(
                &pdf,
                std::path::Path::new(&keep).join(format!("olcek-1-{scale}.pdf")),
            )
            .expect("kept");
        }
        pdfs.push((scale, std::fs::read(&pdf).expect("written")));
    }
    // The openings draw at 0.25 mm (frame, leaf, a window's faces) and 0.125 mm (swing, glass); the sheets are the
    // same, so the differences between them are the openings' pieces.
    let whole: Vec<usize> = pdfs.iter().map(|(_, b)| at_width(b, 0.25)).collect();
    let half: Vec<usize> = pdfs.iter().map(|(_, b)| at_width(b, 0.125)).collect();
    // 1:50 adds four frame sections to 1:100; 1:100 adds the window's two faces to 1:200.
    assert_eq!(
        whole[2] - whole[1],
        4,
        "1:50 against 1:100, 0.25 mm: {whole:?}"
    );
    assert_eq!(
        whole[1] - whole[0],
        2,
        "1:100 against 1:200, 0.25 mm: {whole:?}"
    );
    // The swing goes beyond 1:100; the glass stays.
    assert_eq!(
        (half[2], half[1]),
        (half[1], half[0] + 1),
        "0.125 mm: {half:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
