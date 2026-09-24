//! Break, "Kır" (`apps/web/src/model/ops/break.ts`): removes the part between two
//! picked points; with p2 = p1 the object is split there. On closed shapes
//! the removed part runs counter-clockwise from p1 to p2.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::bulge::bulge_ring_area;
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::ops::curve_cuts::{Cut, break_construction, break_ellipse, construction_of, ellipse_of};
use crate::ops::path::{nearest_s, path_of, sub_path};
use crate::vec2::Vec2;

pub fn break_entity(e: &Entity, p1: Vec2, p2: Vec2) -> Cut {
    let s = &e.shape;
    if matches!(s, Shape::Spline { .. }) {
        return Cut::Error("Eğri kırılamaz; önce Patlat (X) ile çoklu çizgiye dönüştürün.".into());
    }
    if let Some(g) = ellipse_of(s) {
        return break_ellipse(&g, p1, p2);
    }
    if let Some(c) = construction_of(s) {
        return break_construction(&c, p1, p2);
    }
    if !matches!(
        s,
        Shape::Line { .. }
            | Shape::Polyline { .. }
            | Shape::Polygon { .. }
            | Shape::Arc { .. }
            | Shape::Circle { .. }
    ) {
        return Cut::Error(
            "Yalnızca çizgi, çoklu çizgi, kapalı alan, yay ve daire kırılabilir.".into(),
        );
    }
    let Some(path) = path_of(s) else {
        return Cut::Error("Bu nesne kırılamaz.".into());
    };
    let l = path.length;
    let eps = 1e-7 * js_max(1.0, l);
    let mut s1 = nearest_s(&path, p1);
    let mut s2 = nearest_s(&path, p2);
    let at_point = (s1 - s2).abs() <= eps || (path.closed && ((s1 - s2).abs() - l).abs() <= eps);

    if !path.closed {
        if at_point {
            if s1 <= eps || s1 >= l - eps {
                return Cut::Error(
                    "Nesne ucundan kırılamaz; iç kısmında bir nokta gösterin.".into(),
                );
            }
            return Cut::Pieces(vec![sub_path(&path, 0.0, s1, s), sub_path(&path, s1, l, s)]);
        }
        let lo = js_min(s1, s2);
        let hi = js_max(s1, s2);
        let mut pieces = Vec::new();
        if lo > eps {
            pieces.push(sub_path(&path, 0.0, lo, s));
        }
        if hi < l - eps {
            pieces.push(sub_path(&path, hi, l, s));
        }
        return if pieces.is_empty() {
            Cut::Error("İki nokta nesnenin tamamını kapsıyor; silmek için Sil kullanın.".into())
        } else {
            Cut::Pieces(pieces)
        };
    }

    if at_point {
        if matches!(s, Shape::Circle { .. }) {
            return Cut::Error("Daire tek noktadan kırılamaz; ikinci bir nokta gösterin.".into());
        }
        return Cut::Pieces(vec![sub_path(&path, s1, s1 + l, s)]);
    }
    // Circles run counter-clockwise; a clockwise ring swaps the picks so the removed part stays CCW.
    let ccw = match s {
        Shape::Circle { .. } | Shape::Arc { .. } => true,
        Shape::Polygon { pts, bulges, .. } => bulge_ring_area(pts, bulges.as_deref()) > 0.0,
        _ => false,
    };
    if !ccw {
        std::mem::swap(&mut s1, &mut s2);
    }
    // Remove s1 → s2 forwards; keep s2 → s1 (+L).
    let keep_end = if s1 > s2 { s1 } else { s1 + l };
    Cut::Pieces(vec![sub_path(&path, s2, keep_end, s)])
}

pub(crate) static OPS: &[Op] = &[op!("breakEntity", |e: Entity, p1: Vec2, p2: Vec2| {
    break_entity(&e, p1, p2)
})];
