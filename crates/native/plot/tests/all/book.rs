//! A book PDF without a window (docs/adr/0252 §3): the acceptance drawing's
//! sheets (fixtures/acceptance/v1/room.json) over a drawing that holds the
//! scenario's 5000 mm line, read back without KentOS code: A3 pages, the line
//! 100 mm long at 1:50 and 50 mm at 1:100, where its map puts it.

use kentos_contracts::{DocumentSnapshotV1, DrawingFont, DrawingUnit, Entity, Workspace};
use kentos_domain::Document;
use kentos_plot::book::book_pdf;
use kentos_project::new_project::{NewProject, new_project};
use kentos_sheet::model::SheetBook;
use serde_json::{Value, json};

use crate::files::pdf_problems;

const SCENARIO: &str = include_str!("../../../../../fixtures/acceptance/v1/room.json");

fn step(scenario: &Value, what: impl Fn(&Value) -> bool) -> &Value {
    scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| what(s))
        .expect("the step")
}

/// A local mm project with the K11 line, and the scenario's two sheets in its book.
fn drawing(scenario: &Value) -> Document {
    let mut snapshot: DocumentSnapshotV1 = new_project(&NewProject {
        name: "Oda".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(DrawingUnit::Mm),
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a new project");
    let k11 = &step(scenario, |s| s["as"] == "k11")["input"];
    let line: Entity = serde_json::from_value(json!({
        "kind": "line", "id": 1, "layerId": snapshot.active_layer, "attrs": {},
        "a": k11["a"], "b": k11["b"]
    }))
    .expect("a line");
    snapshot.entities.push(line);
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let ops: Vec<kentos_sheet::ops::Op> = serde_json::from_value(
        step(scenario, |s| s["command"] == "cad.sheets.edit")["input"]["ops"].clone(),
    )
    .expect("the sheet core's operations");
    let book = kentos_sheet::ops::apply_all(&SheetBook::default(), &ops)
        .expect("the sheets")
        .book;
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    doc
}

#[test]
fn a_book_s_sheets_print_their_maps_at_scale() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let doc = drawing(&scenario);
    let made = book_pdf(&doc, &[], "kabul", "2026-10-13").expect("a PDF");
    assert_eq!(made.sheets, ["kat-plani", "vaziyet"]);
    // Both fixture viewports are intentionally unlocked; native preflight now names them per page.
    let mut actual=made.findings.iter().map(|f|(f.code.as_str(),f.item.as_deref())).collect::<Vec<_>>();actual.sort();
    assert_eq!(actual,[("no_kerning",None),("viewport_scale_unlocked",Some("kat-plani-harita")),("viewport_scale_unlocked",Some("vaziyet-harita"))]);
    assert!(made.findings.iter().all(|f|f.severity==if f.code=="no_kerning"{kentos_sheet::preflight::Severity::Info}else{kentos_sheet::preflight::Severity::Warning}));
    let mut owners=made.sheet_findings.iter().map(|(id,f)|(id.as_str(),f.code.as_str())).collect::<Vec<_>>();owners.sort();
    assert_eq!(owners,[("kat-plani","no_kerning"),("kat-plani","viewport_scale_unlocked"),("vaziyet","no_kerning"),("vaziyet","viewport_scale_unlocked")]);
    let want = &step(&scenario, |s| s["export"] == "pdf")["expect"]["pdf"];
    let problems = pdf_problems(&made.bytes, want);
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_project_without_sheets_says_so() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let mut doc = drawing(&scenario);
    doc.set_sheets(None).expect("no book");
    let e = book_pdf(&doc, &[], "kabul", "2026-10-13").expect_err("no sheets");
    assert_eq!(e.code, "no_sheets");
}

/// A gradient and a dense hatch on a sheet go to the PDF as vectors (docs/adr/0333 §1): the map is not left blank as a
/// map with no vector form was.
#[test]
fn a_gradient_and_a_dense_hatch_go_to_the_pdf_as_vectors() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let mut doc = drawing(&scenario);
    let k11 = &step(&scenario, |s| s["as"] == "k11")["input"];
    let (ax, ay) = (k11["a"]["x"].as_f64().expect("x"), k11["a"]["y"].as_f64().expect("y"));
    let ring = |dx: f64| {
        json!([{ "x": ax + dx, "y": ay + 0.5 }, { "x": ax + dx + 1.5, "y": ay + 0.5 },
               { "x": ax + dx + 1.5, "y": ay + 2.0 }, { "x": ax + dx, "y": ay + 2.0 }])
    };
    let layer = doc.layers().active().to_owned();
    let plain = book_pdf(&doc, &[], "kabul", "2026-10-17").expect("a PDF").bytes.len();
    for (k, pattern) in [
        json!({ "type": "gradient", "angle": 30, "spacing": 1,
                "gradient": { "shape": "spherical", "inverted": false, "color2": "#FFFFFF" } }),
        json!({ "type": "lines", "angle": 45, "spacing": 0.01 }),
    ]
    .into_iter()
    .enumerate()
    {
        let hatch: Entity = serde_json::from_value(json!({
            "kind": "hatch", "id": 0, "layerId": layer, "attrs": {}, "ring": ring(2.0 * k as f64), "pattern": pattern
        }))
        .expect("a hatch");
        doc.add(hatch).expect("added");
    }
    let made = book_pdf(&doc, &[], "kabul", "2026-10-17").expect("a PDF");
    assert!(
        !made.notes.iter().any(|n| n.contains("vektörle yazılamadı")),
        "{:?}",
        made.notes
    );
    // KENTOS_PLOT_OUT=dir writes the PDF there to look at (docs/adr/0333 Doğrulama).
    if let Some(dir) = std::env::var_os("KENTOS_PLOT_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("tarama-pdf.pdf"), &made.bytes).expect("written");
    }
    // The bands and the lines are in it: the map grew by them (it would not if they were outside its frame).
    assert!(made.bytes.len() > plain + 20_000, "{} against {plain}", made.bytes.len());
}

/// Loaded caches are immutable until an explicit update; stale field values
/// must not escape through desktop/Python's common delivery path.
#[test]
fn stale_loaded_field_cache_blocks_pdf_until_explicit_refresh_and_is_undoable() {
    use kentos_commands::{CallOp, TextField};
    use kentos_native_application::{ExecutionContext, registry};
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    let mut doc = drawing(&scenario);
    let layer = doc.layers().active().to_owned();
    let t: Entity = serde_json::from_value(json!({"kind":"text","id":0,"layerId":layer,"attrs":{},
        "p":{"x":1.,"y":0.5},"text":"Eski","height":0.125,"rotation":0.})).unwrap();
    let slot = doc.add(t).unwrap();
    let uid = doc.uid(slot).unwrap().to_string();
    let mut snapshot = doc.to_snapshot_v2();
    let Entity::Text(t) = snapshot.entities.last_mut().unwrap() else { panic!() };
    t.paragraph.fields = serde_json::from_value::<Vec<TextField>>(json!([
        {"start":0,"end":4,"source":{"kind":"project","value":"name"}}
    ])).unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let error = book_pdf(&doc, &[], "kabul", "2026-11-04").expect_err("stale cached field");
    assert_eq!(error.code, "text_field_stale");
    assert!(error.message.contains("Eski") && error.message.contains("Oda"));
    assert_eq!(doc.to_snapshot_v2(), before);
    let changed = registry::run(&mut ExecutionContext::new(&mut doc), "cad.fields.edit", CallOp::Execute,
        json!({"edit":{"kind":"update","uids":[uid]}})).unwrap();
    assert_eq!(changed["status"], "completed", "{changed}");
    let made = book_pdf(&doc, &[], "kabul", "2026-11-04").unwrap();
    assert!(!made.findings.iter().any(|f| f.code == "text_field_stale"));
    assert!(doc.undo().is_some());
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn model_text_never_silently_exports_question_mark_substitutions() {
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    for (text, x, rejected) in [
        ("Türkçe: İıŞşĞğÇçÖöÜü ±25 °C 20 m²", 1., false),
        ("g\u{306}", 1., true),
        ("Çap ⌀32", 1., true),
        ("Türkçe / العربية / עברית 123", 1., true),
        ("Outside viewport: ⌀32", 100000., false),
    ] {
        let mut doc = drawing(&scenario);
        let layer = doc.layers().active().to_owned();
        doc.add(serde_json::from_value(json!({"kind":"text","id":0,"layerId":layer,"attrs":{},
            "p":{"x":x,"y":0.5},"text":text,"height":0.125,"rotation":0.,"font":"arimo"})).unwrap()).unwrap();
        let before = doc.to_snapshot_v2();
        let result = book_pdf(&doc, &[], "kabul", "2026-11-04");
        if rejected {
            let error = result.expect_err(text);
            assert_eq!(error.code, "map_text_unsupported", "{text}: {error:?}");
            assert!(error.message.contains("U+"));
        } else { result.unwrap_or_else(|e| panic!("{text}: {e:?}")); }
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn unicode_profile_prints_combining_marks_symbols_and_bidi_without_mutating_text() {
    let scenario:Value=serde_json::from_str(SCENARIO).unwrap();
    let mut doc=drawing(&scenario);
    for (n,text) in ["Ög\u{306}ç — İstanbul", "⌀32 ≤ 5‰ ±25 °C", "Türkçe / العربية / עברית 123"].into_iter().enumerate() {
        let layer=doc.layers().active().to_owned();
        doc.add(serde_json::from_value(json!({"kind":"text","id":0,"layerId":layer,"attrs":{},
            "p":{"x":1.,"y":0.5-n as f64*0.2},"text":text,"height":0.125,"rotation":0.,"font":"arimo",
            "layout":{"engine":"unicode-v1"}})).unwrap()).unwrap();
    }
    let before=doc.to_snapshot_v2();
    let made=book_pdf(&doc,&[],"kabul","2026-11-04").unwrap();
    assert!(!made.findings.iter().any(|f|f.code=="map_text_unsupported" || f.code=="text_layout_unsupported"));
    let fallback:Vec<_>=made.findings.iter().filter(|f|f.code=="text_font_fallback").collect();
    assert!(!fallback.is_empty());
    assert!(fallback.iter().all(|f|f.severity==kentos_sheet::preflight::Severity::Info));
    assert_eq!(doc.to_snapshot_v2(),before);
    if let Ok(dir)=std::env::var("KENTOS_TEXT_PDF_OUT") {std::fs::write(std::path::Path::new(&dir).join("unicode-profile-native.pdf"),made.bytes).unwrap();}
    let mut bad=before.clone();
    let Entity::Text(t)=bad.entities.last_mut().unwrap() else {panic!()};
    t.text="Desteklenmeyen 🛸".into();
    let doc=Document::from_snapshot_v2(bad).unwrap();
    assert_eq!(book_pdf(&doc,&[],"kabul","2026-11-04").unwrap_err().code,"text_layout_unsupported");
}

/// Real GUI's saved snapshot is supplied explicitly; screenshots/files stay outside Git.
#[test]
#[ignore]
fn unicode_profile_gui_snapshot_pdf() {
    let path=std::env::var("KENTOS_TEXT_GUI_SNAPSHOT").expect("saved GUI snapshot path");
    let snapshot=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let doc=Document::from_snapshot_v2(snapshot).unwrap();
    let made=book_pdf(&doc,&[],"kabul","2026-11-04").unwrap();
    let out=std::env::var("KENTOS_TEXT_PDF_OUT").expect("PDF output directory");
    std::fs::write(std::path::Path::new(&out).join("unicode-profile-gui.pdf"),made.bytes).unwrap();
    let (book,_)=kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    for (i,sheet) in book.sheets.iter().enumerate() {
        let preview=kentos_plot::preview::page(&doc,&sheet.id,"kabul","2026-11-04",None,144).unwrap();
        std::fs::write(std::path::Path::new(&out).join(format!("unicode-profile-gui-preview-{i}.png")),preview.png).unwrap();
    }
}
/// Independent PDF rasterization catches a preview that re-shapes the source,
/// loses rich-run colours/rules, or rounds georeferenced coordinates to f32.
#[test]
fn unicode_profile_preview_matches_pdf_ink_at_two_scales_and_large_coordinates() {
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    let mut doc = drawing(&scenario);
    let offset = [900_000_000., 400_000_000.];
    let (mut book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    let mut value = serde_json::to_value(&book).unwrap();
    for sheet in value["sheets"].as_array_mut().unwrap() {
        for item in sheet["items"].as_array_mut().unwrap() {
            if item["kind"]["type"] == "map" {
                item["kind"]["view"]["center"] = json!({"x":offset[0]+2.,"y":offset[1]+1.});
                item["kind"]["view"]["rotation"] = json!(17_000);
            }
        }
    }
    book = serde_json::from_value(value).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    for (n, curved) in [false, true].into_iter().enumerate() {
        let layer = doc.layers().active().to_owned();
        let mut text = json!({"kind":"text","id":0,"layerId":layer,"attrs":{},
            "p":{"x":offset[0]+1.,"y":offset[1]+0.5-n as f64},
            "text":"Ög\u{306}ç 1/2 العربية", "height":0.25,"rotation":12.,"font":"arimo",
            "color":"#0055cc","lean":0.15,"mask":true,"layout":{"engine":"unicode-v1"},
            "runs":[{"start":5,"end":8,"script":"stack","underline":true,"color":"#cc3300"}]});
        if curved {
            text["path"] = json!({"pts":[{"x":offset[0]+5.,"y":offset[1]+1.5}],"bulges":[0.41421356237309503]});
        }
        doc.add(serde_json::from_value(text).unwrap()).unwrap();
    }
    let before = doc.to_snapshot_v2();
    let out = std::env::temp_dir().join(format!("kentos-unicode-preview-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let is_coloured = |p: &[u8]| p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]) > 70;
    for (n, sheet) in book.sheets.iter().enumerate() {
        let preview =
            kentos_plot::preview::page(&doc, &sheet.id, "test", "2026-11-04", None, 144).unwrap();
        let pdf = book_pdf(&doc, std::slice::from_ref(&sheet.id), "test", "2026-11-04").unwrap();
        let stem = out.join(n.to_string());
        std::fs::write(stem.with_extension("pdf"), pdf.bytes).unwrap();
        assert!(
            std::process::Command::new("pdftoppm")
                .args(["-r", "144", "-singlefile", "-png"])
                .arg(stem.with_extension("pdf"))
                .arg(&stem)
                .status()
                .unwrap()
                .success()
        );
        let mut reader = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(stem.with_extension("png")).unwrap(),
        ))
        .read_info()
        .unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        assert_eq!([preview.width, preview.height], [info.width, info.height]);
        assert_eq!(info.color_type, png::ColorType::Rgb);
        let a: Vec<_> = preview.rgba.chunks_exact(4).map(is_coloured).collect();
        let b: Vec<_> = bytes[..info.buffer_size()]
            .chunks_exact(3)
            .map(is_coloured)
            .collect();
        for (from, to) in [(&a, &b), (&b, &a)] {
            let mut ink = 0;
            let mut matched = 0;
            for (i, &yes) in from.iter().enumerate() {
                if !yes {
                    continue;
                }
                ink += 1;
                let (x, y) = (
                    (i % info.width as usize) as i32,
                    (i / info.width as usize) as i32,
                );
                if (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (x, y) = (x + dx, y + dy);
                        x >= 0
                            && y >= 0
                            && x < info.width as i32
                            && y < info.height as i32
                            && to[y as usize * info.width as usize + x as usize]
                    })
                }) {
                    matched += 1;
                }
            }
            assert!(
                ink > 200 && matched * 100 >= ink * 99,
                "{}: {matched}/{ink}",
                sheet.name
            );
        }
        for bytes in [&preview.rgba[..], &bytes[..info.buffer_size()]] {
            let channels = if bytes.len() == preview.rgba.len() {
                4
            } else {
                3
            };
            assert!(
                bytes
                    .chunks_exact(channels)
                    .filter(|p| p[0] > p[2].saturating_add(70))
                    .count()
                    > 10,
                "red fraction/rule"
            );
            assert!(
                bytes
                    .chunks_exact(channels)
                    .filter(|p| p[2] > p[0].saturating_add(70))
                    .count()
                    > 100,
                "blue base glyphs"
            );
        }
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    std::fs::remove_dir_all(out).unwrap();
}
#[test]
fn stale_standalone_table_field_blocks_pdf_until_explicit_refresh_and_undo_restores_it() {
    use kentos_commands::{CallOp, Entity};
    use kentos_native_application::{ExecutionContext, registry};
    let scenario: Value = serde_json::from_str(SCENARIO).unwrap();
    let mut doc = drawing(&scenario);
    let layer = doc.layers().active().to_owned();
    let slot = doc.add(serde_json::from_value(json!({"kind":"table","id":0,"layerId":layer,"attrs":{},
        "p":{"x":0.,"y":0.5},"rotation":0.,"height":0.125,"rows":[0.5],"columns":[3.],"cells":[["Eski"]]})).unwrap()).unwrap();
    let uid = doc.uid(slot).unwrap().to_string();
    let mut snapshot = doc.to_snapshot_v2();
    let Entity::Table(table) = snapshot.entities.last_mut().unwrap() else {
        panic!()
    };
    table.fields = serde_json::from_value(
        json!([{"row":0,"col":0,"source":{"kind":"project","value":"name"}}]),
    )
    .unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    assert_eq!(
        book_pdf(&doc, &[], "test", "2026-11-04").unwrap_err().code,
        "table_field_stale"
    );
    assert_eq!(
        kentos_plot::preview::page(&doc, "kat-plani", "test", "2026-11-04", None, 96)
            .err()
            .unwrap()
            .code,
        "table_field_stale"
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.tables.edit",
        CallOp::Execute,
        json!({"operation":{"kind":"refresh","uids":[uid]}}),
    )
    .unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    book_pdf(&doc, &[], "test", "2026-11-04").unwrap();
    assert!(doc.undo().is_some());
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(
        book_pdf(&doc, &[], "test", "2026-11-04").unwrap_err().code,
        "table_field_stale"
    );
}
