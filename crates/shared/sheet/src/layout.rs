//! Constraint relayout (design §3.2): when the paper's size, orientation or
//! margins change, every item moves to its new place by its constraints.
//! The box is the page, the margins or the item's group; each axis keeps
//! what its constraint says. Results are whole micrometres (a half away from
//! zero) and never smaller than 1 mm. A stretched map keeps its scale and
//! shows more: its view is a centre and a scale, never an extent.

use crate::model::{ConstraintBox, Constraints, HConstraint, Item, Page, VConstraint};
use crate::units::{RectUm, round_i64, sat};
use crate::validate::ITEM_MIN;

#[derive(Clone, Copy)]
enum Mode {
    Start,
    End,
    Both,
    Center,
    Scale,
}

/// One axis: the item's `[pos, pos + size)` in a box that moves from `[b0, b0 + l0)` to `[b1, b1 + l1)`.
fn axis(pos: i64, size: i64, b0: i64, l0: i64, b1: i64, l1: i64, mode: Mode) -> (i64, i64) {
    let min = i64::from(ITEM_MIN);
    match mode {
        Mode::Start => (b1 + (pos - b0), size),
        Mode::End => {
            let from_end = (b0 + l0) - (pos + size);
            (b1 + l1 - from_end - size, size)
        }
        Mode::Both => {
            let start = b1 + (pos - b0);
            let end = b1 + l1 - ((b0 + l0) - (pos + size));
            (start, (end - start).max(min))
        }
        Mode::Center => {
            // Twice the centres keep the arithmetic whole.
            let off2 = (2 * pos + size) - (2 * b0 + l0);
            let c2 = 2 * b1 + l1 + off2;
            (round_i64((c2 - size) as f64 / 2.0), size)
        }
        Mode::Scale => {
            if l0 <= 0 {
                return (b1 + (pos - b0), size);
            }
            let r = l1 as f64 / l0 as f64;
            let start = b1 as f64 + (pos - b0) as f64 * r;
            let end = b1 as f64 + (pos + size - b0) as f64 * r;
            let (s, e) = (round_i64(start), round_i64(end));
            (s, (e - s).max(min))
        }
    }
}

fn h_mode(h: HConstraint) -> Mode {
    match h {
        HConstraint::Left => Mode::Start,
        HConstraint::Right => Mode::End,
        HConstraint::LeftRight => Mode::Both,
        HConstraint::Center => Mode::Center,
        HConstraint::Scale => Mode::Scale,
    }
}

fn v_mode(v: VConstraint) -> Mode {
    match v {
        VConstraint::Top => Mode::Start,
        VConstraint::Bottom => Mode::End,
        VConstraint::TopBottom => Mode::Both,
        VConstraint::Center => Mode::Center,
        VConstraint::Scale => Mode::Scale,
    }
}

/// The frame an item with constraints `c` gets when its box goes from `old` to `new`.
pub fn relayout_frame(frame: &RectUm, c: &Constraints, old: &RectUm, new: &RectUm) -> RectUm {
    let (x, w) = axis(
        i64::from(frame.left),
        i64::from(frame.width),
        i64::from(old.left),
        i64::from(old.width),
        i64::from(new.left),
        i64::from(new.width),
        h_mode(c.h),
    );
    let (y, h) = axis(
        i64::from(frame.top),
        i64::from(frame.height),
        i64::from(old.top),
        i64::from(old.height),
        i64::from(new.top),
        i64::from(new.height),
        v_mode(c.v),
    );
    RectUm::new(sat(x), sat(y), sat(w), sat(h))
}

/// The depth of an item in its groups (0: not in a group).
fn depth(items: &[Item], i: usize) -> usize {
    let mut d = 0;
    let mut up = items.get(i).and_then(|it| it.group.as_deref());
    while let Some(g) = up {
        d += 1;
        if d > items.len() {
            break;
        }
        up = items
            .iter()
            .find(|x| x.id == g)
            .and_then(|x| x.group.as_deref());
    }
    d
}

/// The frames of `items` (same order) laid out from page `from` to page `to`;
/// a group's own frame is what its constraints give (it is made its
/// children's together when the book is normalised).
pub fn relayout_items(items: &[Item], from: &Page, to: &Page) -> Vec<RectUm> {
    let mut out: Vec<RectUm> = items.iter().map(|i| i.frame).collect();
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (depth(items, i), i));
    for i in order {
        let it = &items[i];
        let (old, new) = match it.constraints.relative_to {
            ConstraintBox::Page => (from.rect(), to.rect()),
            ConstraintBox::Margins => (from.margin_rect(), to.margin_rect()),
            ConstraintBox::Group => {
                let g = it
                    .group
                    .as_deref()
                    .and_then(|g| items.iter().position(|x| x.id == g));
                match g {
                    Some(gi) => (items[gi].frame, out[gi]),
                    None => (from.margin_rect(), to.margin_rect()),
                }
            }
        };
        out[i] = relayout_frame(&it.frame, &it.constraints, &old, &new);
    }
    out
}

/// `items` with their frames laid out from `from` to `to`.
pub fn relayout(items: &mut [Item], from: &Page, to: &Page) {
    let frames = relayout_items(items, from, to);
    for (it, f) in items.iter_mut().zip(frames) {
        it.frame = f;
    }
    crate::validate::group_frames(items);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(l: i32, t: i32, w: i32, h: i32) -> RectUm {
        RectUm::new(l, t, w, h)
    }

    #[test]
    fn each_constraint_keeps_what_it_says() {
        let old = r(10_000, 10_000, 400_000, 277_000);
        let new = r(10_000, 10_000, 820_000, 574_000);
        let f = r(300_000, 200_000, 100_000, 50_000);
        let c = |h, v| Constraints::new(h, v);
        // Left/top: same distances from the left and top.
        assert_eq!(
            relayout_frame(&f, &c(HConstraint::Left, VConstraint::Top), &old, &new),
            f
        );
        // Right/bottom: same distances from the right (10 mm) and the bottom (37 mm).
        assert_eq!(
            relayout_frame(&f, &c(HConstraint::Right, VConstraint::Bottom), &old, &new),
            r(720_000, 497_000, 100_000, 50_000)
        );
        // Both: it stretches.
        assert_eq!(
            relayout_frame(
                &f,
                &c(HConstraint::LeftRight, VConstraint::TopBottom),
                &old,
                &new
            ),
            r(300_000, 200_000, 520_000, 347_000)
        );
        // Centre: the centre's offset from the box's centre is kept: 350 − 210 = 140 mm across
        // (centre 420 + 140 = 560), 225 − 148.5 = 76.5 mm down (centre 297 + 76.5 = 373.5).
        assert_eq!(
            relayout_frame(&f, &c(HConstraint::Center, VConstraint::Center), &old, &new),
            r(510_000, 348_500, 100_000, 50_000)
        );
        // Scale: 820/400 = 2.05 across (10 + 290·2.05 = 604.5, 10 + 390·2.05 = 809.5) and
        // 574/277 down (10 + 190·574/277 = 403.718 4, 10 + 240·574/277 = 507.328 5).
        assert_eq!(
            relayout_frame(&f, &c(HConstraint::Scale, VConstraint::Scale), &old, &new),
            r(604_500, 403_718, 205_000, 103_611)
        );
    }

    #[test]
    fn a_stretched_item_is_never_smaller_than_a_millimetre() {
        let old = r(0, 0, 400_000, 300_000);
        let new = r(0, 0, 100_000, 300_000);
        // 200 mm from the left and 150 mm from the right do not fit a 100 mm box.
        let f = r(200_000, 0, 50_000, 10_000);
        let c = Constraints::new(HConstraint::LeftRight, VConstraint::Top);
        assert_eq!(
            relayout_frame(&f, &c, &old, &new),
            r(200_000, 0, 1_000, 10_000)
        );
    }
}
