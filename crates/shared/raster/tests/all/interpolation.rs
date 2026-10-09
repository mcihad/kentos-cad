//! İnterpolasyon ve yoğunluk (docs/adr/0232): every case of
//! `scripts/fixtures/interpolation_cases.py` (KentOS code not used there;
//! Sibson exact in rationals, Spline and Kriging in 40-digit mpmath, IDW and
//! TIN held to GDAL too). The points the objects give and the grid's rule
//! exactly; each cell's value as the core works it out within the method's
//! bound of the reference; the written GeoTIFF within a unit in the last
//! place of the reference rounded to 32 bits, the same bytes on one thread
//! and on four; each point's cross-validation and its sums; the variogram's fit.

use kentos_formats::raster::source::{Reader, open_bytes};
use kentos_geometry_core::entity::{Entity as Geo, Shape};
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::from_points::{CrossSummary, PointFinished, PointInput, PointJob, PointSpec};
use kentos_raster::grid::{Grid, nice_cell};
use kentos_raster::interp::kriging::{self, Model};
use kentos_raster::job::READER_BUDGET;
use kentos_raster::points::{Source, gather};
use serde_json::Value;

use crate::host::read;

fn cases() -> Value {
    serde_json::from_slice(&read("interpolation/v1/cases.json")).expect("the cases read")
}

fn sources(c: &Value) -> Vec<Source> {
    serde_json::from_value(c["sources"].clone()).expect("the sources read")
}

fn texts(c: &Value) -> Option<Vec<Option<String>>> {
    c["values"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_str().map(str::to_owned)).collect())
}

fn shapes(c: &Value) -> Vec<Shape> {
    c["shapes"]
        .as_array()
        .expect("shapes")
        .iter()
        .map(|v| {
            let j = kentos_geometry_core::api::json::Json::parse(&v.to_string()).expect("JSON");
            <Geo as kentos_geometry_core::api::json::FromJson>::from_json(&j)
                .expect("a shape")
                .shape
        })
        .collect()
}

fn input(c: &Value) -> PointInput {
    if c["kind"] == "lines" {
        PointInput::Lines {
            shapes: shapes(c),
            weights: texts(c),
        }
    } else {
        PointInput::Sources {
            sources: sources(c),
            values: texts(c),
        }
    }
}

fn spec(c: &Value, cross: bool) -> PointSpec {
    let mut s = serde_json::json!({ "tool": c["tool"], "cell": c["cell"], "cross": cross });
    if c.get("grid").is_some() {
        s["grid"] = c["grid"].clone();
    }
    serde_json::from_value(s).expect("the spec reads")
}

/// A job run to its end: the file's bytes and what it finished with.
fn run(c: &Value, threads: usize, cross: bool) -> (Vec<u8>, PointFinished) {
    let (mut job, header) =
        PointJob::new(input(c), &spec(c, cross), threads).expect("the job starts");
    let mut file = header;
    let mut last = -1.0;
    while !job.done() {
        file.extend(job.step().expect("a step"));
        let share = job.share();
        assert!(share >= last);
        last = share;
    }
    let done = job.finish().expect("it finishes");
    file.extend(&done.tail);
    file[..done.header.len()].copy_from_slice(&done.header);
    (file, done)
}

/// Level 0 of a result whole, bands interleaved.
fn level0(bytes: &[u8]) -> (u32, u32, u32, Vec<f64>) {
    let mut reader: Reader = open_bytes(bytes, None, READER_BUDGET).expect("the result opens");
    let (w, h) = (reader.levels[0].width, reader.levels[0].height);
    for need in reader.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        reader.put_block(&need, &bytes[a..b]).expect("a block");
    }
    let r = reader.region(0, 0, 0, w, h).expect("the level");
    let n = (w * h * r.bands) as usize;
    (w, h, r.bands, (0..n).map(|k| r.samples.get(k)).collect())
}

/// The units in the last place between two 32-bit values (NaN only with NaN).
fn ulps(a: f32, b: f32) -> u32 {
    if a.is_nan() || b.is_nan() {
        return if a.is_nan() && b.is_nan() {
            0
        } else {
            u32::MAX
        };
    }
    let key = |v: f32| {
        let i = v.to_bits() as i32;
        if i < 0 { i32::MIN - i } else { i }
    };
    key(a).abs_diff(key(b))
}

fn want_of(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Each method's bound on a cell's value, relative to max(1, |reference|)
/// (docs/adr/0232's Doğrulama: measured at most 1.3·10⁻¹⁵ for the
/// interpolations, 5·10⁻¹⁰ with a fitted variogram, whose range is found to
/// ~10⁻⁹; 5·10⁻¹⁴ for the kernels, 2·10⁻¹³ for the lines, whose arcs near a
/// tangent lose digits in acos).
fn bound(c: &Value) -> f64 {
    if let Some(t) = c.get("tolerance").and_then(Value::as_f64) {
        return t;
    }
    match c["tool"]["kind"].as_str().expect("a tool") {
        "idw" | "tin" | "naturalNeighbor" | "spline" => 1e-13,
        "kriging" if c["tool"]["variogram"]["fit"] == "auto" => 1e-8,
        "kriging" => 1e-13,
        "kernel" => 1e-12,
        "lineDensity" => 1e-11,
        other => panic!("{other}"),
    }
}

fn close(got: f64, want: f64, tol: f64) -> bool {
    if want.is_nan() {
        return got.is_nan();
    }
    (got - want).abs() <= tol * want.abs().max(1.0)
}

#[test]
fn the_objects_give_the_points_the_reference_says() {
    let all = cases();
    let mut n = 0;
    for c in all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["kind"] == "gather")
    {
        let name = c["name"].as_str().expect("name");
        let got = gather(&sources(c), texts(c).as_deref()).expect("gathered");
        let e = &c["expect"];
        let xy: Vec<Vec2> = e["xy"]
            .as_array()
            .expect("xy")
            .iter()
            .map(|p| Vec2::new(p[0].as_f64().expect("x"), p[1].as_f64().expect("y")))
            .collect();
        assert_eq!(got.xy, xy, "{name}: places");
        let v: Vec<f64> = e["v"].as_array().expect("v").iter().map(want_of).collect();
        assert_eq!(got.v, v, "{name}: values");
        let obj: Vec<u32> = e["object"]
            .as_array()
            .expect("object")
            .iter()
            .map(|o| o.as_u64().expect("index") as u32)
            .collect();
        assert_eq!(got.object, obj, "{name}: objects");
        assert_eq!(
            got.merged as u64,
            e["merged"].as_u64().expect("merged"),
            "{name}"
        );
        assert_eq!(
            got.unread as u64,
            e["unread"].as_u64().expect("unread"),
            "{name}"
        );
        assert_eq!(
            got.no_elevation as u64,
            e["noElevation"].as_u64().expect("noElevation"),
            "{name}"
        );
        n += 1;
    }
    assert_eq!(n, 2);
}

#[test]
fn the_grid_s_rule_is_the_reference_s() {
    let all = cases();
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        match c["kind"].as_str() {
            Some("nice") => {
                assert_eq!(
                    nice_cell(c["size"].as_f64().expect("size")),
                    c["expect"].as_f64().expect("size"),
                    "{name}"
                );
            }
            Some("grid") => {
                let b: Vec<f64> = c["box"]
                    .as_array()
                    .expect("box")
                    .iter()
                    .map(want_of)
                    .collect();
                let got = Grid::of_box([b[0], b[1], b[2], b[3]], c["cell"].as_f64().expect("cell"));
                if c["expect"].get("error").is_some() {
                    assert!(got.is_err(), "{name}: refused");
                    continue;
                }
                let g = got.unwrap_or_else(|e| panic!("{name}: {e}"));
                let aff: Vec<f64> = c["expect"]["affine"]
                    .as_array()
                    .expect("affine")
                    .iter()
                    .map(want_of)
                    .collect();
                assert_eq!(g.affine.to_vec(), aff, "{name}");
                assert_eq!(
                    u64::from(g.width),
                    c["expect"]["width"].as_u64().expect("width"),
                    "{name}"
                );
                assert_eq!(
                    u64::from(g.height),
                    c["expect"]["height"].as_u64().expect("height"),
                    "{name}"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn every_surface_is_the_reference_s_in_its_cells_and_its_file() {
    let all = cases();
    let mut checked = 0;
    for c in all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["kind"] == "surface" || c["kind"] == "lines")
    {
        let name = c["name"].as_str().expect("name");
        let e = &c["expect"];
        let tol = bound(c);
        let (job, _) = PointJob::new(input(c), &spec(c, false), 1)
            .unwrap_or_else(|err| panic!("{name}: {err}"));
        let g = job.grid();
        let aff: Vec<f64> = e["affine"]
            .as_array()
            .expect("affine")
            .iter()
            .map(want_of)
            .collect();
        assert_eq!(g.affine.to_vec(), aff, "{name}: the grid");
        assert_eq!(
            u64::from(g.width),
            e["width"].as_u64().expect("width"),
            "{name}"
        );
        assert_eq!(
            u64::from(g.height),
            e["height"].as_u64().expect("height"),
            "{name}"
        );
        if let Some(r) = e.get("radius").and_then(Value::as_f64) {
            let got = job.notes().radius.expect("a radius");
            assert!(
                (got - r).abs() <= 1e-12 * r,
                "{name}: radius {got}, want {r}"
            );
        }
        let values: Vec<f64> = e["values"]
            .as_array()
            .expect("values")
            .iter()
            .map(want_of)
            .collect();
        let errors: Option<Vec<f64>> = e
            .get("error")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(want_of).collect());
        let mut worst = 0.0f64;
        for j in 0..g.height {
            for i in 0..g.width {
                let k = (j * g.width + i) as usize;
                let (v, err) = job.at(g.center(i, j));
                let want = values[k];
                assert!(
                    close(v, want, tol),
                    "{name}: cell {i},{j}: {v}, want {want}"
                );
                if !want.is_nan() {
                    worst = worst.max((v - want).abs() / want.abs().max(1.0));
                }
                if let Some(errs) = &errors {
                    assert!(
                        close(err, errs[k], tol),
                        "{name}: cell {i},{j}'s error {err}, want {}",
                        errs[k]
                    );
                }
            }
        }
        // The file: within an ulp of the reference in 32 bits, the same on one thread and four.
        let (one, _) = run(c, 1, false);
        let (four, _) = run(c, 4, false);
        assert!(one == four, "{name}: the threads write other bytes");
        let (w, h, bands, samples) = level0(&one);
        assert_eq!((w, h), (g.width, g.height));
        assert_eq!(bands, if errors.is_some() { 2 } else { 1 }, "{name}: bands");
        for (k, want) in values.iter().enumerate() {
            let got = samples[k * bands as usize] as f32;
            assert!(
                ulps(got, *want as f32) <= 1,
                "{name}: cell {k} written {got}, want {want}"
            );
        }
        eprintln!("{name}: worst {worst:.2e} of the bound {tol:.0e}");
        checked += 1;
    }
    assert!(checked >= 20, "{checked} surfaces");
}

#[test]
fn each_point_s_cross_validation_is_the_reference_s() {
    let all = cases();
    let mut checked = 0;
    for c in all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["expect"].get("cross").is_some())
    {
        let name = c["name"].as_str().expect("name");
        let tol = bound(c);
        let (_, done) = run(c, 3, true);
        let want = c["expect"]["cross"].as_array().expect("cross");
        assert_eq!(done.rows.len(), want.len(), "{name}: rows");
        for (row, w) in done.rows.iter().zip(want) {
            let (wv, we) = match w {
                Value::Null => (None, None),
                Value::Array(a) => (a[0].as_f64(), a[1].as_f64()),
                v => (v.as_f64(), None),
            };
            match (row.predicted, wv) {
                (None, None) => {}
                (Some(g), Some(wv)) => assert!(
                    close(g, wv, tol),
                    "{name}: point {}: {g}, want {wv}",
                    row.point
                ),
                (g, wv) => panic!("{name}: point {}: {g:?}, want {wv:?}", row.point),
            }
            if let Some(we) = we {
                let ge = row.error.unwrap_or(f64::NAN);
                assert!(
                    close(ge, we, tol),
                    "{name}: point {}'s error {ge}, want {we}",
                    row.point
                );
            }
        }
        let s = CrossSummary::of(&done.rows);
        let ws = &c["expect"]["summary"];
        assert_eq!(
            s.count as u64,
            ws["count"].as_u64().expect("count"),
            "{name}"
        );
        assert_eq!(
            s.missing as u64,
            ws["missing"].as_u64().expect("missing"),
            "{name}"
        );
        for (got, key) in [(s.mean, "mean"), (s.rmse, "rmse"), (s.mae, "mae")] {
            let w = ws[key].as_f64().expect(key);
            assert!(
                (got - w).abs() <= 1e-8 * w.abs().max(1.0),
                "{name}: {key} {got}, want {w}"
            );
        }
        if let Some(w) = ws.get("stdRmse").and_then(Value::as_f64) {
            let got = s.std_rmse.expect("standardized");
            assert!(
                (got - w).abs() <= 1e-8 * w.abs().max(1.0),
                "{name}: stdRmse {got}, want {w}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 9, "{checked} cross-validations");
}

#[test]
fn the_variogram_s_fit_is_the_reference_s() {
    let all = cases();
    let mut checked = 0;
    for c in all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["kind"] == "fit")
    {
        let name = c["name"].as_str().expect("name");
        let pts = gather(&sources(c), None).expect("gathered");
        let model = match c["model"].as_str().expect("model") {
            "spherical" => Model::Spherical,
            "exponential" => Model::Exponential,
            _ => Model::Gaussian,
        };
        let lags = c["lags"].as_u64().expect("lags") as usize;
        // The bins exactly: the same pairs, in the same order, in float64.
        let bins = kriging::bins(&pts.xy, &pts.v, lags);
        let want_bins = c["expect"]["bins"].as_array().expect("bins");
        assert_eq!(bins.h.len(), want_bins.len(), "{name}: bins");
        for (k, wb) in want_bins.iter().enumerate() {
            assert_eq!(bins.h[k], want_of(&wb[0]), "{name}: bin {k}'s distance");
            assert_eq!(
                bins.gamma[k],
                want_of(&wb[1]),
                "{name}: bin {k}'s semivariance"
            );
            assert_eq!(bins.n[k], want_of(&wb[2]), "{name}: bin {k}'s pairs");
        }
        let g =
            kriging::fit(&pts.xy, &pts.v, model, lags).unwrap_or_else(|e| panic!("{name}: {e}"));
        let e = &c["expect"];
        let (wr, wn, ws) = (
            want_of(&e["range"]),
            want_of(&e["nugget"]),
            want_of(&e["sill"]),
        );
        assert!(
            (g.range - wr).abs() <= 1e-6 * wr,
            "{name}: range {}, want {wr}",
            g.range
        );
        let scale = wn + ws;
        assert!(
            (g.nugget - wn).abs() <= 1e-6 * scale,
            "{name}: nugget {}, want {wn}",
            g.nugget
        );
        assert!(
            (g.sill - ws).abs() <= 1e-6 * scale,
            "{name}: sill {}, want {ws}",
            g.sill
        );
        let err = kriging::best_at(&bins, model, g.range).2;
        let we = want_of(&e["error"]);
        assert!(
            err <= we * (1.0 + 1e-9),
            "{name}: the fit's error {err} above the reference's {we}"
        );
        checked += 1;
    }
    assert!(checked >= 4);
}

fn point(x: f64, y: f64, z: f64) -> Value {
    serde_json::json!({ "kind": "point", "p": { "x": x, "y": y }, "z": z })
}

fn job_of(points: &[Value], tool: Value, cell: f64) -> Result<PointJob, String> {
    let sources: Vec<Source> =
        serde_json::from_value(Value::Array(points.to_vec())).expect("sources");
    let spec: PointSpec =
        serde_json::from_value(serde_json::json!({ "tool": tool, "cell": cell })).expect("spec");
    PointJob::new(
        PointInput::Sources {
            sources,
            values: None,
        },
        &spec,
        1,
    )
    .map(|(j, _)| j)
}

/// Doğal komşu at a point, on a hull edge (linear along it) and outside; TIN outside.
#[test]
fn natural_neighbour_s_edges() {
    let square = [
        point(0.0, 0.0, 10.0),
        point(10.0, 0.0, 20.0),
        point(10.0, 10.0, 30.0),
        point(0.0, 10.0, 40.0),
        point(4.0, 6.0, 25.0),
    ];
    let nn = job_of(
        &square,
        serde_json::json!({ "kind": "naturalNeighbor" }),
        1.0,
    )
    .expect("a job");
    assert_eq!(nn.at(Vec2::new(4.0, 6.0)).0, 25.0);
    assert_eq!(nn.at(Vec2::new(10.0, 10.0)).0, 30.0);
    // On the hull edge from (0, 0) to (10, 0), a quarter along.
    assert!((nn.at(Vec2::new(2.5, 0.0)).0 - 12.5).abs() < 1e-12);
    assert!(nn.at(Vec2::new(-0.5, 5.0)).0.is_nan());
    // Inside, a weighted mean: between the least and the largest.
    let v = nn.at(Vec2::new(5.0, 5.0)).0;
    assert!(v > 10.0 && v < 40.0, "{v}");
    let tin = job_of(&square, serde_json::json!({ "kind": "tin" }), 1.0).expect("a job");
    assert!(tin.at(Vec2::new(10.5, 5.0)).0.is_nan());
    assert!((tin.at(Vec2::new(2.5, 0.0)).0 - 12.5).abs() < 1e-12);
}

#[test]
fn kriging_and_idw_on_a_point_give_its_value() {
    let pts = [
        point(0.0, 0.0, 1.0),
        point(5.0, 1.0, 2.0),
        point(2.0, 4.0, 3.0),
        point(7.0, 6.0, 4.0),
    ];
    let k = job_of(
        &pts,
        serde_json::json!({ "kind": "kriging", "model": "spherical", "variogram": { "fit": "manual", "nugget": 0.1, "sill": 1.0, "range": 10.0 }, "points": 12, "error": true }),
        1.0,
    )
    .expect("a job");
    assert_eq!(k.at(Vec2::new(5.0, 1.0)), (2.0, 0.0));
    let (v, e) = k.at(Vec2::new(3.0, 3.0));
    assert!(v > 1.0 && v < 4.0 && e > 0.0, "{v} {e}");
    let idw = job_of(&pts, serde_json::json!({ "kind": "idw", "power": 2.0, "points": 12, "radius": 1.0, "minPoints": 1 }), 1.0)
        .expect("a job");
    assert_eq!(idw.at(Vec2::new(2.0, 4.0)).0, 3.0);
    // Nothing within the radius.
    assert!(idw.at(Vec2::new(3.5, 2.5)).0.is_nan());
}

#[test]
fn a_spline_over_points_on_a_line_has_no_value() {
    let line: Vec<Value> = (0..6)
        .map(|k| point(f64::from(k), f64::from(k), f64::from(k)))
        .collect();
    let mut more = line.clone();
    more.push(point(0.0, 5.0, 2.0));
    let s = job_of(&more, serde_json::json!({ "kind": "spline", "spline": "regularized", "weight": 0.1, "points": 3 }), 1.0)
        .expect("a job");
    // The three nearest to (2.5, 2.5) are on the line: no plane through them.
    assert!(s.at(Vec2::new(2.5, 2.5)).0.is_nan());
}

#[test]
fn bad_settings_and_inputs_are_refused_saying_why() {
    let line: Vec<Value> = (0..6)
        .map(|k| point(f64::from(k), 2.0 * f64::from(k), 1.0))
        .collect();
    let tin = job_of(&line, serde_json::json!({ "kind": "tin" }), 1.0);
    assert_eq!(
        tin.err().as_deref(),
        Some(kentos_geometry_core::geom::delaunay::TOO_FEW)
    );
    let none: Vec<Value> =
        vec![serde_json::json!({ "kind": "point", "p": { "x": 0.0, "y": 0.0 } })];
    let e = job_of(
        &none,
        serde_json::json!({ "kind": "idw", "power": 2.0, "points": 12 }),
        1.0,
    )
    .expect_err("refused");
    assert!(e.contains("Kotu olan nokta yok"), "{e}");
    let pts = [
        point(0.0, 0.0, 1.0),
        point(5.0, 1.0, 2.0),
        point(2.0, 4.0, 3.0),
    ];
    let e = job_of(
        &pts,
        serde_json::json!({ "kind": "idw", "power": 20.0, "points": 12 }),
        1.0,
    )
    .expect_err("refused");
    assert!(e.contains("Üs"), "{e}");
    let e = job_of(
        &pts,
        serde_json::json!({ "kind": "idw", "power": 2.0, "points": 99 }),
        1.0,
    )
    .expect_err("refused");
    assert!(e.contains("Nokta sayısı"), "{e}");
    let e = job_of(
        &pts,
        serde_json::json!({ "kind": "kriging", "model": "gaussian", "variogram": { "fit": "auto", "lags": 12 }, "points": 12 }),
        1.0,
    )
    .expect_err("refused");
    assert!(e.contains("Variogram"), "{e}");
    let e = job_of(
        &pts,
        serde_json::json!({ "kind": "spline", "spline": "tension", "weight": 0.0, "points": 12 }),
        1.0,
    )
    .expect_err("refused");
    assert!(e.contains("Ağırlık"), "{e}");
    let e = job_of(
        &[point(1.0, 1.0, 1.0)],
        serde_json::json!({ "kind": "idw", "power": 2.0, "points": 12 }),
        0.0,
    )
    .expect_err("refused");
    assert!(e.contains("tek bir yerde"), "{e}");
}

#[test]
fn the_results_are_drawn_as_the_tools_say() {
    let pts = [
        point(0.0, 0.0, 1.0),
        point(5.0, 1.0, 2.0),
        point(2.0, 4.0, 3.0),
        point(7.0, 6.0, 4.0),
    ];
    let k = job_of(
        &pts,
        serde_json::json!({ "kind": "kriging", "model": "spherical", "variogram": { "fit": "manual", "nugget": 0.0, "sill": 1.0, "range": 10.0 }, "points": 12, "error": true }),
        1.0,
    )
    .expect("a job");
    assert_eq!(k.bands(), 2);
    assert_eq!(k.style(1).ramp.as_deref(), Some("Arazi"));
    assert_eq!(k.style(1).bands, vec![1]);
    assert_eq!(k.style(2).ramp.as_deref(), Some("Viridis"));
    assert_eq!(k.style(2).bands, vec![2]);
    assert_eq!(k.style(1).nodata, None);
    let d = job_of(&pts, serde_json::json!({ "kind": "kernel", "radius": 3.0, "kernel": "quartic", "unit": "squareKilometre" }), 1.0)
        .expect("a job");
    assert_eq!(d.style(1).ramp.as_deref(), Some("Sıcaklık"));
    assert_eq!(d.style(1).nodata, Some(0.0));
}
