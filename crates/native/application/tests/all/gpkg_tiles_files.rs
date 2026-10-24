//! `cad.rasters.gpkgread` and `cad.rasters.gpkgwrite`'s files (docs/adr/0556 §6, 0627 §6): the GeoTIFFs read from the
//! tiles of GDAL's GeoPackages (pictures of PNG, JPEG and WebP, gridded coverages), the levels a raster linked to a table
//! reads (each written as a GeoTIFF), and the GeoPackages written from GDAL's GeoTIFFs (pictures as PNG and lossless
//! WebP, heights as coverages). `KENTOS_WRITE_GPKGTILES=1` writes them to `fixtures/gpkg-tiles/v1/out` with a report
//! (`read.json`, `link.json`, `write.json`), which `scripts/fixtures/gpkg_tiles_cases.py --check` opens with GDAL; without
//! it they are written to a folder of the test's. Either way the test reads every written GeoPackage back through the
//! read command and holds its finest zoom to the source's own values.

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, Entity, GpkgTileFormat, GpkgTilesMode, RasterSample, RastersGpkgRead,
    RastersGpkgWrite,
};
use kentos_domain::{Document, Slot};
use kentos_formats::raster::Samples;
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_native_application::ExecutionContext;
use kentos_native_application::gpkg_tiles::{read, write};
use kentos_native_application::raster_files::{self, Bytes};
use serde_json::{Value, json};

/// A raster read from a GeoPackage: the file, the zoom, an area, the output's name.
type ReadCase = (&'static str, Option<u32>, Option<[f64; 4]>, &'static str);
/// A raster written: its GeoTIFF, the table, its size, bands, sample, place and system, the tiles' format.
type WriteCase = (
    &'static str,
    &'static str,
    u32,
    u32,
    u32,
    &'static str,
    [f64; 6],
    u32,
    Option<GpkgTileFormat>,
);

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/gpkg-tiles/v1")
}

fn doc_in(srid: u32, entities: Value) -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Karolar",
        "settings": { "srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
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

fn doc(entities: Value) -> Document {
    doc_in(5256, entities)
}

/// A raster file's whole level 0: its size, bands, samples and nodata.
fn values(path: &Path) -> (u32, u32, u32, Samples, Option<f64>) {
    let mut opened = raster_files::open(Bytes::file(path).unwrap()).unwrap();
    let info = opened.reader.info.clone();
    let region = opened
        .region(0, 0, info.width, info.height)
        .unwrap()
        .unwrap();
    (
        info.width,
        info.height,
        info.bands,
        region.samples,
        info.nodata,
    )
}

/// Samples written as a tiled GeoTIFF (the test's own copy of a level a linked raster reads).
fn write_tif(
    path: &Path,
    (w, h): (u32, u32),
    bands: u32,
    samples: &Samples,
    affine: [f64; 6],
    srid: u32,
    nodata: Option<f64>,
) {
    let image = Image {
        width: w,
        height: h,
        bands,
        sample: samples.kind(),
        alpha: bands == 4,
        geo: Some(Geo {
            affine,
            epsg: (srid != 0).then_some(srid),
            geographic: false,
        }),
        nodata,
    };
    let (mut writer, head) = Writer::new(vec![image], 6).unwrap();
    let mut file = head;
    let side = 256usize;
    let b = bands as usize;
    for ty in 0..h.div_ceil(256) {
        for tx in 0..w.div_ceil(256) {
            let mut t = Samples::filled(samples.kind(), side * side * b, 0.0);
            let (x0, y0) = (tx as usize * side, ty as usize * side);
            let across = side.min(w as usize - x0);
            for v in 0..side.min(h as usize - y0) {
                t.copy_run(
                    v * side * b,
                    samples,
                    ((y0 + v) * w as usize + x0) * b,
                    across * b,
                );
            }
            file.extend_from_slice(&writer.tile(0, tx, ty, &t).unwrap());
        }
    }
    let (dirs, head) = writer.finish().unwrap();
    file.extend_from_slice(&dirs);
    file[..head.len()].copy_from_slice(&head);
    std::fs::write(path, file).unwrap();
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
    let reads: [ReadCase; 24] = [
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
        ("jpeg.gpkg", None, None, "jpeg.tif"),
        ("webp-gercek.gpkg", None, None, "webp.tif"),
        ("karisik.gpkg", None, None, "karisik.tif"),
        ("kapsam-float.gpkg", None, None, "kapsam-float.tif"),
        ("kapsam-float.gpkg", Some(1), None, "kapsam-float-1.tif"),
        ("kapsam-u16.gpkg", None, None, "kapsam-u16.tif"),
        ("kapsam-i16.gpkg", None, None, "kapsam-i16.tif"),
        ("kapsam-olcekli.gpkg", None, None, "kapsam-olcekli.tif"),
        ("kapsam-i16-eksi.gpkg", None, None, "kapsam-i16-eksi.tif"),
        ("kapsam-hassas.gpkg", None, None, "kapsam-hassas.tif"),
        ("kapsam-32767.gpkg", None, None, "kapsam-32767.tif"),
        (
            "kapsam-kucuk-null.gpkg",
            None,
            None,
            "kapsam-kucuk-null.tif",
        ),
        (
            "kapsam-gecersiz-null.gpkg",
            None,
            None,
            "kapsam-gecersiz-null.tif",
        ),
        (
            "kapsam-float-olcek.gpkg",
            None,
            None,
            "kapsam-float-olcek.tif",
        ),
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
            link: None,
            drawing: None,
            expected_revision: None,
        };
        let mut d = doc(json!([]));
        let CommandResult::Completed { output: done, .. } =
            read::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("{file}: not read");
        };
        let (w, h, bands, samples, nodata) = values(&output);
        assert_eq!(
            (w, h, bands, samples.kind(), nodata),
            (
                done.width,
                done.height,
                done.bands,
                done.sample,
                done.nodata
            ),
            "{name}"
        );
        read_report.push(json!({
            "source": file, "output": name, "zoom": done.zoom, "srid": done.srid,
            "origin": [done.affine[0], done.affine[3]], "pixel": [done.affine[1], -done.affine[5]],
            "defaultWindow": area.is_none(), "bands": done.bands, "sample": done.sample.name(),
            "nodata": done.nodata,
        }));
    }
    // Rasters linked to their tables: every level the reader reads (a zoom's tiles, or worked out) as a GeoTIFF.
    let mut link_report = Vec::new();
    for (file, srid, stem) in [
        ("orto.gpkg", 5256, "bagli-orto"),
        ("kapsam-float.gpkg", 5256, "bagli-kapsam"),
        ("merkator.gpkg", 3857, "bagli-merkator"),
        ("jpeg.gpkg", 5256, "bagli-jpeg"),
        ("kapsam-u16.gpkg", 5256, "bagli-u16"),
    ] {
        let mut d = doc_in(srid, json!([]));
        let CommandResult::Completed { output: done, .. } = read::execute(
            &mut ExecutionContext::new(&mut d),
            RastersGpkgRead {
                path: src.join(file).display().to_string(),
                table: None,
                zoom: None,
                area: None,
                output: None,
                layer: Some("altlik".into()),
                overwrite: None,
                link: Some(true),
                drawing: None,
                expected_revision: None,
            },
        ) else {
            panic!("{file}: not linked");
        };
        assert!(done.linked && done.path.is_none(), "{file}");
        let uid = done.uid.as_deref().expect("added");
        let slot = d
            .slot_of(kentos_domain::Uuid::parse_str(uid).unwrap())
            .unwrap();
        let Some(Entity::Raster(r)) = d.get(slot) else {
            panic!("{file}: no raster")
        };
        let fields = r.raster.clone();
        let tiles = fields.tiles.clone().expect("a tile table");
        assert_eq!(
            (fields.width, fields.height, tiles.zoom),
            (done.width, done.height, done.zoom)
        );
        let mut opened = raster_files::open_tiles(&src.join(file), &fields).unwrap();
        let blocks = opened.reader.tile_blocks().expect("tile blocks");
        let levels = opened.reader.levels.clone();
        for (k, lv) in levels.iter().enumerate() {
            // Every level through the product's own reading: level 0's tiles, a coarser zoom's, or worked out.
            let region = opened
                .region_at(k, 0, 0, lv.width, lv.height)
                .unwrap()
                .expect("the level's pixels");
            let f = f64::from(1u32 << k);
            let a = fields.affine;
            let affine = [a[0], a[1] * f, 0.0, a[3], 0.0, a[5] * f];
            let name = format!("{stem}-{k}.tif");
            write_tif(
                &out.join(&name),
                (lv.width, lv.height),
                fields.bands,
                &region.samples,
                affine,
                srid,
                opened.reader.nodata,
            );
            let entry = match blocks.levels.get(k).copied().flatten() {
                Some((t, _, _)) => json!({
                    "source": file, "table": tiles.table, "output": name, "level": k, "zoom": t.zoom.zoom,
                    "x": t.x, "y": t.y, "bands": fields.bands, "sample": fields.sample.name(),
                    "nodata": opened.reader.nodata,
                }),
                None => json!({
                    "source": file, "table": tiles.table, "output": name, "level": k, "computed": true,
                    "bands": fields.bands, "sample": fields.sample.name(),
                }),
            };
            link_report.push(entry);
        }
    }
    // GeoPackages written from GDAL's GeoTIFFs.
    let g1 = [487200.0, 1.0, 0.0, 4420600.0, 0.0, -1.0];
    let writes: [WriteCase; 17] = [
        (
            "rgb.tif",
            "karo",
            700,
            300,
            3,
            "u8",
            [487200.0, 1.5, 0.0, 4420600.0, 0.0, -1.5],
            5256,
            None,
        ),
        (
            "rgba.tif",
            "karo",
            300,
            200,
            4,
            "u8",
            [487200.0, 0.5, 0.0, 4420600.0, 0.0, -0.5],
            5256,
            None,
        ),
        (
            "gri.tif",
            "karo",
            256,
            256,
            1,
            "u8",
            [487200.0, 0.25, 0.0, 4420600.0, 0.0, -0.25],
            5256,
            None,
        ),
        (
            "gri-alfa.tif",
            "karo",
            130,
            90,
            2,
            "u8",
            [487200.0, 2.0, 0.0, 4420600.0, 0.0, -3.0],
            5256,
            None,
        ),
        ("nodata.tif", "karo", 120, 100, 1, "u8", g1, 5256, None),
        (
            "sistemsiz.tif",
            "karo",
            64,
            64,
            3,
            "u8",
            [0.0, 1.0, 0.0, 64.0, 0.0, -1.0],
            0,
            None,
        ),
        (
            "cografi.tif",
            "karo",
            100,
            100,
            3,
            "u8",
            [32.0, 0.001, 0.0, 40.0, 0.0, -0.001],
            4326,
            None,
        ),
        (
            "bos-bir.tif",
            "karo",
            5,
            3,
            3,
            "u8",
            [487200.0, 10.0, 0.0, 4420600.0, 0.0, -10.0],
            5256,
            None,
        ),
        (
            "rgb.tif",
            "webp",
            700,
            300,
            3,
            "u8",
            [487200.0, 1.5, 0.0, 4420600.0, 0.0, -1.5],
            5256,
            Some(GpkgTileFormat::Webp),
        ),
        (
            "rgba.tif",
            "webp",
            300,
            200,
            4,
            "u8",
            [487200.0, 0.5, 0.0, 4420600.0, 0.0, -0.5],
            5256,
            Some(GpkgTileFormat::Webp),
        ),
        (
            "gri-alfa.tif",
            "webp",
            130,
            90,
            2,
            "u8",
            [487200.0, 2.0, 0.0, 4420600.0, 0.0, -3.0],
            5256,
            Some(GpkgTileFormat::Webp),
        ),
        ("dem-u16.tif", "dem", 300, 280, 1, "u16", g1, 5256, None),
        ("dem-i16.tif", "dem", 300, 280, 1, "i16", g1, 5256, None),
        (
            "dem-i16-nodata.tif",
            "dem",
            200,
            150,
            1,
            "i16",
            g1,
            5256,
            None,
        ),
        ("dem-f32.tif", "dem", 520, 300, 1, "f32", g1, 5256, None),
        ("dem-f64.tif", "dem", 100, 80, 1, "f64", g1, 5256, None),
        ("dem-i32.tif", "dem", 100, 80, 1, "i32", g1, 5256, None),
    ];
    let mut write_report = Vec::new();
    for (tif, table, w, h, bands, sample, affine, srid, format) in writes {
        let entities = json!([{
            "kind": "raster", "layerId": "altlik", "attrs": {}, "affine": affine, "width": w, "height": h,
            "bands": bands, "sample": sample, "srid": srid,
            "file": src.join("kaynak").join(tif).display().to_string(),
            "style": { "render": "gray", "bands": [1] }, "id": 1
        }]);
        let mut d = doc(entities);
        let uid = d.uid(Slot(1)).unwrap().to_string();
        let stem = tif.trim_end_matches(".tif");
        let name = if stem.starts_with(table) {
            format!("{stem}.gpkg")
        } else {
            format!("{table}-{stem}.gpkg")
        };
        let gpkg = out.join(&name);
        let input = RastersGpkgWrite {
            uid,
            path: gpkg.display().to_string(),
            table: Some(table.into()),
            mode: GpkgTilesMode::New,
            identifier: None,
            format,
            overwrite: Some(true),
            drawing: None,
        };
        let CommandResult::Completed { output: done, .. } =
            write::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("{tif}: not written");
        };
        assert_eq!(done.srid, srid);
        // The finest zoom read back through the read command holds the source's own values.
        let back = out.join(format!("geri-{}", name.replace(".gpkg", ".tif")));
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
                link: None,
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
        let (_, _, _, want, source_nodata) = values(&src.join("kaynak").join(tif));
        let (_, _, _, got, back_nodata) = values(&back);
        let (sb, n) = (bands as usize, (w * h) as usize);
        if sample == "u8" {
            let (Samples::U8(want), Samples::U8(got)) = (&want, &got) else {
                panic!("{tif}: not bytes")
            };
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
        } else {
            // A height: its value as the tile holds it (a 32-bit float tile rounds), nodata where the source has.
            let float = got.kind() == RasterSample::F32;
            for i in 0..n {
                let s = want.get(i);
                let g = got.get(i);
                if source_nodata == Some(s) {
                    assert_eq!(Some(g), back_nodata, "{tif} pixel {i}: nodata");
                } else if float {
                    assert_eq!(g, f64::from(s as f32), "{tif} pixel {i}");
                } else {
                    assert_eq!(g, s, "{tif} pixel {i}");
                }
            }
        }
        write_report.push(json!({
            "file": name, "table": table, "source": format!("kaynak/{tif}"), "bands": done.bands,
            "affine": affine, "tiles": done.tiles, "coverage": done.coverage, "sample": sample,
            "format": done.zooms[0].formats[0],
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
                format: None,
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
                link: None,
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
        for (name, report) in [
            ("read.json", &read_report),
            ("link.json", &link_report),
            ("write.json", &write_report),
        ] {
            std::fs::write(
                out.join(name),
                serde_json::to_string_pretty(report).unwrap() + "\n",
            )
            .unwrap();
        }
    } else {
        let _ = std::fs::remove_dir_all(&out);
    }
}
