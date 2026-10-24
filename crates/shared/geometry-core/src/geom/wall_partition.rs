//! Local binary64 frame for a straight partition. No screen or document dependencies.
use crate::Vec2;
use crate::jsmath::{js_hypot, js_max, js_min};

#[derive(Clone, Copy)]
pub struct Edge {
    pub a: Vec2,
    pub b: Vec2,
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
    pub face: [Vec2; 2],
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
fn center(edge: Edge) -> (Vec2, Vec2) {
    let d = sub(edge.b, edge.a);
    let l = js_hypot(d.x, d.y);
    let n = Vec2::new(-d.y / l, d.x / l);
    (add(edge.a, mul(n, edge.left - edge.width / 2.0)), d)
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
    let origin = add(source.a, mul(n, offset));
    let (t0, t1) = match extent {
        Extent::Source => (0.0, length),
        Extent::Length {
            anchor,
            length: l,
            direction,
        } => {
            if !anchor.is_finite() || !l.is_finite() || l <= 0.0 {
                return Err("Başlangıç ve boy sonlu, boy sıfırdan büyük olmalı.");
            }
            let span = match direction {
                -1 => (anchor - l, anchor),
                0 => (anchor - l / 2.0, anchor + l / 2.0),
                _ => (anchor, anchor + l),
            };
            if span.0 < 0.0 || span.1 > length {
                return Err(
                    "Verilen boy kaynak kenarın uçlarını aşıyor; boyu veya başlangıcı düzeltin.",
                );
            }
            span
        }
        Extent::Between(a, b) => {
            let intersect = |edge: Edge| -> Result<f64, &'static str> {
                let (at, v) = center(edge);
                let q = sub(at, origin);
                let den = cross(u, v);
                if den == 0.0 {
                    return Err("Sınır duvarı bölmeye paralel; kesişen başka bir kenar seçin.");
                }
                let t = cross(q, v) / den;
                let s = cross(q, u) / den;
                if !t.is_finite() || !(0.0..=1.0).contains(&s) {
                    return Err("Bölme seçilen sınır kenarına ulaşmıyor; başka bir kenar seçin.");
                }
                Ok(t)
            };
            let a = intersect(a)?;
            let b = intersect(b)?;
            (js_min(a, b), js_max(a, b))
        }
    };
    if t1 <= t0 {
        return Err("Bölmenin iki sınırı aynı yerde; ayrı sınırlar seçin.");
    }
    let face_offset = if right {
        source.left - source.width
    } else {
        source.left
    };
    let face = [
        add(source.a, mul(n, face_offset)),
        add(source.b, mul(n, face_offset)),
    ];
    let from = add(face[0], mul(u, (t0 + t1) / 2.0));
    let to = add(from, mul(n, side * clear));
    let path = [add(origin, mul(u, t0)), add(origin, mul(u, t1))];
    if path.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) || path[0] == path[1] {
        return Err("Bölme bu koordinatlarda çizilemiyor.");
    }
    Ok(Placement {
        path,
        face,
        dimension: [from, to],
        clear,
        axis: axis_distance,
    })
}
