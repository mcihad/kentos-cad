//! Örnekleme ve ağ üretimi (docs/adr/0220): grids of cells and lines over
//! the objects' box, regular and seeded random points in areas and on
//! paths, points along paths and objects cut into pieces of at most so many
//! vertices, one rule for both platforms: the desktop's tools call the
//! functions, the web's (`model/ops/sampling.ts`) the operations in `calls`
//! with whole lists. The independent reference is
//! scripts/fixtures/sampling_cases.py.
//!
//! - `grid` and `cover`: Ağ oluştur and its Nesnelere değenler (§2).
//! - `inside` and `regular`: Düzenli noktalar (§3).
//! - `random` and `scatter`: the seeded numbers and Rastgele noktalar (§4).
//! - `walk` and `along`: paths walked by length, Hat boyunca nokta (§5).
//! - `cut` and `subdivide`: an area cut along a line, Alt bölümlere ayır (§6).

pub mod along;
pub mod calls;
pub mod cover;
pub mod cut;
pub mod grid;
pub mod inside;
pub mod random;
pub mod regular;
pub mod scatter;
pub mod subdivide;
#[cfg(test)]
mod tests;
pub mod walk;

use crate::entity::{Shape, entity_bounds};
use crate::geometry::{Bounds, is_empty_bounds};
use crate::jsmath::{js_max, js_min};
use crate::ops::convert::Zs;
use crate::ops::geoprocess::{Class, class_of};
use inside::Inside;
use scatter::Target;
use walk::walks_of;

pub(crate) use calls::OPS;

/// The most objects a tool writes (§1).
pub const MOST: usize = 1_000_000;

/// The box of the objects a grid covers (§2): their own boxes, construction lines and rays left out; none when no object has one.
pub fn extent(shapes: &[Shape]) -> Option<Bounds> {
    let mut out: Option<Bounds> = None;
    for s in shapes {
        if matches!(s, Shape::Xline { .. } | Shape::Ray { .. }) {
            continue;
        }
        let b = entity_bounds(s);
        if is_empty_bounds(&b)
            || !(b.min_x.is_finite()
                && b.max_x.is_finite()
                && b.min_y.is_finite()
                && b.max_y.is_finite())
        {
            continue;
        }
        out = Some(match out {
            None => b,
            Some(a) => Bounds {
                min_x: js_min(a.min_x, b.min_x),
                min_y: js_min(a.min_y, b.min_y),
                max_x: js_max(a.max_x, b.max_x),
                max_y: js_max(a.max_y, b.max_y),
            },
        });
    }
    out
}

/// An object's areas as Düzenli noktalar takes them; none for an object without one.
pub fn inside_of(s: &Shape) -> Option<Inside> {
    match class_of(s) {
        Class::Areas(areas) if !areas.is_empty() => Inside::new(&areas),
        _ => None,
    }
}

/// What Rastgele noktalar puts an object's points in or on: its areas, its paths, or nothing.
pub fn target_of(s: &Shape, zs: Option<&Zs>) -> Target {
    match class_of(s) {
        Class::Areas(areas) if !areas.is_empty() => {
            Inside::new(&areas).map_or(Target::Nothing, Target::Areas)
        }
        Class::Paths(_) => Target::Paths(walks_of(s, zs)),
        _ => Target::Nothing,
    }
}

/// A typed count as Rastgele noktalar takes it: the whole part, none below zero; none when it is no finite number.
pub fn count_of(v: Option<f64>) -> Option<u64> {
    let v = v?;
    if !v.is_finite() {
        return None;
    }
    Some(if v > 0.0 { v.trunc() as u64 } else { 0 })
}
