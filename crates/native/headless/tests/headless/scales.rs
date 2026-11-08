//! Kapı ve pencere ölçeğe göre (docs/adr/3010): the same storey on 1:200, 1:100 and 1:50 sheets, written as PDF and read
//! back. A door is its leaf at 1:200, its leaf and swing at 1:100, with its frame's sections at 1:50; a window its glass
//! at 1:200, its frame's faces and glass at 1:100, with its frame's sections at 1:50. Before, every scale printed every
//! piece.
//!
//! `KENTOS_SCALE_PDF_DIR=dir` keeps the three PDFs there.
//!
//! The plan's cut (docs/adr/3012): a window over it is dashed and its wall whole; a slab of the level above is dashed on
//! the plan below it.

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
    // Its pieces by the scale, every object at its own pen (the line groups are their own test, docs/adr/3013).
    own_pens(&mut s);
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

/// Every object at its own pen whatever the scale (docs/adr/3013): the project's line groups set to nothing.
fn own_pens(s: &mut Session) {
    set_line_groups(s, None);
}

/// The project's line groups (docs/adr/3013) through `cad.settings.set`.
fn set_line_groups(s: &mut Session, groups: Option<kentos_commands::LineGroups>) {
    let mut arch = s
        .document()
        .settings()
        .arch
        .clone()
        .expect("a Mimari project");
    arch.line_groups = groups;
    done(
        s.call(json!({ "command": "cad.settings.set", "input": { "arch": arch } }))
            .expect("answers"),
    );
}

/// The stroked paths of `pdf`'s first page by their width, in thousandths of a mm.
fn widths(pdf: &[u8]) -> std::collections::BTreeMap<u32, usize> {
    let page = kentos_formats::pdf::read_page(pdf, 1).expect("its first page");
    let mut out = std::collections::BTreeMap::new();
    for s in page.paths.iter().filter_map(|p| p.stroke.as_ref()) {
        *out.entry((s.width / PT_PER_MM * 1000.0).round() as u32)
            .or_insert(0) += 1;
    }
    out
}

/// docs/adr/3013: the same storey on 1:200, 1:100, 1:50 and 1:20 sheets of a new Mimari project, each printed with
/// its line groups and with every object at its own pen. The walls' 0.5 mm goes to the group's wide line (0.25, 0.35,
/// 0.5, 0.7: TS 88-23 / ISO 128-23 Tablo 2), the openings' 0.25 to its narrow one; the sheet's frame and title block
/// stay as they are. `KENTOS_SCALE_PDF_DIR=dir` keeps the PDFs there (`grup-1-<scale>.pdf`, `kendi-1-<scale>.pdf`).
#[test]
fn the_lines_print_by_the_line_group_of_their_maps_scale() {
    let mut s = Session::new(&NewDrawing {
        name: "Çizgi grubu".into(),
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
    assert_eq!(
        s.document()
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.line_groups),
        Some(kentos_commands::LineGroups::ByScale),
        "a new Mimari project weighs its lines by the scale"
    );
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
    let mut objects = vec![host.clone()];
    for (form, at) in [("door", 1.5), ("window", 3.5)] {
        let made = done(
            s.call(json!({ "command": "cad.openings.create", "input": {
                "layerId": layer,
                "openings": [{ "host": host, "at": at, "form": form }],
                "lineWeight": 0.25
            } }))
            .expect("answers"),
        );
        objects.push(made["uids"][0].as_str().expect("an opening").to_owned());
    }
    let dir = std::env::temp_dir().join(format!("kentos-grup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let keep = std::env::var("KENTOS_SCALE_PDF_DIR").ok();
    let mut sheets = Vec::new();
    for (scale, first) in [(200, "A-101"), (100, "A-102"), (50, "A-103"), (20, "A-104")] {
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
        sheets.push((scale, id));
    }
    if let Some(keep) = &keep {
        // The drawing with its four sheets, for the real window (fixtures/interaction/real/cizgi-grubu.txt).
        s.save(Some(&std::path::Path::new(keep).join("cizgi-grubu.kcad")))
            .expect("kept");
    }
    let print = |s: &mut Session, name: &str, scale: u32, id: &str| {
        let pdf = dir.join(format!("{name}-1-{scale}.pdf"));
        s.export_pdf(&pdf, &[id.to_owned()]).expect("the PDF");
        if let Some(keep) = &keep {
            std::fs::copy(
                &pdf,
                std::path::Path::new(keep).join(format!("{name}-1-{scale}.pdf")),
            )
            .expect("kept");
        }
        widths(&std::fs::read(&pdf).expect("written"))
    };
    let grouped: Vec<_> = sheets
        .iter()
        .map(|(scale, id)| print(&mut s, "grup", *scale, id))
        .collect();
    own_pens(&mut s);
    let own: Vec<_> = sheets
        .iter()
        .map(|(scale, id)| print(&mut s, "kendi", *scale, id))
        .collect();
    // The sheets without the building: their frames and title blocks alone, taken from both.
    done(
        s.call(json!({ "command": "cad.entities.delete", "input": { "uids": objects } }))
            .expect("answers"),
    );
    let frames: Vec<_> = sheets
        .iter()
        .map(|(scale, id)| print(&mut s, "cerceve", *scale, id))
        .collect();
    let objects_of = |all: &std::collections::BTreeMap<u32, usize>,
                      frame: &std::collections::BTreeMap<u32, usize>| {
        let mut out = Vec::new();
        for (w, k) in all {
            let left = k.saturating_sub(frame.get(w).copied().unwrap_or(0));
            out.extend(std::iter::repeat_n(*w, left));
        }
        out
    };
    for (i, (scale, _)) in sheets.iter().enumerate() {
        let (g, o) = (
            objects_of(&grouped[i], &frames[i]),
            objects_of(&own[i], &frames[i]),
        );
        eprintln!(
            "1:{scale}: the building's strokes, thousandths of a mm: with line groups {g:?}; each its own {o:?}"
        );
        // Each stroke at the group's share of its own width: the walls' 0.5 mm the wide line, the openings' 0.25 and
        // 0.125 mm the narrow line and its half (a PDF's widths are rounded: ±1 µm).
        let share =
            kentos_commands::line_group(f64::from(*scale)) / kentos_commands::LINE_GROUP_BASE;
        let mut want: Vec<u32> = o
            .iter()
            .map(|w| (f64::from(*w) * share).round() as u32)
            .collect();
        want.sort_unstable();
        assert_eq!(g.len(), want.len(), "1:{scale}: {g:?} against {want:?}");
        for (got, w) in g.iter().zip(&want) {
            assert!(got.abs_diff(*w) <= 1, "1:{scale}: {g:?} against {want:?}");
        }
        let wide = (kentos_commands::line_group(f64::from(*scale)) * 1000.0).round() as u32;
        assert!(g.contains(&wide), "1:{scale}: the walls at {wide}: {g:?}");
    }
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

/// docs/adr/3012, step 3: 1. kat's balcony over Zemin kat's entrance shows on Zemin kat's 1:50 sheet dashed and narrow
/// (half its pen), its railing left out; on 1. kat's own sheet it is solid, with its railing. Mimarlar Odası §14.5.3 u:
/// "Planın geçtiği düzlem ile tavan arasında kalan imalat nokta nokta işlenir. (Saçak ara kat çıkma vb.)"
#[test]
fn an_upper_slab_shows_dashed_on_the_plan_below_it() {
    let mut s = Session::new(&NewDrawing {
        name: "Çıkma".into(),
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
    let mut arch = s
        .document()
        .settings()
        .arch
        .clone()
        .expect("a Mimari project");
    arch.levels.push(
        serde_json::from_value(
            json!({"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}),
        )
        .expect("a level"),
    );
    done(
        s.call(json!({ "command": "cad.settings.set", "input": { "arch": arch } }))
            .expect("answers"),
    );
    done(
        s.call(json!({ "command": "cad.walls.rectangle", "input": {
            "layerId": layer,
            "frame": { "kind": "corners", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 8.0, "y": 6.0 } },
            "width": 0.25, "lineWeight": 0.5,
            "arch": { "level": "zemin" }
        } }))
        .expect("answers"),
    );
    // The walls' frame is their inner face: the balcony steps out of their outer face, 25 cm under it.
    let p = |x: f64, y: f64| json!({ "x": x, "y": y });
    done(
        s.call(json!({ "command": "cad.slabs.create", "input": {
            "layerId": layer, "lineWeight": 0.35,
            "slabs": [{
                "parts": [{ "outer": { "pts": [p(2.5, -1.75), p(5.5, -1.75), p(5.5, -0.25), p(2.5, -0.25)] } }],
                "role": "balcony", "edge": { "kind": "railing", "width": 0.05 },
                "arch": { "level": "kat-1" }
            }]
        } }))
        .expect("answers"),
    );
    done(
        s.call(json!({ "command": "cad.sheets.levels", "input": { "options": { "scale": 50 } } }))
            .expect("answers"),
    );
    let dir = std::env::temp_dir().join(format!("kentos-cikma-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let pdf = dir.join("cikma-1-50.pdf");
    s.export_pdf(&pdf, &[]).expect("the PDF");
    if let Ok(keep) = std::env::var("KENTOS_SCALE_PDF_DIR") {
        std::fs::copy(&pdf, std::path::Path::new(&keep).join("cikma-1-50.pdf")).expect("kept");
        // The drawing too, opened on Zemin kat alone, for the real window (fixtures/interaction/real/ust-kat-doseme.txt).
        let mut arch = s
            .document()
            .settings()
            .arch
            .clone()
            .expect("a Mimari project");
        arch.active_level = Some(Some("zemin".into()));
        arch.level_view = Some(kentos_commands::LevelView::Active);
        done(
            s.call(json!({ "command": "cad.settings.set", "input": { "arch": arch } }))
                .expect("answers"),
        );
        s.save(Some(&std::path::Path::new(&keep).join("cikma.kcad")))
            .expect("kept");
    }
    let bytes = std::fs::read(&pdf).expect("written");
    let strokes = |page: u32| -> Vec<(f64, bool)> {
        kentos_formats::pdf::read_page(&bytes, page)
            .expect("its page")
            .paths
            .iter()
            .filter_map(|p| p.stroke.as_ref())
            .map(|s| ((s.width / PT_PER_MM * 1000.0).round() / 1000.0, s.dashed))
            .collect()
    };
    let count = |strokes: &[(f64, bool)], mm: f64, dashed: bool| {
        strokes
            .iter()
            .filter(|(w, d)| (w - mm).abs() < 0.01 && *d == dashed)
            .count()
    };
    let (ground, first) = (strokes(1), strokes(2));
    eprintln!(
        "Zemin kat: 0.175 mm dashed {}, 0.35 mm solid {}; 1. kat: 0.175 mm dashed {}, 0.35 mm solid {}",
        count(&ground, 0.175, true),
        count(&ground, 0.35, false),
        count(&first, 0.175, true),
        count(&first, 0.35, false)
    );
    // Zemin kat: the balcony's ring dashed at half its pen. 1. kat: its own plan's, solid with its railing. The two
    // sheets' frames and title blocks are alike (three dashed 0.175 mm lines on each before): the difference is the
    // balcony's.
    assert!(
        count(&ground, 0.175, true) > count(&first, 0.175, true),
        "{ground:?} {first:?}"
    );
    assert!(
        count(&first, 0.35, false) >= count(&ground, 0.35, false) + 2,
        "{ground:?} {first:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/3014: KentOS's wall styles and a column on 1:20 and 1:50 sheets. TS 5319 (Teknik resim – Taramalar):
/// the heat insulation of Dış duvar 25 and the insulation of Bölme duvar 10 are its SN55, a zigzag between the layer's
/// two faces, one line along the wall; the concrete of a column at 1:20 its SN46. `KENTOS_SCALE_PDF_DIR=dir` keeps the
/// PDFs (`malzeme-1-<scale>.pdf`) and the drawing (`malzeme.kcad`) there.
#[test]
fn the_materials_print_by_ts_5319() {
    let mut s = Session::new(&NewDrawing {
        name: "Malzeme".into(),
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
    for (y, width, style) in [
        (0.0, 0.25, "Dış duvar 25"),
        (1.0, 0.20, "Gazbeton 20"),
        (2.0, 0.25, "Betonarme perde 25"),
        (3.0, 0.10, "Bölme duvar 10"),
    ] {
        let made = done(
            s.call(json!({ "command": "cad.walls.create", "input": {
                "layerId": layer,
                "walls": [{ "pts": [{ "x": 0.0, "y": y }, { "x": 3.0, "y": y }], "width": width,
                            "arch": { "level": "zemin" } }],
                "lineWeight": 0.5
            } }))
            .expect("answers"),
        );
        let uid = made["uids"][0].as_str().expect("a wall").to_owned();
        done(
            s.call(json!({ "command": "cad.walls.edit", "input": {
                "uids": [uid], "operation": { "kind": "set", "style": style }
            } }))
            .expect("answers"),
        );
    }
    done(
        s.call(json!({ "command": "cad.columns.create", "input": {
            "layerId": layer, "lineWeight": 0.5,
            "columns": [{ "at": { "x": 4.0, "y": 1.5 }, "width": 0.3, "depth": 0.5, "tag": "S1" }]
        } }))
        .expect("answers"),
    );
    let dir = std::env::temp_dir().join(format!("kentos-malzeme-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let keep = std::env::var("KENTOS_SCALE_PDF_DIR").ok();
    let mut pages = Vec::new();
    for (scale, first) in [(20, "A-101"), (50, "A-102")] {
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
        let pdf = dir.join(format!("malzeme-1-{scale}.pdf"));
        s.export_pdf(&pdf, &[id]).expect("the PDF");
        if let Some(keep) = &keep {
            std::fs::copy(
                &pdf,
                std::path::Path::new(keep).join(format!("malzeme-1-{scale}.pdf")),
            )
            .expect("kept");
        }
        pages.push((
            scale,
            kentos_formats::pdf::read_page(&std::fs::read(&pdf).expect("written"), 1)
                .expect("its first page"),
        ));
    }
    if let Some(keep) = &keep {
        s.save(Some(&std::path::Path::new(keep).join("malzeme.kcad")))
            .expect("kept");
    }
    for (scale, page) in &pages {
        // A zigzag is one stroked line of many points; a tiled hatch is lines of two.
        let zigzags = page
            .paths
            .iter()
            .filter(|p| p.stroke.is_some())
            .flat_map(|p| &p.parts)
            .filter(|(pts, closed)| !closed && pts.len() >= 12)
            .count();
        eprintln!("1:{scale}: {zigzags} lines of 12 points or more");
        // Dış duvar 25's heat insulation and Bölme duvar 10's insulation (shown at 1:20 only).
        let want = if *scale == 20 { 2 } else { 1 };
        assert!(zigzags >= want, "1:{scale}: {zigzags} zigzags");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
