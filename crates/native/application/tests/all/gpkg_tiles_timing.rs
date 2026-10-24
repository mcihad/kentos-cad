//! How long the GeoPackage tiles take (docs/adr/0627 §4): a 4096 × 4096 colour raster written as a PNG and as a lossless
//! WebP pyramid, a 4096 × 4096 height as a float coverage, the PNG table read back whole as a GeoTIFF, and a raster
//! linked to it opening and reading its first tile at the finest and the coarsest level. Release, on its own:
//! `cargo test --release -p kentos-native-application --test all gpkg_tiles_timing -- --ignored --nocapture`.

use std::path::Path;
use std::time::Instant;

use kentos_commands::{
    CommandResult, Entity, GpkgTileFormat, GpkgTilesMode, RasterSample, RastersGpkgRead,
    RastersGpkgWrite,
};
use kentos_domain::{Document, Slot};
use kentos_formats::raster::Samples;
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_native_application::ExecutionContext;
use kentos_native_application::gpkg_tiles::{read, write};
use kentos_native_application::raster_files;
use serde_json::json;

const SIDE: u32 = 4096;

fn doc(entities: serde_json::Value) -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Süre",
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

/// A tiled GeoTIFF of `bands` samples each `value(x, y, band)` gives.
fn tif(path: &Path, bands: u32, sample: RasterSample, value: impl Fn(u32, u32, u32) -> f64) {
    let image = Image {
        width: SIDE,
        height: SIDE,
        bands,
        sample,
        alpha: false,
        geo: Some(Geo {
            affine: [487200.0, 0.5, 0.0, 4420600.0, 0.0, -0.5],
            epsg: Some(5256),
            geographic: false,
        }),
        nodata: None,
    };
    let (mut writer, head) = Writer::new(vec![image], 1).expect("a writer");
    let mut file = head;
    for ty in 0..SIDE / 256 {
        for tx in 0..SIDE / 256 {
            let mut t = Samples::filled(sample, 256 * 256 * bands as usize, 0.0);
            for v in 0..256 {
                for u in 0..256 {
                    for b in 0..bands {
                        let i = ((v * 256 + u) * bands + b) as usize;
                        t.set(i, value(tx * 256 + u, ty * 256 + v, b));
                    }
                }
            }
            file.extend_from_slice(&writer.tile(0, tx, ty, &t).expect("a tile"));
        }
    }
    let (dirs, head) = writer.finish().expect("finished");
    file.extend_from_slice(&dirs);
    file[..head.len()].copy_from_slice(&head);
    std::fs::write(path, file).expect("written");
}

fn raster(path: &Path, bands: u32, sample: &str) -> Document {
    doc(json!([{
        "kind": "raster", "layerId": "altlik", "attrs": {}, "affine": [487200.0, 0.5, 0.0, 4420600.0, 0.0, -0.5],
        "width": SIDE, "height": SIDE, "bands": bands, "sample": sample, "srid": 5256,
        "file": path.display().to_string(), "style": { "render": "gray", "bands": [1] }, "id": 1
    }]))
}

fn written(d: &mut Document, path: &Path, format: Option<GpkgTileFormat>) -> u64 {
    let uid = d.uid(Slot(1)).expect("the raster").to_string();
    let r = write::execute(
        &mut ExecutionContext::new(d),
        RastersGpkgWrite {
            uid,
            path: path.display().to_string(),
            table: Some("karo".into()),
            mode: GpkgTilesMode::New,
            identifier: None,
            format,
            overwrite: Some(true),
            drawing: None,
        },
    );
    let CommandResult::Completed { output, .. } = r else {
        panic!("not written: {r:?}")
    };
    output.tiles
}

#[test]
#[ignore = "a timing, run on its own in release"]
fn gpkg_tiles_timing() {
    let dir = std::env::temp_dir().join(format!("kentos-gpkg-timing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    // A picture with texture (no flat runs a coder would make nothing of) and a smooth height.
    let rgb = dir.join("rgb.tif");
    tif(&rgb, 3, RasterSample::U8, |x, y, b| {
        f64::from((x * 7 + y * 3 + b * 85 + (x ^ y) % 17) % 256)
    });
    let dem = dir.join("dem.tif");
    tif(&dem, 1, RasterSample::F32, |x, y, _| {
        850.0 + 40.0 * (f64::from(x) / 37.0).sin() + 25.0 * (f64::from(y) / 29.0).cos()
    });
    let line = |what: &str, t: Instant, extra: String| {
        println!(
            "{what}: {:.0} ms{extra}",
            t.elapsed().as_secs_f64() * 1000.0
        );
    };
    let png = dir.join("png.gpkg");
    let t = Instant::now();
    let tiles = written(&mut raster(&rgb, 3, "u8"), &png, None);
    line(
        "4096² RGB → PNG piramidi",
        t,
        format!(
            ", {tiles} karo, {} MB",
            std::fs::metadata(&png).map_or(0, |m| m.len()) >> 20
        ),
    );
    let webp = dir.join("webp.gpkg");
    let t = Instant::now();
    let tiles = written(
        &mut raster(&rgb, 3, "u8"),
        &webp,
        Some(GpkgTileFormat::Webp),
    );
    line(
        "4096² RGB → kayıpsız WebP piramidi",
        t,
        format!(
            ", {tiles} karo, {} MB",
            std::fs::metadata(&webp).map_or(0, |m| m.len()) >> 20
        ),
    );
    let cov = dir.join("kapsam.gpkg");
    let t = Instant::now();
    let tiles = written(&mut raster(&dem, 1, "f32"), &cov, None);
    line(
        "4096² f32 → kayan noktalı kapsam",
        t,
        format!(
            ", {tiles} karo, {} MB",
            std::fs::metadata(&cov).map_or(0, |m| m.len()) >> 20
        ),
    );
    for (name, file) in [("PNG", &png), ("WebP", &webp), ("kapsam", &cov)] {
        let out = dir.join(format!("geri-{name}.tif"));
        let mut d = doc(json!([]));
        let t = Instant::now();
        let r = read::execute(
            &mut ExecutionContext::new(&mut d),
            RastersGpkgRead {
                path: file.display().to_string(),
                table: None,
                zoom: None,
                area: None,
                output: Some(out.display().to_string()),
                layer: None,
                overwrite: Some(true),
                link: None,
                drawing: None,
                expected_revision: None,
            },
        );
        assert!(matches!(r, CommandResult::Completed { .. }), "{r:?}");
        line(
            &format!("{name} tablosu → GeoTIFF (en ince düzey)"),
            t,
            String::new(),
        );
        // Linked: opening, the first tile of the finest level, the coarsest level whole.
        let mut d = doc(json!([]));
        let r = read::execute(
            &mut ExecutionContext::new(&mut d),
            RastersGpkgRead {
                path: file.display().to_string(),
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
        );
        let CommandResult::Completed { output, .. } = r else {
            panic!("not linked: {r:?}")
        };
        let uid =
            kentos_domain::Uuid::parse_str(output.uid.as_deref().expect("added")).expect("a uid");
        let Some(Entity::Raster(e)) = d.slot_of(uid).and_then(|s| d.get(s)) else {
            panic!("no raster")
        };
        let fields = e.raster.clone();
        let t = Instant::now();
        let mut opened = raster_files::open_tiles(file, &fields).expect("opens");
        line(&format!("{name}: bağlı rasterin açılışı"), t, String::new());
        let t = Instant::now();
        opened
            .region_at(0, 0, 0, 256, 256)
            .expect("read")
            .expect("pixels");
        line(
            &format!("{name}: en ince düzeyin ilk karosu"),
            t,
            String::new(),
        );
        let top = opened.reader.levels.len() - 1;
        let (w, h) = (
            opened.reader.levels[top].width,
            opened.reader.levels[top].height,
        );
        let t = Instant::now();
        opened
            .region_at(top, 0, 0, w, h)
            .expect("read")
            .expect("pixels");
        line(
            &format!("{name}: en kaba düzeyin tamamı ({w} × {h})"),
            t,
            String::new(),
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
