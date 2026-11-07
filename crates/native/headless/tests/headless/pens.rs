//! Kalem hiyerarşisi on paper (docs/adr/3009): a building's walls, a door and a window on a 1:50 sheet, written as PDF
//! and read back. What is cut (the walls' faces, wide) is drawn at the walls' pen; the door's frame and leaf at the
//! door's; its swing, the window's glass and the boundaries inside a wall at half (ISO 128-23's narrow line, Table 2's
//! 1:2). Before, every piece of an opening had the opening's one weight.

use kentos_commands::{CadWork, DrawingFont, DrawingUnit, Workspace};
use kentos_headless::{NewDrawing, Session};
use serde_json::{Value, json};

/// One point in millimetres (a PDF's unit is 1/72 inch).
const PT_PER_MM: f64 = 72.0 / 25.4;

fn done(answer: Value) -> Value {
    assert!(answer["error"].is_null(), "{answer}");
    answer["output"].clone()
}

#[test]
fn a_1_50_sheet_draws_cut_walls_wide_doors_narrow_and_their_swings_at_half() {
    let mut s = Session::new(&NewDrawing {
        name: "Kalem".into(),
        srid: 0,
        plot_scale: 50.0,
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
    // Walls at 0.5 mm, a door and a window at 0.25 mm, on Zemin kat.
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
    done(
        s.call(json!({ "command": "cad.sheets.levels", "input": { "options": { "scale": 50 } } }))
            .expect("answers"),
    );
    let dir = std::env::temp_dir().join(format!("kentos-kalem-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let pdf = dir.join("kalem-1-50.pdf");
    s.export_pdf(&pdf, &[]).expect("the PDF");
    let bytes = std::fs::read(&pdf).expect("written");
    if let Ok(keep) = std::env::var("KENTOS_PEN_PDF") {
        std::fs::copy(&pdf, keep).expect("kept");
    }
    let page = kentos_formats::pdf::read_page(&bytes, 1).expect("its first page");
    let mm: Vec<f64> = page
        .paths
        .iter()
        .filter_map(|p| p.stroke.as_ref())
        .map(|s| (s.width / PT_PER_MM * 1000.0).round() / 1000.0)
        .collect();
    let has = |w: f64| mm.iter().any(|m| (m - w).abs() < 0.01);
    assert!(has(0.5), "the walls' faces, wide: {mm:?}");
    assert!(has(0.25), "the door's frame and leaf: {mm:?}");
    assert!(
        has(0.125),
        "the door's swing and the window's glass, half the door's: {mm:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
