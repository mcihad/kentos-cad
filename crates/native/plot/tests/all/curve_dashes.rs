//! CAD-ADV-11, ADR 1121 and ADR 1122 in the real vector and PDF output: a CAD line type's dash cut on the source
//! curve at 1:1, 1:20 and 1:100 (arc, circle, ellipse, bulge path, NURBS), and a dotted wide polyline's dots as the
//! band's cross-sections. Expected places are the curves' closed forms (r·θ, the band's width), not the output.
use kentos_domain::Document;
use kentos_plot::{book, maps::MapSource};
use kentos_scene_query::Spatial;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    pdf::MapPath,
    project::project_sheets,
};
use serde_json::{Value, json};

fn source() -> (Value, Value) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    (fx["model"].clone(), fx["book"].clone())
}

fn document(raw: Value, page: Value) -> (Document, SheetBook) {
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    (doc, book)
}

/// The first sheet's map as the PDF receives it (ground metres).
fn vector_paths(doc: &Document, book: &SheetBook, scale: f64) -> Vec<MapPath> {
    let render = book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list
        .prims
        .iter()
        .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
        .unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = book::library_of(doc);
    let src = MapSource {
        doc,
        store: spatial.store(),
        library: &library,
        origin: kentos_geometry_core::Vec2::new(0., 0.),
        map: None,
        level: None,
        phase: None,
        hidden: &[],
        pens: &[],
        mono: false,
        options: &[],
    };
    let kentos_sheet::pdf::MapContent::Vector(v) = kentos_plot::maps::vector_content(
        &src,
        &map.layers,
        &map.layer_overrides,
        scale,
        map.extent.unwrap(),
        &[],
    )
    .unwrap() else {
        panic!("expected vectors")
    };
    v.layers.into_iter().flat_map(|l| l.paths).collect()
}

/// One entity on the drawing's layer with a 3 mm dash / 1 mm gap paper pattern, the sheet at 1:`scale`.
fn drawn(entity: Value, scale: u32, pattern: Value) -> (Document, SheetBook) {
    let (mut raw, mut page) = source();
    raw["settings"]["lineTypes"] = json!([pattern]);
    let mut entity = entity;
    entity["id"] = 1.into();
    entity["layerId"] = raw["entities"][0]["layerId"].clone();
    entity["attrs"] = json!({});
    entity["linePattern"] = "P".into();
    raw["entities"] = json!([entity]);
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
    document(raw, page)
}

/// ADR 1122: every dash of a circle and an arc starts at a multiple of the 4 mm period of true arc length (paper),
/// runs 3 mm, at three scales; the PDF is produced and the document is untouched. A cut is the curve's own point,
/// so the measure is the reading's rounding (`TOL`, paper mm); a chord-measured dash would drift about 0.01 mm by
/// the 16th dash at the plot's 0.005 mm sagitta.
const TOL: f64 = 1e-6;
#[test]
fn curve_dashes_are_cut_at_true_arc_distances_at_three_scales() {
    let pattern = json!({"name":"P","unit":"paper","elements":[{"length":3.},{"length":-1.}]});
    for scale in [1u32, 20, 100] {
        let s = f64::from(scale);
        let r = s * 0.01; // 10 mm on the paper at every scale
        for kind in ["arc", "circle", "cw"] {
            let entity = match kind {
                "arc" => json!({"kind":"arc","c":{"x":0.,"y":0.},"r":r,"a0":0.3,"a1":2.9}),
                "circle" => json!({"kind":"circle","c":{"x":0.,"y":0.},"r":r}),
                _ => json!({"kind":"polyline","pts":[{"x":r,"y":0.},{"x":-r,"y":0.}],"bulges":[-1.]}),
            };
            let (doc, book) = drawn(entity, scale, pattern.clone());
            let before = doc.to_snapshot_v2();
            let paths = vector_paths(&doc, &book, s);
            let dashed: Vec<_> = paths
                .iter()
                .filter(|p| p.stroke.as_ref().is_some_and(|s| !s.dash.is_empty()))
                .collect();
            assert!(dashed.is_empty(), "{scale}/{kind}: no chord-measured dash left");
            let pieces: Vec<_> = paths.iter().filter(|p| p.stroke.is_some()).collect();
            // Paper millimetres along the curve from its start, by angle.
            let along = |q: [f64; 2]| {
                let a = q[1].atan2(q[0]);
                let mm = |t: f64| t * r * 1000. / s;
                // A start read a rounding below its own angle is the start, not a whole turn on.
                let turn = |t: f64| {
                    let t = t.rem_euclid(std::f64::consts::TAU);
                    if std::f64::consts::TAU - t < 1e-9 { 0. } else { t }
                };
                match kind {
                    "arc" => mm(turn(a - 0.3)),
                    "circle" => mm(turn(a)),
                    _ => mm(turn(-a)),
                }
            };
            let length = match kind {
                "arc" => 2.6 * 10.,
                "circle" => std::f64::consts::TAU * 10.,
                _ => std::f64::consts::PI * 10.,
            };
            let expected = (length / 4.).ceil() as usize - usize::from(kind == "circle");
            assert_eq!(pieces.len(), expected, "{scale}/{kind}");
            for p in &pieces {
                let (a, b) = (along(p.points[0]), along(*p.points.last().unwrap()));
                let seam = kind == "circle" && b < a;
                if !seam {
                    assert!((a - 4. * (a / 4.).round()).abs() < TOL, "{scale}/{kind}: start {a}");
                    let run = b - a;
                    assert!(
                        (run - 3.).abs() < TOL || (b - length).abs() < TOL,
                        "{scale}/{kind}: run {run} at {a}"
                    );
                } else {
                    // The run through the seam: 60..62.83 and 0..3 joined; it ends 3 mm past the seam.
                    assert!((b - 3.).abs() < TOL && (a - 60.).abs() < TOL, "{a} {b}");
                }
            }
            book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
            assert_eq!(doc.to_snapshot_v2(), before);
        }
    }
}

/// ADR 1122: an ellipse's and a NURBS curve's dashes by their own arc length, compared with the length
/// the geometry core integrates for the same curve (independently of the cut).
#[test]
fn ellipse_and_nurbs_dashes_follow_their_arc_length() {
    use kentos_geometry_core::geom::{ellipse, nurbs};
    let pattern = json!({"name":"P","unit":"paper","elements":[{"length":3.},{"length":-1.}]});
    for scale in [1u32, 20, 100] {
        let s = f64::from(scale);
        let r = s * 0.01;
        let shapes = [
            json!({"kind":"ellipse","c":{"x":0.,"y":0.},"major":{"x":r,"y":0.},"ratio":0.4,"t0":0.2,"t1":2.8}),
            json!({"kind":"nurbs","degree":2,"ctrl":[{"x":r,"y":0.},{"x":r,"y":r},{"x":0.,"y":r}],
                   "knots":[0.,0.,0.,1.,1.,1.],"weights":[1.,std::f64::consts::FRAC_1_SQRT_2,1.]}),
        ];
        for entity in shapes {
            let (doc, book) = drawn(entity.clone(), scale, pattern.clone());
            let paths = vector_paths(&doc, &book, s);
            let pieces: Vec<_> = paths.iter().filter(|p| p.stroke.is_some()).collect();
            assert!(pieces.iter().all(|p| p.stroke.as_ref().unwrap().dash.is_empty()));
            // Arc length from the start to a point, in paper mm.
            let along: Box<dyn Fn([f64; 2]) -> f64> = if entity["kind"] == "ellipse" {
                let e = ellipse::EllipseGeom {
                    c: kentos_geometry_core::Vec2::new(0., 0.),
                    major: kentos_geometry_core::Vec2::new(r, 0.),
                    ratio: 0.4,
                    t0: 0.2,
                    t1: 0.2,
                };
                Box::new(move |q| {
                    let t = ellipse::param_of_point(&e, kentos_geometry_core::Vec2::new(q[0], q[1]));
                    if (t - 0.2).abs() < 1e-9 {
                        return 0.;
                    }
                    let part = ellipse::EllipseGeom { t1: t, ..e };
                    ellipse::ellipse_length(&part) * 1000. / s
                })
            } else {
                let ctrl = vec![
                    kentos_geometry_core::Vec2::new(r, 0.),
                    kentos_geometry_core::Vec2::new(r, r),
                    kentos_geometry_core::Vec2::new(0., r),
                ];
                Box::new(move |q| {
                    let knots = [0., 0., 0., 1., 1., 1.];
                    let w = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
                    let n = nurbs::Nurbs { degree: 2, ctrl: &ctrl, knots: &knots, weights: Some(&w) };
                    let u = n.closest(kentos_geometry_core::Vec2::new(q[0], q[1]));
                    n.length_between(0., u) * 1000. / s
                })
            };
            assert!(pieces.len() >= 3, "{scale}: {}", pieces.len());
            for p in &pieces[..pieces.len() - 1] {
                let a = along(p.points[0]);
                let b = along(*p.points.last().unwrap());
                assert!((a - 4. * (a / 4.).round()).abs() < TOL, "{scale}/{}: start {a}", entity["kind"]);
                assert!((b - a - 3.).abs() < TOL, "{scale}/{}: run {}", entity["kind"], b - a);
            }
            book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
        }
    }
}

/// ADR 1121: a dot on a wide polyline is the band's cross-section, the band's width long, at its dot's distance;
/// where the width is zero the output is refused, never drawn without its dots.
#[test]
fn a_dotted_wide_polyline_prints_its_cross_sections() {
    let dotted = json!({"name":"P","unit":"paper","elements":[{"length":0.},{"length":-2.}]});
    for scale in [1u32, 20, 100] {
        let s = f64::from(scale);
        let half = s * 0.01; // 20 mm long on the paper
        let w = s * 0.0005; // 0.5 mm wide on the paper
        let entity = json!({"kind":"polyline","pts":[{"x":-half,"y":0.},{"x":half,"y":0.}],"widths":[[w,w],[0.,0.]]});
        let (doc, book) = drawn(entity, scale, dotted.clone());
        let paths = vector_paths(&doc, &book, s);
        let cuts: Vec<_> = paths
            .iter()
            .filter(|p| p.stroke.is_some() && p.points.len() == 2)
            .collect();
        assert_eq!(cuts.len(), 11, "{scale}: a dot every 2 mm over 20 mm, both ends");
        for (k, c) in cuts.iter().enumerate() {
            let x = -half + k as f64 * 2. * s / 1000.;
            assert!((c.points[0][0] - x).abs() < 1e-12 && (c.points[1][0] - x).abs() < 1e-12, "{scale}/{k}");
            assert!((c.points[0][1] - w / 2.).abs() < 1e-12 && (c.points[1][1] + w / 2.).abs() < 1e-12);
        }
        book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
        // A taper to nothing at the start: the first dot has no width there.
        let tapered = json!({"kind":"polyline","pts":[{"x":-half,"y":0.},{"x":half,"y":0.}],"widths":[[0.,w],[0.,0.]]});
        let (doc, _) = drawn(tapered, scale, dotted.clone());
        let refused = book::book_pdf(&doc, &[], "Test", "2026-11-06");
        assert_eq!(refused.err().map(|e| e.code).as_deref(), Some("linetype_unsupported"), "{scale}");
    }
}

/// The evidence pictures of ADR 1121/1122 (not a check): at 1:20 and 1:100 the sheet's PDF and its CPU preview of
/// a dashed circle, arc, ellipse, NURBS and bulge path, a dash-dot arc and a dotted wide polyline, each beside blue
/// reference ticks at every 4 mm of true arc length. `cargo test -p kentos-plot --test all
/// curve_dashes::curve_dash_pictures -- --ignored`; written to `.run/gui-evidence/` (never into git).
#[test]
#[ignore]
fn curve_dash_pictures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&dir).unwrap();
    for scale in [20u32, 100] {
        let s = f64::from(scale);
        let mm = s / 1000.; // one paper millimetre in metres
        let (mut raw, mut page) = source();
        raw["settings"]["lineTypes"] = json!([
            {"name":"P","unit":"paper","elements":[{"length":3.},{"length":-1.}]},
            {"name":"N","unit":"paper","elements":[{"length":0.},{"length":-2.}]},
            {"name":"D","unit":"paper","elements":[{"length":4.},{"length":-1.},{"length":0.},{"length":-1.}]}
        ]);
        let layer = raw["entities"][0]["layerId"].clone();
        let mut list = Vec::new();
        let mut add = |mut e: Value, pattern: Option<&str>| {
            e["id"] = json!(list.len() + 1);
            e["layerId"] = layer.clone();
            e["attrs"] = json!({});
            if let Some(p) = pattern {
                e["linePattern"] = json!(p);
            } else {
                e["color"] = json!("#1f6feb");
            }
            list.push(e);
        };
        let r = 15. * mm;
        let c = |x: f64, y: f64| json!({"x": x * mm, "y": y * mm});
        add(json!({"kind":"circle","c":c(-60.,20.),"r":r}), Some("P"));
        add(json!({"kind":"arc","c":c(-20.,20.),"r":r,"a0":0.3,"a1":2.9}), Some("P"));
        add(json!({"kind":"ellipse","c":c(25.,20.),"major":{"x":20.*mm,"y":0.},"ratio":0.45,"t0":0.,"t1":0.}), Some("P"));
        add(json!({"kind":"nurbs","degree":2,"ctrl":[c(50.,5.),c(70.,40.),c(90.,5.)],"knots":[0.,0.,0.,1.,1.,1.]}), Some("P"));
        add(json!({"kind":"polyline","pts":[c(-80.,-25.),c(-50.,-25.),c(-50.,-5.)],"bulges":[0.,0.6,0.]}), Some("P"));
        add(json!({"kind":"arc","c":c(-5.,-30.),"r":r,"a0":0.2,"a1":3.}), Some("D"));
        add(json!({"kind":"polyline","pts":[c(25.,-30.),c(85.,-30.)],"widths":[[1.5*mm,1.5*mm],[0.,0.]]}), Some("N"));
        // Blue ticks just outside the circle and the arc at every 4 mm of their own arc length.
        for (cx, a0, sweep) in [(-60., 0., std::f64::consts::TAU), (-20., 0.3, 2.6)] {
            let n = (sweep * 15. / 4.).floor() as i32;
            for k in 0..=n {
                let a = a0 + f64::from(k) * 4. / 15.;
                let (u, v) = (a.cos(), a.sin());
                add(json!({"kind":"line","a":c(cx + 16. * u, 20. + 16. * v),"b":c(cx + 18. * u, 20. + 18. * v)}), None);
            }
        }
        raw["entities"] = Value::Array(list);
        page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
        page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
        let (doc, _) = document(raw, page);
        let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
        std::fs::write(dir.join(format!("adr1122-egri-1-{scale}.pdf")), &pdf.bytes).unwrap();
        let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 300).unwrap();
        std::fs::write(dir.join(format!("adr1122-egri-1-{scale}-cpu.png")), &image.png).unwrap();
    }
}
