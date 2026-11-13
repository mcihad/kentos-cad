//! plan/cad-advanced.md CAD-ADV-11: “Çizim sırası, maskeler, saydamlık ve saç teli çizgilerinde model, viewport ve PDF
//! eşdeğerliği aransın. Ekran temasını değiştirmek çizimin atanmış veya basılacak rengini değiştirmesin.”
//! One drawing, its objects in this order: a red dashed line A (0.5 mm), a mask over its middle (an area's wipeout,
//! ADR 0310), a hairline B (weight 0, the layer's theme colour `fg`) drawn after the mask across it, and a blue line C
//! at transparency 0.5. Printed through two viewports (1:50 and 1:20) the vector PDF keeps that order, the mask is the
//! paper's white over A only, B is the device's thinnest line in the paper palette's ink, C's ink is half opaque, and A's dashes are
//! the same paper millimetres in both; the CPU picture shows the same at the mask (A gone, B there).
use kentos_domain::Document;
use kentos_plot::{book, maps::MapSource};
use kentos_scene_query::Spatial;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    pdf::MapPath,
    project::project_sheets,
};
use serde_json::{Value, json};

fn drawing(scale: u32) -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/native-preflight.json")).unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    let line = |id: u32, a: [f64; 2], b: [f64; 2], more: Value| {
        let mut e = json!({"kind": "line", "id": id, "layerId": "cizim", "attrs": {},
            "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}});
        e.as_object_mut().unwrap().extend(more.as_object().unwrap().clone());
        e
    };
    raw["entities"] = json!([
        line(1, [0.0, 1.0], [5.0, 1.0], json!({"color": "#ff0000", "lineWeight": 0.5, "lineType": "dashed"})),
        {"kind": "polygon", "id": 2, "layerId": "cizim", "attrs": {}, "color": "#00aa00",
         "pts": [{"x": 2.0, "y": 0.5}, {"x": 3.0, "y": 0.5}, {"x": 3.0, "y": 1.5}, {"x": 2.0, "y": 1.5}],
         "wipeout": {"frame": false}},
        line(3, [2.5, 0.0], [2.5, 2.0], json!({"lineWeight": 0.0})),
        line(4, [0.0, 0.2], [5.0, 0.2], json!({"color": "#0000ff", "lineWeight": 0.35, "transparency": 0.5}))
    ]);
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x": 2.5, "y": 1.0});
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    (doc, book)
}

/// The first sheet's map paths, bottom first, as the PDF receives them.
fn paths(doc: &Document, book: &SheetBook, scale: f64) -> Vec<MapPath> {
    let render = book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list.prims.iter().find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None }).unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = book::library_of(doc);
    let src = MapSource {
        doc, store: spatial.store(), library: &library, origin: kentos_geometry_core::Vec2::new(0., 0.), map: None,
        level: None, phase: None, hidden: &[], pens: &[], mono: false, options: &[],
    };
    let labels = kentos_plot::labels::map_labels(doc, &spatial, map);
    let kentos_sheet::pdf::MapContent::Vector(v) =
        kentos_plot::maps::vector_content(&src, &map.layers, &map.layer_overrides, scale, map.extent.unwrap(), &labels).unwrap()
    else {
        panic!("expected vectors")
    };
    v.layers.into_iter().flat_map(|l| l.paths).collect()
}

fn stroke_color(p: &MapPath) -> Option<&str> {
    p.stroke.as_ref().map(|s| s.color.as_str())
}

#[test]
fn order_mask_transparency_and_hairline_print_the_same_through_two_viewports() {
    let mut dashes = Vec::new();
    for scale in [50u32, 20] {
        let (doc, b) = drawing(scale);
        let before = doc.to_snapshot_v2();
        let got = paths(&doc, &b, f64::from(scale));
        let first = |f: &dyn Fn(&MapPath) -> bool| got.iter().position(|p| f(p)).unwrap_or_else(|| panic!("1:{scale}: {got:#?}"));
        let a = first(&|p| stroke_color(p) == Some("#ff0000"));
        let mask = first(&|p| p.closed && p.fill.is_some());
        let hair = first(&|p| p.stroke.as_ref().is_some_and(|s| s.width == 0.0));
        let c = first(&|p| stroke_color(p).is_some_and(|c| c.starts_with("#0000ff")));
        assert!(a < mask && mask < hair && hair < c, "1:{scale}: the drawing's order, A {a} mask {mask} B {hair} C {c}");
        let m = &got[mask];
        assert_eq!(m.fill.as_deref(), Some("#ffffff"), "1:{scale}: the mask is the paper's white");
        assert!(m.stroke.is_none(), "1:{scale}: a mask without its frame draws no edge");
        // The theme's `fg` prints in the paper palette's fixed ink (`maps.rs`, the web's `paperPalette`), whatever the
        // screen's theme: no screen colour reaches the paper.
        assert_eq!(stroke_color(&got[hair]), Some("#111111"), "1:{scale}: fg prints the paper's ink");
        let alpha = stroke_color(&got[c]).unwrap();
        assert!(alpha.len() == 9 && u8::from_str_radix(&alpha[7..], 16).unwrap().abs_diff(128) <= 1,
            "1:{scale}: transparency 0.5 is half the ink: {alpha}");
        let sa = got[a].stroke.as_ref().unwrap();
        assert!((sa.width - 0.5).abs() < 1e-12, "1:{scale}: 0.5 mm on paper at any scale");
        dashes.push(sa.dash.clone());
        assert_eq!(doc.to_snapshot_v2(), before, "printing changes nothing");
    }
    assert!(!dashes[0].is_empty(), "A is dashed");
    assert_eq!(dashes[0], dashes[1], "the pattern is paper millimetres in both viewports");
}

/// The CPU picture: at the mask's centre A is gone (paper) and B, drawn after it, is ink; A outside the mask is red.
#[test]
fn the_cpu_picture_keeps_the_order_and_the_mask() {
    let (doc, _) = drawing(20);
    let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 200).unwrap();
    // The map's centre (2.5, 1.0) is the paper's (215, 125) mm (frame 20…410 × 10…240); at 1:20 a metre is 50 mm.
    let k = 200. / 25.4;
    let rgba = |x: f64, y: f64| {
        let (mx, my) = (215. + (x - 2.5) * 50., 125. - (y - 1.0) * 50.);
        let (px, py) = ((mx * k).round() as usize, (my * k).round() as usize);
        let at = (py * image.width as usize + px) * 4;
        [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2]]
    };
    // Inside the mask, off B: paper.
    assert_eq!(rgba(2.2, 1.0), [255, 255, 255], "A under the mask is covered");
    // B over the mask: dark (a hairline is a device pixel; look at the nearest).
    let near_b = (-2i32..=2).map(|d| rgba(2.5 + f64::from(d) * 0.0025, 1.3)).map(|c| c[0]).min().unwrap();
    assert!(near_b < 128, "B, drawn after the mask, is over it: {near_b}");
}

/// The picture for the eye: the drawing's PDF at 1:20, `.run/gui-evidence/cad-adv-11-sira-maske.pdf` (render it with
/// pdftoppm). Run by hand.
#[test]
#[ignore = "a picture for the eye, run by hand"]
fn order_and_mask_picture() {
    let (doc, _) = drawing(20);
    let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("cad-adv-11-sira-maske.pdf"), pdf.bytes).unwrap();
}
