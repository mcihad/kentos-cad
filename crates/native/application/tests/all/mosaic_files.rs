//! `cad.rasters.mosaic`'s files (docs/adr/0517 §4): the VRTs the command writes for the mosaics GDAL's own are
//! made of (fixtures/raster/v1/mosaic/gdal-*.vrt). `KENTOS_WRITE_VRT=1` writes them beside the files as
//! kentos-*.vrt, which `scripts/fixtures/raster_mosaic_cases.py --check` opens with GDAL (their pixels must be GDAL's
//! own VRT's) and `kentos-formats`' test `raster_mosaic` reads; without it they are written to a folder of the test's
//! and read back.

use std::path::PathBuf;

use kentos_commands::{CommandResult, MosaicOrder, MosaicResolution, RastersMosaic};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, rasters_mosaic};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1/mosaic")
}

fn doc() -> Document {
    let setup = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "Mozaik",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "altlik", "name": "Altlık", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "altlik", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

#[test]
fn mosaic_files() {
    let write = std::env::var_os("KENTOS_WRITE_VRT").is_some();
    let out = if write {
        folder()
    } else {
        let d = std::env::temp_dir().join(format!("kentos-mosaic-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    // Top first: the reverse of the files gdalbuildvrt was given (bottom first).
    let mosaics: [(&str, &[&str], MosaicResolution); 3] = [
        ("dem-verilen", &["dem-a.tif", "dem-b.tif", "dem-c.tif"], MosaicResolution::Highest),
        ("dem-kaba", &["dem-e.tif", "dem-c.tif", "dem-b.tif", "dem-a.tif"], MosaicResolution::Lowest),
        ("orto", &["orto-2023.tif", "orto-2021.tif", "orto-2019.tif"], MosaicResolution::Highest),
    ];
    for (name, files, resolution) in mosaics {
        let path = out.join(format!("kentos-{name}.vrt"));
        let input = RastersMosaic {
            sources: files.iter().map(|f| folder().join(f).display().to_string()).collect(),
            path: path.display().to_string(),
            order: MosaicOrder::Given,
            resolution,
            pixel_size: None,
            nodata: None,
            layer: None,
            overwrite: Some(true),
            drawing: None,
            expected_revision: None,
        };
        let mut doc = doc();
        match rasters_mosaic::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { output, .. } => {
                let text = std::fs::read_to_string(&path).unwrap();
                assert_eq!(text.len() as u64, output.bytes);
                let v = kentos_formats::raster::vrt::parse(&text).unwrap();
                assert_eq!((v.width, v.height), (output.width, output.height));
                // Beside its files the VRT names them relatively, so that it moves with them.
                if write {
                    assert!(v.bands[0].sources.iter().all(|s| s.relative), "{name}");
                }
            }
            other => panic!("{name}: {other:?}"),
        }
    }
    if !write {
        let _ = std::fs::remove_dir_all(&out);
    }
}
