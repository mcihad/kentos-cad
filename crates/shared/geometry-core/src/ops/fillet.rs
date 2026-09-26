//! Fillet and chamfer (`apps/web/src/model/ops/fillet.ts`): two lines joined by a
//! tangent arc or a straight cut, and the same at a corner of a path.

use crate::api::Op;
use crate::api::json::{FromJson, Json, Nullable, ToJson, field, read_field};
use crate::geom::arc::{ArcGeom, norm_angle, sweep};
use crate::geom::bulge::{BulgePath, bulge_at, clean_bulge_path};
use crate::geom::intersect::line_line;
use crate::jsmath::{PI, acos, atan2, js_hypot, js_max, js_min, js_sign, sin, tan};
use crate::op;
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub a: Vec2,
    pub b: Vec2,
}

crate::json_struct!(Seg { a, b });

/// Shortened lines and the arc (or cut) between them, or why there is none.
pub enum Corner<T> {
    Ok {
        line1: Seg,
        line2: Seg,
        join: Option<T>,
    },
    Error(String),
}

fn write_corner<T: ToJson>(c: &Corner<T>, key: &str, out: &mut String) {
    out.push('{');
    let mut first = true;
    match c {
        Corner::Ok { line1, line2, join } => {
            field(out, &mut first, "line1", line1);
            field(out, &mut first, "line2", line2);
            // A sharp corner says so with null, as the TypeScript does.
            field(out, &mut first, key, &Nullable(join));
        }
        Corner::Error(e) => field(out, &mut first, "error", e),
    }
    out.push('}');
}

pub struct Fillet(pub Corner<ArcGeom>);
pub struct Chamfer(pub Corner<Seg>);

impl ToJson for Fillet {
    fn write_json(&self, out: &mut String) {
        write_corner(&self.0, "arc", out);
    }
}

impl ToJson for Chamfer {
    fn write_json(&self, out: &mut String) {
        write_corner(&self.0, "cut", out);
    }
}

fn unit(v: Vec2) -> Vec2 {
    let l = js_hypot(v.x, v.y);
    Vec2::new(v.x / l, v.y / l)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// The kept part of a line: direction away from the corner X, the end kept, and how far it reaches.
struct Kept {
    u: Vec2,
    far: Vec2,
    reach: f64,
}

fn kept_side(l: Seg, pick: Vec2, x: Vec2) -> Kept {
    let d = unit(Vec2::new(l.b.x - l.a.x, l.b.y - l.a.y));
    let s = dot(Vec2::new(pick.x - x.x, pick.y - x.y), d);
    let u = if s >= 0.0 { d } else { Vec2::new(-d.x, -d.y) };
    // Kept end: the endpoint furthest along u from the corner.
    let far = if dot(Vec2::new(l.a.x - x.x, l.a.y - x.y), u)
        >= dot(Vec2::new(l.b.x - x.x, l.b.y - x.y), u)
    {
        l.a
    } else {
        l.b
    };
    Kept {
        u,
        far,
        reach: dot(Vec2::new(far.x - x.x, far.y - x.y), u),
    }
}

const NO_SIDE: &str = "Seçilen tarafta çizgi yok; köşeye doğru uzanan kısma tıklayın.";

/// Joins two lines with a tangent arc of radius r (r = 0 makes a sharp corner).
pub fn fillet_lines(l1: Seg, pick1: Vec2, l2: Seg, pick2: Vec2, r: f64) -> Fillet {
    let Some(hit) = line_line(l1.a, l1.b, l2.a, l2.b) else {
        return Fillet(Corner::Error(
            "Çizgiler paralel; köşe oluşturulamaz.".into(),
        ));
    };
    let x = hit.p;
    let k1 = kept_side(l1, pick1, x);
    let k2 = kept_side(l2, pick2, x);
    if k1.reach <= 1e-9 || k2.reach <= 1e-9 {
        return Fillet(Corner::Error(NO_SIDE.into()));
    }
    if r <= 0.0 {
        return Fillet(Corner::Ok {
            line1: Seg { a: k1.far, b: x },
            line2: Seg { a: k2.far, b: x },
            join: None,
        });
    }
    let cos_phi = js_max(-1.0, js_min(1.0, dot(k1.u, k2.u)));
    let phi = acos(cos_phi);
    if phi < 1e-6 || PI - phi < 1e-6 {
        return Fillet(Corner::Error(
            "Çizgiler aynı doğrultuda; yuvarlatılamaz.".into(),
        ));
    }
    let half = phi / 2.0;
    let dt = r / tan(half);
    if dt > k1.reach + 1e-9 || dt > k2.reach + 1e-9 {
        return Fillet(Corner::Error("Yarıçap bu çizgiler için çok büyük.".into()));
    }
    let t1 = Vec2::new(x.x + k1.u.x * dt, x.y + k1.u.y * dt);
    let t2 = Vec2::new(x.x + k2.u.x * dt, x.y + k2.u.y * dt);
    let bis = unit(Vec2::new(k1.u.x + k2.u.x, k1.u.y + k2.u.y));
    let dc = r / sin(half);
    let c = Vec2::new(x.x + bis.x * dc, x.y + bis.y * dc);
    let mut a0 = norm_angle(atan2(t1.y - c.y, t1.x - c.x));
    let mut a1 = norm_angle(atan2(t2.y - c.y, t2.x - c.x));
    // The fillet is always the minor arc between the tangent points.
    if sweep(a0, a1) > PI {
        (a0, a1) = (a1, a0);
    }
    Fillet(Corner::Ok {
        line1: Seg { a: k1.far, b: t1 },
        line2: Seg { a: k2.far, b: t2 },
        join: Some(ArcGeom { c, r, a0, a1 }),
    })
}

/// Chamfer ("Pah") at distances d1 and d2 from the intersection (zero both: a sharp corner).
pub fn chamfer_lines(l1: Seg, pick1: Vec2, l2: Seg, pick2: Vec2, d1: f64, d2: f64) -> Chamfer {
    let Some(hit) = line_line(l1.a, l1.b, l2.a, l2.b) else {
        return Chamfer(Corner::Error("Çizgiler paralel; pah kırılamaz.".into()));
    };
    let x = hit.p;
    let k1 = kept_side(l1, pick1, x);
    let k2 = kept_side(l2, pick2, x);
    if k1.reach <= 1e-9 || k2.reach <= 1e-9 {
        return Chamfer(Corner::Error(NO_SIDE.into()));
    }
    if d1 <= 0.0 && d2 <= 0.0 {
        return Chamfer(Corner::Ok {
            line1: Seg { a: k1.far, b: x },
            line2: Seg { a: k2.far, b: x },
            join: None,
        });
    }
    if d1 > k1.reach + 1e-9 || d2 > k2.reach + 1e-9 {
        return Chamfer(Corner::Error("Pah mesafesi çizgi boyundan büyük.".into()));
    }
    let c1 = Vec2::new(x.x + k1.u.x * d1, x.y + k1.u.y * d1);
    let c2 = Vec2::new(x.x + k2.u.x * d2, x.y + k2.u.y * d2);
    Chamfer(Corner::Ok {
        line1: Seg { a: k1.far, b: c1 },
        line2: Seg { a: k2.far, b: c2 },
        join: Some(Seg { a: c1, b: c2 }),
    })
}

/// Round (`radius`) or chamfer (`d1`, `d2`) a corner of a path.
pub enum CornerOp {
    Radius(f64),
    Chamfer(f64, f64),
}

impl FromJson for CornerOp {
    fn from_json(v: &Json) -> Result<CornerOp, String> {
        let Json::Obj(fields) = v else {
            return Err("nesne bekleniyordu".into());
        };
        // `'radius' in op` decides, as in TypeScript.
        if fields.iter().any(|(k, _)| k == "radius") {
            Ok(CornerOp::Radius(read_field(v, "radius")?))
        } else {
            Ok(CornerOp::Chamfer(
                read_field(v, "d1")?,
                read_field(v, "d2")?,
            ))
        }
    }
}

/// The path with its corner rounded or cut, or why not.
pub enum CornerResult {
    Path(BulgePath),
    Error(String),
}

impl ToJson for CornerResult {
    fn write_json(&self, out: &mut String) {
        match self {
            CornerResult::Path(p) => p.write_json(out),
            CornerResult::Error(e) => {
                out.push('{');
                let mut first = true;
                field(out, &mut first, "error", e);
                out.push('}');
            }
        }
    }
}

/// Rounds or chamfers the corner at vertex `index`; both neighbouring
/// segments must be straight. Err where the TypeScript reads past the path.
pub fn corner_of_path(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    index: usize,
    op: &CornerOp,
) -> Result<CornerResult, String> {
    let n = pts.len();
    let err = |m: &str| Ok(CornerResult::Error(m.into()));
    if !closed && (index == 0 || index + 1 >= n) {
        return err("Açık çoklu çizginin uç noktası köşe değildir.");
    }
    if index >= n {
        return Err(format!("{index}. köşe yok (çizgide {n} köşe var)."));
    }
    let i_prev = (index + n - 1) % n;
    let i_next = (index + 1) % n;
    if bulge_at(bulges, i_prev).abs() > 1e-12 || bulge_at(bulges, index).abs() > 1e-12 {
        return err(
            "Köşeyi oluşturan kenarlardan biri yay; yalnızca düz kenarlar arasındaki köşe işlenebilir.",
        );
    }
    let v = pts[index];
    let p = pts[i_prev];
    let nx = pts[i_next];
    let lp = js_hypot(p.x - v.x, p.y - v.y);
    let ln = js_hypot(nx.x - v.x, nx.y - v.y);
    if lp < 1e-12 || ln < 1e-12 {
        return err("Köşede sıfır uzunluklu kenar var.");
    }
    let u1 = Vec2::new((p.x - v.x) / lp, (p.y - v.y) / lp);
    let u2 = Vec2::new((nx.x - v.x) / ln, (nx.y - v.y) / ln);
    let phi = acos(js_max(-1.0, js_min(1.0, dot(u1, u2))));
    if phi < 1e-6 || PI - phi < 1e-6 {
        return err("Kenarlar aynı doğrultuda; burada köşe yok.");
    }
    let (d1, d2, bulge) = match *op {
        CornerOp::Radius(radius) => {
            if radius <= 0.0 {
                return err("Köşe zaten keskin; sıfırdan büyük bir yarıçap girin.");
            }
            let d = radius / tan(phi / 2.0);
            // Turning left at the corner means the fillet runs counter-clockwise.
            let turn = js_sign(-u1.x * u2.y + u1.y * u2.x);
            (d, d, turn * tan((PI - phi) / 4.0))
        }
        CornerOp::Chamfer(d1, d2) => {
            if d1 <= 0.0 && d2 <= 0.0 {
                return err("Pah mesafeleri sıfırdan büyük olmalı.");
            }
            (d1, d2, 0.0)
        }
    };
    if d1 > lp + 1e-9 || d2 > ln + 1e-9 {
        return err(match op {
            CornerOp::Radius(_) => "Yarıçap bu kenarlar için çok büyük.",
            CornerOp::Chamfer(..) => "Pah mesafesi kenar boyundan büyük.",
        });
    }
    let t1 = Vec2::new(v.x + u1.x * d1, v.y + u1.y * d1);
    let t2 = Vec2::new(v.x + u2.x * d2, v.y + u2.y * d2);
    let mut out_p: Vec<Vec2> = pts[..index].to_vec();
    out_p.push(t1);
    out_p.push(t2);
    out_p.extend_from_slice(&pts[index + 1..]);
    let src: Vec<f64> = (0..n).map(|i| bulge_at(bulges, i)).collect();
    let mut out_b: Vec<f64> = src[..index].to_vec();
    out_b.push(bulge);
    out_b.extend_from_slice(&src[index..]);
    Ok(CornerResult::Path(clean_bulge_path(
        &out_p,
        Some(&out_b),
        closed,
        1e-9,
    )))
}

/// Every corner of a closed ring rounded (`radius`) or cut (`d1`, `d2`):
/// the rectangle tool's corner style (`RectangleTool`, AutoCAD RECTANG's
/// Fillet and Chamfer; docs/adr/0032). Corners are done last first, so the
/// ones still to do keep their indices. The first corner that cannot be done
/// answers with its reason, and nothing of the others is kept.
pub fn corners_of_ring(ring: &[Vec2], op: &CornerOp) -> Result<CornerResult, String> {
    let mut path = BulgePath {
        pts: ring.to_vec(),
        bulges: None,
    };
    for index in (0..ring.len()).rev() {
        match corner_of_path(&path.pts, path.bulges.as_deref(), true, index, op)? {
            CornerResult::Path(next) => path = next,
            error => return Ok(error),
        }
    }
    Ok(CornerResult::Path(path))
}

pub(crate) static OPS: &[Op] = &[
    op!("filletLines", |l1: Seg,
                        pick1: Vec2,
                        l2: Seg,
                        pick2: Vec2,
                        r: f64| fillet_lines(
        l1, pick1, l2, pick2, r
    )),
    op!("chamferLines", |l1: Seg,
                         pick1: Vec2,
                         l2: Seg,
                         pick2: Vec2,
                         d1: f64,
                         d2: f64| {
        chamfer_lines(l1, pick1, l2, pick2, d1, d2)
    }),
    op!("cornerOfPath", |pts: Vec<Vec2>,
                         bulges: Option<Vec<f64>>,
                         closed: bool,
                         index: usize,
                         op: CornerOp| {
        corner_of_path(&pts, bulges.as_deref(), closed, index, &op)
    }),
    op!("cornersOfRing", |ring: Vec<Vec2>, op: CornerOp| {
        corners_of_ring(&ring, &op)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Vec<Vec2> {
        [(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]
            .map(|(x, y)| Vec2::new(x, y))
            .to_vec()
    }

    #[test]
    fn a_rectangle_gets_every_corner_rounded_or_cut() {
        let Ok(CornerResult::Path(round)) = corners_of_ring(&rect(), &CornerOp::Radius(2.0)) else {
            panic!("rounded");
        };
        // Four tangent points a side, one quarter arc (tan 22.5°) per corner.
        assert_eq!(round.pts.len(), 8);
        let bulges = round.bulges.expect("arcs");
        let arcs: Vec<f64> = bulges.iter().copied().filter(|b| *b != 0.0).collect();
        assert_eq!(arcs.len(), 4);
        assert!(arcs.iter().all(|b| (b - tan(PI / 8.0)).abs() < 1e-12));
        let Ok(CornerResult::Path(cut)) = corners_of_ring(&rect(), &CornerOp::Chamfer(1.5, 1.5))
        else {
            panic!("cut");
        };
        assert_eq!(cut.pts.len(), 8);
        assert!(cut.bulges.is_none_or(|b| b.iter().all(|x| *x == 0.0)));
    }

    #[test]
    fn a_corner_that_cannot_be_done_answers_for_all() {
        // Two radii of 6 do not fit on a side of 10.
        match corners_of_ring(&rect(), &CornerOp::Radius(6.0)) {
            Ok(CornerResult::Error(e)) => assert_eq!(e, "Yarıçap bu kenarlar için çok büyük."),
            _ => panic!("refused"),
        }
        match corners_of_ring(&rect(), &CornerOp::Radius(0.0)) {
            Ok(CornerResult::Error(e)) => {
                assert_eq!(e, "Köşe zaten keskin; sıfırdan büyük bir yarıçap girin.")
            }
            _ => panic!("refused"),
        }
    }
}
