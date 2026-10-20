//! Raster oturt's record beyond the shared cases (docs/adr/0518): the command's own resampling of a linked raster is
//! gdalwarp's (fixtures/raster/v1/georef.json's thin plate, nearest, checked there against gdalwarp sample for sample),
//! its source is left as it was, and the record run again makes the same grid and the same file, byte for byte.

use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{CallOp, Entity};
use kentos_domain::{Document, Slot};
use kentos_formats::raster::source::open_bytes;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-georef-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

fn run(doc: &mut Document, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), "cad.rasters.georef", op, input).expect("runs")
}

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{h:016x}")
}

fn doc(file: &Path) -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Pafta",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "pafta", "name": "Pafta", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "pafta",
        "entities": [{ "kind": "raster", "id": 1, "layerId": "pafta", "attrs": {},
                       "affine": [487000.0, 0.25, 0.0, 4420100.0, 0.0, -0.25], "width": 333, "height": 257, "bands": 1,
                       "sample": "u8", "file": file, "srid": 0, "style": { "render": "gray", "bands": [1] } }],
        "styles": { "items": [], "categories": [] }
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// Band `k` (from 0) of a written GeoTIFF's level 0, and its size.
fn band(bytes: &[u8], k: usize) -> (Vec<u8>, u32, u32) {
    let mut r = open_bytes(bytes, None, 256 << 20).expect("the output reads");
    let (w, h) = (r.levels[0].width, r.levels[0].height);
    for need in r.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        r.put_block(&need, &bytes[a..b]).expect("a block");
    }
    let region = r.region(0, 0, 0, w, h).expect("level 0");
    let n = region.bands as usize;
    let out = (0..(w * h) as usize).map(|i| region.samples.get(i * n + k) as u8).collect();
    (out, w, h)
}

/// The reference's resampling with the inverse exact at every pixel (fixtures/raster/v1/georef.json: gdalwarp's own
/// samples, its grey and alpha).
fn exact(source: &Path, warp: &Value) -> (Vec<u8>, Vec<u8>) {
    use kentos_formats::raster::warp as w;
    use kentos_formats::raster::{Samples, TILE};
    use kentos_geometry_core::ops::georef::{Gcp, Method, solve};
    use kentos_geometry_core::vec2::Vec2;
    let bytes = std::fs::read(source).expect("source");
    let mut reader = open_bytes(&bytes, None, 256 << 20).expect("reads");
    let (sw, sh) = (reader.info.width, reader.info.height);
    for need in reader.needs(0, 0, 0, sw, sh) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &bytes[a..b]).expect("a block");
    }
    let v2 = |v: &Value| Vec2::new(v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0));
    let gcps: Vec<Gcp> = warp["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|p| Gcp { pixel: v2(&p["pixel"]), target: v2(&p["target"]), used: true })
        .collect();
    let g = solve(&gcps, Method::ThinPlate).expect("solved");
    let a: Vec<f64> = warp["grid"]["affine"].as_array().expect("affine").iter().filter_map(Value::as_f64).collect();
    let grid = w::Grid {
        affine: [a[0], a[1], a[2], a[3], a[4], a[5]],
        width: warp["grid"]["width"].as_u64().unwrap_or(0) as u32,
        height: warp["grid"]["height"].as_u64().unwrap_or(0) as u32,
    };
    let (ow, oh) = (grid.width as usize, grid.height as usize);
    let (mut grey, mut alpha) = (vec![0u8; ow * oh], vec![0u8; ow * oh]);
    let inverse = |p: Vec2| g.inverse(p);
    for ty in 0..grid.height.div_ceil(TILE) {
        for tx in 0..grid.width.div_ceil(TILE) {
            let (pixels, bbox) = w::source_pixels(&grid, tx, ty, 1, &inverse);
            let Some((x, y, rw, rh)) = bbox.and_then(|b| w::region_for(b, sw, sh)) else {
                continue;
            };
            let region = reader.region(0, x, y, rw, rh).expect("the region");
            let Samples::U8(v) = w::render(&pixels, &region, sw, sh, true, reader.info.sample, true, 0.0, None) else {
                panic!("8 bits")
            };
            for j in 0..TILE as usize {
                for i in 0..TILE as usize {
                    let (gx, gy) = (tx as usize * TILE as usize + i, ty as usize * TILE as usize + j);
                    if gx < ow && gy < oh {
                        let k = (j * TILE as usize + i) * 4;
                        grey[gy * ow + gx] = v[k];
                        alpha[gy * ow + gx] = v[k + 3];
                    }
                }
            }
        }
    }
    (grey, alpha)
}

#[test]
fn a_resampling_is_gdalwarps_leaves_its_source_and_is_made_again_the_same() {
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures().join("georef.json")).expect("georef.json"))
            .expect("JSON");
    let warp = &reference["warp"];
    let dir = scratch("warp");
    let source = dir.join("pafta.tif");
    std::fs::copy(fixtures().join("files").join(warp["source"].as_str().expect("source")), &source).expect("copy");
    let before = sha256_hex(&std::fs::read(&source).expect("source"));
    let mut d = doc(&source);
    let uid = d.uid(Slot(1)).expect("the raster").to_string();
    let points: Vec<Value> = warp["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|p| json!({ "pixel": p["pixel"], "target": p["target"] }))
        .collect();
    let input = json!({ "uid": uid, "method": "thinPlate", "points": points, "pixel": warp["pixel"],
                        "resampling": "nearest", "output": dir.join("cikti.tif") });
    let answer = run(&mut d, CallOp::Execute, input.clone());
    assert_eq!(answer["status"], "completed", "{answer}");
    let record = answer["output"]["raster"]["georef"].clone();
    // The grid is the reference's (the forward of the border, on multiples of the pixel).
    assert_eq!(record["output"]["affine"], warp["grid"]["affine"]);
    assert_eq!(record["output"]["width"], warp["grid"]["width"]);
    assert_eq!(record["output"]["height"], warp["grid"]["height"]);
    // The samples are gdalwarp's (grey spread to red, green and blue, alpha 0 outside the source) but where the
    // inverse between its exact knots (every 8 pixels, docs/adr/0204 §6) takes a pixel next door: a few hundredths of
    // a pixel off, a nearest sample at a pixel's edge.
    let written = std::fs::read(dir.join("cikti.tif")).expect("the output");
    let (grey, w, h) = band(&written, 0);
    let (alpha, ..) = band(&written, 3);
    assert_eq!((w, h), (299, 237));
    let (exact_grey, exact_alpha) = exact(&source, warp);
    assert_eq!(fnv(&exact_grey), warp["gray"].as_str().expect("gray"), "the exact inverse is gdalwarp's");
    assert_eq!(fnv(&exact_alpha), warp["alpha"].as_str().expect("alpha"), "the exact inverse is gdalwarp's");
    let differ = grey.iter().zip(&exact_grey).filter(|(a, b)| a != b).count()
        + alpha.iter().zip(&exact_alpha).filter(|(a, b)| a != b).count();
    // 149 of 70 863 here (grey and alpha counted apart): 0,2 %.
    assert!(differ * 200 <= grey.len(), "{differ} of {} pixels differ from gdalwarp's", grey.len());
    assert_eq!(record["output"]["digest"], json!(sha256_hex(&written)));
    // The source is as it was, and the record names it with its digest.
    assert_eq!(sha256_hex(&std::fs::read(&source).expect("source")), before);
    assert_eq!(record["source"]["digest"], json!(before));
    assert_eq!(record["source"]["file"], json!(source));
    // Done again from the record (its points are the source's pixels): the same grid, the same bytes.
    let mut again = input;
    again["output"] = json!(dir.join("yeniden.tif"));
    again["pixel"] = record["output"]["pixel"].clone();
    let second = run(&mut d, CallOp::Execute, again);
    assert_eq!(second["status"], "completed", "{second}");
    let record2 = &second["output"]["raster"]["georef"];
    assert_eq!(record2["output"]["affine"], record["output"]["affine"]);
    assert_eq!(record2["source"], record["source"]);
    assert_eq!(std::fs::read(dir.join("yeniden.tif")).expect("again"), written);
    // Undo puts back the raster as it was before the second run.
    assert!(d.undo().is_some());
    match d.get(Slot(1)) {
        Some(Entity::Raster(r)) => assert_eq!(r.raster.file.as_deref(), Some(dir.join("cikti.tif").to_str().expect("path"))),
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_hosts_own_resampling_is_taken_when_it_is_the_plans_grid() {
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures().join("georef.json")).expect("georef.json"))
            .expect("JSON");
    let warp = &reference["warp"];
    let dir = scratch("made");
    let source = dir.join("pafta.tif");
    std::fs::copy(fixtures().join("files").join(warp["source"].as_str().expect("source")), &source).expect("copy");
    let mut d = doc(&source);
    let uid = d.uid(Slot(1)).expect("the raster").to_string();
    let points: Vec<Value> = warp["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|p| json!({ "pixel": p["pixel"], "target": p["target"] }))
        .collect();
    let input = json!({ "uid": uid, "method": "thinPlate", "points": points, "pixel": 0.6, "resampling": "nearest",
                        "output": dir.join("a.tif") });
    // The host resamples by the plan (here the command itself into a.tif, then the file given as made).
    let planned = run(&mut d, CallOp::Plan, input.clone());
    assert_eq!(planned["status"], "completed", "{planned}");
    let first = run(&mut d, CallOp::Execute, input.clone());
    assert_eq!(first["status"], "completed", "{first}");
    assert_eq!(planned["output"]["raster"]["affine"], first["output"]["raster"]["affine"]);
    assert!(d.undo().is_some());
    let mut made = input.clone();
    made.as_object_mut().expect("input").remove("output");
    made["made"] = json!({ "file": dir.join("a.tif") });
    let taken = run(&mut d, CallOp::Execute, made);
    assert_eq!(taken["status"], "completed", "{taken}");
    assert_eq!(taken["output"]["raster"]["file"], json!(dir.join("a.tif")));
    assert_eq!(taken["output"]["raster"]["georef"], first["output"]["raster"]["georef"]);
    // Another pixel's grid is not the file's.
    assert!(d.undo().is_some());
    let mut other = input;
    other.as_object_mut().expect("input").remove("output");
    other["pixel"] = json!(0.5);
    other["made"] = json!({ "file": dir.join("a.tif") });
    let refused = run(&mut d, CallOp::Execute, other);
    assert_eq!(refused["error"]["code"], "georef_output_mismatch", "{refused}");
    let _ = std::fs::remove_dir_all(&dir);
}
