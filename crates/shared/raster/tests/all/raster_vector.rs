//! Raster ve vektör dönüşümü's runs end to end (docs/adr/0234): small
//! rasters written as tiled GeoTIFFs in memory, each vectorizing tool's job
//! run as a host runs it, and Rasterleştir's GeoTIFF read back. The
//! independent reference's cases are `vector_cases`; these are the rules'
//! plain checks.

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::open_bytes;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::OpsFinished;
use kentos_raster::vector::{FeatureKind, Features};
use serde_json::{Value, json};

use crate::raster_ops::{Got, Raster, number, run, sample_of};

const PLACE: [f64; 6] = [1000.0, 2.0, 0.0, 2000.0, 0.0, -2.0];

fn features(rasters: &[Raster], tool: Value, threads: usize) -> Result<Features, String> {
    match run(rasters, tool, vec![], threads)? {
        Got::Done(OpsFinished::Features(f)) => Ok(f),
        _ => Err("features were wanted".into()),
    }
}

/// Feature `k`'s rings (or its line, or its point) as world points.
fn shape_of(f: &Features, k: usize) -> Vec<Vec<[f64; 2]>> {
    let rings_before: usize = match f.kind {
        FeatureKind::Polygons => f.rings[..k].iter().map(|&n| n as usize).sum(),
        _ => k,
    };
    let count = match f.kind {
        FeatureKind::Polygons => f.rings[k] as usize,
        _ => 1,
    };
    let mut at: usize = f.sizes[..rings_before].iter().map(|&n| n as usize).sum();
    (rings_before..rings_before + count)
        .map(|r| {
            let n = f.sizes[r] as usize;
            let ring = (at..at + n)
                .map(|p| [f.xy[2 * p], f.xy[2 * p + 1]])
                .collect();
            at += n;
            ring
        })
        .collect()
}

/// Cell-space corners to the world of `PLACE`.
fn world(corners: &[[i64; 2]]) -> Vec<[f64; 2]> {
    corners
        .iter()
        .map(|&[u, v]| [1000.0 + 2.0 * u as f64, 2000.0 - 2.0 * v as f64])
        .collect()
}

#[test]
fn regions_become_areas_with_their_rings() {
    #[rustfmt::skip]
    let values = vec![
        1.0, 1.0, 2.0, 2.0,
        1.0, 3.0, 3.0, 2.0,
        1.0, 1.0, 2.0, 2.0,
    ];
    let r = Raster::f32(PLACE, 4, 3, values);
    for threads in [1, 3] {
        let f = features(
            &[r_copy(&r)],
            json!({ "kind": "toPolygons", "band": 1, "connect": "four" }),
            threads,
        )
        .unwrap();
        assert_eq!(f.kind, FeatureKind::Polygons);
        // By their first cells, row by row.
        assert_eq!(f.values, vec![1.0, 2.0, 3.0]);
        assert_eq!(f.texts, vec!["1", "2", "3"]);
        assert_eq!(f.rings, vec![1, 1, 1]);
        assert_eq!(
            shape_of(&f, 0),
            vec![world(&[
                [0, 0],
                [0, 3],
                [2, 3],
                [2, 2],
                [1, 2],
                [1, 1],
                [2, 1],
                [2, 0]
            ])]
        );
        assert_eq!(
            shape_of(&f, 1),
            vec![world(&[
                [2, 0],
                [2, 1],
                [3, 1],
                [3, 2],
                [2, 2],
                [2, 3],
                [4, 3],
                [4, 0]
            ])]
        );
        assert_eq!(
            shape_of(&f, 2),
            vec![world(&[[1, 1], [1, 2], [3, 2], [3, 1]])]
        );
    }
}

fn r_copy(r: &Raster) -> Raster {
    Raster {
        affine: r.affine,
        width: r.width,
        height: r.height,
        bands: r.bands,
        sample: r.sample,
        nodata: r.nodata,
        alpha: r.alpha,
        values: r.values.clone(),
    }
}

#[test]
fn corners_join_regions_when_eight_and_holes_touch_the_outline() {
    #[rustfmt::skip]
    let values = vec![
        7.0, 0.0, 0.0,
        0.0, 7.0, 0.0,
        0.0, 0.0, 7.0,
    ];
    let r = Raster::f32(PLACE, 3, 3, values);
    let four = features(
        &[r_copy(&r)],
        json!({ "kind": "toPolygons", "band": 1, "connect": "four" }),
        2,
    )
    .unwrap();
    assert_eq!(four.values, vec![7.0, 0.0, 0.0, 7.0, 7.0]);
    let eight = features(
        &[r],
        json!({ "kind": "toPolygons", "band": 1, "connect": "eight" }),
        2,
    )
    .unwrap();
    assert_eq!(eight.values, vec![7.0, 0.0]);
    // A ring of 5s round a 9, the ring's cell at the middle of the bottom
    // missing (no value): the 9 is a hole of the 5s' area.
    let n = f64::NAN;
    #[rustfmt::skip]
    let ring = vec![
        5.0, 5.0, 5.0,
        5.0, 9.0, 5.0,
        5.0, n,   5.0,
    ];
    let f = features(
        &[Raster::f32(PLACE, 3, 3, ring)],
        json!({ "kind": "toPolygons", "band": 1, "connect": "four" }),
        1,
    )
    .unwrap();
    assert_eq!(f.values, vec![5.0, 9.0]);
    // The 5s' outline runs round the missing cell; the 9's cell is not a hole
    // (it touches the missing cell, which is outside).
    assert_eq!(f.rings, vec![1, 1]);
}

#[test]
fn a_thick_bar_thins_to_a_line() {
    let mut values = vec![0.0; 50];
    for j in 1..4 {
        for i in 1..9 {
            values[j * 10 + i] = 1.0;
        }
    }
    let f = features(
        &[Raster::f32(PLACE, 10, 5, values)],
        json!({ "kind": "toLines", "band": 1, "select": "nonZero", "spur": 0, "simplify": 0.0 }),
        2,
    )
    .unwrap();
    assert_eq!(f.kind, FeatureKind::Lines);
    assert_eq!(
        shape_of(&f, 0),
        vec![vec![[1003.0, 1995.0], [1013.0, 1995.0]]]
    );
    // Within a range only: none.
    let none = features(
        &[Raster::f32(PLACE, 10, 5, vec![1.0; 50])],
        json!({ "kind": "toLines", "band": 1, "select": "range", "min": 2.0, "max": 3.0, "spur": 0, "simplify": 1.0 }),
        1,
    )
    .unwrap();
    assert!(none.is_empty());
}

#[test]
fn points_by_step_and_extrema() {
    let ramp: Vec<f64> = (0..20)
        .map(|k| f64::from(k % 5) + 10.0 * f64::from(k / 5))
        .collect();
    let f = features(
        &[Raster::f32(PLACE, 5, 4, ramp)],
        json!({ "kind": "toPoints", "band": 1, "mode": "step", "step": 2 }),
        2,
    )
    .unwrap();
    assert_eq!(f.values, vec![11.0, 13.0, 31.0, 33.0]);
    assert_eq!(
        f.xy,
        vec![
            1003.0, 1997.0, 1007.0, 1997.0, 1003.0, 1993.0, 1007.0, 1993.0
        ]
    );
    let mut dem = vec![0.0; 25];
    dem[12] = 9.0;
    dem[20] = -1.0;
    let f = features(
        &[Raster::f32(PLACE, 5, 5, dem)],
        json!({ "kind": "toPoints", "band": 1, "mode": "extrema", "radius": 1 }),
        3,
    )
    .unwrap();
    assert_eq!(f.values, vec![9.0, -1.0]);
    assert_eq!(f.tags, vec![1, 2]);
    assert_eq!(f.texts, vec!["9", "-1"]);
}

/// An RGB raster: white, the cells `dark` says dark grey.
fn sheet(w: u32, h: u32, dark: &dyn Fn(u32, u32) -> bool) -> Raster {
    let mut values = Vec::with_capacity((w * h * 3) as usize);
    for j in 0..h {
        for i in 0..w {
            let v = if dark(i, j) { 30.0 } else { 255.0 };
            values.extend([v, v, v]);
        }
    }
    Raster {
        affine: PLACE,
        width: w,
        height: h,
        bands: 3,
        sample: RasterSample::U8,
        nodata: None,
        alpha: false,
        values,
    }
}

#[test]
fn a_line_is_captured_beyond_its_first_window() {
    // A bar three cells thick across 3000 columns: the window grows from 1024 to the whole width.
    let r = sheet(3000, 9, &|_, j| (3..6).contains(&j));
    // The click two cells off the line, at column 1500.
    let (x, y) = (1000.0 + 2.0 * 1500.5, 2000.0 - 2.0 * 1.5);
    let f = features(
        &[r],
        json!({ "kind": "captureLine", "x": x, "y": y, "tolerance": 60.0, "spur": 5, "simplify": 1.0 }),
        2,
    )
    .unwrap();
    assert_eq!(f.len(), 1);
    let line = &shape_of(&f, 0)[0];
    assert_eq!(line.len(), 2);
    assert!(line.iter().all(|p| p[1] == 2000.0 - 2.0 * 4.5));
    assert!(
        line[0][0] < 1010.0 && line[1][0] > 1000.0 + 2.0 * 2990.0,
        "{line:?}"
    );
}

#[test]
fn an_area_is_closed_inside_its_lines() {
    // A frame of dark cells from (2, 2) to (9, 7), a dark dot at (5, 5).
    let frame = |i: u32, j: u32| {
        let on_frame =
            (2..=9).contains(&i) && (2..=7).contains(&j) && (i == 2 || i == 9 || j == 2 || j == 7);
        on_frame || (i, j) == (5, 5)
    };
    let (x, y) = (1000.0 + 2.0 * 4.5, 2000.0 - 2.0 * 3.5);
    let run_with = |holes: &str| {
        features(
            &[sheet(12, 10, &frame)],
            json!({ "kind": "closeArea", "x": x, "y": y, "tolerance": 60.0, "holes": holes, "simplify": 1.0 }),
            2,
        )
        .unwrap()
    };
    let filled = run_with("fill");
    assert_eq!(filled.rings, vec![1]);
    assert_eq!(
        shape_of(&filled, 0),
        vec![world(&[[3, 3], [3, 7], [9, 7], [9, 3]])]
    );
    assert_eq!(filled.tags, vec![0]);
    // The dot kept as a hole: a ring that would fall under three corners keeps its own.
    let kept = run_with("keep");
    assert_eq!(
        shape_of(&kept, 0),
        vec![
            world(&[[3, 3], [3, 7], [9, 7], [9, 3]]),
            world(&[[5, 5], [6, 5], [6, 6], [5, 6]])
        ]
    );
    // Outside the frame the white reaches the sheet's edge: refused.
    let out = features(
        &[sheet(12, 10, &frame)],
        json!({ "kind": "closeArea", "x": 1001.0, "y": 1999.0, "tolerance": 60.0, "holes": "fill", "simplify": 1.0 }),
        1,
    );
    assert!(out.unwrap_err().contains("Alan kapanmıyor"));
    // Off the raster.
    let off = features(
        &[sheet(12, 10, &frame)],
        json!({ "kind": "closeArea", "x": 900.0, "y": 1999.0, "tolerance": 60.0, "holes": "fill", "simplify": 1.0 }),
        1,
    );
    assert!(off.unwrap_err().contains("rasterin dışında"));
}

/// Rasterleştir over `shapes` (their values' texts, or the constant 1) onto the grid; level 0 read back.
fn burn(
    shapes: Vec<Shape>,
    texts: Option<Vec<Option<String>>>,
    tool: Value,
    grid: Option<Value>,
    cell: f64,
) -> Result<(u32, u32, Vec<f64>, kentos_raster::from_points::Notes), String> {
    let mut spec = json!({ "tool": tool, "cell": cell });
    if let Some(g) = grid {
        spec["grid"] = g;
    }
    let spec: PointSpec = serde_json::from_value(spec).map_err(|e| e.to_string())?;
    let (mut job, header) = PointJob::new(
        PointInput::Lines {
            shapes,
            weights: texts,
        },
        &spec,
        2,
    )?;
    let mut file = header;
    while !job.done() {
        file.extend(job.step()?);
    }
    let done = job.finish()?;
    file.extend(&done.tail);
    file[..done.header.len()].copy_from_slice(&done.header);
    let mut reader = open_bytes(&file, None, READER_BUDGET).map_err(|e| e.0)?;
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &file[a..b]).map_err(|e| e.0)?;
    }
    let r = reader.region(0, 0, 0, w, h).ok_or("the level")?;
    Ok((
        w,
        h,
        (0..(w * h) as usize).map(|k| r.samples.get(k)).collect(),
        done.notes,
    ))
}

#[test]
fn objects_burn_onto_their_box_and_onto_a_raster_s_grid() {
    let square = Shape::Polygon {
        pts: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(0.0, 4.0),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    let road = Shape::Line {
        a: Vec2::new(0.5, 0.5),
        b: Vec2::new(9.5, 0.5),
    };
    let well = Shape::Point {
        p: Vec2::new(8.5, 3.5),
        z: None,
        parts: None,
    };
    let texts = Some(vec![
        Some("3".into()),
        Some("7,5".into()),
        Some("yok".into()),
    ]);
    // The box 0 … 9.5 × 0 … 4: cell 1 asked for, 10 × 4 cells.
    let (w, h, v, notes) = burn(
        vec![square.clone(), road.clone(), well.clone()],
        texts.clone(),
        json!({ "kind": "rasterize", "value": 1.0, "overlap": "last", "sample": "f32" }),
        None,
        1.0,
    )
    .unwrap();
    assert_eq!((w, h), (10, 4));
    assert_eq!(notes.unread, 1);
    assert_eq!(notes.taken, 2);
    // The bottom row (y 0 … 1) is the road's: 7.5 over the square's 3.
    assert!(v[30..40].iter().all(|&x| x == 7.5), "{:?}", &v[30..40]);
    assert!(v[0..4].iter().all(|&x| x == 3.0) && v[4..10].iter().all(|x| x.is_nan()));
    // A byte raster: no value is 255, the well (constant 1) at its cell.
    let (_, _, v, _) = burn(
        vec![square, road, well],
        None,
        json!({ "kind": "rasterize", "value": 1.0, "overlap": "count", "sample": "u8" }),
        Some(json!({ "affine": [0.0, 1.0, 0.0, 4.0, 0.0, -1.0], "width": 10, "height": 4 })),
        0.0,
    )
    .unwrap();
    assert_eq!(v[8], 1.0);
    assert_eq!(v[9], 255.0);
    assert_eq!(&v[30..33], &[2.0, 2.0, 2.0]);
    assert_eq!(v[35], 1.0);
}

/// An input raster as the cases write it.
fn raster_of(r: &Value) -> Raster {
    let a: Vec<f64> = r["affine"]
        .as_array()
        .expect("affine")
        .iter()
        .map(number)
        .collect();
    Raster {
        affine: [a[0], a[1], a[2], a[3], a[4], a[5]],
        width: r["width"].as_u64().expect("width") as u32,
        height: r["height"].as_u64().expect("height") as u32,
        bands: r["bands"].as_u64().expect("bands") as u32,
        sample: sample_of(r["sample"].as_str().unwrap_or("f32")),
        nodata: match &r["nodata"] {
            Value::String(s) if s == "nan" => Some(f64::NAN),
            Value::Number(n) => n.as_f64(),
            _ => None,
        },
        alpha: r["alpha"].as_bool().unwrap_or(false),
        values: r["values"]
            .as_array()
            .expect("values")
            .iter()
            .map(number)
            .collect(),
    }
}

fn shapes_of(c: &Value) -> Vec<Shape> {
    c["shapes"]
        .as_array()
        .expect("shapes")
        .iter()
        .map(|s| {
            let j = kentos_geometry_core::api::json::Json::parse(&s.to_string()).expect("a shape");
            <kentos_geometry_core::entity::Entity as kentos_geometry_core::api::json::FromJson>::from_json(&j)
                .expect("an entity")
                .shape
        })
        .collect()
}

/// Two float lists the same, NaN only with NaN (null in the case).
fn same(got: &[f64], want: &[Value]) -> bool {
    got.len() == want.len()
        && got.iter().zip(want).all(|(g, w)| match w.as_f64() {
            Some(x) => *g == x,
            None => g.is_nan(),
        })
}

fn check_features(name: &str, f: &Features, want: &Value) -> Result<(), String> {
    let kind = match f.kind {
        FeatureKind::Polygons => "polygons",
        FeatureKind::Lines => "lines",
        FeatureKind::Points => "points",
    };
    let list = |k: &str| want[k].as_array().cloned().unwrap_or_default();
    let ints = |k: &str| -> Vec<u64> {
        list(k)
            .iter()
            .map(|v| v.as_u64().unwrap_or(u64::MAX))
            .collect()
    };
    let texts: Vec<String> = list("texts")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect();
    let checks = [
        ("kind", kind == want["kind"].as_str().unwrap_or("")),
        ("values", same(&f.values, &list("values"))),
        ("texts", f.texts == texts),
        (
            "tags",
            f.tags.iter().map(|&t| u64::from(t)).collect::<Vec<_>>() == ints("tags"),
        ),
        (
            "rings",
            f.rings.iter().map(|&t| u64::from(t)).collect::<Vec<_>>() == ints("rings"),
        ),
        (
            "sizes",
            f.sizes.iter().map(|&t| u64::from(t)).collect::<Vec<_>>() == ints("sizes"),
        ),
        ("xy", same(&f.xy, &list("xy"))),
    ];
    match checks.iter().find(|(_, ok)| !ok) {
        None => Ok(()),
        Some((what, _)) => Err(format!(
            "{name}: {what} differ: got {:?} / {:?} / {:?}",
            f.values,
            f.sizes,
            &f.xy[..f.xy.len().min(16)]
        )),
    }
}

/// Every case of `scripts/fixtures/raster_vector_cases.py` (KentOS code not
/// used there): the vectorizing tools' features bit by bit on one thread
/// and on three; Rasterleştir's every sample and its notes.
#[test]
fn vector_cases() {
    let doc: Value = serde_json::from_slice(&crate::host::read("raster-vector/v1/cases.json"))
        .expect("the cases");
    let mut failures = Vec::new();
    let mut checked = 0;
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let expect = &c["expect"];
        if c["tool"]["kind"] == "rasterize" {
            let texts: Option<Vec<Option<String>>> = c["values"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_str().map(str::to_owned)).collect());
            let grid = c.get("grid").cloned();
            let ran = burn(
                shapes_of(c),
                texts,
                c["tool"].clone(),
                grid,
                c["cell"].as_f64().unwrap_or(0.0),
            );
            match (ran, expect["refused"].as_str()) {
                (Err(e), Some(why)) if e.contains(why) => {}
                (Err(e), _) => failures.push(format!("{name}: {e}")),
                (Ok(_), Some(why)) => {
                    failures.push(format!("{name}: ran, should be refused ({why})"))
                }
                (Ok((w, h, v, notes)), None) => {
                    let r = &expect["raster"];
                    let n = &expect["notes"];
                    if (u64::from(w), u64::from(h))
                        != (
                            r["width"].as_u64().unwrap_or(0),
                            r["height"].as_u64().unwrap_or(0),
                        )
                    {
                        failures.push(format!("{name}: {w} × {h}"));
                    } else if !same(&v, r["values"].as_array().expect("values")) {
                        failures.push(format!("{name}: the samples differ"));
                    } else if (
                        notes.taken as u64,
                        notes.unread as u64,
                        notes.outside as u64,
                        notes.empty,
                    ) != (
                        n["taken"].as_u64().unwrap_or(0),
                        n["unread"].as_u64().unwrap_or(0),
                        n["outside"].as_u64().unwrap_or(0),
                        n["empty"].as_u64().unwrap_or(0),
                    ) {
                        failures.push(format!("{name}: notes {notes:?}"));
                    } else {
                        checked += 1;
                    }
                }
            }
            continue;
        }
        let rasters: Vec<Raster> = c["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .map(raster_of)
            .collect();
        for threads in [1, 3] {
            let got = features(&rasters, c["tool"].clone(), threads);
            match (got, expect["refused"].as_str()) {
                (Err(e), Some(why)) if e.contains(why) => {}
                (Err(e), _) => failures.push(format!("{name} ({threads}): {e}")),
                (Ok(_), Some(why)) => {
                    failures.push(format!("{name}: ran, should be refused ({why})"))
                }
                (Ok(f), None) => match check_features(name, &f, &expect["features"]) {
                    Ok(()) => checked += 1,
                    Err(e) => failures.push(e),
                },
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked >= 50, "{checked}");
}
