//! The atlas on a large layer (docs/adr/0638 §4): 100 000 parcels under a coverage layer of 100 sheet squares, a page
//! each; the plan and the PDF timed. Release: `cargo test --release -p kentos-plot --test all atlas_timing -- --ignored
//! --nocapture`.

use std::time::Instant;

use kentos_plot::atlas::{AtlasInput, plan};
use serde_json::{Value, json};

use crate::atlas_drawing::{ORIGIN, sheet_book};

/// `side` × `side` parcels of 10 × 10 m, and the coverage layer `pafta`: squares of `per` × `per` parcels.
fn large(side: usize, per: usize) -> kentos_domain::Document {
    let mut entities: Vec<Value> = Vec::with_capacity(side * side + (side / per).pow(2));
    for i in 0..side * side {
        let (c, r) = ((i % side) as f64, (i / side) as f64);
        let (x, y) = (ORIGIN.0 + c * 10.0, ORIGIN.1 + r * 10.0);
        entities.push(json!({
            "kind": "polygon", "id": entities.len() + 1, "layerId": "parsel",
            "pts": [{"x": x, "y": y}, {"x": x + 10.0, "y": y}, {"x": x + 10.0, "y": y + 10.0}, {"x": x, "y": y + 10.0}],
            "attrs": {"Ada": format!("A{}", i / side + 1), "Parsel": (i + 1).to_string(), "Durum": "aktif"}
        }));
    }
    let n = side / per;
    let w = per as f64 * 10.0;
    for i in 0..n * n {
        let (c, r) = ((i % n) as f64, (i / n) as f64);
        let (x, y) = (ORIGIN.0 + c * w, ORIGIN.1 + r * w);
        entities.push(json!({
            "kind": "polygon", "id": entities.len() + 1, "layerId": "pafta",
            "pts": [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + w}, {"x": x, "y": y + w}],
            "attrs": {"Ada": "P", "Parsel": (i + 1).to_string(), "Durum": "aktif"}
        }));
    }
    let layer = |id: &str, name: &str, color: &str| {
        json!({"id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
               "style": {"color": color, "lineType": "continuous", "lineWeight": 0.35}, "children": []})
    };
    let drawing = json!({
        "format": "kentos.document", "version": 1, "name": "Büyük atlas",
        "settings": {"srid": 5253, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": ORIGIN.0, "y": ORIGIN.1},
        "layers": [layer("parsel", "Parsel", "#0090FF"), layer("pafta", "Pafta", "#E5484D")],
        "activeLayer": "parsel", "entities": entities, "styles": {"items": [], "categories": []}
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
    let mut book = sheet_book();
    if let Some(a) = book.sheets[0].atlas.as_mut() {
        a.layer = "pafta".into();
    }
    // The page's table of parcels reads the coverage layer; this run times the maps.
    book.sheets[0].items.retain(|i| i.id != "parseller");
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    doc
}

#[test]
#[ignore]
fn an_atlas_on_a_hundred_thousand_parcels() {
    let t = Instant::now();
    let doc = large(320, 32);
    println!("çizim: {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    let input = AtlasInput {
        sheet: "atlas-paftasi",
        user: "kabul",
        date: "2026-10-25",
        ..AtlasInput::default()
    };
    let t = Instant::now();
    let planned = plan(&doc, &input).expect("a plan");
    println!(
        "plan: {:.0} ms, {} sayfa",
        t.elapsed().as_secs_f64() * 1e3,
        planned.pages.len()
    );
    let t = Instant::now();
    let made = planned.pdf(&doc).expect("a PDF");
    println!(
        "PDF: {:.0} ms, {} bayt",
        t.elapsed().as_secs_f64() * 1e3,
        made.bytes.len()
    );
    if let Some(out) = std::env::var_os("KENTOS_ATLAS_OUT") {
        std::fs::write(out, &made.bytes).expect("writes");
    }
    assert_eq!(planned.pages.len(), 100);
}

/// The same atlas read from its source (docs/adr/0638 §3): `.run/perf/atlas-100k.gpkg` (100 000 parcels and 100 sheet
/// squares, written by GDAL; the ADR says how), the squares bound whole as the coverage, the parcels as a part of one
/// square; each page reads its square's parcels from the file.
#[test]
#[ignore]
fn an_atlas_read_from_its_source() {
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.run/perf/atlas-100k.gpkg");
    if !file.exists() {
        println!(
            "{} yok: ADR 0638'in Doğrulama'sındaki betikle yazın",
            file.display()
        );
        return;
    }
    let dir = std::env::temp_dir().join(format!("kentos-atlas-kaynak-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let gpkg = dir.join("atlas.gpkg");
    std::fs::copy(&file, &gpkg).expect("copied");
    let layer = |id: &str, name: &str| {
        json!({"id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
               "style": {"color": "#555555", "lineType": "continuous", "lineWeight": 0.35}, "children": []})
    };
    let drawing = json!({
        "format": "kentos.document", "version": 1, "name": "Kaynaktan atlas",
        "settings": {"srid": 5253, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": ORIGIN.0, "y": ORIGIN.1}, "layers": [layer("yol", "Yol")], "activeLayer": "yol",
        "entities": [], "styles": {"items": [], "categories": []}
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
    let mut call = |name: &str, input: Value| {
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc),
            name,
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let t = Instant::now();
    let pafta = call(
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": gpkg, "table": "pafta" } }),
    )["output"]["link"]["layer"]
        .clone();
    call(
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": gpkg, "table": "parsel",
                          "extent": [ORIGIN.0, ORIGIN.1, ORIGIN.0 + 320.0, ORIGIN.1 + 320.0] } }),
    );
    println!("bağlama: {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    let mut book = sheet_book();
    if let Some(a) = book.sheets[0].atlas.as_mut() {
        a.layer = pafta.as_str().unwrap_or_default().to_owned();
    }
    book.sheets[0].items.retain(|i| i.id != "parseller");
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    let input = AtlasInput {
        sheet: "atlas-paftasi",
        coverage: kentos_contracts::AtlasCoverage::Source,
        user: "kabul",
        date: "2026-10-25",
        ..AtlasInput::default()
    };
    let t = Instant::now();
    let planned = plan(&doc, &input).expect("a plan");
    println!(
        "plan (kaynaktan): {:.0} ms, {} sayfa",
        t.elapsed().as_secs_f64() * 1e3,
        planned.pages.len()
    );
    let t = Instant::now();
    let made = planned.pdf(&doc).expect("a PDF");
    println!(
        "PDF (kaynaktan): {:.0} ms, {} bayt; okunan {:?}",
        t.elapsed().as_secs_f64() * 1e3,
        made.bytes.len(),
        made.read
            .iter()
            .map(|r| (r.table.clone(), r.rows, r.pages))
            .collect::<Vec<_>>()
    );
    assert_eq!(planned.pages.len(), 100);
    std::fs::remove_dir_all(&dir).ok();
}
