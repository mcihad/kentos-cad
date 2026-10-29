//! Local binary64 frame for a measured partition (docs/adr/0657): parallel to a straight source edge, concentric with
//! an arc one (docs/adr/0734); its limits straight or arc wall edges. No screen or document dependencies.
use crate::Vec2;
use crate::geom::bulge::bulge_arc;
use crate::jsmath::{PI, atan2, cos, js_hypot, js_max, js_min, sin, tan};

#[derive(Clone, Copy)]
pub struct Edge {
    pub a: Vec2,
    pub b: Vec2,
    /// Its arc's bulge (the tangent of a quarter of its sweep, counter-clockwise positive); 0 straight.
    pub bulge: f64,
    pub left: f64,
    pub width: f64,
}
pub enum Extent {
    Source,
    Between(Edge, Edge),
    Length {
        anchor: f64,
        length: f64,
        direction: i8,
    },
}
pub struct Placement {
    pub path: [Vec2; 2],
    /// The partition's own arc on an arc source (its share of the source's circle); 0 straight.
    pub bulge: f64,
    pub face: [Vec2; 2],
    /// The source face's arc: the source edge's bulge on an arc source; 0 straight.
    pub face_bulge: f64,
    pub dimension: [Vec2; 2],
    pub clear: f64,
    pub axis: f64,
}
fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}
fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}
fn mul(a: Vec2, t: f64) -> Vec2 {
    Vec2::new(a.x * t, a.y * t)
}
fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}
fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}
fn center(edge: Edge) -> (Vec2, Vec2) {
    let d = sub(edge.b, edge.a);
    let l = js_hypot(d.x, d.y);
    let n = Vec2::new(-d.y / l, d.x / l);
    (add(edge.a, mul(n, edge.left - edge.width / 2.0)), d)
}
fn sign(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { 1.0 }
}
fn dir(angle: f64) -> Vec2 {
    Vec2::new(cos(angle), sin(angle))
}
/// `x` in [0, 2π).
fn turn(x: f64) -> f64 {
    let t = x.rem_euclid(2.0 * PI);
    if t >= 2.0 * PI { 0.0 } else { t }
}
/// A tolerance on an arc's angle: a point at an end computed through a circle's centre is not lost.
const END: f64 = 1e-12;

/// A limit's centre line: a line (`at` + `v`·s, s in [0, 1]) or an arc of a circle (`c`, `r`) from `a0` by `sweep`.
enum Limit {
    Line {
        at: Vec2,
        v: Vec2,
    },
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        sweep: f64,
    },
}
fn limit(edge: Edge) -> Result<Limit, &'static str> {
    if edge.bulge == 0.0 {
        let (at, v) = center(edge);
        return Ok(Limit::Line { at, v });
    }
    let arc =
        bulge_arc(edge.a, edge.b, edge.bulge).ok_or("Sınır duvarının kenarı sıfır uzunlukta.")?;
    // The left offset of a counter-clockwise arc moves towards its centre.
    let r = arc.r - sign(arc.sweep) * (edge.left - edge.width / 2.0);
    if !(r > 0.0) {
        return Err("Sınır duvarının ekseni yayın merkezini aşıyor; başka bir kenar seçin.");
    }
    Ok(Limit::Arc {
        c: arc.c,
        r,
        a0: arc.a0,
        sweep: arc.sweep,
    })
}
/// Whether `p`, on the circle about `c`, lies on the arc from `a0` by `sweep`.
fn on_arc(p: Vec2, c: Vec2, a0: f64, sweep: f64) -> bool {
    let along = turn(sign(sweep) * (atan2(p.y - c.y, p.x - c.x) - a0));
    along <= sweep.abs() + END || 2.0 * PI - along <= END
}

/// The partition's centre line: the source's parallel line, or its concentric circle.
enum Carrier {
    /// From `origin` along the unit `u`; t its length from the source's start.
    Line { origin: Vec2, u: Vec2 },
    /// About `c` of radius `r`, from the source start's angle `a0` towards the sign of `sweep`, t its length along.
    Circle {
        c: Vec2,
        r: f64,
        a0: f64,
        sweep: f64,
    },
}
impl Carrier {
    /// The source's span along the carrier: [0, its length].
    fn span(&self, length: f64) -> f64 {
        match self {
            Carrier::Line { .. } => length,
            Carrier::Circle { r, sweep, .. } => r * sweep.abs(),
        }
    }
    fn at(&self, t: f64) -> Vec2 {
        match self {
            Carrier::Line { origin, u } => add(*origin, mul(*u, t)),
            Carrier::Circle { c, r, a0, sweep } => add(*c, mul(dir(a0 + sign(*sweep) * t / r), *r)),
        }
    }
    /// The length along of a point on the carrier, the one nearest the source's middle on a circle.
    fn along(&self, p: Vec2) -> f64 {
        match self {
            Carrier::Line { origin, u } => dot(sub(p, *origin), *u),
            Carrier::Circle { c, r, a0, sweep } => {
                let half = sweep.abs() / 2.0;
                let angle = sign(*sweep) * (atan2(p.y - c.y, p.x - c.x) - a0) - half;
                let wrapped = angle - 2.0 * PI * ((angle + PI) / (2.0 * PI)).floor();
                r * (half + wrapped)
            }
        }
    }
    /// Where the carrier meets a limit's centre line: the lengths along of the meeting points that lie on the limit.
    fn meet(&self, limit: &Limit) -> Result<Vec<f64>, &'static str> {
        let mut found = Vec::new();
        match (self, limit) {
            (Carrier::Line { origin, u }, Limit::Line { at, v }) => {
                let q = sub(*at, *origin);
                let den = cross(*u, *v);
                if den == 0.0 {
                    return Err("Sınır duvarı bölmeye paralel; kesişen başka bir kenar seçin.");
                }
                let t = cross(q, *v) / den;
                let s = cross(q, *u) / den;
                if t.is_finite() && (0.0..=1.0).contains(&s) {
                    found.push(t);
                }
            }
            (Carrier::Line { origin, u }, Limit::Arc { c, r, a0, sweep }) => {
                let w = sub(*origin, *c);
                let b = dot(*u, w);
                let disc = b * b - (dot(w, w) - r * r);
                if disc >= 0.0 {
                    let root = disc.sqrt();
                    for t in [-b - root, -b + root] {
                        if on_arc(self.at(t), *c, *a0, *sweep) {
                            found.push(t);
                        }
                    }
                }
            }
            (Carrier::Circle { c, r, .. }, Limit::Line { at, v }) => {
                let w = sub(*at, *c);
                let (a, b) = (dot(*v, *v), dot(*v, w));
                let disc = b * b - a * (dot(w, w) - r * r);
                if disc >= 0.0 {
                    let root = disc.sqrt();
                    for s in [(-b - root) / a, (-b + root) / a] {
                        if (0.0..=1.0).contains(&s) {
                            found.push(self.along(add(*at, mul(*v, s))));
                        }
                    }
                }
            }
            (
                Carrier::Circle { c, r, .. },
                Limit::Arc {
                    c: c2,
                    r: r2,
                    a0,
                    sweep,
                },
            ) => {
                let d = sub(*c2, *c);
                let dist = js_hypot(d.x, d.y);
                if dist <= 1e-9 * js_max(1.0, js_max(*r, *r2)) {
                    return Err(
                        "Sınır duvarı bölmeyle eş merkezli; kesişen başka bir kenar seçin.",
                    );
                }
                let e = mul(d, 1.0 / dist);
                let x = (r * r - r2 * r2 + dist * dist) / (2.0 * dist);
                let h2 = r * r - x * x;
                if h2 >= 0.0 {
                    let h = h2.sqrt();
                    let n = Vec2::new(-e.y, e.x);
                    for side in [-1.0, 1.0] {
                        let p = add(add(*c, mul(e, x)), mul(n, side * h));
                        if on_arc(p, *c2, *a0, *sweep) {
                            found.push(self.along(p));
                        }
                    }
                }
            }
        }
        Ok(found)
    }
}

pub fn locate(
    source: Edge,
    right: bool,
    axis: bool,
    distance: f64,
    width: f64,
    extent: Extent,
) -> Result<Placement, &'static str> {
    let d = sub(source.b, source.a);
    let length = js_hypot(d.x, d.y);
    if !length.is_finite() || length <= 0.0 {
        return Err("Kaynak duvar kenarı sıfır uzunlukta.");
    }
    if !distance.is_finite() || distance <= 0.0 || !width.is_finite() || width <= 0.0 {
        return Err("Uzaklık ve duvar kalınlığı sıfırdan büyük sonlu uzunluklar olmalı.");
    }
    let u = mul(d, 1.0 / length);
    let n = Vec2::new(-u.y, u.x);
    let side = if right { -1.0 } else { 1.0 };
    let source_center = source.left - source.width / 2.0;
    let axis_distance = if axis {
        distance
    } else {
        distance + (source.width + width) / 2.0
    };
    let clear = axis_distance - (source.width + width) / 2.0;
    if clear <= 0.0 {
        return Err("Eksen aralığı duvarların kalınlığından sonra net boşluk bırakmıyor.");
    }
    let offset = source_center + side * axis_distance;
    let face_offset = if right {
        source.left - source.width
    } else {
        source.left
    };
    let arc = if source.bulge == 0.0 {
        None
    } else {
        Some(
            bulge_arc(source.a, source.b, source.bulge)
                .ok_or("Kaynak duvar kenarı sıfır uzunlukta.")?,
        )
    };
    let carrier = match arc {
        None => Carrier::Line {
            origin: add(source.a, mul(n, offset)),
            u,
        },
        Some(arc) => {
            // The left offset of a counter-clockwise arc moves towards its centre (docs/adr/0734 §1).
            let r = arc.r - sign(arc.sweep) * offset;
            if !(r - width / 2.0 > 0.0) {
                return Err(
                    "Bölme yayın merkezine ulaşıyor; daha küçük bir uzaklık ya da yayın dış yanını seçin.",
                );
            }
            Carrier::Circle {
                c: arc.c,
                r,
                a0: arc.a0,
                sweep: arc.sweep,
            }
        }
    };
    let span = carrier.span(length);
    let (t0, t1) = match extent {
        Extent::Source => (0.0, span),
        Extent::Length {
            anchor,
            length: l,
            direction,
        } => {
            if !anchor.is_finite() || !l.is_finite() || l <= 0.0 {
                return Err("Başlangıç ve boy sonlu, boy sıfırdan büyük olmalı.");
            }
            let range = match direction {
                -1 => (anchor - l, anchor),
                0 => (anchor - l / 2.0, anchor + l / 2.0),
                _ => (anchor, anchor + l),
            };
            if range.0 < 0.0 || range.1 > span {
                return Err(
                    "Verilen boy kaynak kenarın uçlarını aşıyor; boyu veya başlangıcı düzeltin.",
                );
            }
            range
        }
        Extent::Between(a, b) => {
            let middle = span / 2.0;
            let meet = |edge: Edge| -> Result<f64, &'static str> {
                let found = carrier.meet(&limit(edge)?)?;
                found
                    .into_iter()
                    .min_by(|x, y| {
                        (x - middle)
                            .abs()
                            .total_cmp(&(y - middle).abs())
                            .then(x.total_cmp(y))
                    })
                    .ok_or("Bölme seçilen sınır kenarına ulaşmıyor; başka bir kenar seçin.")
            };
            let a = meet(a)?;
            let b = meet(b)?;
            (js_min(a, b), js_max(a, b))
        }
    };
    if t1 <= t0 {
        return Err("Bölmenin iki sınırı aynı yerde; ayrı sınırlar seçin.");
    }
    let (path, bulge, face, face_bulge, dimension) = match (&carrier, arc) {
        (Carrier::Circle { c, r, a0, sweep }, Some(arc)) => {
            let face_r = arc.r - sign(arc.sweep) * face_offset;
            let face = [
                add(*c, mul(dir(*a0), face_r)),
                add(*c, mul(dir(a0 + arc.sweep), face_r)),
            ];
            let middle = dir(a0 + sign(*sweep) * (t0 + t1) / 2.0 / r);
            let from = add(*c, mul(middle, face_r));
            let to = add(*c, mul(middle, face_r - sign(arc.sweep) * side * clear));
            let bulge = tan(sign(*sweep) * (t1 - t0) / r / 4.0);
            (
                [carrier.at(t0), carrier.at(t1)],
                bulge,
                face,
                source.bulge,
                [from, to],
            )
        }
        _ => {
            let face = [
                add(source.a, mul(n, face_offset)),
                add(source.b, mul(n, face_offset)),
            ];
            let from = add(face[0], mul(u, (t0 + t1) / 2.0));
            let to = add(from, mul(n, side * clear));
            ([carrier.at(t0), carrier.at(t1)], 0.0, face, 0.0, [from, to])
        }
    };
    if path.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
        || path[0] == path[1]
        || !bulge.is_finite()
    {
        return Err("Bölme bu koordinatlarda çizilemiyor.");
    }
    Ok(Placement {
        path,
        bulge,
        face,
        face_bulge,
        dimension,
        clear,
        axis: axis_distance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::wall::line_at;

    /// The radius of an open two-point line's arc as the wall draws it at `d` from its path.
    fn radius(a: Vec2, b: Vec2, bulge: f64, d: f64) -> f64 {
        let line = line_at(&[a, b], Some(&[bulge, 0.0]), false, d).expect("a face");
        bulge_arc(line.pts[0], line.pts[1], line.bulges[0])
            .expect("an arc")
            .r
    }

    /// The clear distance is the gap between the faces the two walls draw (docs/adr/0734 §1), on both turns and sides.
    #[test]
    fn the_clear_gap_lies_between_the_drawn_faces() {
        let q = tan(PI / 8.0);
        for (a, b, bulge) in [
            (Vec2::new(5.0, 0.0), Vec2::new(0.0, 5.0), q),
            (Vec2::new(0.0, 5.0), Vec2::new(5.0, 0.0), -q),
        ] {
            for right in [false, true] {
                let source = Edge {
                    a,
                    b,
                    bulge,
                    left: 0.1,
                    width: 0.2,
                };
                let p =
                    locate(source, right, false, 1.5, 0.1, Extent::Source).expect("a partition");
                let face = radius(a, b, bulge, if right { -0.1 } else { 0.1 });
                let gap = [0.05, -0.05]
                    .iter()
                    .map(|d| (radius(p.path[0], p.path[1], p.bulge, *d) - face).abs())
                    .fold(f64::INFINITY, js_min);
                assert!((gap - 1.5).abs() < 1e-12, "{gap}");
            }
        }
    }
}
