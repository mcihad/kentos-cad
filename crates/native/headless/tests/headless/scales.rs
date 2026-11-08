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

/// The library's double bed (`catalog/v1/blocks/kentos-temel.kblok`, Mobilya/Yatak): its outline and two pillows.
fn bed_library() -> String {
    let library: Value = serde_json::from_str(include_str!(
        "../../../../../catalog/v1/blocks/kentos-temel.kblok"
    ))
    .expect("the library");
    let bed: Vec<Value> = library["blocks"]
        .as_array()
        .expect("its blocks")
        .iter()
        .filter(|b| b["name"] == "Çift kişilik yatak")
        .cloned()
        .collect();
    json!({ "format": "kentos.blocks", "version": 1, "blocks": bed }).to_string()
}

/// Kolon, merdiven, rampa ve mobilya ölçeğe göre (docs/adr/3011): a tagged column, a numbered stair with railings, a
/// ramp with railings and the library's double bed, the same storey on 1:200, 1:100 and 1:50 sheets, written as PDF
/// and read back.
/// - 1:200: the column's outline, the stair's band, treads, cut and arrow, the ramp's band, arrow, slope and levels,
///   the bed's outline;
/// - 1:100: and the column's name, the railings, the treads' numbers, the bed's pillows;
/// - 1:50: and the column's size and the stair's risers and treads in words.
///
/// `KENTOS_SCALE_PDF_DIR=dir` keeps the three PDFs there (`parca-1-<scale>.pdf`).
#[test]
fn columns_stairs_ramps_and_furniture_print_by_their_maps_scale() {
    let mut s = Session::new(&NewDrawing {
        name: "Ölçek 2".into(),
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
    let call = |s: &mut Session, command: &str, input: Value| {
        done(
            s.call(json!({ "command": command, "input": input }))
                .expect("answers"),
        )
    };
    call(
        &mut s,
        "cad.columns.create",
        json!({ "layerId": layer,
        "columns": [{ "at": { "x": 1.0, "y": 4.0 }, "width": 0.3, "depth": 0.5, "tag": "S1" }] }),
    );
    call(
        &mut s,
        "cad.stairs.create",
        json!({ "layerId": layer,
        "stairs": [{ "pts": [{ "x": 3.0, "y": 4.0 }, { "x": 5.8, "y": 4.0 }], "width": 1.2, "risers": 11,
            "riserHeight": 0.175, "tread": 0.28, "numbering": true, "railings": "both", "cut": 6 }] }),
    );
    call(
        &mut s,
        "cad.ramps.create",
        json!({ "layerId": layer,
        "ramps": [{ "pts": [{ "x": 1.0, "y": 1.0 }, { "x": 7.0, "y": 1.0 }], "width": 1.2, "rise": 0.36,
            "slope": 6.0, "railings": "both" }] }),
    );
    call(
        &mut s,
        "cad.blocks.edit",
        json!({ "operation": "import", "library": bed_library() }),
    );
    call(
        &mut s,
        "cad.blocks.insert",
        json!({ "block": "0192f5b0-0000-7000-8000-000000000007",
        "at": { "x": 7.5, "y": 3.0 }, "layerId": layer }),
    );
    let dir = std::env::temp_dir().join(format!("kentos-olcek-parca-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut pages = Vec::new();
    for (scale, first) in [(200, "A-101"), (100, "A-102"), (50, "A-103")] {
        let made = done(
            s.call(json!({ "command": "cad.sheets.levels", "input": {
                "options": { "scale": scale }, "first": first, "again": true
            } }))
            .expect("answers"),
        );
        let id = made["sheets"]
            .as_array()
            .and_then(|all| {
                all.iter()
                    .find(|sheet| sheet["name"].as_str().is_some_and(|n| n.starts_with(first)))
            })
            .and_then(|sheet| sheet["id"].as_str())
            .expect("its sheet")
            .to_owned();
        let pdf = dir.join(format!("parca-1-{scale}.pdf"));
        s.export_pdf(&pdf, &[id]).expect("the PDF");
        if let Ok(keep) = std::env::var("KENTOS_SCALE_PDF_DIR") {
            std::fs::copy(
                &pdf,
                std::path::Path::new(&keep).join(format!("parca-1-{scale}.pdf")),
            )
            .expect("kept");
        }
        let bytes = std::fs::read(&pdf).expect("written");
        pages.push(kentos_formats::pdf::read_page(&bytes, 1).expect("its first page"));
    }
    let texts: Vec<Vec<String>> = pages
        .iter()
        .map(|p| p.texts.iter().map(|t| t.text.trim().to_owned()).collect())
        .collect();
    let has = |k: usize, w: &str| texts[k].iter().any(|t| t == w);
    let strokes: Vec<usize> = pages
        .iter()
        .map(|p| p.paths.iter().filter(|q| q.stroke.is_some()).count())
        .collect();
    eprintln!("strokes {strokes:?}");
    let words = "11 × 17.5/28";
    // The column's name from 1:100, its size from 1:50; the treads' numbers from 1:100; the stair in words from 1:50.
    assert!(!has(0, "S1") && has(1, "S1") && has(2, "S1"), "{texts:?}");
    assert!(!has(1, "30/50") && has(2, "30/50"), "{texts:?}");
    assert!(
        !has(0, "1") && has(1, "1") && has(1, "6") && has(2, "6"),
        "{texts:?}"
    );
    assert!(!has(1, words) && has(2, words), "{texts:?}");
    // The ramp's slope and levels at every scale.
    assert!(
        (0..3).all(|k| has(k, "%6.00") && has(k, "+0.36")),
        "{texts:?}"
    );
    // 1:100 adds the stair's railings, the ramp's and the bed's two pillows to 1:200; 1:50 no line more.
    assert_eq!(strokes[1] - strokes[0], 8, "{strokes:?}");
    assert_eq!(strokes[2], strokes[1], "{strokes:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Kesme düzleminin üstündeki pencere (docs/adr/3012): on one wall a window across the plan's cut (sill 0.9) and a
/// bathroom's high window over it (sill 1.6), on a 1:50 sheet written as PDF and read back. The low window cuts its
/// wall and is drawn solid; the high one leaves its wall whole and is drawn dashed: the wall's faces run through it.
/// Before, both cut their wall and were drawn solid.
///
/// `KENTOS_SCALE_PDF_DIR=dir` keeps the PDF there (`kesme-1-50.pdf`).
#[test]
fn a_window_over_the_cut_leaves_its_wall_whole_on_paper() {
    let mut s = Session::new(&NewDrawing {
        name: "Kesme".into(),
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
    for (at, sill) in [(1.5, 0.9), (3.5, 1.6)] {
        done(
            s.call(json!({ "command": "cad.openings.create", "input": {
                "layerId": layer,
                "openings": [{ "host": host, "at": at, "form": "window", "width": 1.0, "sill": sill, "head": 2.2 }],
                "lineWeight": 0.25
            } }))
            .expect("answers"),
        );
    }
    done(
        s.call(json!({ "command": "cad.sheets.levels", "input": { "options": { "scale": 50 } } }))
            .expect("answers"),
    );
    let dir = std::env::temp_dir().join(format!("kentos-kesme-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let pdf = dir.join("kesme-1-50.pdf");
    s.export_pdf(&pdf, &[]).expect("the PDF");
    if let Ok(keep) = std::env::var("KENTOS_SCALE_PDF_DIR") {
        std::fs::copy(&pdf, std::path::Path::new(&keep).join("kesme-1-50.pdf")).expect("kept");
        // The drawing too, for the real window (fixtures/interaction/real/kesme-yuksekligi.txt).
        s.save(Some(&std::path::Path::new(&keep).join("kesme.kcad")))
            .expect("kept");
    }
    let page = kentos_formats::pdf::read_page(&std::fs::read(&pdf).expect("written"), 1)
        .expect("its first page");
    let strokes: Vec<(f64, bool)> = page
        .paths
        .iter()
        .filter_map(|p| p.stroke.as_ref())
        .map(|s| ((s.width / PT_PER_MM * 1000.0).round() / 1000.0, s.dashed))
        .collect();
    let count = |mm: f64, dashed: bool| {
        strokes
            .iter()
            .filter(|(w, d)| (w - mm).abs() < 0.01 && *d == dashed)
            .count()
    };
    eprintln!(
        "walls 0.5 mm: {} solid; windows 0.25 mm: {} solid, {} dashed; 0.125 mm: {} solid, {} dashed",
        count(0.5, false),
        count(0.25, false),
        count(0.25, true),
        count(0.125, false),
        count(0.125, true)
    );
    // The high window's parts dashed at its pen and half its pen; the low one's solid.
    assert!(
        count(0.25, true) >= 2 && count(0.125, true) >= 1,
        "{strokes:?}"
    );
    assert!(
        count(0.25, false) >= 2 && count(0.125, false) >= 1,
        "{strokes:?}"
    );
    let low_cut = (count(0.5, false), count(0.25, false), count(0.25, true));
    // The project's cut raised to 1.80 m (docs/adr/3012, step 2): the 1.60 sill is under it, both windows are cut.
    let mut arch = s
        .document()
        .settings()
        .arch
        .clone()
        .expect("a Mimari project");
    assert_eq!(
        arch.cut_height,
        Some(1.2),
        "a new Mimari project cuts at 1.20"
    );
    arch.cut_height = Some(1.8);
    done(
        s.call(json!({ "command": "cad.settings.set", "input": { "arch": arch } }))
            .expect("answers"),
    );
    let high = dir.join("kesme-1-80.pdf");
    s.export_pdf(&high, &[]).expect("the PDF");
    if let Ok(keep) = std::env::var("KENTOS_SCALE_PDF_DIR") {
        std::fs::copy(&high, std::path::Path::new(&keep).join("kesme-1-80.pdf")).expect("kept");
    }
    let page = kentos_formats::pdf::read_page(&std::fs::read(&high).expect("written"), 1)
        .expect("its first page");
    let strokes: Vec<(f64, bool)> = page
        .paths
        .iter()
        .filter_map(|p| p.stroke.as_ref())
        .map(|s| ((s.width / PT_PER_MM * 1000.0).round() / 1000.0, s.dashed))
        .collect();
    let count = |mm: f64, dashed: bool| {
        strokes
            .iter()
            .filter(|(w, d)| (w - mm).abs() < 0.01 && *d == dashed)
            .count()
    };
    eprintln!(
        "cut 1.80: walls 0.5 mm: {} solid; windows 0.25 mm: {} solid, {} dashed (cut 1.20: {low_cut:?})",
        count(0.5, false),
        count(0.25, false),
        count(0.25, true)
    );
    assert_eq!(count(0.25, true) + count(0.125, true), 0, "{strokes:?}");
    assert!(
        count(0.5, false) > low_cut.0,
        "the wall is cut twice: {strokes:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
