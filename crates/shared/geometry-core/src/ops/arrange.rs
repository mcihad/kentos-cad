//! Hizala ve dağıt (docs/adr/0194 §2): every object moves on its own, along
//! one axis, so that its box's side or middle meets a reference (`at`, an
//! easting or a northing), or the boxes stand at equal gaps between the
//! first and the last. A box is the geometry store's: an insert's with its
//! block's placed pieces, a text's in the drawing's typeface.
//!
//! The tools preview with it and `cad.entities.transform`'s `arrange`
//! writes with it on both platforms (the web through `arrangeBoxes`,
//! `arrangeAt`, `arrangeUnion` and `arrangeMoves`). The order of the
//! operations is the ADR's, so `scripts/fixtures/arrange_cases.py` meets
//! the values bit for bit (`fixtures/arrange/v1/cases.json`).

use crate::api::Op;
use crate::api::json::Json;
use crate::block::{Blocks, pieces_bounds};
use crate::entity::{Entity, Shape, entity_bounds_in};
use crate::geometry::{Bounds, empty_bounds, is_empty_bounds};
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::text::Font;
use crate::vec2::Vec2;

/// What the tool's methods and the command's `mode` do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
    Horizontal,
    Vertical,
}

impl Mode {
    /// The methods in the tool's order.
    pub const ALL: [Mode; 8] = [
        Mode::Left,
        Mode::Center,
        Mode::Right,
        Mode::Top,
        Mode::Middle,
        Mode::Bottom,
        Mode::Horizontal,
        Mode::Vertical,
    ];

    /// The command's word for it.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Left => "left",
            Mode::Center => "center",
            Mode::Right => "right",
            Mode::Top => "top",
            Mode::Middle => "middle",
            Mode::Bottom => "bottom",
            Mode::Horizontal => "horizontal",
            Mode::Vertical => "vertical",
        }
    }

    pub fn from_key(key: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == key)
    }

    /// The method's name, the undo step's too.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Left => "Sola hizala",
            Mode::Center => "Ortala",
            Mode::Right => "Sağa hizala",
            Mode::Top => "Üste hizala",
            Mode::Middle => "Ortaya hizala",
            Mode::Bottom => "Alta hizala",
            Mode::Horizontal => "Yatay dağıt",
            Mode::Vertical => "Dikey dağıt",
        }
    }

    /// Whether it meets a reference (the six alignments) rather than spreads.
    pub fn aligns(self) -> bool {
        !matches!(self, Mode::Horizontal | Mode::Vertical)
    }

    /// Whether it moves objects east and west (else north and south).
    pub fn eastward(self) -> bool {
        matches!(
            self,
            Mode::Left | Mode::Center | Mode::Right | Mode::Horizontal
        )
    }
}

/// An object's box as the geometry store keeps it (`Store::expansion`): its
/// own, and an insert's widened by its block's placed pieces.
pub fn object_bounds(shape: &Shape, blocks: &Blocks, font: Font) -> Bounds {
    let own = entity_bounds_in(shape, font);
    if !matches!(shape, Shape::Insert { .. }) {
        return own;
    }
    pieces_bounds(own, &blocks.expand(shape), font)
}

/// The side or middle of a box the mode meets: a reference box's `at`. A
/// spread's is its axis's middle.
pub fn at_of(b: &Bounds, mode: Mode) -> f64 {
    match mode {
        Mode::Left => b.min_x,
        Mode::Center | Mode::Horizontal => (b.min_x + b.max_x) / 2.0,
        Mode::Right => b.max_x,
        Mode::Top => b.max_y,
        Mode::Middle | Mode::Vertical => (b.min_y + b.max_y) / 2.0,
        Mode::Bottom => b.min_y,
    }
}

/// The boxes' union (the selection's box); none for none.
pub fn union(boxes: &[Bounds]) -> Option<Bounds> {
    let mut u = empty_bounds();
    for b in boxes {
        u.min_x = js_min(u.min_x, b.min_x);
        u.min_y = js_min(u.min_y, b.min_y);
        u.max_x = js_max(u.max_x, b.max_x);
        u.max_y = js_max(u.max_y, b.max_y);
    }
    (!is_empty_bounds(&u)).then_some(u)
}

/// Each box's displacement, east and north: an alignment's to `at` (none
/// moves without it), a spread's at equal gaps (fewer than three move
/// nothing).
pub fn moves(boxes: &[Bounds], mode: Mode, at: Option<f64>) -> Vec<Vec2> {
    let along = |d: f64| {
        if mode.eastward() {
            Vec2::new(d, 0.0)
        } else {
            Vec2::new(0.0, d)
        }
    };
    if mode.aligns() {
        let Some(at) = at else {
            return vec![Vec2::new(0.0, 0.0); boxes.len()];
        };
        return boxes.iter().map(|b| along(at - at_of(b, mode))).collect();
    }
    let n = boxes.len();
    let mut out = vec![Vec2::new(0.0, 0.0); n];
    if n < 3 {
        return out;
    }
    let span = |b: &Bounds| {
        if mode.eastward() {
            (b.min_x, b.max_x)
        } else {
            (b.min_y, b.max_y)
        }
    };
    // By their middles, equals in the given order (a stable sort).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| at_of(&boxes[i], mode).total_cmp(&at_of(&boxes[j], mode)));
    let (first_lo, _) = span(&boxes[order[0]]);
    let (_, last_hi) = span(&boxes[order[n - 1]]);
    let mut sizes = 0.0;
    for &i in &order {
        let (lo, hi) = span(&boxes[i]);
        sizes += hi - lo;
    }
    let gap = ((last_hi - first_lo) - sizes) / (n - 1) as f64;
    let mut start = first_lo;
    for k in 1..n - 1 {
        let (prev_lo, prev_hi) = span(&boxes[order[k - 1]]);
        start = (start + (prev_hi - prev_lo)) + gap;
        let (lo, _) = span(&boxes[order[k]]);
        out[order[k]] = along(start - lo);
    }
    out
}

fn mode_of(key: &str) -> Result<Mode, String> {
    Mode::from_key(key).ok_or_else(|| format!("“{key}” bir hizalama ya da dağıtma kipi değil."))
}

pub(crate) static OPS: &[Op] = &[
    // The objects' boxes, their blocks' definitions given (the contract's list).
    op!(
        "arrangeBoxes",
        |objects: Vec<Entity>, blocks: Json, font: Option<String>| {
            crate::ops::hatch_region::blocks_of(&blocks).map(|b| {
                let font = font.as_deref().map_or(Font::DEFAULT, Font::from_id);
                objects
                    .iter()
                    .map(|e| object_bounds(&e.shape, &b, font))
                    .collect::<Vec<Bounds>>()
            })
        }
    ),
    op!("arrangeAt", |b: Bounds, mode: String| {
        mode_of(&mode).map(|m| at_of(&b, m))
    }),
    op!("arrangeUnion", |boxes: Vec<Bounds>| union(&boxes)),
    op!("arrangeMoves", |boxes: Vec<Bounds>,
                         mode: String,
                         at: Option<f64>| {
        mode_of(&mode).map(|m| moves(&boxes, m, at))
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
        Bounds {
            min_x: x0,
            min_y: y0,
            max_x: x1,
            max_y: y1,
        }
    }

    #[test]
    fn an_alignment_moves_along_one_axis_only() {
        let boxes = [b(0.0, 0.0, 4.0, 2.0), b(10.0, 5.0, 12.0, 9.0)];
        let m = moves(&boxes, Mode::Right, Some(20.0));
        assert_eq!(m, [Vec2::new(16.0, 0.0), Vec2::new(8.0, 0.0)]);
        let m = moves(&boxes, Mode::Middle, Some(1.0));
        assert_eq!(m, [Vec2::new(0.0, 0.0), Vec2::new(0.0, -6.0)]);
        assert_eq!(moves(&boxes, Mode::Left, None), [Vec2::new(0.0, 0.0); 2]);
    }

    #[test]
    fn a_spread_keeps_the_outer_two_and_evens_the_gaps() {
        let boxes = [
            b(30.0, 0.0, 40.0, 5.0),
            b(0.0, 0.0, 2.0, 5.0),
            b(9.0, 0.0, 21.0, 5.0),
        ];
        let m = moves(&boxes, Mode::Horizontal, None);
        // Gaps (40 − 0 − 24) / 2 = 8: the middle one from 9 to 10.
        assert_eq!(
            m,
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0)
            ]
        );
        assert_eq!(
            moves(&boxes[..2], Mode::Horizontal, None),
            [Vec2::new(0.0, 0.0); 2]
        );
    }

    #[test]
    fn modes_read_their_words() {
        for m in Mode::ALL {
            assert_eq!(Mode::from_key(m.key()), Some(m));
        }
        assert!(mode_of("diagonal").is_err());
    }
}
