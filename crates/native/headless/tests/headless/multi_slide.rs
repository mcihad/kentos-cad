//! Three- and four-leaf sliding doors and windows out to DXF and to a 1:50 PDF (docs/adr/0792): each door's leaves,
//! arrows and rails as its block's plain parts, read back as itself; the plan's tags and the schedule's numbers and
//! sizes read back from the PDF with poppler's `pdftotext -bbox`: each opening's number once in its own tag on paper,
//! once in its schedule row, at the paper height its tag and its table give.
use kentos_commands::{DxfReadOptions, Entity, OpeningOperation};
use kentos_headless::Session;
use serde_json::{Value, json};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/multi-slide.json");
/// The doors' and the window's persistent ids, numbers and middles along the wall (the wall runs along x at y = 0).
const OPENINGS: [(&str, &str, f64); 3] = [
    ("0192f5a1-2590-7000-8000-000000000072", "S3", 2.0),
    ("0192f5a1-2590-7000-8000-000000000073", "S4", 6.0),
    ("0192f5a1-2590-7000-8000-000000000074", "P3", 10.0),
];
/// Where each tag stands, metres below the wall.
const TAG_Y: f64 = 1.0;
/// The schedule's top left corner in the drawing.
const SCHEDULE: [f64; 2] = [0.0, -2.5];
/// The map's centre on the page (A3 landscape, 20 mm left and 10 mm top margins) and in the drawing, mm and m.
const PAGE: [f64; 2] = [20.0 + 390.0 / 2.0, 10.0 + 277.0 / 2.0];
const AT: [f64; 2] = [6.0, -2.0];
/// PDF points per millimetre.
const PT: f64 = 72.0 / 25.4;

fn session() -> Session {
    let model: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    Session::from_bytes(&kentos_kcad::encode_verified(&model).unwrap()).unwrap()
}

fn call(session: &mut Session, command: &str, input: Value) -> Value {
    let done = session.call(json!({"command": command, "input": input})).unwrap();
    assert_eq!(done["status"], "completed", "{command}: {done}");
    done
}

/// Where a point of the drawing is on the 1:50 page, mm.
fn on_page(x: f64, y: f64) -> [f64; 2] {
    [PAGE[0] + (x - AT[0]) * 20.0, PAGE[1] - (y - AT[1]) * 20.0]
}

#[test]
fn multi_leaf_sliding_doors_go_to_dxf_and_come_back() {
    let mut session = session();
    let folder = super::scratch("multi-slide-dxf");
    let path = folder.join("cok-kanatli.dxf");
    call(&mut session, "file.export.dxf", json!({"path": path}));
    let back = kentos_formats::dxf::read(&std::fs::read(&path).unwrap(), &DxfReadOptions::default())
        .unwrap()
        .entities;
    let slides: Vec<_> = back
        .iter()
        .filter_map(|e| match e {
            Entity::Opening(o) if o.opening.operation == OpeningOperation::Slide => Some(o.opening.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(slides.len(), 3, "the three sliding openings come back");
    let s4 = slides.iter().find(|o| o.number.as_deref() == Some("S4")).expect("S4");
    assert_eq!(s4.leaves, 4);
    assert_eq!(s4.build.as_ref().and_then(|b| b.both_sides), Some(true));
    let s3 = slides.iter().find(|o| o.number.as_deref() == Some("S3")).expect("S3");
    assert_eq!(s3.build.as_ref().and_then(|b| b.tracks), Some(3));
    assert_eq!(s3.sizes().panels.as_deref(), Some("OXX"));
    // The blocks other programs see: each leaf a closed polyline (its band), each sliding leaf's arrow three LINEs, each
    // rail one. S3: 3 leaves, 2 arrows; S4: 4 leaves, 2 arrows, 2 rails; P3: its two face lines, 3 leaves, 1 arrow.
    let text = std::fs::read_to_string(&path).unwrap();
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let count = |kind: &str| lines.windows(2).filter(|w| w[0] == "0" && w[1] == kind).count();
    assert!(count("LINE") >= 6 + (6 + 2) + (2 + 3), "{} LINE", count("LINE"));
    assert!(count("LWPOLYLINE") >= 3 + 4 + 3, "{} LWPOLYLINE", count("LWPOLYLINE"));
    let _ = std::fs::remove_dir_all(folder);
}

/// A word poppler read: its text and its box on the page, mm (x right, y down).
#[derive(Debug, Clone)]
struct Word {
    text: String,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// The first page's words as `pdftotext -bbox` reads them; none when poppler is not installed.
fn words(pdf: &std::path::Path) -> Option<Vec<Word>> {
    let out = std::process::Command::new("pdftotext")
        .args(["-bbox", "-f", "1", "-l", "1"])
        .arg(pdf)
        .arg("-")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let attr = |tag: &str, name: &str| -> f64 {
        let key = format!("{name}=\"");
        let at = tag.find(&key).expect("the attribute") + key.len();
        tag[at..].split('"').next().unwrap().parse::<f64>().unwrap() / PT
    };
    Some(
        text.split("<word ")
            .skip(1)
            .map(|chunk| {
                let (tag, rest) = chunk.split_once('>').unwrap();
                let word = rest.split("</word>").next().unwrap();
                Word {
                    text: word.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">"),
                    x0: attr(tag, "xMin"),
                    y0: attr(tag, "yMin"),
                    x1: attr(tag, "xMax"),
                    y1: attr(tag, "yMax"),
                }
            })
            .collect(),
    )
}

/// The drawing with its tags below the wall and its schedule at `schedule`, and a 1:50 A3 sheet whose map shows it;
/// `crop`: the map is bound to a view `width` × 8 m about the map's centre whose recipe cuts the map to it.
fn sheet(schedule: [f64; 2], crop: Option<f64>, scale: u32) -> std::path::PathBuf {
    let mut model: Value = serde_json::from_str(MODEL).unwrap();
    if let Some(width) = crop {
        model["settings"]["namedViews"] = json!([{"id": "v-orta", "name": "Orta", "center": {"x": AT[0], "y": AT[1]},
            "width": width, "height": 8.0, "twist": 0.0, "recipe": {"version": 1, "scale": 50, "crop": true}}]);
    }
    let model: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(model).unwrap();
    let mut session = Session::from_bytes(&kentos_kcad::encode_verified(&model).unwrap()).unwrap();
    let tags: Vec<Value> = OPENINGS
        .iter()
        .map(|(uid, _, x)| json!({"of": uid, "at": {"x": x, "y": -TAG_Y}}))
        .collect();
    call(&mut session, "cad.tags.create", json!({"layerId": "kapi", "tags": tags}));
    let columns = json!([{"field": "number"}, {"field": "roughWidth"}, {"field": "clearWidth"}, {"field": "leaves"},
        {"field": "tracks"}, {"field": "panels"}]);
    call(
        &mut session,
        "cad.schedules.create",
        json!({"layerId": "kapi", "at": {"x": schedule[0], "y": schedule[1]}, "of": "openings", "columns": columns}),
    );
    let mut kind = json!({ "type": "map", "view": { "type": "fixed", "center": { "x": AT[0], "y": AT[1] },
        "scale": scale, "rotation": 0 } });
    if crop.is_some() {
        let view = model.settings.named_views[0].clone();
        kind["source"] = json!({"type": "viewLink", "view": view});
    }
    let sheet = json!({ "op": "addSheet", "sheet": {
        "id": "surme", "name": "Sürme kapılar 1:50",
        "page": { "paper": "a3", "orientation": "landscape", "size": { "width": 420000, "height": 297000 },
            "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
        "items": [{ "id": "surme-harita", "name": "Harita",
            "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 }, "kind": kind }]
    }});
    call(&mut session, "cad.sheets.edit", json!({"ops": [sheet]}));
    let folder = super::scratch(&format!("multi-slide-pdf-{}-{scale}", crop.is_some()));
    let path = folder.join("surme.pdf");
    let pdf = kentos_plot::book::book_pdf(session.document(), &[], "Sürme", "2026-10-31").unwrap();
    std::fs::write(&path, pdf.bytes).unwrap();
    path
}

/// A tag's and a table's words: 2.5 mm high on paper, poppler's box a line (1.2 × the size) high.
fn paper_high(w: &Word) -> bool {
    let h = w.y1 - w.y0;
    (h - 1.2 * 2.5).abs() < 0.3
}

#[test]
fn a_1_50_sheet_reads_back_each_number_once_in_its_tag_and_its_row() {
    let path = sheet(SCHEDULE, None, 50);
    let Some(words) = words(&path) else {
        eprintln!("poppler yok; PDF okunmadı");
        return;
    };
    let table = on_page(SCHEDULE[0], SCHEDULE[1]);
    // The schedule's rows: its number, its rough opening, a door's clear width, its leaves, tracks and layout.
    let rows = [
        ("S3", ["2.400", "1.508", "3", "3", "OXX"].as_slice()),
        ("S4", ["3.200", "1.490", "4", "2", "OXXO"].as_slice()),
        ("P3", ["2.400", "3", "2", "OXO"].as_slice()),
    ];
    for ((_, number, x), (row_number, cells)) in OPENINGS.iter().zip(rows) {
        assert_eq!(*number, row_number);
        let found: Vec<&Word> = words.iter().filter(|w| w.text == *number).collect();
        assert_eq!(found.len(), 2, "{number}: once in its tag, once in the schedule: {found:?}");
        // Its tag on paper about the tag's point, 2.5 mm high.
        let at = on_page(*x, -TAG_Y);
        let tag: Vec<_> = found
            .iter()
            .filter(|w| (w.x0 + w.x1) / 2.0 - at[0] < 3.0 && at[0] - (w.x0 + w.x1) / 2.0 < 3.0)
            .filter(|w| ((w.y0 + w.y1) / 2.0 - at[1]).abs() < 3.0)
            .collect();
        assert_eq!(tag.len(), 1, "{number}'s tag at {at:?}: {found:?}");
        assert!(paper_high(tag[0]), "{number}'s tag {:?}", tag[0]);
        // Its schedule row inside the table, its sizes on the row's line.
        let row: Vec<_> = found.iter().filter(|w| w.x0 >= table[0] && w.y0 >= table[1]).collect();
        assert_eq!(row.len(), 1, "{number}'s row: {found:?}");
        let line: Vec<&str> = words
            .iter()
            .filter(|w| (w.y0 - row[0].y0).abs() < 0.3 && w.x0 > row[0].x0)
            .map(|w| w.text.as_str())
            .collect();
        assert_eq!(line, cells, "{number}'s row");
        assert!(words.iter().filter(|w| (w.y0 - row[0].y0).abs() < 0.3).all(paper_high), "{number}'s row height");
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// Cut to its view's area (docs/adr/0781 §2, 0789 §3): the tags outside the cut are not in the file, so a reader of the
/// PDF's words finds only what shows; the schedule inside it keeps every row.
#[test]
fn a_cut_map_writes_no_tag_outside_its_cut() {
    let path = sheet([2.8, -2.5], Some(7.0), 50);
    let Some(words) = words(&path) else {
        eprintln!("poppler yok; PDF okunmadı");
        return;
    };
    let count = |n: &str| words.iter().filter(|w| w.text == n).count();
    // The view: 2.5–9.5 m along the wall; S3's tag (2 m) and P3's (10 m) outside it, S4's (6 m) inside.
    assert_eq!(count("S3"), 1, "S3 only in its row");
    assert_eq!(count("S4"), 2, "S4 in its tag and its row");
    assert_eq!(count("P3"), 1, "P3 only in its row");
    let cut = [on_page(2.5, 0.0)[0], on_page(9.5, 0.0)[0]];
    assert!(
        words.iter().all(|w| w.x0 >= cut[0] - 0.5 && w.x1 <= cut[1] + 0.5),
        "every word inside the cut {cut:?}: {words:?}"
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// At 1:100 a tag keeps its paper height (2.5 mm, docs/adr/0685) where it stands, the schedule (a table in the drawing,
/// its words' height in metres from the project's 1:50) prints half as high: a sheet's scale does not resize a table.
#[test]
fn at_1_100_a_tag_keeps_its_paper_height_and_the_schedule_halves() {
    let path = sheet(SCHEDULE, None, 100);
    let Some(words) = words(&path) else {
        eprintln!("poppler yok; PDF okunmadı");
        return;
    };
    for (_, number, x) in OPENINGS {
        let at = [PAGE[0] + (x - AT[0]) * 10.0, PAGE[1] - (-TAG_Y - AT[1]) * 10.0];
        let tag = words
            .iter()
            .find(|w| w.text == number && ((w.x0 + w.x1) / 2.0 - at[0]).abs() < 3.0 && ((w.y0 + w.y1) / 2.0 - at[1]).abs() < 3.0)
            .unwrap_or_else(|| panic!("{number}'s tag at {at:?}: {words:?}"));
        assert!(paper_high(tag), "{number}'s tag {tag:?}");
    }
    let row = words.iter().find(|w| w.text == "OXXO").expect("S4's row");
    assert!(((row.y1 - row.y0) - 1.2 * 1.25).abs() < 0.2, "{row:?}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
