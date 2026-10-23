//! Independent physical text cases and a real PDF reader (CAD-SHEET-06).
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_sheet::{
    display::{self, Prim, RenderInputs},
    model::SheetBook,
};
use serde_json::{Value, json};

#[test]
fn shared_output_cases_keep_text_size_font_coordinates_and_searchable_pdf() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/headless-output.json"
    ))
    .unwrap();
    assert!(data["cases"].as_array().unwrap().len() >= 20);
    let out = std::env::temp_dir().join(format!("kentos-sheet-output-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    for (i, case) in data["cases"].as_array().unwrap().iter().enumerate() {
        let mut doc =
            Document::from_snapshot(serde_json::from_value(case["setup"].clone()).unwrap())
                .unwrap();
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(case["sheet"].clone()).unwrap());
        doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let list = display::display_list(&book, "a", &RenderInputs::default()).unwrap();
        let map = list
            .prims
            .iter()
            .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
            .unwrap();
        let mut spatial = Spatial::new();
        spatial.reload(&doc);
        let labels = kentos_plot::labels::map_labels(&doc, &spatial, map);
        let expected = &case["expected"];
        let text = labels
            .iter()
            .find(|l| l.text == expected["text"].as_str().unwrap());
        if expected["visible"] == true {
            let l = text.unwrap_or_else(|| panic!("{}: {labels:?}", case["name"]));
            assert!(
                (l.size - expected["size"].as_f64().unwrap()).abs() < 1e-10,
                "{}",
                case["name"]
            );
            assert!(
                (l.at.x - 2.5).abs() < 1e-10 && l.at.y.abs() < 1e-10,
                "{}: {l:?}",
                case["name"]
            );
            assert_eq!(l.rotation, expected["rotation"].as_f64().unwrap());
            assert_eq!(serde_json::to_value(l.font).unwrap(), expected["font"]);
        } else {
            assert!(text.is_none(), "{}", case["name"]);
        }
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "test", "2026-10-22").unwrap();
        assert!(pdf.notes.is_empty(), "{:?}", pdf.notes);
        let path = out.join(format!("{i}.pdf"));
        std::fs::write(&path, &pdf.bytes).unwrap();
        if expected["visible"] == true {
            let problems = super::files::pdf_text_problems(
                &path,
                &json!({"pages":[{"texts":[expected["text"].clone()]}]}),
            )
            .expect("pdftotext must be installed for the PDF acceptance");
            if !problems.is_empty() {
                // Poppler's layout mode reorders rotated lines into their bounding rows.
                // Raw text preserves glyph order; spaces can be inferred differently at a turn.
                let raw = std::process::Command::new("pdftotext")
                    .arg("-raw")
                    .arg(&path)
                    .arg("-")
                    .output()
                    .unwrap();
                assert!(raw.status.success());
                let compact =
                    |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
                assert!(
                    compact(&String::from_utf8_lossy(&raw.stdout))
                        .contains(&compact(expected["text"].as_str().unwrap())),
                    "{}: {problems:?}",
                    case["name"]
                );
            }
        }
    }
}

#[test]
fn a_rich_map_prints_as_an_actual_picture_with_its_text_and_pattern() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/headless-output.json"
    ))
    .unwrap();
    let case = &data["cases"][0];
    let mut doc =
        Document::from_snapshot(serde_json::from_value(case["setup"].clone()).unwrap()).unwrap();
    doc.remove(&[kentos_domain::Slot(1)]);
    let mut text = doc.get(kentos_domain::Slot(2)).unwrap().clone();
    if let kentos_contracts::Entity::Text(t) = &mut text {
        t.height = 0.2;
        t.mask = true;
    }
    doc.update(kentos_domain::Slot(2), text);
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(case["sheet"].clone()).unwrap());
    book.sheets[0].export.dpi = 96;
    let mut style = doc.layers().get("cizim").unwrap().style.clone();
    style.renderer = Some(
        json!({"type":"single","symbols":{"fill":{"type":"fill","layers":[{"id":"p","type":"patternFill","marker":{"type":"marker","layers":[{"id":"m","type":"shape","shape":"circle","size":0.8,"fill":"#3366cc","stroke":null}]},"spacingX":3,"spacingY":3}]}}}),
    );
    doc.set_layer_style("cizim", style, "Desen");
    doc.add(serde_json::from_value(json!({"kind":"polygon","id":0,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":-1},{"x":5,"y":-1},{"x":5,"y":1},{"x":0,"y":1}]})).unwrap()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let pdf = kentos_plot::book::book_pdf(&doc, &[], "test", "2026-10-22").unwrap();
    assert!(String::from_utf8_lossy(&pdf.bytes).contains("/Subtype /Image"));
    assert_eq!(pdf.notes.len(), 1, "{:?}", pdf.notes);
    assert!(pdf.notes[0].contains("desen dolgusu"));
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.run/shots/sheet06-rich.pdf");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, &pdf.bytes).unwrap();
    let rendered = std::process::Command::new("pdftoppm")
        .args(["-singlefile", "-scale-to", "1600", "-png"])
        .arg(&out)
        .arg(out.with_extension(""))
        .output()
        .unwrap();
    assert!(
        rendered.status.success(),
        "{}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let png = std::fs::File::open(out.with_extension("png")).unwrap();
    let mut reader = png::Decoder::new(std::io::BufReader::new(png))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    let stride = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("{other:?}"),
    };
    let blue = pixels[..info.buffer_size()]
        .chunks_exact(stride)
        .filter(|p| p[2] > 120 && p[2] > p[0].saturating_add(40))
        .count();
    let black = pixels[..info.buffer_size()]
        .chunks_exact(stride)
        .filter(|p| {
            p[0] < 210
                && p[1] < 210
                && p[2] < 210
                && p[0].abs_diff(p[1]) < 10
                && p[1].abs_diff(p[2]) < 10
        })
        .count();
    assert!(blue > 500, "pattern missing: {blue}");
    assert!(black > 50, "map text missing: {black}");
}

#[test]
fn a_template_embeds_its_picture_bytes_and_reopens_with_the_same_output() {
    let mut snapshot: kentos_contracts::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/sheets.json")).unwrap();
    let original = snapshot.sheets.take().unwrap();
    let (book, _) = kentos_sheet::project::read_project_sheets(&original).unwrap();
    let s = &book.sheets[0];
    let template = json!({
        "schema":"kentos.sheet.template/1",
        "meta":{"id":"user:test","revision":1,"name":"Resimli"},
        "sheet":s,
        "assets":original.pictures.iter().map(|p|json!({"meta":book.asset(&p.sha256).unwrap(),"data":kentos_contracts::encoding::base64_encode(&p.bytes)})).collect::<Vec<_>>()
    });
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let input = serde_json::from_value(json!({"ops":[{"op":"addSheet","sheet":{"id":"copy","name":"Copy","page":s.page,"items":[]}},{"op":"applyTemplate","sheet":"copy","template":template,"ids":{"sheet":"copy","items":s.items.iter().enumerate().map(|(i,_)|format!("copy-{i}")).collect::<Vec<_>>()},"options":{}}]})).unwrap();
    let answer = kentos_native_application::sheets_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc),
        input,
    );
    assert!(
        matches!(answer, kentos_contracts::CommandResult::Completed { .. }),
        "{answer:?}"
    );
    assert_eq!(doc.sheets().unwrap().pictures, original.pictures);
    let reopened = Document::from_snapshot_v2(doc.to_snapshot_v2()).unwrap();
    let a = kentos_plot::book::book_pdf(&doc, &[], "test", "2026-10-22").unwrap();
    let b = kentos_plot::book::book_pdf(&reopened, &[], "test", "2026-10-22").unwrap();
    // Reopening resets the runtime revision. Source provenance must report each real
    // revision even though model, text and embedded image content stay identical.
    let ca=kentos_plot::provenance::Capture::new(&doc,"test","2026-10-22",None).unwrap();
    let cb=kentos_plot::provenance::Capture::new(&reopened,"test","2026-10-22",None).unwrap();
    assert_eq!(ca.pin().sha256,cb.pin().sha256);
    assert_eq!(ca.pin().revision,doc.revision().to_string());
    assert_eq!(cb.pin().revision,reopened.revision().to_string());
    assert_eq!(super::files::pdf_streams(&a.bytes),super::files::pdf_streams(&b.bytes));
    let (printed,_) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    for sheet in &printed.sheets {
        let pa=kentos_plot::preview::page_captured(&ca,&sheet.id,96).unwrap();
        let pb=kentos_plot::preview::page_captured(&cb,&sheet.id,96).unwrap();
        assert_eq!((pa.width,pa.height),(pb.width,pb.height));
        assert_eq!(pa.rgba,pb.rgba);
    }
    assert!(String::from_utf8_lossy(&a.bytes).contains("/Subtype /Image"));
    assert!(String::from_utf8_lossy(&b.bytes).contains("/Subtype /Image"));
}

#[test]
fn an_unreadable_map_image_fails_instead_of_exporting_a_blank_frame() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/headless-output.json"
    ))
    .unwrap();
    let mut doc =
        Document::from_snapshot(serde_json::from_value(data["cases"][0]["setup"].clone()).unwrap())
            .unwrap();
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    book.sheets[0].export.dpi = 96;
    doc.add(serde_json::from_value(json!({"kind":"image","id":0,"layerId":"cizim","attrs":{},"file":"/no-such-kentos-image.png","width":1,"height":1,"p":{"x":0,"y":0},"rotation":0,"opacity":1})).unwrap()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let e = kentos_plot::book::book_pdf(&doc, &[], "test", "2026-10-22").unwrap_err();
    assert_eq!(e.code, "plot_asset_unreadable");
}

#[test]
fn svg_assets_render_with_no_window_or_system_fonts() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/headless-output.json"
    ))
    .unwrap();
    let mut doc =
        Document::from_snapshot(serde_json::from_value(data["cases"][0]["setup"].clone()).unwrap())
            .unwrap();
    let svg=br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="45" fill="#3366cc"/><text x="20" y="55" font-family="Barlow" font-size="20">K</text></svg>"##.to_vec();
    let hash = kentos_contracts::encoding::sha256_hex(&svg);
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    book.assets.push(serde_json::from_value(json!({"sha256":hash,"name":"logo.svg","kind":"svg","width":100,"height":100,"bytes":svg.len()})).unwrap());
    book.sheets[0].items=vec![serde_json::from_value(json!({"id":"logo","name":"Logo","frame":{"left":10000,"top":10000,"width":30000,"height":30000},"kind":{"type":"picture","asset":hash}})).unwrap()];
    doc.set_sheets(
        kentos_sheet::project::project_sheets(&book, |h| (h == hash).then(|| svg.clone())).unwrap(),
    )
    .unwrap();
    let pdf = kentos_plot::book::book_pdf(&doc, &[], "test", "2026-10-22").unwrap();
    assert!(String::from_utf8_lossy(&pdf.bytes).contains("/Subtype /Image"));
    assert!(!pdf.findings.iter().any(|f| f.code == "svg_not_in_pdf"));
}
