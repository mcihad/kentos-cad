//! Deep zoom (plan/render.md RM-24): an object's drawn geometry with its paths and rings split at the edge of the
//! view's box (`View::deep`, `kentos_geometry_core::geom::deep`). The pieces in the view have their ends near it,
//! so the GPU's float32 offsets from the camera place them to a fraction of a pixel however deep the view.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Part, Shape, ellipse_geom};
use kentos_geometry_core::geom::arc::sweep;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::geom::deep::{
    arc_in_box, bulge_path_in_box, ellipse_in_box, split_at_box,
};
use kentos_geometry_core::geom::ellipse::{ellipse_sweep, is_full_ellipse, minor_axis};
use kentos_geometry_core::geom::nurbs;
use kentos_geometry_core::geom::spline::catmull_rom_in_box;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::TAU;

use super::Deep;
use crate::style::compile::Geom;

/// A circle's, an arc's or an ellipse's geometry in a deep view (`geom::deep`): round to the view's tolerance in and
/// near its box, as coarse as the drawing draws it away from the box (72 steps a turn), its crossings of the box's
/// edge points of it; every point on the curve. A NURBS and a spline likewise, never coarser than the drawing draws
/// them, and open as it draws them (docs/adr/9233). None for any other shape, which is drawn as it is (or `refined`)
/// and split (`split`).
pub(super) fn curve(s: &Shape, d: &Deep) -> Option<Vec<Geom>> {
    let (b, tol) = (&d.bounds, d.tolerance);
    match *s {
        Shape::Circle { c, r } => {
            let mut ring = arc_in_box(c, r, 0.0, TAU, b, tol);
            // A ring: its first point not repeated.
            ring.pop();
            Some(vec![Geom::Line(vec![(ring, true)])])
        }
        Shape::Arc { c, r, a0, a1 } => Some(vec![Geom::Line(vec![(
            arc_in_box(c, r, a0, sweep(a0, a1), b, tol),
            false,
        )])]),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let e = ellipse_geom(c, major, ratio, t0, t1);
            let full = is_full_ellipse(&e);
            let mut pts = ellipse_in_box(c, major, minor_axis(&e), t0, ellipse_sweep(&e), b, tol);
            if full {
                pts.pop();
            }
            Some(vec![Geom::Line(vec![(pts, full)])])
        }
        Shape::Nurbs { .. } => {
            let pts = nurbs::of(s)?.tessellate_in_box(b, tol);
            (pts.len() >= 2).then(|| vec![Geom::Line(vec![(pts, false)])])
        }
        Shape::Spline { ref pts, closed } => Some(vec![Geom::Line(vec![(
            catmull_rom_in_box(pts, closed, 16.0, b, tol),
            false,
        )])]),
        _ => None,
    }
}

/// A polyline or an area with bulged edges as one whose arcs are points (`geom::deep::bulge_path_in_box`): round to
/// the view's tolerance in and near its box, 72 a turn away from it; its vertices as they are. Drawn through the
/// drawing's own way (`drawn`) as any straight one is, then split. None for any other shape, and for a wide polyline,
/// whose band follows its vertices' widths.
pub(super) fn refined(s: &Shape, d: &Deep) -> Option<Shape> {
    let path = |pts: &[Vec2], bulges: Option<&[f64]>, closed: bool| {
        bulge_path_in_box(pts, bulges, closed, &d.bounds, d.tolerance)
    };
    let ring = |r: &Ring| Ring {
        pts: path(&r.pts, r.bulges.as_deref(), true),
        bulges: None,
    };
    let holes = |h: &Option<Vec<Ring>>| h.as_ref().map(|h| h.iter().map(ring).collect());
    let bent = |bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>, parts: &Option<Vec<Part>>| {
        has_bulges(bulges.as_deref())
            || holes
                .iter()
                .flatten()
                .any(|h| has_bulges(h.bulges.as_deref()))
            || parts.iter().flatten().any(|p| {
                has_bulges(p.bulges.as_deref())
                    || p.holes
                        .iter()
                        .flatten()
                        .any(|h| has_bulges(h.bulges.as_deref()))
            })
    };
    match s {
        Shape::Polyline {
            pts,
            bulges,
            holes: h,
            parts,
            closed,
            widths,
        } => {
            let wide = widths.iter().flatten().any(|[a, b]| *a > 0.0 || *b > 0.0);
            (!wide && bent(bulges, h, parts)).then(|| Shape::Polyline {
                pts: path(pts, bulges.as_deref(), *closed),
                bulges: None,
                holes: holes(h),
                parts: parts.as_ref().map(|parts| {
                    parts
                        .iter()
                        .map(|p| Part {
                            pts: path(&p.pts, p.bulges.as_deref(), false),
                            bulges: None,
                            holes: holes(&p.holes),
                        })
                        .collect()
                }),
                closed: *closed,
                widths: widths.clone(),
            })
        }
        Shape::Polygon {
            pts,
            bulges,
            holes: h,
            parts,
        } => bent(bulges, h, parts).then(|| Shape::Polygon {
            pts: path(pts, bulges.as_deref(), true),
            bulges: None,
            holes: holes(h),
            parts: parts.as_ref().map(|parts| {
                parts
                    .iter()
                    .map(|p| Part {
                        pts: path(&p.pts, p.bulges.as_deref(), true),
                        bulges: None,
                        holes: holes(&p.holes),
                    })
                    .collect()
            }),
        }),
        _ => None,
    }
}

/// Curves round to `tolerance` all along, as a deep view rounds them in its box (`curve`, `refined`): the screen's
/// own outside a deep view (`View::curve_tolerance`, plan/render.md RM-09).
pub(super) fn everywhere(tolerance: f64) -> Deep {
    Deep {
        bounds: Bounds {
            min_x: f64::NEG_INFINITY,
            min_y: f64::NEG_INFINITY,
            max_x: f64::INFINITY,
            max_y: f64::INFINITY,
        },
        tolerance,
    }
}

/// `g` with every path and ring split where it crosses the edge of `b`; a mark as it is.
pub(super) fn split(g: Geom, b: &Bounds) -> Geom {
    let ring = |r: Vec<Vec2>| split_at_box(&r, true, b).unwrap_or(r);
    match g {
        Geom::Line(paths) => Geom::Line(
            paths
                .into_iter()
                .map(|(p, closed)| (split_at_box(&p, closed, b).unwrap_or(p), closed))
                .collect(),
        ),
        Geom::Fill(rings) => Geom::Fill(rings.into_iter().map(ring).collect()),
        Geom::Fills(parts) => Geom::Fills(
            parts
                .into_iter()
                .map(|rings| rings.into_iter().map(ring).collect())
                .collect(),
        ),
        mark @ Geom::Marker(_) => mark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::build::{LayerObjects, MODE_SET, Program, View, build_layer_with};
    use kentos_geometry_core::api::json::Json;
    use kentos_geometry_core::geom::deep::COARSE_STEP;
    use kentos_geometry_core::jsmath::{hypot, maximum};
    use kentos_geometry_core::store::Store;

    const BOX: Bounds = Bounds {
        min_x: -1.0,
        min_y: -1.0,
        max_x: 1.0,
        max_y: 1.0,
    };

    #[test]
    fn paths_and_rings_are_split_and_marks_kept() {
        let v = Vec2::new;
        let Geom::Line(paths) = split(
            Geom::Line(vec![(vec![v(-50.0, 0.0), v(50.0, 0.0)], false)]),
            &BOX,
        ) else {
            panic!("a line");
        };
        assert_eq!(
            paths[0].0,
            [v(-50.0, 0.0), v(-1.0, 0.0), v(1.0, 0.0), v(50.0, 0.0)]
        );
        let square = vec![v(-50.0, -50.0), v(50.0, -50.0), v(50.0, 0.5), v(-50.0, 0.5)];
        let Geom::Fill(rings) = split(Geom::Fill(vec![square.clone()]), &BOX) else {
            panic!("an area");
        };
        // The top edge crosses the box: its two points put in, and nothing else.
        assert_eq!(rings[0].len(), square.len() + 2);
        let Geom::Fills(parts) = split(Geom::Fills(vec![vec![square]]), &BOX) else {
            panic!("areas");
        };
        assert_eq!(parts[0][0], rings[0]);
        let mark = split(Geom::Marker(v(100.0, 100.0)), &BOX);
        assert!(matches!(mark, Geom::Marker(p) if p == v(100.0, 100.0)));
    }

    /// The segments a layer of one long line draws, built with `view`.
    fn segments(view: View) -> f64 {
        segments_of(
            r#"[{"id":1,"layerId":"a","kind":"line","attrs":{},"a":{"x":-5000,"y":0.25},"b":{"x":5000,"y":0.25}}]"#,
            view,
        )
    }

    /// The segments a layer of the object `json` draws, built with `view`.
    fn segments_of(json: &str, view: View) -> f64 {
        let mut store = Store::new();
        store.put_json(json).expect("object");
        let program = Program::read(
            r##"{"symbols":{},"renderer":null,"sets":[{"line":{"type":"line","layers":[{"id":"r","type":"simpleLine","color":"#FF0000","width":0.3}]}}],"refs":[],"colors":["#FF0000"],"assets":{}}"##,
        )
        .expect("program");
        let ids = [1.0];
        let how = [MODE_SET, 0, 0, 0];
        let lens = vec![-1; program.fields.len()];
        let o = LayerObjects {
            ids: &ids,
            objects: &how,
            texts: "",
            text_lens: &lens,
            numbers: &[],
            pieces: &[0; 8],
            bands: &[],
        };
        let out = build_layer_with(
            &store,
            &program,
            &o,
            None,
            Vec2::new(0.0, 0.0),
            1000.0,
            false,
            view,
        )
        .expect("build");
        let Json::Arr(batches) = Json::parse(&out.json).expect("json") else {
            panic!("{}", out.json);
        };
        batches
            .iter()
            .map(|b| match b.get("len") {
                Json::Num(n) => *n,
                _ => 0.0,
            })
            .sum()
    }

    #[test]
    fn a_deep_view_draws_a_long_line_through_it_in_three_pieces() {
        let plain = segments(View::default());
        let deep = segments(View {
            deep: Some(Deep {
                bounds: BOX,
                tolerance: 1e-3,
            }),
            ..View::default()
        });
        assert!(plain > 0.0, "the line is drawn");
        assert!(
            deep > plain,
            "the line's piece in the box is drawn on its own: {plain} → {deep}"
        );
        // A box the line misses: drawn as it is.
        let beside = segments(View {
            deep: Some(Deep {
                bounds: Bounds {
                    min_x: -1.0,
                    min_y: 10.0,
                    max_x: 1.0,
                    max_y: 12.0,
                },
                tolerance: 1e-3,
            }),
            ..View::default()
        });
        assert_eq!(beside, plain);
    }

    #[test]
    fn a_deep_view_s_circle_is_round_in_its_box() {
        let (c, r) = (Vec2::new(-49.5, 0.3), 50.0);
        let d = Deep {
            bounds: BOX,
            tolerance: 1e-4,
        };
        let Some(parts) = curve(&Shape::Circle { c, r }, &d) else {
            panic!("a circle's own geometry");
        };
        let [Geom::Line(paths)] = parts.as_slice() else {
            panic!("one line");
        };
        let (ring, closed) = &paths[0];
        assert!(*closed);
        // More points than the drawing's 72 chords: the ones in the box.
        assert!(ring.len() > 72 && ring.len() < 72 + 80, "{}", ring.len());
        // An arc away from the box: the drawing's own step.
        let far = curve(
            &Shape::Arc {
                c: Vec2::new(100.0, 100.0),
                r: 10.0,
                a0: 0.0,
                a1: TAU / 4.0,
            },
            &d,
        );
        let Some([Geom::Line(paths)]) = far.as_deref() else {
            panic!("an arc");
        };
        assert_eq!(
            paths[0].0.len(),
            (TAU / 4.0 / COARSE_STEP).ceil() as usize + 1
        );
        assert!(
            curve(
                &Shape::Point {
                    p: Vec2::new(0.0, 0.0),
                    z: None,
                    parts: None
                },
                &d
            )
            .is_none()
        );
    }

    #[test]
    fn a_deep_view_s_ellipse_and_bulged_edges_are_round_in_its_box() {
        let d = Deep {
            bounds: BOX,
            tolerance: 1e-4,
        };
        // A whole 40 × 10 m ellipse whose major end lies in the box: a ring, finer in the box than 72 steps a turn.
        let e = Shape::Ellipse {
            c: Vec2::new(-39.8, 0.1),
            major: Vec2::new(40.0, 0.0),
            ratio: 0.25,
            t0: 0.0,
            t1: TAU,
        };
        let parts = curve(&e, &d);
        let Some([Geom::Line(paths)]) = parts.as_deref() else {
            panic!("an ellipse");
        };
        assert!(paths[0].1, "a whole ellipse is a ring");
        assert!(
            paths[0].0.len() > 72 && paths[0].0.len() < 72 + 200,
            "{}",
            paths[0].0.len()
        );

        // A half disc bulging right through the box: its arc's points put in, every one on the arc.
        let v = Vec2::new;
        let area = Shape::Polygon {
            pts: vec![v(-19.5, -20.0), v(-19.5, 20.0)],
            bulges: Some(vec![1.0, 0.0]),
            holes: None,
            parts: None,
        };
        let Some(Shape::Polygon {
            pts, bulges: None, ..
        }) = refined(&area, &d)
        else {
            panic!("the area with its arc as points");
        };
        assert!(pts.len() > 36, "{}", pts.len());
        assert_eq!(pts[0], v(-19.5, -20.0));
        assert!(
            pts.iter()
                .all(|p| p.x == -19.5 || (hypot(p.x + 19.5, p.y) - 20.0).abs() < 1e-9)
        );
        // A straight area, and a wide polyline (its band follows its vertices): drawn as they are.
        let straight = Shape::Polygon {
            pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0)],
            bulges: Some(vec![0.0; 3]),
            holes: None,
            parts: None,
        };
        assert!(refined(&straight, &d).is_none());
        let wide = Shape::Polyline {
            pts: vec![v(-19.5, -20.0), v(-19.5, 20.0)],
            bulges: Some(vec![1.0]),
            holes: None,
            parts: None,
            closed: false,
            widths: Some(vec![[0.5, 0.5], [0.5, 0.5]]),
        };
        assert!(refined(&wide, &d).is_none());
    }

    #[test]
    fn a_deep_view_draws_a_bulged_polyline_s_arc_finely_in_its_box() {
        let json = r#"[{"id":1,"layerId":"a","kind":"polyline","attrs":{},"pts":[{"x":-19.5,"y":-20},{"x":-19.5,"y":20}],"bulges":[1]}]"#;
        let plain = segments_of(json, View::default());
        let deep = segments_of(
            json,
            View {
                deep: Some(Deep {
                    bounds: BOX,
                    tolerance: 1e-4,
                }),
                ..View::default()
            },
        );
        assert!(plain > 0.0, "the arc is drawn");
        assert!(deep > plain, "finer in the box: {plain} → {deep}");
    }

    /// plan/render.md RM-09: outside a deep view the screen's curves are round to its tolerance all along, a big circle
    /// with as many chords as keep every one within it; without one, the drawing's 72 chords a turn.
    #[test]
    fn the_screen_s_curves_are_round_to_its_tolerance_everywhere() {
        let (c, r, tol) = (Vec2::new(10.0, -3.0), 22.0, 5e-4);
        let Some(parts) = curve(&Shape::Circle { c, r }, &everywhere(tol)) else {
            panic!("a circle");
        };
        let [Geom::Line(paths)] = parts.as_slice() else {
            panic!("one line");
        };
        let ring = &paths[0].0;
        // Every chord's middle within the tolerance of the circle, the closing one too.
        let worst = (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                r - hypot((a.x + b.x) / 2.0 - c.x, (a.y + b.y) / 2.0 - c.y)
            })
            .fold(0.0, maximum);
        assert!(worst <= tol * 1.000_001, "{worst}");
        // 2 acos(1 − tol / r) a chord: about 466 a turn for 22 m at half a millimetre.
        assert!((460..=470).contains(&ring.len()), "{}", ring.len());
        // A view with the tolerance draws more segments than one without; a deep view's box takes precedence.
        let json =
            r#"[{"id":1,"layerId":"a","kind":"circle","attrs":{},"c":{"x":10,"y":-3},"r":22}]"#;
        let plain = segments_of(json, View::default());
        let round = segments_of(
            json,
            View {
                curve_tolerance: Some(tol),
                ..View::default()
            },
        );
        assert!(round > 6.0 * plain, "{plain} → {round}");
    }
}
