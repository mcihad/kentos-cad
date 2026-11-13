//! ADR 1126 §3: a table that follows a table style prints the style's fill, colour and alignment where it sets none
//! itself, its own cell styles over the style's; a table without a style is as before.
use kentos_domain::Document;
use kentos_scene_query::Spatial;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    project::project_sheets,
};
use serde_json::{Value, json};

fn drawn(table_style: Option<&str>, own: Value) -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    raw["settings"]["tableStyles"] = json!([{"id":"ofis","name":"OFIS",
        "title":{"fill":"#eeeeee","align":"center"},"header":{"color":"#c0392b","fill":"#dddddd"},
        "data":{"align":"right","color":"#2244dd"}}]);
    let mut t = json!({"kind":"table","id":1,"attrs":{},"p":{"x":-2.0,"y":1.5},"rotation":0.0,"height":0.25,
        "rows":[0.6,0.5,0.5],"columns":[2.0,2.0],"cells":[["Liste",""],["Ada","Alan"],["101","512"]],
        "merges":[{"row":0,"col":0,"rows":1,"cols":2}],"header":true,"styles":own});
    if let Some(s) = table_style {
        t["tableStyle"] = json!(s);
    }
    t["layerId"] = raw["entities"][0]["layerId"].clone();
    raw["entities"] = json!([t]);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(50);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    (doc, book)
}

fn labels(doc: &Document, book: &SheetBook) -> Vec<kentos_plot::maps::MapLabel> {
    let render = kentos_plot::book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list.prims.iter().find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None }).unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    kentos_plot::labels::map_labels(doc, &spatial, map)
}

#[test]
fn a_table_prints_its_styles_fills_and_colours_under_its_own() {
    let (doc, book) = drawn(Some("ofis"), json!([]));
    let found = labels(&doc, &book);
    let fills: Vec<&str> = found.iter().filter(|l| l.fill.is_some()).map(|l| l.color.as_str()).collect();
    assert!(fills.contains(&"#eeeeee") && fills.contains(&"#dddddd"), "{fills:?}");
    let colour = |words: &str| found.iter().find(|l| l.text == words).map(|l| l.color.clone());
    assert_eq!(colour("Ada").as_deref(), Some("#c0392b"), "the heading row's colour");
    assert_eq!(colour("512").as_deref(), Some("#2244dd"), "a data cell's colour");
    // Its own cell style stays over the style's.
    let (doc, book) = drawn(Some("ofis"), json!([{"row":2,"col":1,"rows":1,"cols":1,"color":"#00aa00"}]));
    let found = labels(&doc, &book);
    assert_eq!(found.iter().find(|l| l.text == "512").map(|l| l.color.as_str()), Some("#00aa00"));
    assert_eq!(found.iter().find(|l| l.text == "101").map(|l| l.color.as_str()), Some("#2244dd"));
    // No style: no fill, the layer's colour.
    let (doc, book) = drawn(None, json!([]));
    let found = labels(&doc, &book);
    assert!(found.iter().all(|l| l.fill.is_none()));
    assert_ne!(found.iter().find(|l| l.text == "512").map(|l| l.color.as_str()), Some("#2244dd"));
    kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
}

// ADR 1127 §1: a table style's heights, margins and borders print as fixtures/table/v1/styled-plot.json (written by
// scripts/fixtures/table_style_plot_cases.py from the ADR, without KentOS code) has them, at 1:100 and 1:500.

fn styled_doc(case: &Value, style: &Value, follow: bool) -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    raw["settings"]["plotScale"] = case["scale"].clone();
    raw["settings"]["tableStyles"] = json!([style]);
    let mut t = case["table"].clone();
    t["layerId"] = raw["entities"][0]["layerId"].clone();
    if !follow {
        t.as_object_mut().unwrap().remove("tableStyle");
    }
    raw["entities"] = json!([t]);
    let view = &mut page["sheets"][0]["items"][0]["kind"]["view"];
    view["center"] = json!({"x": case["center"][0], "y": case["center"][1]});
    view["scale"] = json!(case["scale"].as_f64().unwrap() as u32);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    (doc, book)
}

/// The map's printed paths with a stroke and two points (a table's lines), and its CPU picture (8 px a paper mm: the thinnest border two pixels).
fn printed(doc: &Document, book: &SheetBook) -> (Vec<kentos_sheet::pdf::MapPath>, kentos_plot::MapImage) {
    let render = kentos_plot::book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list.prims.iter().find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None }).unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = kentos_plot::book::library_of(doc);
    let c = map.view.center.unwrap();
    let src = kentos_plot::maps::MapSource {
        doc,
        store: spatial.store(),
        library: &library,
        origin: kentos_geometry_core::Vec2::new(c.x, c.y),
        map: None,
        level: None,
        phase: None,
        hidden: &[],
        pens: &[],
        mono: false,
        options: &[],
    };
    let content = kentos_plot::maps::vector_content(
        &src,
        &map.layers,
        &map.layer_overrides,
        map.view.scale,
        map.extent.unwrap(),
        &[],
    )
    .unwrap();
    let kentos_sheet::pdf::MapContent::Vector(v) = content else { panic!("a vector map") };
    let paths = v
        .layers
        .into_iter()
        .flat_map(|l| l.paths)
        .filter(|p| p.stroke.is_some() && p.points.len() == 2)
        .collect();
    let image = kentos_plot::render_map_image(&src, map, &[], [3120, 1840], None).unwrap();
    (paths, image)
}

fn point(v: &Value) -> [f64; 2] {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

/// The printed paths from `a` to `b` (either way), their points within a nanometre.
fn path_on(paths: &[kentos_sheet::pdf::MapPath], a: [f64; 2], b: [f64; 2]) -> Vec<&kentos_sheet::pdf::MapPath> {
    let near = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9;
    paths
        .iter()
        .filter(|p| (near(p.points[0], a) && near(p.points[1], b)) || (near(p.points[0], b) && near(p.points[1], a)))
        .collect()
}

/// Whether the picture has an opaque pixel within 40 of `rgb` in every channel.
fn has_colour(image: &kentos_plot::MapImage, rgb: [u8; 3]) -> bool {
    image.rgba.chunks_exact(4).any(|p| (0..3).all(|i| p[i].abs_diff(rgb[i]) <= 40) && p[3] > 200)
}

#[test]
fn a_table_prints_its_styles_heights_margins_and_borders_at_two_scales() {
    let fx: Value = serde_json::from_str(include_str!("../../../../../fixtures/table/v1/styled-plot.json")).unwrap();
    let style = &fx["tableStyle"];
    for case in fx["cases"].as_array().unwrap() {
        let scale = case["scale"].as_f64().unwrap();
        let (doc, book) = styled_doc(case, style, true);
        // Its words: where the reference puts them, at their kind's height on the paper.
        let found = labels(&doc, &book);
        for want in case["labels"].as_array().unwrap() {
            let text = want["text"].as_str().unwrap();
            let got = found
                .iter()
                .find(|l| l.text == text)
                .unwrap_or_else(|| panic!("1:{scale} {text}: {found:?}"));
            let at = point(&want["at"]);
            assert!(
                (got.at.x - at[0]).abs() < 1e-9 && (got.at.y - at[1]).abs() < 1e-9,
                "1:{scale} {text}: {:?} {at:?}",
                got.at
            );
            assert!((got.size - want["size"].as_f64().unwrap()).abs() < 1e-9, "1:{scale} {text}: {}", got.size);
        }
        let (paths, image) = printed(&doc, &book);
        // The table's own lines: the layer's look, each once.
        let plain = case["plain"].as_array().unwrap();
        let first = path_on(&paths, point(&plain[0][0]), point(&plain[0][1]));
        assert_eq!(first.len(), 1, "1:{scale}: {paths:?}");
        let own = first[0].stroke.clone().unwrap();
        assert!((own.width - 0.25).abs() < 1e-12, "the layer's weight: {own:?}");
        for line in plain {
            let on = path_on(&paths, point(&line[0]), point(&line[1]));
            assert_eq!(on.len(), 1, "1:{scale} plain {line:?}");
            assert_eq!(on[0].stroke.as_ref().unwrap(), &own, "1:{scale} plain {line:?}");
        }
        // Its borders: their kind's weight and colour, the table's where the kind gives none; each once.
        let borders = case["borders"].as_array().unwrap();
        for b in borders {
            let on = path_on(&paths, point(&b["line"][0]), point(&b["line"][1]));
            assert_eq!(on.len(), 1, "1:{scale} border {b}: {paths:?}");
            let s = on[0].stroke.as_ref().unwrap();
            let width = b["look"][0].as_f64().unwrap_or(own.width);
            let colour = b["look"][1].as_str().unwrap_or(&own.color);
            assert!((s.width - width).abs() < 1e-12, "1:{scale} border {b}: {s:?}");
            assert_eq!(s.color.to_lowercase(), colour.to_lowercase(), "1:{scale} border {b}");
        }
        assert_eq!(paths.len(), plain.len() + borders.len(), "1:{scale}: no other line");
        // The CPU picture draws the borders' colours.
        assert!(has_colour(&image, [0xc0, 0x39, 0x2b]) && has_colour(&image, [0x22, 0x44, 0xdd]), "1:{scale}");
        kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();

        // The same table without its style: the table's height, half of it in from the left, its lines its own.
        let (doc, book) = styled_doc(case, style, false);
        let found = labels(&doc, &book);
        let h = case["table"]["height"].as_f64().unwrap();
        let x0 = case["table"]["p"]["x"].as_f64().unwrap();
        let data = found.iter().find(|l| l.text == "101").unwrap();
        assert!((data.at.x - (x0 + 0.5 * h)).abs() < 1e-9 && (data.size - h * 1000.0 / scale).abs() < 1e-9);
        let (paths, image) = printed(&doc, &book);
        assert!(paths.iter().all(|p| p.stroke.as_ref() == Some(&own)), "1:{scale}: every line the table's");
        assert!(!has_colour(&image, [0xc0, 0x39, 0x2b]) && !has_colour(&image, [0x22, 0x44, 0xdd]), "1:{scale}");
    }
}
