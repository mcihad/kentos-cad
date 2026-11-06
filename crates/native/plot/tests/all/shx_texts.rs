//! ADR 1125: a text whose style names an embedded SHX font prints in the font's own strokes, in the vector PDF and
//! the CPU picture, where the independent reference (`fixtures/shx/v1/expected.json`, scripts/fixtures/
//! shx_font_reference.py) puts them, at the object's line weight; what the font cannot draw blocks the PDF with
//! `text_shx_unsupported`, and a font the project does not embed stays an explicit substitute.
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_plot::{book, maps::MapSource};
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    pdf::MapPath,
    project::project_sheets,
};
use serde_json::{Value, json};

fn expected() -> Value {
    serde_json::from_str(include_str!("../../../../../fixtures/shx/v1/expected.json")).unwrap()
}

/// The fixture's sheet at 1:`scale` round (11, 21), its one text the reference case `case`'s fields in the SHX style;
/// `fonts`: the SHX files the project embeds.
fn drawn(case: &Value, scale: u32, fonts: &[&str], words: Option<&str>) -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    raw["settings"]["textStyles"] = json!([{"id":"shx","name":"SHX","font":"barlow",
        "fontFile":"kentos-uni.shx","bigFontFile":"kentos-big.shx"}]);
    let items: Vec<Value> = fonts
        .iter()
        .map(|f| {
            let bytes = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/shx/v1").join(f),
            )
            .unwrap();
            kentos_contracts::shape_resource::item(f, &bytes).unwrap()
        })
        .collect();
    raw["styles"] = json!({"items": items, "categories": []});
    let mut text = case["text"].clone();
    if let Some(w) = words {
        text["text"] = json!(w);
    }
    for (k, v) in [("kind", json!("text")), ("id", json!(1)), ("attrs", json!({})), ("textStyle", json!("shx")),
        ("lineWeight", json!(0.35))]
    {
        text[k] = v;
    }
    text["layerId"] = raw["entities"][0]["layerId"].clone();
    raw["entities"] = json!([text]);
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":11.,"y":21.});
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    (doc, book)
}

/// The first sheet's map as the PDF receives it: its paths and how many texts it writes in a face.
fn vectors(doc: &Document, book: &SheetBook, scale: f64) -> (Vec<MapPath>, usize) {
    let render = book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list
        .prims
        .iter()
        .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
        .unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = book::library_of(doc);
    let src = MapSource {
        doc,
        store: spatial.store(),
        library: &library,
        origin: kentos_geometry_core::Vec2::new(0., 0.),
        map: None,
        level: None,
        phase: None,
        hidden: &[],
        pens: &[],
        mono: false,
        options: &[],
    };
    let labels = kentos_plot::labels::map_labels(doc, &spatial, map);
    let kentos_sheet::pdf::MapContent::Vector(v) =
        kentos_plot::maps::vector_content(&src, &map.layers, &map.layer_overrides, scale, map.extent.unwrap(), &labels)
            .unwrap()
    else {
        panic!("expected vectors")
    };
    let texts = v.layers.iter().map(|l| l.texts.len()).sum();
    (v.layers.into_iter().flat_map(|l| l.paths).collect(), texts)
}

fn findings(doc: &Document, b: &SheetBook, code: &str) -> Vec<kentos_sheet::preflight::Finding> {
    let render = book::base_inputs(doc, "Test", "2026-11-06");
    kentos_plot::preflight::inspect(doc, b, "ground", &render, None)
        .unwrap()
        .into_iter()
        .filter(|f| f.code == code)
        .collect()
}

#[test]
fn an_embedded_shx_font_prints_its_strokes_where_the_reference_puts_them() {
    let expected = expected();
    let cases = expected["texts"].as_array().unwrap();
    for name in ["Sol taban", "Çarpan, eğiklik ve dönüş birlikte", "Üç satır, aralık 1,5, üst sağ", "Big Font karakteri",
        "Kutuya kaydırma, ortalı"]
    {
        let case = cases.iter().find(|c| c["name"] == name).unwrap();
        for scale in [100u32, 250] {
            let (doc, b) = drawn(case, scale, &["kentos-uni.shx", "kentos-big.shx"], None);
            let before = doc.to_snapshot_v2();
            let (paths, texts) = vectors(&doc, &b, f64::from(scale));
            assert_eq!(texts, 0, "{name} 1:{scale}: no substitute face written");
            let strokes: Vec<_> = paths.iter().filter(|p| p.stroke.as_ref().is_some_and(|s| s.cap == kentos_sheet::LineCap::Round)).collect();
            let want = case["paths"].as_array().unwrap();
            assert_eq!(strokes.len(), want.len(), "{name} 1:{scale}");
            for (got, want) in strokes.iter().zip(want) {
                let s = got.stroke.as_ref().unwrap();
                assert!((s.width - 0.35).abs() < 1e-12 && s.dash.is_empty() && !got.closed, "{name}: {s:?}");
                let want = want.as_array().unwrap();
                assert_eq!(got.points.len(), want.len(), "{name}");
                for (g, w) in got.points.iter().zip(want) {
                    assert!((g[0] - w[0].as_f64().unwrap()).abs() < 1e-9 && (g[1] - w[1].as_f64().unwrap()).abs() < 1e-9,
                        "{name} 1:{scale}: {g:?} {w}");
                }
            }
            assert!(findings(&doc, &b, "text_font_substituted").is_empty() && findings(&doc, &b, "text_shx_unsupported").is_empty());
            let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
            assert!(!String::from_utf8_lossy(&pdf.bytes).contains("/Subtype /Image"), "{name}: vectors only");
            assert_eq!(doc.to_snapshot_v2(), before);
        }
    }
}

/// The CPU picture draws the same strokes: ink on the reference's stroke points, none at the middle of a glyph's
/// open side (the 'U''s top, between its arms).
#[test]
fn the_cpu_picture_draws_the_shx_strokes() {
    let expected = expected();
    let case = expected["texts"].as_array().unwrap().iter().find(|c| c["name"] == "Sol taban").unwrap().clone();
    let (doc, _) = drawn(&case, 20, &["kentos-uni.shx", "kentos-big.shx"], None);
    let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 200).unwrap();
    // The map's centre (11, 21) is the paper's (215, 125) mm; at 1:20 a ground metre is 50 mm.
    let k = 200. / 25.4;
    let px = |x: f64, y: f64| {
        let (mx, my) = (215. + (x - 11.) * 50., 125. - (y - 21.) * 50.);
        ((mx * k) as usize, (my * k) as usize)
    };
    let dark = |(x, y): (usize, usize)| {
        let w = image.width as usize;
        (y.saturating_sub(1)..=y + 1).any(|y| (x.saturating_sub(1)..=x + 1).any(|x| image.rgba[(y * w + x) * 4] < 128))
    };
    let mut on = 0;
    for path in case["paths"].as_array().unwrap() {
        for p in path.as_array().unwrap() {
            on += usize::from(dark(px(p[0].as_f64().unwrap(), p[1].as_f64().unwrap())));
        }
    }
    let total: usize = case["paths"].as_array().unwrap().iter().map(|p| p.as_array().unwrap().len()).sum();
    assert_eq!(on, total, "every stroke point inked");
    // U spans x 1.0–1.75 cap heights (2.5 m): its open top's middle, half a cap height down, is paper.
    assert!(!dark(px(10. + 1.375 * 2.5, 20. + 0.75 * 2.5)));
}

#[test]
fn what_the_shx_font_cannot_draw_blocks_the_pdf_and_a_missing_font_is_a_named_substitute() {
    let expected = expected();
    let case = expected["texts"].as_array().unwrap().iter().find(|c| c["name"] == "Sol taban").unwrap().clone();
    // A letter the embedded font lacks: an error, the PDF refused, never a gap or a quiet substitute.
    let (doc, b) = drawn(&case, 20, &["kentos-uni.shx", "kentos-big.shx"], Some("SAU"));
    let errors = findings(&doc, &b, "text_shx_unsupported");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].message.contains("U+0041") && errors[0].severity == kentos_sheet::preflight::Severity::Error);
    let refused = book::book_pdf(&doc, &[], "Test", "2026-11-06");
    assert_eq!(refused.err().map(|e| e.code).as_deref(), Some("text_shx_unsupported"));
    // A Unicode letter and a Big Font: not looked up (no code page), said so.
    let (doc, b) = drawn(&case, 20, &["kentos-uni.shx", "kentos-big.shx"], Some("Sç"));
    assert!(findings(&doc, &b, "text_shx_unsupported")[0].message.contains("kod sayfasını bildirmez"));
    // The Big Font not embedded: the style's face draws it, a warning names the reason; the PDF is made.
    let (doc, b) = drawn(&case, 20, &["kentos-uni.shx"], None);
    let warned = findings(&doc, &b, "text_font_substituted");
    assert!(warned.len() == 1 && warned[0].message.contains("kentos-big.shx"), "{warned:?}");
    let (_, texts) = vectors(&doc, &b, 20.);
    assert_eq!(texts, 1, "the substitute face writes it");
    book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
}

/// The evidence pictures of ADR 1125 (not a check): an SHX text (Unicode, a Big Font code, width factor, slant,
/// turn) at 1:250, its PDF and CPU preview, written to `.run/gui-evidence/` (never into git).
/// `cargo test -p kentos-plot --test all shx_texts::shx_text_pictures -- --ignored`.
#[test]
#[ignore]
fn shx_text_pictures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&dir).unwrap();
    let expected = expected();
    let case = expected["texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Çarpan, eğiklik ve dönüş birlikte")
        .unwrap()
        .clone();
    let (doc, _) = drawn(&case, 250, &["kentos-uni.shx", "kentos-big.shx"], Some("ŞU-S\\M+18140"));
    let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
    std::fs::write(dir.join("adr1125-shx-metin-1-250.pdf"), &pdf.bytes).unwrap();
    let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 300).unwrap();
    std::fs::write(dir.join("adr1125-shx-metin-1-250-cpu.png"), &image.png).unwrap();
}

/// docs/adr/1020, 1022: a block's text (one line or several) in an embedded SHX style prints the same strokes as the text object would
/// where the insert puts it (an insert turns in radians, a text in degrees). The reference case's text is put in a block so that the insert (moved, and turned a
/// quarter) places it at the case's own point and turn: the strokes are the reference's, at the insert's weight;
/// no substitute face is written and nothing is said substituted.
#[test]
fn a_blocks_text_prints_in_its_embedded_shx_font_where_the_insert_puts_it() {
    let expected = expected();
    let cases = expected["texts"].as_array().unwrap();
    // One line, and (docs/adr/1022) several lines and a wrapped box, drawn whole at their first line.
    for name in ["Sol taban", "Çarpan, eğiklik ve dönüş birlikte", "Big Font karakteri", "Üç satır, aralık 1,5, üst sağ",
        "Kutuya kaydırma, ortalı"]
    {
        let case = cases.iter().find(|c| c["name"] == name).unwrap();
        for (turn, at) in [(0.0f64, [3.0, -2.0]), (90.0, [4.0, 1.0])] {
            let (mut doc, b) = drawn(case, 100, &["kentos-uni.shx", "kentos-big.shx"], None);
            let mut raw = serde_json::to_value(doc.to_snapshot()).unwrap();
            let mut text = raw["entities"][0].clone();
            let (p, r) = (&case["text"]["p"], case["text"]["rotation"].as_f64().unwrap_or(0.));
            let d = [p["x"].as_f64().unwrap() - at[0], p["y"].as_f64().unwrap() - at[1]];
            // The block's point q with turn(q) + at = p.
            let (s, c) = (-turn).to_radians().sin_cos();
            text["p"] = json!({"x": d[0] * c - d[1] * s, "y": d[0] * s + d[1] * c});
            text["rotation"] = json!(r - turn);
            text["layerId"] = json!("");
            text.as_object_mut().unwrap().remove("lineWeight");
            let block = "0192f5a1-5555-7000-8000-000000000001";
            raw["blocks"] = json!([{"id": block, "name": "SHX-BLOK", "base": {"x": 0.0, "y": 0.0}, "entities": [text]}]);
            raw["entities"] = json!([{"kind": "insert", "id": 1, "layerId": raw["entities"][0]["layerId"], "attrs": {},
                "lineWeight": 0.35, "block": block, "p": {"x": at[0], "y": at[1]}, "scale": 1.0, "rotation": turn.to_radians()}]);
            let sheets = doc.sheets().cloned();
            doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
            doc.set_sheets(sheets).unwrap();
            let (paths, texts) = vectors(&doc, &b, 100.);
            assert_eq!(texts, 0, "{name} {turn}°: no substitute face written");
            let strokes: Vec<_> = paths.iter().filter(|p| p.stroke.as_ref().is_some_and(|s| s.cap == kentos_sheet::LineCap::Round)).collect();
            let want = case["paths"].as_array().unwrap();
            assert_eq!(strokes.len(), want.len(), "{name} {turn}°");
            for (got, want) in strokes.iter().zip(want) {
                assert!((got.stroke.as_ref().unwrap().width - 0.35).abs() < 1e-12, "{name}: the insert's weight");
                let want = want.as_array().unwrap();
                assert_eq!(got.points.len(), want.len(), "{name}");
                for (g, w) in got.points.iter().zip(want) {
                    assert!((g[0] - w[0].as_f64().unwrap()).abs() < 1e-9 && (g[1] - w[1].as_f64().unwrap()).abs() < 1e-9,
                        "{name} {turn}°: {g:?} {w}");
                }
            }
            assert!(findings(&doc, &b, "text_font_substituted").is_empty(), "{name}: drawn in its font, not substituted");
            assert!(findings(&doc, &b, "text_shx_unsupported").is_empty());
        }
    }
}

/// docs/adr/1023: a dimension whose style names an SHX text style (DIMTXSTY) writes its value in that font. The
/// dimension shows the words “SU” (its text over the value) 2.5 high, horizontal: its strokes are the reference case
/// “Taban ortası” (the same words, height and baseline-centre alignment) moved to where the dimension puts its value;
/// a dimension whose style names no text style is written in its typeface, as before.
#[test]
fn a_dimension_writes_its_value_in_its_styles_shx_font() {
    let expected = expected();
    let case = expected["texts"].as_array().unwrap().iter().find(|c| c["name"] == "Taban ortası").unwrap().clone();
    for named in [true, false] {
        let (doc, b) = drawn(&case, 100, &["kentos-uni.shx", "kentos-big.shx"], None);
        let mut raw = serde_json::to_value(doc.to_snapshot()).unwrap();
        let layer = raw["entities"][0]["layerId"].clone();
        let mut style = json!({"id": "olcu", "name": "Ölçü", "height": 2.5});
        if named {
            style["textStyle"] = json!("shx");
        }
        raw["settings"]["dimensionStyles"] = json!([style]);
        raw["entities"] = json!([{"kind": "dimension", "id": 1, "layerId": layer, "attrs": {},
            "a": {"x": 6.0, "y": 18.0}, "b": {"x": 16.0, "y": 18.0}, "offset": 2.0, "height": 2.5,
            "dimStyle": "olcu", "text": "SU", "lineWeight": 0.35}]);
        let sheets = doc.sheets().cloned();
        let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
        doc.set_sheets(sheets).unwrap();
        let mut spatial = Spatial::new();
        spatial.reload(&doc);
        let render = book::base_inputs(&doc, "Test", "2026-11-06");
        let list = display::display_list(&b, "ground", &render).unwrap();
        let map = list.prims.iter().find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None }).unwrap();
        let labels = kentos_plot::labels::map_labels(&doc, &spatial, map);
        let value = labels.iter().find(|l| l.text == "SU").expect("the value's label");
        if !named {
            assert!(value.shx.is_none(), "no text style: the typeface, as before");
            continue;
        }
        let (paths, weight) = value.shx.as_ref().expect("drawn in the SHX font");
        assert!((weight - 0.35).abs() < 1e-12, "the dimension's weight");
        let (dx, dy) = (value.at.x - 10.0, value.at.y - 20.0);
        let want = case["paths"].as_array().unwrap();
        assert_eq!(paths.len(), want.len());
        for (got, want) in paths.iter().zip(want) {
            let want = want.as_array().unwrap();
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(want) {
                assert!((g[0] - (w[0].as_f64().unwrap() + dx)).abs() < 1e-9 && (g[1] - (w[1].as_f64().unwrap() + dy)).abs() < 1e-9,
                    "{g:?} {w} + ({dx}, {dy})");
            }
        }
        let (vector, texts) = vectors(&doc, &b, 100.);
        assert_eq!(texts, 0, "no substitute face written for the value");
        assert!(vector.iter().any(|p| p.stroke.as_ref().is_some_and(|s| s.cap == kentos_sheet::LineCap::Round)));
        assert!(findings(&doc, &b, "text_shx_unsupported").is_empty() && findings(&doc, &b, "text_font_substituted").is_empty());
        // Its measured value (the test font has no digits): an error naming the character, the PDF refused.
        let mut raw = serde_json::to_value(doc.to_snapshot()).unwrap();
        raw["entities"][0].as_object_mut().unwrap().remove("text");
        let sheets = doc.sheets().cloned();
        let mut measured = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
        measured.set_sheets(sheets).unwrap();
        let errors = findings(&measured, &b, "text_shx_unsupported");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("Ölçü değeri") && errors[0].message.contains("U+0031"), "{}", errors[0].message);
        let refused = book::book_pdf(&measured, &[], "Test", "2026-11-06");
        assert_eq!(refused.err().map(|e| e.code).as_deref(), Some("text_shx_unsupported"));
    }
}
