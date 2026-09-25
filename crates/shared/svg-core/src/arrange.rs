//! Arranging shapes in the SVG editor (`apps/web/src/style/svg/arrange.ts`):
//! align and distribute (Inkscape's Align and Distribute), the numeric
//! transforms (move, scale, rotate, skew, matrix; together or each
//! separately) and arrays for pattern and symbol design (rectangular,
//! polar, mirror copy, as a CAD array). A group moves as one. Everything
//! here gives matrices or new shapes; the editor applies them as one undo
//! step.

use kentos_geometry_core::api::json::{FromJson, Json, read_field};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{
    PI, cos, js_cmp, js_max, js_min, js_round, sin, stable_sort, tan,
};

use crate::model::{rotate_about as rotation, shape_box, transform_shape};
use crate::ops::Ids;
use crate::path::{Matrix, apply, multiply};
use crate::shape::{Obj, Pt};

/// Things that move together: a group, or a shape on its own; in the order they were chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub ids: Vec<String>,
    pub bx: Bounds,
}

kentos_geometry_core::json_struct!(Unit { ids, bx => "box" });

fn union_box(a: &Bounds, b: &Bounds) -> Bounds {
    Bounds {
        min_x: js_min(a.min_x, b.min_x),
        min_y: js_min(a.min_y, b.min_y),
        max_x: js_max(a.max_x, b.max_x),
        max_y: js_max(a.max_y, b.max_y),
    }
}

fn area(b: &Bounds) -> f64 {
    (b.max_x - b.min_x) * (b.max_y - b.min_y)
}

pub fn translate(dx: f64, dy: f64) -> Matrix {
    [1.0, 0.0, 0.0, 1.0, dx, dy]
}

/// The chosen shapes as units, in the order of `order` (the selection's order).
pub fn units_of(shapes: &[Obj], order: &[String]) -> Result<Vec<Unit>, String> {
    // A Map by id: the last shape with an id wins.
    let by_id = |id: &str| shapes.iter().rev().find(|s| s.text("id") == Some(id));
    let mut units: Vec<(String, Unit)> = Vec::new();
    for id in order {
        let Some(s) = by_id(id) else { continue };
        let key = match s.get("group") {
            g if crate::shape::truthy(g) => format!("g:{}", text_of(g)),
            _ => format!("s:{}", s.text("id").unwrap_or("")),
        };
        let b = shape_box(s)?;
        match units.iter_mut().find(|(k, _)| *k == key) {
            Some((_, u)) => {
                u.ids.push(id.clone());
                u.bx = union_box(&u.bx, &b);
            }
            None => units.push((
                key,
                Unit {
                    ids: vec![id.clone()],
                    bx: b,
                },
            )),
        }
    }
    Ok(units.into_iter().map(|(_, u)| u).collect())
}

/// A group's name as `${s.group}` writes it.
fn text_of(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        Json::Num(x) => kentos_style_core::js::number::to_string(*x),
        Json::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// The box the units align against. One unit alone aligns to the canvas.
pub fn align_reference(units: &[Unit], to: &str, width: f64, height: f64) -> Bounds {
    let canvas = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: width,
        max_y: height,
    };
    if to == "canvas" || units.len() < 2 {
        return canvas;
    }
    match to {
        "first" => units[0].bx,
        "last" => units[units.len() - 1].bx,
        "biggest" => {
            units[1..]
                .iter()
                .fold(
                    &units[0],
                    |a, b| if area(&b.bx) > area(&a.bx) { b } else { a },
                )
                .bx
        }
        "smallest" => {
            units[1..]
                .iter()
                .fold(
                    &units[0],
                    |a, b| if area(&b.bx) < area(&a.bx) { b } else { a },
                )
                .bx
        }
        _ => units[1..]
            .iter()
            .fold(units[0].bx, |a, u| union_box(&a, &u.bx)),
    }
}

fn align_shift(b: &Bounds, r: &Bounds, side: &str) -> Pt {
    match side {
        "left" => [r.min_x - b.min_x, 0.0],
        "right" => [r.max_x - b.max_x, 0.0],
        "hcenter" => [(r.min_x + r.max_x - b.min_x - b.max_x) / 2.0, 0.0],
        "top" => [0.0, r.min_y - b.min_y],
        "bottom" => [0.0, r.max_y - b.max_y],
        "vcenter" => [0.0, (r.min_y + r.max_y - b.min_y - b.max_y) / 2.0],
        // No such side: TypeScript's switch fell through to undefined.
        _ => [f64::NAN, f64::NAN],
    }
}

/// One translation per unit putting its edge or centre on the reference's.
/// `as_one` moves the whole selection as one block (useful against the canvas).
pub fn align_moves(
    units: &[Unit],
    side: &str,
    to: &str,
    width: f64,
    height: f64,
    as_one: bool,
) -> Vec<Matrix> {
    let r = align_reference(units, to, width, height);
    if as_one && !units.is_empty() {
        let all = units[1..]
            .iter()
            .fold(units[0].bx, |a, u| union_box(&a, &u.bx));
        let [dx, dy] = align_shift(&all, &r, side);
        return units.iter().map(|_| translate(dx, dy)).collect();
    }
    units
        .iter()
        .map(|u| {
            let [dx, dy] = align_shift(&u.bx, &r, side);
            translate(dx, dy)
        })
        .collect()
}

/// Even spacing between the outermost units (they stay): of edges or
/// centres, or equal gaps between neighbouring boxes.
pub fn distribute_moves(units: &[Unit], how: &str) -> Vec<Matrix> {
    let mut out: Vec<Matrix> = units.iter().map(|_| translate(0.0, 0.0)).collect();
    if units.len() < 3 {
        return out;
    }
    let horizontal = matches!(how, "left" | "hcenter" | "right" | "hgap");
    let lo = |b: &Bounds| if horizontal { b.min_x } else { b.min_y };
    let hi = |b: &Bounds| if horizontal { b.max_x } else { b.max_y };
    let key = |b: &Bounds| match how {
        "left" | "top" | "hgap" | "vgap" => lo(b),
        "right" | "bottom" => hi(b),
        _ => (lo(b) + hi(b)) / 2.0,
    };
    let mut order: Vec<usize> = (0..units.len()).collect();
    stable_sort(&mut order, &mut |&a, &b| {
        js_cmp(key(&units[a].bx) - key(&units[b].bx), 0.0)
    });
    let mv = |d: f64| {
        if horizontal {
            translate(d, 0.0)
        } else {
            translate(0.0, d)
        }
    };
    if how == "hgap" || how == "vgap" {
        let first = units[order[0]].bx;
        let last = units[order[order.len() - 1]].bx;
        let sizes = order
            .iter()
            .fold(0.0, |s, &i| s + hi(&units[i].bx) - lo(&units[i].bx));
        let gap = (hi(&last) - lo(&first) - sizes) / (order.len() - 1) as f64;
        let mut at = lo(&first);
        for &i in &order {
            out[i] = mv(at - lo(&units[i].bx));
            at += hi(&units[i].bx) - lo(&units[i].bx) + gap;
        }
        return out;
    }
    let a = key(&units[order[0]].bx);
    let step = (key(&units[order[order.len() - 1]].bx) - a) / (order.len() - 1) as f64;
    for (k, &i) in order.iter().enumerate() {
        out[i] = mv(a + step * k as f64 - key(&units[i].bx));
    }
    out
}

// ── Transforms ─────────────────────────────────────────────────────────

/// A box point: corners, edge middles, centre (`tl`, `t`, `tr`, `l`, `c`, `r`, `bl`, `b`, `br`).
pub fn anchor_point(b: &Bounds, a: &str) -> Pt {
    let x = if a.ends_with('l') {
        b.min_x
    } else if a.ends_with('r') {
        b.max_x
    } else {
        (b.min_x + b.max_x) / 2.0
    };
    let y = if a.starts_with('t') {
        b.min_y
    } else if a.starts_with('b') {
        b.max_y
    } else {
        (b.min_y + b.max_y) / 2.0
    };
    [x, y]
}

pub fn scale_about(sx: f64, sy: f64, p: Pt) -> Matrix {
    [sx, 0.0, 0.0, sy, p[0] - sx * p[0], p[1] - sy * p[1]]
}

/// Skew by angles (degrees): x leans with y by `ax`, y with x by `ay`, about p.
pub fn skew_about(ax: f64, ay: f64, p: Pt) -> Matrix {
    let tx = tan((ax * PI) / 180.0);
    let ty = tan((ay * PI) / 180.0);
    [1.0, ty, tx, 1.0, -tx * p[1], -ty * p[0]]
}

/// Rotation by `deg`, counter-clockwise on screen when `ccw` (the drawing's y runs down).
pub fn rotate_about(deg: f64, p: Pt, ccw: bool) -> Matrix {
    rotation(if ccw { -deg } else { deg }, p[0], p[1])
}

/// A numeric transform (the Transform panel).
pub enum TransformSpec {
    Move { x: f64, y: f64, relative: bool },
    Scale { sx: f64, sy: f64, anchor: String },
    Rotate { deg: f64, ccw: bool, about: About },
    Skew { ax: f64, ay: f64, anchor: String },
    Matrix { m: Matrix },
}

pub enum About {
    Anchor(String),
    Point(Pt),
}

impl FromJson for TransformSpec {
    fn from_json(v: &Json) -> Result<TransformSpec, String> {
        let kind: String = read_field(v, "kind")?;
        let flag = |k: &str| crate::shape::truthy(v.get(k));
        let text = |k: &str| match v.get(k) {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        };
        Ok(match kind.as_str() {
            "move" => TransformSpec::Move {
                x: read_field(v, "x")?,
                y: read_field(v, "y")?,
                relative: flag("relative"),
            },
            "scale" => TransformSpec::Scale {
                sx: read_field(v, "sx")?,
                sy: read_field(v, "sy")?,
                anchor: text("anchor"),
            },
            "rotate" => TransformSpec::Rotate {
                deg: read_field(v, "deg")?,
                ccw: flag("ccw"),
                about: match v.get("about") {
                    Json::Str(s) => About::Anchor(s.clone()),
                    p => About::Point(Pt::from_json(p)?),
                },
            },
            "skew" => TransformSpec::Skew {
                ax: read_field(v, "ax")?,
                ay: read_field(v, "ay")?,
                anchor: text("anchor"),
            },
            "matrix" => TransformSpec::Matrix {
                m: read_field(v, "m")?,
            },
            other => return Err(format!("bilinmeyen dönüşüm “{other}”")),
        })
    }
}

/// The matrix of every unit. Together, the selection's box is the frame;
/// separately, each unit's own box (a relative move then steps each unit
/// one move further than the one before, spreading them out).
pub fn transform_moves(units: &[Unit], spec: &TransformSpec, separately: bool) -> Vec<Matrix> {
    if units.is_empty() {
        return Vec::new();
    }
    let all = units[1..]
        .iter()
        .fold(units[0].bx, |a, u| union_box(&a, &u.bx));
    let one = |b: &Bounds, k: usize| -> Matrix {
        match spec {
            TransformSpec::Move { x, y, relative } => {
                if *relative {
                    let f = if separately { (k + 1) as f64 } else { 1.0 };
                    translate(x * f, y * f)
                } else {
                    translate(x - b.min_x, y - b.min_y)
                }
            }
            TransformSpec::Scale { sx, sy, anchor } => {
                scale_about(sx / 100.0, sy / 100.0, anchor_point(b, anchor))
            }
            TransformSpec::Rotate { deg, ccw, about } => rotate_about(
                *deg,
                match about {
                    About::Anchor(a) => anchor_point(b, a),
                    About::Point(p) => *p,
                },
                *ccw,
            ),
            TransformSpec::Skew { ax, ay, anchor } => skew_about(*ax, *ay, anchor_point(b, anchor)),
            TransformSpec::Matrix { m } => {
                // Separately: about each box's top-left corner, as if it were the origin.
                if separately {
                    multiply(
                        &translate(b.min_x, b.min_y),
                        &multiply(m, &translate(-b.min_x, -b.min_y)),
                    )
                } else {
                    *m
                }
            }
        }
    };
    units
        .iter()
        .enumerate()
        .map(|(k, u)| one(if separately { &u.bx } else { &all }, k))
        .collect()
}

/// A matrix is usable: finite and not flattening everything to a line.
pub fn invertible(m: &Matrix) -> bool {
    m.iter().all(|x| x.is_finite()) && (m[0] * m[3] - m[1] * m[2]).abs() > 1e-12
}

// ── Arrays ─────────────────────────────────────────────────────────────

/// Offsets of every copy of a rectangular array (the original, row 0 column 0, is left out).
pub fn rect_array(b: &Bounds, rows: f64, cols: f64, dx: f64, dy: f64, gap: bool) -> Vec<Matrix> {
    let sx = if gap { b.max_x - b.min_x + dx } else { dx };
    let sy = if gap { b.max_y - b.min_y + dy } else { dy };
    let mut out = Vec::new();
    let nr = js_max(1.0, js_round(rows));
    let nc = js_max(1.0, js_round(cols));
    let mut r = 0.0;
    while r < nr {
        let mut c = 0.0;
        while c < nc {
            if r != 0.0 || c != 0.0 {
                out.push(translate(c * sx, r * sy));
            }
            c += 1.0;
        }
        r += 1.0;
    }
    out
}

pub fn polar_array(
    b: &Bounds,
    count: f64,
    angle: f64,
    centre: Pt,
    rotate: bool,
    ccw: bool,
) -> Vec<Matrix> {
    let n = js_max(1.0, js_round(count));
    let full = angle.abs() >= 360.0 - 1e-9;
    let step = if n > 1.0 {
        angle / (if full { n } else { n - 1.0 })
    } else {
        0.0
    };
    let mid: Pt = [(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0];
    let mut out = Vec::new();
    let mut k = 1.0;
    while k < n {
        let r = rotate_about(step * k, centre, ccw);
        if rotate {
            out.push(r);
        } else {
            let [x, y] = apply(&r, mid[0], mid[1]);
            out.push(translate(x - mid[0], y - mid[1]));
        }
        k += 1.0;
    }
    out
}

/// Reflection across a line through p: vertical, horizontal, or at `deg` from the x axis.
pub fn mirror_matrix(axis: &str, p: Pt, deg: f64) -> Matrix {
    let a = match axis {
        "v" => PI / 2.0,
        "h" => 0.0,
        _ => (deg * PI) / 180.0,
    };
    let c = cos(2.0 * a);
    let s = sin(2.0 * a);
    // x' = p + R(x − p), R the reflection across the direction a.
    [
        c,
        s,
        s,
        -c,
        p[0] - c * p[0] - s * p[1],
        p[1] - s * p[0] + c * p[1],
    ]
}

/// Copies of the shapes under each matrix: new ids, and each copy's groups its own.
pub fn copies_of(shapes: &[Obj], matrices: &[Matrix], ids: &mut Ids) -> Result<Vec<Obj>, String> {
    let mut out = Vec::new();
    for m in matrices {
        let mut groups: Vec<(String, Json)> = Vec::new();
        for s in shapes {
            let g = match s.get("group") {
                g if crate::shape::truthy(g) => {
                    let key = text_of(g);
                    match groups.iter().find(|(k, _)| *k == key) {
                        Some((_, v)) => Some(v.clone()),
                        None => {
                            let v = ids.fresh();
                            groups.push((key, v.clone()));
                            Some(v)
                        }
                    }
                }
                _ => None,
            };
            let Some(mut copy) = transform_shape(s, m)? else {
                return Err("Bilinmeyen şekil türü.".into());
            };
            copy.set("id", ids.fresh());
            copy.put("group", g);
            out.push(copy);
        }
    }
    Ok(out)
}
