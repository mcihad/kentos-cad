//! Point input (`apps/web/src/tools/coordinateInput.ts`, `apps/web/src/tools/tracking.ts`,
//! `apps/web/src/tools/pointCalc.ts`): typed relative, polar and distance input, the
//! ortho and polar cursor, and the point calculator's own arithmetic around
//! the surveying constructions (`geom::survey`).

use crate::api::Op;
use crate::api::json::Nullable;
use crate::geom::survey::{along_line, polar_point};
use crate::geometry::dist;
use crate::jsmath::{PI, atan2, cos, js_hypot, js_round, sin};
use crate::op;
use crate::vec2::Vec2;

/// `@dY,dX`: the last point moved by the typed differences.
pub fn relative_point(last: Vec2, dx: f64, dy: f64) -> Vec2 {
    Vec2::new(last.x + dx, last.y + dy)
}

/// `@distance<angle`: from the last point, the angle in degrees counter-clockwise from east.
pub fn polar_offset(last: Vec2, distance: f64, angle: f64) -> Vec2 {
    let a = (angle * PI) / 180.0;
    Vec2::new(last.x + cos(a) * distance, last.y + sin(a) * distance)
}

/// A bare number: that far from the last point towards the cursor; None when they coincide.
pub fn toward_point(last: Vec2, cursor: Vec2, distance: f64) -> Option<Vec2> {
    let dx = cursor.x - last.x;
    let dy = cursor.y - last.y;
    let l = js_hypot(dx, dy);
    if l < 1e-9 {
        return None;
    }
    Some(Vec2::new(
        last.x + (dx / l) * distance,
        last.y + (dy / l) * distance,
    ))
}

/// A polar-tracking ray the cursor is locked to (angle in degrees, CCW from east).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tracking {
    pub origin: Vec2,
    pub angle: f64,
}

crate::json_struct!(Tracking { origin, angle });

/// The constrained cursor and the polar ray it is locked to (written as `null` when none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constrained {
    pub point: Vec2,
    pub tracking: Option<Tracking>,
}

impl crate::api::json::ToJson for Constrained {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        crate::api::json::field(out, &mut first, "point", &self.point);
        crate::api::json::field(out, &mut first, "tracking", &Nullable(&self.tracking));
        out.push('}');
    }
}

/// The effective cursor for point input from `from`: an exact point (object
/// snap or tracking) stays, ortho keeps the larger of the two moves, polar
/// tracking (`polar_step` degrees; None when off) locks onto the nearest
/// ray within `tol`.
pub fn constrain_cursor(
    from: Option<Vec2>,
    world: Vec2,
    exact: bool,
    ortho: bool,
    polar_step: Option<f64>,
    tol: f64,
) -> Constrained {
    let free = Constrained {
        point: world,
        tracking: None,
    };
    let Some(from) = from else { return free };
    if exact {
        return free;
    }
    let dx = world.x - from.x;
    let dy = world.y - from.y;
    if ortho {
        let point = if dx.abs() > dy.abs() {
            Vec2::new(world.x, from.y)
        } else {
            Vec2::new(from.x, world.y)
        };
        return Constrained {
            point,
            tracking: None,
        };
    }
    let Some(inc) = polar_step else { return free };
    let ang = (atan2(dy, dx) * 180.0) / PI;
    let snapped = js_round(ang / inc) * inc;
    let rad = (snapped * PI) / 180.0;
    let ux = cos(rad);
    let uy = sin(rad);
    let along = dx * ux + dy * uy;
    let off = (-dx * uy + dy * ux).abs();
    if along <= 0.0 || off > tol {
        return free;
    }
    Constrained {
        point: Vec2::new(from.x + ux * along, from.y + uy * along),
        tracking: Some(Tracking {
            origin: from,
            angle: ((snapped % 360.0) + 360.0) % 360.0,
        }),
    }
}

/// İki nokta ortası.
pub fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// Hat üzerinde nokta by ratio: `num/den` of the way from A to B.
pub fn along_ratio(a: Vec2, b: Vec2, num: f64, den: f64) -> Option<Vec2> {
    along_line(a, b, (dist(a, b) * num) / den)
}

/// Açı-mesafe with the angle in the project's unit (`deg`, else grads), clockwise from S→R.
pub fn calc_polar(s: Vec2, r: Vec2, angle: f64, unit: &str, distance: f64) -> Option<Vec2> {
    let per_unit = if unit == "deg" {
        PI / 180.0
    } else {
        PI / 200.0
    };
    polar_point(s, r, angle * per_unit, distance)
}

/// The candidate nearest to p (the first of equally near ones); None for none.
pub fn nearest_of(points: &[Vec2], p: Vec2) -> Option<Vec2> {
    let mut it = points.iter().copied();
    let first = it.next()?;
    // `reduce((a, b) => dist(a, p) <= dist(b, p) ? a : b)`: a NaN distance hands over to b.
    Some(it.fold(first, |a, b| if dist(a, p) <= dist(b, p) { a } else { b }))
}

pub(crate) static OPS: &[Op] = &[
    op!("relativePoint", |last: Vec2, dx: f64, dy: f64| {
        relative_point(last, dx, dy)
    }),
    op!("polarOffset", |last: Vec2, distance: f64, angle: f64| {
        polar_offset(last, distance, angle)
    }),
    op!("towardPoint", |last: Vec2, cursor: Vec2, distance: f64| {
        toward_point(last, cursor, distance)
    }),
    op!("constrainCursor", |from: Option<Vec2>,
                            world: Vec2,
                            exact: bool,
                            ortho: bool,
                            polar_step: Option<f64>,
                            tol: f64| {
        constrain_cursor(from, world, exact, ortho, polar_step, tol)
    }),
    op!("midpoint", |a: Vec2, b: Vec2| midpoint(a, b)),
    op!("alongRatio", |a: Vec2, b: Vec2, num: f64, den: f64| {
        along_ratio(a, b, num, den)
    }),
    op!("calcPolar", |s: Vec2,
                      r: Vec2,
                      angle: f64,
                      unit: String,
                      distance: f64| calc_polar(
        s, r, angle, &unit, distance
    )),
    op!("nearestOf", |points: Vec<Vec2>, p: Vec2| nearest_of(
        &points, p
    )),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::to_string;

    #[test]
    fn a_free_cursor_says_so_with_null_tracking() {
        // TypeScript returned `tracking: null`, not a missing field.
        let free = constrain_cursor(None, Vec2::new(5.0, 7.0), false, true, Some(15.0), 1.0);
        assert_eq!(
            to_string(&free),
            "{\"point\":{\"x\":5,\"y\":7},\"tracking\":null}"
        );
        let locked = constrain_cursor(
            Some(Vec2::new(0.0, 0.0)),
            Vec2::new(10.0, 0.2),
            false,
            false,
            Some(15.0),
            1.0,
        );
        assert_eq!(
            locked.tracking,
            Some(Tracking {
                origin: Vec2::new(0.0, 0.0),
                angle: 0.0
            })
        );
        assert_eq!(locked.point, Vec2::new(10.0, 0.0));
    }
}
