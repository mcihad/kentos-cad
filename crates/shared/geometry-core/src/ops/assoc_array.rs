//! The places of an associative array's items (docs/adr/0300 §2): the first
//! is the source insert's own placement, the others that placement moved by
//! the array command's maps (`tools::editing`): a grid, along a
//! displacement, round a centre (an item that does not turn moves by its
//! base point), or along a path from its start. The maps are rigid, so an
//! item keeps the source's scale and mirror and turns with its map.

use crate::entity::Shape;
use crate::geom::affine::{Affine, apply};
use crate::jsmath::atan2;
use crate::tools::editing::{
    grid_array_transforms, line_array_transforms, path_array_transforms, polar_array_transforms,
};
use crate::vec2::Vec2;

/// Where an item's block goes: an insert's placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub p: Vec2,
    pub scale: f64,
    pub rotation: f64,
    pub mirror: bool,
}

/// An associative array's layout, as `cad_array::ArrayLayout` gives it; a path array's path is the path's shape.
pub enum Layout<'a> {
    Grid {
        rows: u32,
        cols: u32,
        dx: f64,
        dy: f64,
    },
    Line {
        count: u32,
        dx: f64,
        dy: f64,
        fit: bool,
    },
    /// `fill` in degrees, as the array command's.
    Polar {
        center: Vec2,
        count: u32,
        fill: f64,
        rotate: bool,
    },
    Path {
        path: &'a Shape,
        count: u32,
        spacing: Option<f64>,
        align: bool,
    },
}

/// The placement moved by a rigid map.
fn moved(source: &Placement, m: &Affine) -> Placement {
    Placement {
        p: apply(m, source.p),
        scale: source.scale,
        rotation: source.rotation + atan2(m[1], m[0]),
        mirror: source.mirror,
    }
}

/// The items' placements, the source's first; none when the layout gives no maps (a count out of its range, a
/// path that is not a line, an arc, a circle or a polyline, or places past an open path's end).
pub fn item_placements(source: Placement, layout: &Layout<'_>) -> Option<Vec<Placement>> {
    let maps = match *layout {
        Layout::Grid { rows, cols, dx, dy } => {
            let places = u64::from(rows) * u64::from(cols);
            if rows == 0 || cols == 0 || !(2..=10_000).contains(&places) {
                return None;
            }
            grid_array_transforms(f64::from(rows), f64::from(cols), dx, dy)
        }
        Layout::Line { count, dx, dy, fit } => {
            if !(2..=10_000).contains(&count) {
                return None;
            }
            line_array_transforms(f64::from(count), dx, dy, fit)
        }
        Layout::Polar {
            center,
            count,
            fill,
            rotate,
        } => {
            if !(2..=1_000).contains(&count) {
                return None;
            }
            polar_array_transforms(center, f64::from(count), fill, rotate, source.p)
        }
        Layout::Path {
            path,
            count,
            spacing,
            align,
        } => path_array_transforms(path, f64::from(count), spacing, align)?,
    };
    let mut out = Vec::with_capacity(maps.len() + 1);
    out.push(source);
    out.extend(maps.iter().map(|m| moved(&source, m)));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64) -> Placement {
        Placement {
            p: Vec2::new(x, y),
            scale: 2.0,
            rotation: 0.5,
            mirror: true,
        }
    }

    #[test]
    fn a_grid_moves_row_after_row_and_keeps_the_turn() {
        let items = item_placements(
            at(1.0, 2.0),
            &Layout::Grid {
                rows: 2,
                cols: 2,
                dx: 10.0,
                dy: 5.0,
            },
        )
        .expect("places");
        let ps: Vec<_> = items.iter().map(|i| (i.p.x, i.p.y, i.rotation)).collect();
        assert_eq!(
            ps,
            [
                (1.0, 2.0, 0.5),
                (11.0, 2.0, 0.5),
                (1.0, 7.0, 0.5),
                (11.0, 7.0, 0.5)
            ]
        );
        assert!(items.iter().all(|i| i.scale == 2.0 && i.mirror));
    }

    #[test]
    fn a_turning_polar_array_turns_its_items_and_one_that_does_not_moves_its_base_point() {
        let c = Vec2::new(0.0, 0.0);
        let turning = item_placements(
            at(10.0, 0.0),
            &Layout::Polar {
                center: c,
                count: 4,
                fill: 360.0,
                rotate: true,
            },
        )
        .expect("places");
        assert!((turning[1].p.x).abs() < 1e-12 && (turning[1].p.y - 10.0).abs() < 1e-12);
        assert!((turning[1].rotation - (0.5 + std::f64::consts::FRAC_PI_2)).abs() < 1e-12);
        let still = item_placements(
            at(10.0, 0.0),
            &Layout::Polar {
                center: c,
                count: 4,
                fill: 360.0,
                rotate: false,
            },
        )
        .expect("places");
        assert!((still[2].p.x + 10.0).abs() < 1e-12 && still[2].p.y.abs() < 1e-12);
        assert_eq!(still[2].rotation, 0.5);
    }

    #[test]
    fn counts_out_of_range_give_no_places() {
        assert!(
            item_placements(
                at(0.0, 0.0),
                &Layout::Grid {
                    rows: 1,
                    cols: 1,
                    dx: 1.0,
                    dy: 1.0
                }
            )
            .is_none()
        );
        assert!(
            item_placements(
                at(0.0, 0.0),
                &Layout::Line {
                    count: 10_001,
                    dx: 1.0,
                    dy: 0.0,
                    fit: false
                }
            )
            .is_none()
        );
    }
}
