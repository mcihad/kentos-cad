//! The analysis area (docs/adr/0535): every surface case of `terrain/v1/cases.json` run on the whole raster and on
//! two areas (one inside, one past an edge and a corner) gives on the area's cells exactly the whole run's values
//! there: a cell's neighbours outside the area are read from the source, not repeated, and a row's axes are the
//! source row's. The window's grid is the source's own, its origin at the window's first cell. Renkli kabartma's
//! ramp without bounds is the area's band's (its first pass over the area), so its case is left out; contours'
//! lines have their own test.

use kentos_formats::raster::source::Reader;
use kentos_raster::frame::Frame;
use kentos_raster::job::{Spec, Window, window_affine, window_of};
use serde_json::Value;

use crate::host::{Ran, opened, read, run};

fn samples(reader: &mut Reader, bytes: &[u8]) -> (u32, u32, u32, Vec<f64>) {
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &bytes[a..b]).expect("a block");
    }
    let r = reader.region(0, 0, 0, w, h).expect("the level");
    let n = (w * h * r.bands) as usize;
    (w, h, r.bands, (0..n).map(|k| r.samples.get(k)).collect())
}

/// The area whose cells are columns `i0..i1`, rows `j0..j1` of the grid (their corners' box, a little inside).
fn area_of(affine: [f64; 6], (i0, j0, i1, j1): (f64, f64, f64, f64)) -> [f64; 4] {
    let [e, a, b, n, c, d] = affine;
    let p = |i: f64, j: f64| (e + a * i + b * j, n + c * i + d * j);
    let pts = [p(i0, j0), p(i1, j0), p(i0, j1), p(i1, j1)];
    let xs = pts.iter().map(|q| q.0);
    let ys = pts.iter().map(|q| q.1);
    [
        xs.clone().fold(f64::INFINITY, f64::min),
        ys.clone().fold(f64::INFINITY, f64::min),
        xs.fold(f64::NEG_INFINITY, f64::max),
        ys.fold(f64::NEG_INFINITY, f64::max),
    ]
}

#[test]
fn an_area_s_cells_are_the_whole_raster_s() {
    let doc: Value = serde_json::from_slice(&read("terrain/v1/cases.json")).expect("the cases");
    let dems = doc["dems"].as_array().expect("dems");
    let mut checked = 0;
    for case in doc["cases"].as_array().expect("cases") {
        let tool = &case["spec"]["tool"];
        if tool["kind"] == "colorRelief" && (tool["min"].is_null() || tool["max"].is_null()) {
            continue;
        }
        let name = format!(
            "{} / {}",
            case["dem"].as_str().unwrap(),
            case["name"].as_str().unwrap()
        );
        let dem = dems
            .iter()
            .find(|d| d["name"] == case["dem"])
            .expect("the DEM");
        let tif = read(&format!("terrain/v1/{}", dem["file"].as_str().unwrap()));
        let spec: Spec = serde_json::from_value(case["spec"].clone()).expect("the spec");
        let Ran::Raster(whole) = run(&tif, &spec, 4).expect("the whole") else {
            panic!("{name}: a raster");
        };
        let (w, h, bands, all) = samples(&mut opened(&whole), &whole);
        let (fw, fh) = (f64::from(w), f64::from(h));
        let frame = Frame::new(spec.affine, w, h, false).expect("the frame");
        for cells in [
            (3.2, 2.4, fw * 0.6, fh * 0.7),
            (fw * 0.5, -4.0, fw + 6.0, fh * 0.4),
        ] {
            let area = area_of(spec.affine, cells);
            let win: Window = window_of(&frame, area).expect("the window");
            let mut part = spec.clone();
            part.window = Some(area);
            let Ran::Raster(out) = run(&tif, &part, 1).expect("the area") else {
                panic!("{name}: a raster");
            };
            let mut reader = opened(&out);
            assert_eq!(
                reader.info.affine,
                Some(window_affine(spec.affine, win)),
                "{name}: the window's grid"
            );
            let (pw, ph, pb, got) = samples(&mut reader, &out);
            assert_eq!(
                (pw, ph, pb),
                (win.width, win.height, bands),
                "{name}: {win:?}"
            );
            for j in 0..ph {
                for i in 0..pw {
                    for c in 0..bands {
                        let g = got[((j * pw + i) * bands + c) as usize];
                        let want = all[(((win.y0 + j) * w + win.x0 + i) * bands + c) as usize];
                        assert!(
                            g.to_bits() == want.to_bits() || (g.is_nan() && want.is_nan()),
                            "{name}: {win:?} ({i}, {j}) band {c}: {g} for {want}"
                        );
                    }
                }
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 50, "every case but the ramps without bounds");
}

#[test]
fn an_area_beside_the_raster_or_turned_round_is_refused() {
    let frame =
        Frame::new([500100.0, 10.0, 0.0, 4420330.0, 0.0, -10.0], 41, 33, false).expect("frame");
    let beside = window_of(&frame, [400000.0, 4420000.0, 400100.0, 4420100.0]).unwrap_err();
    assert!(beside.contains("kesişmiyor"), "{beside}");
    let turned = window_of(&frame, [500300.0, 4420000.0, 500200.0, 4420100.0]).unwrap_err();
    assert!(turned.contains("dört sayı"), "{turned}");
    // A side on a cell's edge takes no cell beyond it.
    let w = window_of(&frame, [500100.0, 4420230.0, 500200.0, 4420330.0]).expect("window");
    assert_eq!(
        w,
        Window {
            x0: 0,
            y0: 0,
            width: 10,
            height: 10
        }
    );
}
