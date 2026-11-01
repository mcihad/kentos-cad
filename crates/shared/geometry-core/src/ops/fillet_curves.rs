//! Analytic circular fillets. Offset loci find all centres; the two picks choose
//! the nearest admissible contacts. No tessellation or screen tolerance is used.
use crate::entity::Shape;
use crate::geom::arc::{ON_ARC_EPS, norm_angle, on_arc, sweep};
use crate::geom::intersect::{circle_circle, line_circle_params};
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_min, sin};
use crate::vec2::Vec2;

const EPS: f64 = 1e-9;
pub const KINDS: &str = "Bu yuvarlama çizgi, yay ve daire çiftlerinde çalışır; elips/NURBS ve yaylı çoklu çizgi için destek henüz yok.";

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}
fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}
fn scale(a: Vec2, k: f64) -> Vec2 {
    Vec2::new(a.x * k, a.y * k)
}
fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}
fn length(a: Vec2) -> f64 {
    js_hypot(a.x, a.y)
}

#[derive(Clone, Copy)]
enum Curve {
    Line(Vec2, Vec2),
    Round(Vec2, f64),
}
fn curve(s: &Shape, origin: Vec2) -> Option<Curve> {
    let finite = |p: Vec2| p.x.is_finite() && p.y.is_finite();
    match *s {
        Shape::Line { a, b } if finite(a) && finite(b) && length(sub(b, a)) > EPS => {
            Some(Curve::Line(sub(a, origin), sub(b, origin)))
        }
        Shape::Arc { c, r, a0, a1 } if finite(c) && r.is_finite() && r > EPS && a0.is_finite() && a1.is_finite() => {
            Some(Curve::Round(sub(c, origin), r))
        }
        Shape::Circle { c, r } if finite(c) && r.is_finite() && r > EPS => {
            Some(Curve::Round(sub(c, origin), r))
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Locus {
    Line(Vec2, Vec2),
    Circle(Vec2, f64),
}
fn loci(c: Curve, r: f64) -> Vec<Locus> {
    match c {
        Curve::Line(a, b) => {
            let d = sub(b, a);
            let n = scale(Vec2::new(-d.y, d.x), r / length(d));
            vec![
                Locus::Line(add(a, n), add(b, n)),
                Locus::Line(sub(a, n), sub(b, n)),
            ]
        }
        Curve::Round(c, radius) => vec![
            Locus::Circle(c, radius + r),
            Locus::Circle(c, (radius - r).abs()),
        ],
    }
}
fn intersections(a: Locus, b: Locus) -> Vec<Vec2> {
    match (a, b) {
        (Locus::Circle(a, r), Locus::Circle(b, s)) => circle_circle(a, r, b, s),
        (Locus::Line(a, b), Locus::Circle(c, r)) | (Locus::Circle(c, r), Locus::Line(a, b)) => {
            line_circle_params(a, b, c, r)
                .into_iter()
                .map(|t| add(a, scale(sub(b, a), t)))
                .collect()
        }
        (Locus::Line(a, b), Locus::Line(c, d)) => crate::geom::intersect::line_line(a, b, c, d)
            .into_iter()
            .map(|h| h.p)
            .collect(),
    }
}
fn contacts(curve: Curve, center: Vec2, radius: f64) -> Vec<Vec2> {
    match curve {
        Curve::Line(a, b) => {
            let d = sub(b, a);
            vec![add(a, scale(d, dot(sub(center, a), d) / dot(d, d)))]
        }
        Curve::Round(c, r) => {
            let d = sub(center, c);
            let n = length(d);
            if n <= EPS {
                return Vec::new();
            } // coincident loci: infinitely many, no unique contact
            [-1., 1.]
                .into_iter()
                .map(|sign| add(c, scale(d, sign * r / n)))
                .filter(|q| {
                    (length(sub(*q, center)) - radius).abs() <= EPS * js_max(js_max(radius, r), 1.)
                })
                .collect()
        }
    }
}

/// Trim an open curve on the side that keeps its pick. Circles stay circles.
/// Arc contacts outside its finite span are refused, rather than silently extended.
fn trimmed(s: &Shape, q: Vec2, pick: Vec2, origin: Vec2) -> Option<Shape> {
    match *s {
        Shape::Line { a, b } => {
            let (a, b) = (sub(a, origin), sub(b, origin));
            let d = sub(b, a);
            let along = dot(sub(pick, q), d);
            let keep_a = if along.abs() > EPS * length(d) {
                along < 0.
            } else {
                length(sub(a, q)) >= length(sub(b, q))
            };
            let kept = if keep_a { a } else { b };
            if length(sub(kept, q)) <= EPS {
                return None;
            }
            Some(if keep_a {
                Shape::Line {
                    a: add(a, origin),
                    b: add(q, origin),
                }
            } else {
                Shape::Line {
                    a: add(q, origin),
                    b: add(b, origin),
                }
            })
        }
        Shape::Circle { .. } => Some(s.clone()),
        Shape::Arc { c, r, a0, a1 } => {
            let c_local = sub(c, origin);
            let theta = norm_angle(atan2(q.y - c_local.y, q.x - c_local.x));
            let sw = sweep(a0, a1);
            if !on_arc(theta, a0, sw, ON_ARC_EPS) {
                return None;
            }
            let t = norm_angle(theta - a0);
            let t = if t > sw && t > std::f64::consts::TAU - ON_ARC_EPS {
                0.
            } else {
                js_min(t, sw)
            };
            let mut picked = norm_angle(atan2(pick.y - c_local.y, pick.x - c_local.x) - a0);
            if picked > sw {
                // A real pointer need not land exactly on the finite arc. A
                // pick just before its start belongs to that end, not one turn later.
                let point = |t: f64| add(c_local, Vec2::new(r * cos(t), r * sin(t)));
                picked = if length(sub(pick, point(a0))) <= length(sub(pick, point(a1))) {
                    0.
                } else {
                    sw
                };
            }
            let keep_start = picked <= t;
            if (if keep_start { t } else { sw - t }) * r <= EPS {
                return None;
            }
            Some(if keep_start {
                Shape::Arc {
                    c,
                    r,
                    a0,
                    a1: theta,
                }
            } else {
                Shape::Arc {
                    c,
                    r,
                    a0: theta,
                    a1,
                }
            })
        }
        _ => None,
    }
}

pub struct CurvedFillet {
    pub first: Shape,
    pub second: Shape,
    pub arc: Option<Shape>,
}

/// G1 is directional: two collinear tangents pointing into the same side
/// make a cusp, not a smooth join. The retained curve and connector must
/// leave the contact in opposite directions. A full circle has both sides.
fn smooth(kept: &Shape, q: Vec2, center: Vec2, start: f64, origin: Vec2) -> bool {
    let away = match *kept {
        Shape::Circle { .. } => return true,
        Shape::Line { a, b } => {
            let (a, b) = (sub(a, origin), sub(b, origin));
            if length(sub(q, a)) <= length(sub(q, b)) {
                sub(b, q)
            } else {
                sub(a, q)
            }
        }
        Shape::Arc { c, a0, a1, .. } => {
            let v = sub(q, sub(c, origin));
            let t = Vec2::new(-v.y, v.x);
            let theta = norm_angle(atan2(v.y, v.x));
            let from_start = js_min(norm_angle(theta - a0), norm_angle(a0 - theta));
            let from_end = js_min(norm_angle(theta - a1), norm_angle(a1 - theta));
            if from_start <= from_end {
                t
            } else {
                scale(t, -1.)
            }
        }
        _ => return false,
    };
    let v = sub(q, center);
    let t = Vec2::new(-v.y, v.x);
    let theta = norm_angle(atan2(v.y, v.x));
    let at_start = js_min(norm_angle(theta - start), norm_angle(start - theta)) <= ON_ARC_EPS;
    dot(away, if at_start { t } else { scale(t, -1.) }) < 0.
}

/// Fillet of two independent lines/arcs/circles. Line–line callers keep the
/// existing corner solver (its endpoint/extension semantics are unchanged).
pub fn fillet(
    first: &Shape,
    pick1: Vec2,
    second: &Shape,
    pick2: Vec2,
    radius: f64,
) -> Result<CurvedFillet, String> {
    if !radius.is_finite() || radius < 0. {
        return Err("Yarıçap sonlu ve sıfırdan büyük veya sıfır olmalı.".into());
    }
    if [pick1.x, pick1.y, pick2.x, pick2.y]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err("Tıklanan noktalar sonlu koordinatlar olmalı.".into());
    }
    let origin = match *first {
        Shape::Line { a, .. } => a,
        Shape::Arc { c, .. } | Shape::Circle { c, .. } => c,
        _ => return Err(KINDS.into()),
    };
    let a = curve(first, origin).ok_or(KINDS)?;
    let b = curve(second, origin).ok_or(KINDS)?;
    let (p1, p2) = (sub(pick1, origin), sub(pick2, origin));
    let mut best: Option<(f64, CurvedFillet)> = None;
    for l in loci(a, radius) {
        for m in loci(b, radius) {
            for center in intersections(l, m) {
                let (qa, qb) = if radius == 0. {
                    (vec![center], vec![center])
                } else {
                    (contacts(a, center, radius), contacts(b, center, radius))
                };
                for q1 in &qa {
                    for q2 in &qb {
                        let Some(next1) = trimmed(first, *q1, p1, origin) else {
                            continue;
                        };
                        let Some(next2) = trimmed(second, *q2, p2, origin) else {
                            continue;
                        };
                        let arc = if radius == 0. {
                            None
                        } else {
                            if length(sub(*q1, *q2)) <= EPS {
                                continue;
                            }
                            let mut a0 = norm_angle(atan2(q1.y - center.y, q1.x - center.x));
                            let mut a1 = norm_angle(atan2(q2.y - center.y, q2.x - center.x));
                            if sweep(a0, a1) > std::f64::consts::PI {
                                (a0, a1) = (a1, a0);
                            }
                            if !smooth(&next1, *q1, center, a0, origin)
                                || !smooth(&next2, *q2, center, a0, origin)
                            {
                                continue;
                            }
                            Some(Shape::Arc {
                                c: add(center, origin),
                                r: radius,
                                a0,
                                a1,
                            })
                        };
                        if next1 == *first && next2 == *second && arc.is_none() {
                            continue;
                        }
                        let score = length(sub(*q1, p1)) + length(sub(*q2, p2));
                        if best.as_ref().is_none_or(|(old, _)| score < *old) {
                            best = Some((
                                score,
                                CurvedFillet {
                                    first: next1,
                                    second: next2,
                                    arc,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_,f)|f).ok_or_else(||"Bu tarafta yarıçapa uygun, yayların uçlarını koruyan teğet çözüm yok. Seçim yerini veya yarıçapı değiştirin; eş merkezli/çöken eğriler yuvarlanmaz.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, TAU};
    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
    fn arc_of(s: Option<Shape>) -> (Vec2, f64, f64, f64) {
        let Some(Shape::Arc { c, r, a0, a1 }) = s else {
            panic!("analytic arc");
        };
        (c, r, a0, a1)
    }
    fn ends(c: Vec2, r: f64, a0: f64, a1: f64) -> [Vec2; 2] {
        [a0, a1].map(|a| add(c, p(r * a.cos(), r * a.sin())))
    }
    #[test]
    fn line_arc_uses_exact_offset_loci_and_keeps_the_clicked_arc_span() {
        let line = Shape::Line {
            a: p(-5., 0.),
            b: p(10., 0.),
        };
        let arc = Shape::Arc {
            c: p(4., 3.),
            r: 2.,
            a0: PI,
            a1: 0.,
        };
        let made = fillet(&line, p(0., 0.), &arc, p(2., 3.), 1.).unwrap();
        let (c, r, a0, a1) = arc_of(made.arc);
        // Independent analytic result: (cx-4)^2+(1-3)^2=(2+1)^2.
        near(c.x, 4. - 5_f64.sqrt());
        near(c.y, 1.);
        near(r, 1.);
        let endpoints = ends(c, r, a0, a1);
        let line_contact = endpoints.iter().find(|q| q.y.abs() < 1e-8).unwrap();
        near(line_contact.x, c.x);
        let circle_contact = endpoints.iter().find(|q| q.y.abs() > 1e-8).unwrap();
        near(circle_contact.x, 4. - 2. * 5_f64.sqrt() / 3.);
        near(circle_contact.y, 5. / 3.);
        let v = sub(*circle_contact, p(4., 3.));
        let w = sub(*circle_contact, c);
        near(v.x * w.y - v.y * w.x, 0.);
        near(length(v), 2.); // tangent normals collinear
        let Shape::Arc { a0, a1, .. } = made.second else {
            panic!()
        };
        near(a0, PI);
        assert!(a1 > PI && a1 < TAU);
        assert!(matches!(made.first,Shape::Line{a,b} if a==p(-5.,0.) && (b.x-c.x).abs()<1e-8));
    }
    #[test]
    fn picks_select_both_solutions_and_circles_are_never_trimmed() {
        let a = Shape::Circle {
            c: p(-3., 0.),
            r: 4.,
        };
        let b = Shape::Circle {
            c: p(3., 0.),
            r: 4.,
        };
        for sign in [-1., 1.] {
            let made = fillet(&a, p(-1., sign * 3.5), &b, p(1., sign * 3.5), 1.).unwrap();
            assert_eq!(made.first, a);
            assert_eq!(made.second, b);
            let (c, r, t0, t1) = arc_of(made.arc);
            near(c.x, 0.);
            near(c.y, sign * 4.);
            for q in ends(c, r, t0, t1) {
                near(length(sub(q, c)), 1.);
                let on = if q.x < 0. { p(-3., 0.) } else { p(3., 0.) };
                near(length(sub(q, on)), 4.);
                let (v, w) = (sub(q, c), sub(q, on));
                near(v.x * w.y - v.y * w.x, 0.);
            }
        }
    }
    #[test]
    fn finite_arcs_remain_arcs_and_large_coordinates_keep_small_details() {
        for shift in [p(0., 0.), p(700000., 4500000.)] {
            let a = Shape::Arc {
                c: add(p(-3., 0.), shift),
                r: 4.,
                a0: 0.,
                a1: PI,
            };
            let b = Shape::Arc {
                c: add(p(3., 0.), shift),
                r: 4.,
                a0: 0.,
                a1: PI,
            };
            let made = fillet(&a, add(p(-1., 3.5), shift), &b, add(p(1., 3.5), shift), 1.).unwrap();
            let (c, _, _, _) = arc_of(made.arc);
            near(c.x, shift.x);
            near(c.y, shift.y + 4.);
            assert!(matches!(made.first,Shape::Arc{a0,a1,..} if a0>0. && a1==PI));
            assert!(matches!(made.second,Shape::Arc{a0,a1,..} if a0==0. && a1<PI));
        }
    }
    #[test]
    fn impossible_collapsed_nonfinite_and_unsupported_curves_are_refused() {
        let a = Shape::Circle {
            c: p(0., 0.),
            r: 2.,
        };
        let b = Shape::Circle {
            c: p(20., 0.),
            r: 2.,
        };
        assert!(fillet(&a, p(2., 0.), &b, p(18., 0.), 1.).is_err());
        assert!(fillet(&a, p(2., 0.), &a, p(-2., 0.), 1.).is_err());
        for r in [-1., f64::NAN, f64::INFINITY, 0.] {
            assert!(fillet(&a, p(2., 0.), &b, p(18., 0.), r).is_err());
        }
        let short = Shape::Arc {
            c: p(3., 0.),
            r: 4.,
            a0: PI,
            a1: PI + 0.01,
        };
        assert!(fillet(&a, p(2., 0.), &short, p(-1., 0.), 0.1).is_err());
    }
    #[test]
    fn a_real_pick_just_outside_an_arc_endpoint_keeps_the_nearest_end_without_a_cusp() {
        let line = Shape::Line {
            a: p(-5., 0.),
            b: p(10., 0.),
        };
        let arc = Shape::Arc {
            c: p(4., 3.),
            r: 2.,
            a0: PI,
            a1: 0.,
        };
        let exact = fillet(&line, p(-3., 0.), &arc, p(2., 3.), 1.).unwrap();
        for pick in [p(2.049, 3.007), p(1.99, 3.02), p(2.01, 2.99)] {
            let made = fillet(&line, p(-3., 0.), &arc, pick, 1.).unwrap();
            assert_eq!(made.first, exact.first);
            assert_eq!(made.second, exact.second);
            assert_eq!(made.arc, exact.arc);
            assert!(matches!(made.second,Shape::Arc{a0,..} if a0==PI));
        }
    }

    #[test]
    fn zero_radius_joins_at_a_true_intersection_and_pair_order_does_not_change_the_arc() {
        let line=Shape::Line {a:p(-5.,0.),b:p(10.,0.)};
        let circle=Shape::Circle {c:p(4.,1.),r:2.};
        let zero=fillet(&line,p(-5.,0.),&circle,p(2.,1.),0.).unwrap();
        assert!(zero.arc.is_none());assert_eq!(zero.second,circle);
        let Shape::Line {b,..}=zero.first else {panic!()};
        near(b.x,4.-3_f64.sqrt());near(b.y,0.);
        let f=fillet(&line,p(-5.,0.),&circle,p(2.,1.),1.).unwrap();
        let g=fillet(&circle,p(2.,1.),&line,p(-5.,0.),1.).unwrap();
        assert_eq!(f.first,g.second);assert_eq!(f.second,g.first);
        let (a,ar,a0,a1)=arc_of(f.arc);let (b,br,b0,b1)=arc_of(g.arc);
        near(a.x,b.x);near(a.y,b.y);near(ar,br);near(a0,b0);near(a1,b1);
        assert!(fillet(&line,p(0.,0.),&Shape::Circle {c:p(0.,0.),r:f64::INFINITY},p(1.,0.),1.).is_err());
    }
}
