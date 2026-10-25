//! The rows round a bound layer's part in the book's PDF (docs/adr/0636 §2): the headless plot reads the band as the
//! desktop does, so the labels near a part's edge sit alike; one it cannot read is said.

use kentos_contracts::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_interaction::spatial::Spatial;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Bant",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    Document::from_snapshot(DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup"))
        .expect("setup")
}

#[test]
fn a_part_s_band_holds_room_in_the_book_and_one_not_read_is_said() {
    // komsu.gpkg: A with a bump, B and C beside it; the area holds A alone (bound_sources_cases.py's K_AREA).
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/geopackage/v1/komsu.gpkg");
    let file = std::env::temp_dir().join(format!("kentos-plot-bant-{}.gpkg", std::process::id()));
    std::fs::copy(&src, &file).expect("copied");
    let mut d = doc();
    let linked = registry::run(
        &mut ExecutionContext::new(&mut d),
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel",
                          "extent": [486_990.0, 4_419_990.0, 487_009.0, 4_420_030.0] } }),
    )
    .expect("runs");
    assert_eq!(linked["status"], "completed", "{linked}");
    let mut spatial = Spatial::new();
    spatial.reload(&d);
    let notes = kentos_plot::book::with_bands(&d, &mut spatial);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(spatial.ghost_total(), 2, "B and C hold room");
    // The file written again by someone else: not the version the layer matches, said in words.
    std::fs::write(&file, b"degisti").expect("written");
    let mut spatial = Spatial::new();
    spatial.reload(&d);
    let notes = kentos_plot::book::with_bands(&d, &mut spatial);
    assert_eq!(spatial.ghost_total(), 0);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("komşular okunamadı"), "{notes:?}");
    let _ = std::fs::remove_file(&file);
}
