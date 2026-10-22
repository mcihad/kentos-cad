//! Analytic paper copies; model entities remain in their original document (ADR 0593).
use kentos_commands::Entity;
use kentos_geometry_core::{
    Vec2,
    entity::Shape,
    ops::warp::{Warp, warp_shape},
};
use kentos_sheet::{
    cad::{CadShape, Geometry, Polyline},
    model::Item,
    units::{RectUm, SizeUm},
};
use serde_json::{Value, json};

pub fn supports(e: &Entity) -> bool {
    match e {
        Entity::Circle(_) | Entity::Arc(_) | Entity::Ellipse(_) | Entity::Nurbs(_) => true,
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|b| *b != 0.))
                || p.widths.is_some()
                || p.holes.is_some()
                || p.parts.is_some()
        }
        _ => false,
    }
}
pub fn read(e: &Entity, height: f64) -> Option<(RectUm, CadShape)> {
    let shape = crate::geometry::shape(e);
    let b = kentos_geometry_core::entity::entity_bounds(&shape);
    let left = (b.min_x * 1e6).floor();
    let top = (height * 1000. - b.max_y * 1e6).floor();
    let width = ((b.max_x * 1e6).ceil() - left).max(1000.);
    let h = ((height * 1000. - b.min_y * 1e6).ceil() - top).max(1000.);
    if [left, top, width, h]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e9)
    {
        return None;
    }
    let frame = RectUm::new(left as i32, top as i32, width as i32, h as i32);
    let pt = |p: kentos_commands::Vec2| [p.x * 1e6 - left, height * 1000. - p.y * 1e6 - top];
    let sweep = kentos_geometry_core::geom::arc::sweep;
    let geometry = match e {
        Entity::Circle(c) => Geometry::Circle {
            center: pt(c.c),
            radius: c.r * 1e6,
        },
        Entity::Arc(a) => Geometry::Arc {
            center: pt(a.c),
            radius: a.r * 1e6,
            start: -a.a0,
            sweep: -sweep(a.a0, a.a1),
        },
        Entity::Ellipse(a) => Geometry::Ellipse {
            center: pt(a.c),
            major: [a.major.x * 1e6, -a.major.y * 1e6],
            ratio: a.ratio,
            start: -a.t0,
            sweep: -sweep(a.t0, a.t1),
        },
        Entity::Nurbs(n) => Geometry::Nurbs {
            degree: n.degree as u8,
            control: n.ctrl.iter().copied().map(pt).collect(),
            knots: n.knots.clone(),
            weights: n.weights.clone().unwrap_or_default(),
            closed: n.closed,
            periodic: n.periodic,
            fit: n.fit.iter().flatten().copied().map(pt).collect(),
            tangents: n.tangents.map(|ts| ts.map(|t| t.map(|p| [p.x, -p.y]))),
            fit_tolerance: n.fit_tolerance.map(|t| t * 1e6),
        },
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let closed = matches!(e, Entity::Polygon(_)) || p.closed;
            let line = |pts: &[kentos_commands::Vec2],
                        bulges: Option<&Vec<f64>>,
                        closed,
                        widths: Option<&Vec<[f64; 2]>>| Polyline {
                points: pts.iter().copied().map(pt).collect(),
                bulges: bulges
                    .map(|bs| bs.iter().map(|b| -b).collect())
                    .unwrap_or_default(),
                closed,
                widths: widths
                    .map(|ws| ws.iter().map(|w| [w[0] * 1e6, w[1] * 1e6]).collect())
                    .unwrap_or_default(),
            };
            let mut paths = vec![line(&p.pts, p.bulges.as_ref(), closed, p.widths.as_ref())];
            for hole in p.holes.iter().flatten() {
                paths.push(line(&hole.pts, hole.bulges.as_ref(), true, None));
            }
            for part in p.parts.iter().flatten() {
                paths.push(line(&part.pts, part.bulges.as_ref(), closed, None));
                for hole in part.holes.iter().flatten() {
                    paths.push(line(&hole.pts, hole.bulges.as_ref(), true, None));
                }
            }
            Geometry::Polyline { paths }
        }
        _ => return None,
    };
    let cad = CadShape {
        reference: SizeUm {
            width: frame.width,
            height: frame.height,
        },
        geometry,
    };
    kentos_sheet::cad::paths(&cad, &frame).ok()?;
    Some((frame, cad))
}
fn v(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}
/// Exact conic image through the existing affine core. Widths require a similarity;
/// a nonuniform wide path is reported rather than silently losing its width.
pub fn write(cad: &CadShape, item: &Item, height: f64, base: &Value) -> Option<(Vec<Entity>, u32)> {
    let frame = item.content_rect();
    let sx = f64::from(frame.width) / f64::from(cad.reference.width);
    let sy = f64::from(frame.height) / f64::from(cad.reference.height);
    let p = |q: [f64; 2]| {
        let p = crate::cad_layouts::world_point(
            item,
            kentos_sheet::cad::to_paper(cad, &frame, q),
            height,
        );
        Vec2::new(p.x, p.y)
    };
    let zero = p([0., 0.]);
    let a = f64::from(item.rotation).to_radians() / 1000.;
    let (sn, cs) = a.sin_cos();
    let warp = Warp::Affine {
        from: Vec2::new(0., 0.),
        to: zero,
        m: [
            cs * sx / 1e6,
            -sn * sx / 1e6,
            -sn * sy / 1e6,
            -cs * sy / 1e6,
        ],
    };
    let interval = |start: f64, sweep: f64| {
        if sweep > 0. {
            (start, start + sweep)
        } else {
            (start + sweep, start)
        }
    };
    let nurbs = |degree: usize,
                 control: &[[f64; 2]],
                 knots: Vec<f64>,
                 weights: Vec<f64>,
                 closed,
                 periodic,
                 fit: Vec<[f64; 2]>,
                 tangents: Option<[Option<[f64; 2]>; 2]>,
                 tol: Option<f64>| Shape::Nurbs {
        degree,
        ctrl: control.iter().copied().map(v).collect(),
        knots,
        weights: (!weights.is_empty()).then_some(weights),
        closed,
        periodic,
        fit: (!fit.is_empty()).then(|| fit.into_iter().map(v).collect()),
        tangents: tangents.map(|ts| ts.map(|t| t.map(v))),
        fit_tolerance: tol,
    };
    let mut shapes = match &cad.geometry {
        Geometry::Circle { center, radius } => vec![Shape::Circle {
            c: v(*center),
            r: *radius,
        }],
        Geometry::Arc {
            center,
            radius,
            start,
            sweep,
        } => {
            let (a0, a1) = interval(*start, *sweep);
            vec![Shape::Arc {
                c: v(*center),
                r: *radius,
                a0,
                a1,
            }]
        }
        Geometry::Ellipse {
            center,
            major,
            ratio,
            start,
            sweep,
        } => {
            let (t0, t1) = interval(*start, *sweep);
            vec![Shape::Ellipse {
                c: v(*center),
                major: v(*major),
                ratio: *ratio,
                t0,
                t1,
            }]
        }
        Geometry::Bezier { points, closed } => {
            let degree = points.len() - 1;
            vec![nurbs(
                degree,
                points,
                [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                vec![],
                *closed,
                false,
                vec![],
                None,
                None,
            )]
        }
        Geometry::Nurbs {
            degree,
            control,
            knots,
            weights,
            closed,
            periodic,
            fit,
            tangents,
            fit_tolerance,
        } => vec![nurbs(
            *degree as usize,
            control,
            knots.clone(),
            weights.clone(),
            *closed,
            *periodic,
            fit.clone(),
            *tangents,
            *fit_tolerance,
        )],
        Geometry::Polyline { paths } => {
            let mut shapes = vec![];
            for line in paths {
                if (sx - sy).abs() > 1e-10 * sx.max(sy) {
                    if line.widths.iter().flatten().any(|w| *w > 0.) {
                        return None;
                    }
                    let n = if line.closed {
                        line.points.len()
                    } else {
                        line.points.len() - 1
                    };
                    for i in 0..n {
                        let a = v(line.points[i]);
                        let b = v(line.points[(i + 1) % line.points.len()]);
                        if let Some(arc) = kentos_geometry_core::geom::bulge::bulge_arc(
                            a,
                            b,
                            line.bulges.get(i).copied().unwrap_or(0.),
                        ) {
                            let (a0, a1) = interval(arc.a0, arc.sweep);
                            shapes.push(Shape::Arc {
                                c: arc.c,
                                r: arc.r,
                                a0,
                                a1,
                            });
                        } else {
                            shapes.push(Shape::Line { a, b });
                        }
                    }
                } else {
                    shapes.push(Shape::Polyline {
                        pts: line.points.iter().copied().map(v).collect(),
                        bulges: (!line.bulges.is_empty()).then(|| line.bulges.clone()),
                        holes: None,
                        parts: None,
                        closed: line.closed,
                        widths: (!line.widths.is_empty()).then(|| line.widths.clone()),
                    });
                }
            }
            shapes
        }
    };
    let mut out = vec![];
    for shape in shapes.drain(..) {
        // Affine control/fit coordinates are exact; the generic warp intentionally
        // drops fit metadata, so map that metadata explicitly for paper copies.
        let mapped = if let Shape::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
            closed,
            periodic,
            fit,
            tangents,
            fit_tolerance,
        } = shape
        {
            Shape::Nurbs {
                degree,
                ctrl: ctrl.into_iter().map(|q| p([q.x, q.y])).collect(),
                knots,
                weights,
                closed,
                periodic,
                fit: fit.map(|ps| ps.into_iter().map(|q| p([q.x, q.y])).collect()),
                tangents: tangents.map(|ts| {
                    ts.map(|t| {
                        t.map(|t| {
                            Vec2::new(
                                cs * sx * t.x - sn * sy * t.y,
                                -sn * sx * t.x - cs * sy * t.y,
                            )
                        })
                    })
                }),
                fit_tolerance: fit_tolerance.map(|t| t * sx.max(sy) / 1e6),
            }
        } else {
            warp_shape(&shape, &[], &warp).ok()?.shape
        };
        let mut raw = serde_json::to_value(crate::geometry::edit_geometry(mapped)?).ok()?;
        for (k, value) in base.as_object()? {
            raw[k] = value.clone();
        }
        raw["id"] = json!(out.len() + base["id"].as_u64().unwrap_or(1) as usize);
        if let kentos_sheet::kinds::ItemKind::Shape(shape) = &item.kind
            && let Some(stroke) = &shape.stroke
        {
            raw["color"] = json!(stroke.color);
            raw["lineWeight"] = json!(f64::from(stroke.width) / 1000.);
        }
        out.push(serde_json::from_value(raw).ok()?);
    }
    Some((out, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_sheet::{
        kinds::{ItemKind, ShapeItem, ShapeKind},
        style::Stroke,
    };
    #[test]
    fn independent_paper_curves_keep_source_and_roundtrip_analytic_entities() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/paper-cad.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let e: Entity = serde_json::from_value(case["entity"].clone()).unwrap();
            let (frame, cad) = read(&e, 297.).unwrap_or_else(|| panic!("{}", case["name"]));
            let source = serde_json::to_value(&cad).unwrap();
            let item = Item::new(
                "curve",
                "Eğri",
                frame,
                ItemKind::Shape(ShapeItem {
                    shape: ShapeKind::Cad(cad.clone()),
                    stroke: Some(Stroke::solid("#123456", 250)),
                    fill: None,
                }),
            );
            let (out, loss) =
                write(&cad, &item, 297., &json!({"id":1,"layerId":"0","attrs":{}})).unwrap();
            assert_eq!(loss, 0);
            assert_eq!(out.len(), 1, "{}", case["name"]);
            let back = &out[0];
            match (&e, back) {
                (Entity::Circle(a), Entity::Circle(b)) => {
                    assert!((a.r - b.r).abs() < 1e-12);
                    assert!((a.c.x - b.c.x).abs() < 1e-12);
                    assert!((a.c.y - b.c.y).abs() < 1e-12);
                }
                (Entity::Arc(a), Entity::Arc(b)) => {
                    assert!((a.r - b.r).abs() < 1e-12);
                    let sweep = kentos_geometry_core::geom::arc::sweep;
                    assert!((sweep(a.a0, a.a1) - sweep(b.a0, b.a1)).abs() < 1e-10);
                }
                (Entity::Ellipse(a), Entity::Ellipse(b)) => {
                    assert!(
                        (a.major.x.hypot(a.major.y) - b.major.x.hypot(b.major.y)).abs() < 1e-12
                    );
                    assert!((a.ratio - b.ratio).abs() < 1e-12);
                }
                (Entity::Nurbs(a), Entity::Nurbs(b)) => {
                    assert_eq!(a.degree, b.degree);
                    assert_eq!(a.knots, b.knots);
                    assert_eq!(a.weights, b.weights);
                    assert_eq!(a.periodic, b.periodic);
                    assert_eq!(a.closed, b.closed);
                    for (a, b) in a.ctrl.iter().zip(&b.ctrl) {
                        assert!((a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
                    }
                    assert!((a.fit_tolerance.unwrap() - b.fit_tolerance.unwrap()).abs() < 1e-15);
                    assert!(b.fit.is_some() && b.tangents.is_some());
                    for (a, b) in a.fit.iter().flatten().zip(b.fit.iter().flatten()) {
                        assert!((a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
                    }
                    for (a, b) in a.tangents.iter().flatten().zip(b.tangents.iter().flatten()) {
                        assert_eq!(a, b);
                    }
                }
                (Entity::Polyline(a), Entity::Polyline(b)) => {
                    assert_eq!(a.bulges, b.bulges);
                    for (a, b) in a
                        .widths
                        .iter()
                        .flatten()
                        .flatten()
                        .zip(b.widths.iter().flatten().flatten())
                    {
                        assert!((a - b).abs() < 1e-12);
                    }
                }
                _ => panic!("{}: {} became {}", case["name"], e.kind(), back.kind()),
            }
            assert_eq!(serde_json::to_value(&cad).unwrap(), source);
            if let Entity::Circle(c) = e
                && c.r < 0.0005
            {
                assert_eq!(frame.width, 1000);
                if let Geometry::Circle { radius, .. } = cad.geometry {
                    assert!((radius - case["radiusUm"].as_f64().unwrap()).abs() < 1e-9);
                }
            }
        }
    }
    #[test]
    fn affine_circle_is_an_exact_ellipse_and_wide_nonsimilar_path_is_reported() {
        let e:Entity=serde_json::from_value(json!({"kind":"circle","id":1,"layerId":"0","attrs":{},"c":{"x":0.05,"y":0.05},"r":0.01})).unwrap();
        let (frame, cad) = read(&e, 297.).unwrap();
        let mut item = Item::new(
            "curve",
            "Eğri",
            frame,
            ItemKind::Shape(ShapeItem {
                shape: ShapeKind::Cad(cad.clone()),
                stroke: None,
                fill: None,
            }),
        );
        item.frame.width *= 2;
        item.rotation = 37000;
        let (out, _) = write(&cad, &item, 297., &json!({"id":1,"layerId":"0","attrs":{}})).unwrap();
        let Entity::Ellipse(e) = &out[0] else {
            panic!()
        };
        assert!((e.major.x.hypot(e.major.y) - 0.02).abs() < 1e-12);
        assert!((e.ratio - 0.5).abs() < 1e-12);
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/paper-cad.json"))
                .unwrap();
        let e: Entity = serde_json::from_value(
            fixture["cases"].as_array().unwrap().last().unwrap()["entity"].clone(),
        )
        .unwrap();
        let (frame, cad) = read(&e, 297.).unwrap();
        let mut item = Item::new(
            "wide",
            "Geniş çizgi",
            frame,
            ItemKind::Shape(ShapeItem {
                shape: ShapeKind::Cad(cad.clone()),
                stroke: None,
                fill: None,
            }),
        );
        item.frame.width *= 2;
        assert!(write(&cad, &item, 297., &json!({"id":1,"layerId":"0","attrs":{}})).is_none());
    }
}
