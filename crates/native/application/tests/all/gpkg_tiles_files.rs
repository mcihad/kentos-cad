//! `cad.rasters.gpkgread` and `cad.rasters.gpkgwrite`'s files (docs/adr/0556 §6): the GeoTIFFs read from the tiles of
//! GDAL's GeoPackages and the GeoPackages written from GDAL's GeoTIFFs. `KENTOS_WRITE_GPKGTILES=1` writes them to
//! `fixtures/gpkg-tiles/v1/out` with a report (`read.json`, `write.json`), which
//! `scripts/fixtures/gpkg_tiles_cases.py --check` opens with GDAL; without it they are written to a folder of the
//! test's. Either way the test reads every written GeoPackage back through the read command and holds its finest zoom
//! to the source's own pixels.

use std::path::PathBuf;

use kentos_commands::{CommandResult, GpkgTilesMode, RastersGpkgRead, RastersGpkgWrite};
use kentos_domain::{Document, Slot};
use kentos_formats::raster::Samples;
use kentos_native_application::ExecutionContext;
use kentos_native_application::gpkg_tiles::{read, write};
use kentos_native_application::raster_files::{self, Bytes};
use serde_json::{Value, json};

/// A raster read from a GeoPackage: the file, the zoom, an area, the output's name.
type ReadCase = (&'static str, Option<u32>, Option<[f64; 4]>, &'static str);
/// A raster written: its GeoTIFF, the table, its size, bands, place and system.
type WriteCase = (&'static str, &'static str, u32, u32, u32, [f64; 6], u32);

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/gpkg-tiles/v1")
}

fn doc(entities: Value) -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Karolar",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "altlik", "name": "Altlık", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "altlik", "entities": entities, "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn pixels(path: &std::path::Path) -> (u32, u32, u32, Vec<u8>) {
    let mut opened = raster_files::open(Bytes::file(path).unwrap()).unwrap();
    let (w, h, bands) = (
        opened.reader.info.width,
        opened.reader.info.height,
        opened.reader.info.bands,
    );
    let region = opened.region(0, 0, w, h).unwrap().unwrap();
    let Samples::U8(v) = region.samples else {
        panic!("not bytes")
    };
    (w, h, bands, v)
}

#[test]
fn gpkg_tiles_files() {
    let keep = std::env::var_os("KENTOS_WRITE_GPKGTILES").is_some();
    let src = folder();
    let out = if keep {
        let d = src.join("out");
        std::fs::create_dir_all(&d).unwrap();
        d
    } else {
        let d = std::env::temp_dir().join(format!("kentos-gpkg-tiles-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    // Rasters read from GDAL's GeoPackages: (the file, the zoom, an area, the output's name).
    let reads: [ReadCase; 10] = [
        ("orto.gpkg", None, None, "orto-2.tif"),
        ("orto.gpkg", Some(1), None, "orto-1.tif"),
        ("orto.gpkg", Some(0), None, "orto-0.tif"),
        (
            "orto.gpkg",
            None,
            Some([487201.3, 4420020.6, 487700.9, 4420300.2]),
            "orto-kesir.tif",
        ),
        (
            "orto.gpkg",
            None,
            Some([487000.0, 4419000.0, 489900.0, 4420800.0]),
            "orto-tasan.tif",
        ),
        ("kaymis.gpkg", None, None, "kaymis-2.tif"),
        ("kaymis.gpkg", Some(0), None, "kaymis-0.tif"),
        ("gri.gpkg", None, None, "gri.tif"),
        ("gri-saf.gpkg", None, None, "gri-saf.tif"),
        ("eksik.gpkg", None, None, "eksik.tif"),
    ];
    let mut read_report = Vec::new();
    for (file, zoom, area, name) in reads {
        let output = out.join(name);
        let input = RastersGpkgRead {
            path: src.join(file).display().to_string(),
            table: None,
            zoom,
            area,
            output: Some(output.display().to_string()),
            layer: None,
            overwrite: Some(true),
            drawing: None,
            expected_revision: None,
        };
        let mut d = doc(json!([]));
        let CommandResult::Completed { output: done, .. } =
            read::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("{file}: not read");
        };
        assert_eq!(done.bands, 4);
        let (w, h, bands, _) = pixels(&output);
        assert_eq!((w, h, bands), (done.width, done.height, 4), "{name}");
        read_report.push(json!({
            "source": file, "output": name, "zoom": done.zoom, "srid": done.srid,
            "origin": [done.affine[0], done.affine[3]], "pixel": [done.affine[1], -done.affine[5]],
            "defaultWindow": area.is_none(),
        }));
    }
    // GeoPackages written from GDAL's GeoTIFFs.
    let writes: [WriteCase; 8] = [
        (
            "rgb.tif",
            "karo",
            700,
            300,
            3,
            [487200.0, 1.5, 0.0, 4420600.0, 0.0, -1.5],
            5256,
        ),
        (
            "rgba.tif",
            "karo",
            300,
            200,
            4,
            [487200.0, 0.5, 0.0, 4420600.0, 0.0, -0.5],
            5256,
        ),
        (
            "gri.tif",
            "karo",
            256,
            256,
            1,
            [487200.0, 0.25, 0.0, 4420600.0, 0.0, -0.25],
            5256,
        ),
        (
            "gri-alfa.tif",
            "karo",
            130,
            90,
            2,
            [487200.0, 2.0, 0.0, 4420600.0, 0.0, -3.0],
            5256,
        ),
        (
            "nodata.tif",
            "karo",
            120,
            100,
            1,
            [487200.0, 1.0, 0.0, 4420600.0, 0.0, -1.0],
            5256,
        ),
        (
            "sistemsiz.tif",
            "karo",
            64,
            64,
            3,
            [0.0, 1.0, 0.0, 64.0, 0.0, -1.0],
            0,
        ),
        (
            "cografi.tif",
            "karo",
            100,
            100,
            3,
            [32.0, 0.001, 0.0, 40.0, 0.0, -0.001],
            4326,
        ),
        (
            "bos-bir.tif",
            "karo",
            5,
            3,
            3,
            [487200.0, 10.0, 0.0, 4420600.0, 0.0, -10.0],
            5256,
        ),
    ];
    let mut write_report = Vec::new();
    for (tif, table, w, h, bands, affine, srid) in writes {
        let entities = json!([{
            "kind": "raster", "layerId": "altlik", "attrs": {}, "affine": affine, "width": w, "height": h,
            "bands": bands, "sample": "u8", "srid": srid,
            "file": src.join("kaynak").join(tif).display().to_string(),
            "style": { "render": "gray", "bands": [1] }, "id": 1
        }]);
        let mut d = doc(entities);
        let uid = d.uid(Slot(1)).unwrap().to_string();
        let name = format!("karo-{}.gpkg", tif.trim_end_matches(".tif"));
        let gpkg = out.join(&name);
        let input = RastersGpkgWrite {
            uid,
            path: gpkg.display().to_string(),
            table: Some(table.into()),
            mode: GpkgTilesMode::New,
            identifier: None,
            overwrite: Some(true),
            drawing: None,
        };
        let CommandResult::Completed { output: done, .. } =
            write::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("{tif}: not written");
        };
        assert_eq!(done.srid, srid);
        // The finest zoom read back through the read command holds the source's own pixels.
        let back = out.join(format!("geri-{}.tif", tif.trim_end_matches(".tif")));
        let mut d2 = doc(json!([]));
        let CommandResult::Completed {
            output: read_back, ..
        } = read::execute(
            &mut ExecutionContext::new(&mut d2),
            RastersGpkgRead {
                path: gpkg.display().to_string(),
                table: None,
                zoom: None,
                area: None,
                output: Some(back.display().to_string()),
                layer: None,
                overwrite: Some(true),
                drawing: None,
                expected_revision: None,
            },
        )
        else {
            panic!("{tif}: not read back");
        };
        assert_eq!((read_back.width, read_back.height), (w, h), "{tif}");
        assert_eq!(read_back.affine, affine, "{tif}");
        assert_eq!(read_back.tiles_missing, 0, "{tif}");
        let (_, _, _, want) = pixels(&src.join("kaynak").join(tif));
        let (_, _, _, got) = pixels(&back);
        let (sb, n) = (bands as usize, (w * h) as usize);
        for i in 0..n {
            let g = &got[i * 4..i * 4 + 4];
            let s = &want[i * sb..i * sb + sb];
            let expected = match sb {
                1 => [s[0], s[0], s[0], 255],
                2 => [s[0], s[0], s[0], s[1]],
                3 => [s[0], s[1], s[2], 255],
                _ => [s[0], s[1], s[2], s[3]],
            };
            assert_eq!(g, expected, "{tif} pixel {i}");
        }
        write_report.push(json!({
            "file": name, "table": table, "source": format!("kaynak/{tif}"), "bands": done.bands,
            "affine": affine, "tiles": done.tiles,
        }));
        let _ = std::fs::remove_file(&back);
    }
    // A file that cannot be put in place leaves nothing half written and the place as it was: a folder where the
    // file should go (the rename fails after the file was made whole beside it).
    {
        let entities = json!([{
            "kind": "raster", "layerId": "altlik", "attrs": {}, "affine": [487200.0, 1.5, 0.0, 4420600.0, 0.0, -1.5],
            "width": 700, "height": 300, "bands": 3, "sample": "u8", "srid": 5256,
            "file": src.join("kaynak").join("rgb.tif").display().to_string(),
            "style": { "render": "rgb", "bands": [1, 2, 3] }, "id": 1
        }]);
        let mut d = doc(entities);
        let uid = d.uid(Slot(1)).unwrap().to_string();
        let blocked =
            std::env::temp_dir().join(format!("kentos-gpkg-blocked-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&blocked);
        std::fs::create_dir_all(blocked.join("karo.gpkg")).unwrap();
        let result = write::execute(
            &mut ExecutionContext::new(&mut d),
            RastersGpkgWrite {
                uid,
                path: blocked.join("karo.gpkg").display().to_string(),
                table: None,
                mode: GpkgTilesMode::New,
                identifier: None,
                overwrite: Some(true),
                drawing: None,
            },
        );
        let CommandResult::Failed { error } = result else {
            panic!("blocked write: {result:?}")
        };
        assert_eq!(error.code, "file_unwritable");
        assert!(blocked.join("karo.gpkg").is_dir());
        let left: Vec<_> = std::fs::read_dir(&blocked)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "{left:?}");
        // The same for the GeoTIFF the read command writes.
        std::fs::create_dir_all(blocked.join("orto.tif")).unwrap();
        let mut d = doc(json!([]));
        let result = read::execute(
            &mut ExecutionContext::new(&mut d),
            RastersGpkgRead {
                path: src.join("orto.gpkg").display().to_string(),
                table: None,
                zoom: None,
                area: None,
                output: Some(blocked.join("orto.tif").display().to_string()),
                layer: Some("altlik".into()),
                overwrite: Some(true),
                drawing: None,
                expected_revision: None,
            },
        );
        let CommandResult::Failed { error } = result else {
            panic!("blocked read: {result:?}")
        };
        assert_eq!(error.code, "file_unwritable");
        assert!(
            !d.can_undo(),
            "a raster was added though its file was not written"
        );
        assert_eq!(std::fs::read_dir(&blocked).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&blocked);
    }
    if keep {
        std::fs::write(
            out.join("read.json"),
            serde_json::to_string_pretty(&read_report).unwrap() + "\n",
        )
        .unwrap();
        std::fs::write(
            out.join("write.json"),
            serde_json::to_string_pretty(&write_report).unwrap() + "\n",
        )
        .unwrap();
    } else {
        let _ = std::fs::remove_dir_all(&out);
    }
}
