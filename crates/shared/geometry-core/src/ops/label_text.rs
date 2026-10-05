//! Etiketleri yazıya çevir (docs/adr/0175 §1): a layer's label as a text
//! object, as the sheet writes it on the paper at 1:N
//! (`apps/web/src/app/sheet/mapLabels.ts`): the style's size in CSS px on
//! the paper (25.4/96 mm each) is the text's height on the ground, the
//! placement's offsets are paper px, a label along a line turns to read
//! upright; out of the style's scale range or on an object smaller than its
//! smallest feature it is not written; one touching an earlier label's
//! 8 px paper cells (counted from the drawing's origin) is thinned unless
//! every label is wanted. Where an object's label sits is `spot`, which the
//! store's labels of the drawing use too. The independent reference is
//! `scripts/fixtures/label_text_cases.py`.

use std::collections::HashSet;

use crate::api::Op;
use crate::entity::{Entity, Shape, entity_anchor, entity_bounds_in, entity_vertices, label_part};
use crate::geometry::Bounds;
use crate::jsmath::{PI, atan2, cos, js_min, sin};
use crate::op;
use crate::store::labels::{LabelLook, Placement};
use crate::text::{Font, TextAlign, width_em};
use crate::vec2::Vec2;

/// CSS px per metre of paper: 96 px an inch.
const PX_PER_M: f64 = 96.0 / 0.0254;
/// The thinning's cells, paper px (the sheet's and the overlay's `LabelRoom`).
const CELL: f64 = 8.0;
/// A label over this many cells is bigger than any paper: it is written and
/// takes none, so that a style's absurd size cannot stall the thinning.
const MOST_CELLS: i64 = 1 << 20;

/// Where an object's label sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spot {
    /// Centre and beside: the object's anchor; corner: its box's top left.
    At(Vec2),
    /// Along: between two vertices.
    Between(Vec2, Vec2),
}

/// Where an object's label sits for `placement` (`Store::labels`): its
/// anchor, its box's top left, or the vertices a third of the way along
/// (0.35 of them) of its `label_part`; none for a path without vertices.
pub fn spot(shape: &Shape, bounds: &Bounds, placement: Placement) -> Option<Spot> {
    match placement {
        Placement::Center | Placement::Beside => entity_anchor(shape).map(Spot::At),
        Placement::Corner => Some(Spot::At(Vec2::new(bounds.min_x, bounds.max_y))),
        Placement::Along => {
            let pts = entity_vertices(&label_part(shape));
            if pts.len() < 2 {
                return None;
            }
            let i = ((pts.len() as f64 * 0.35).floor() as usize).min(pts.len() - 2);
            Some(Spot::Between(pts[i], pts[i + 1]))
        }
    }
}

/// A label's text: the template's first `{label}` the label, literally (no
/// `$&` and such, as JavaScript's `replace` has); without a template, or
/// with an empty one, the label.
pub fn fill_template(template: Option<&str>, label: &str) -> String {
    match template {
        Some(t) if !t.is_empty() => t.replacen("{label}", label, 1),
        _ => label.to_owned(),
    }
}

/// A label to write: where its object puts it, how big the object is, its
/// text and its layer's label style (sizes CSS px, scales px per metre).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelItem {
    pub placement: Placement,
    /// The anchor (centre, beside), the box's top left (corner) or the first vertex (along).
    pub p: Vec2,
    /// Along: the second vertex.
    pub q: Option<Vec2>,
    /// The smaller side of the object's box, metres.
    pub feature: f64,
    /// The text, its template filled (`fill_template`).
    pub text: String,
    /// The text's width in em in the drawing's typeface (`text::width_em`).
    pub em: f64,
    pub size: f64,
    pub grow: Option<f64>,
    pub max_size: Option<f64>,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
    pub min_feature_px: Option<f64>,
}

crate::json_struct!(LabelItem {
    placement,
    p,
    q,
    feature,
    text,
    em,
    size,
    grow,
    max_size => "maxSize",
    min_scale => "minScale",
    max_scale => "maxScale",
    min_feature_px => "minFeaturePx",
});

/// A label written as a text object.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelText {
    /// The label it was made from (its place in the input).
    pub item: usize,
    pub text: String,
    pub p: Vec2,
    /// On the ground, metres.
    pub height: f64,
    /// Degrees counter-clockwise from east.
    pub rotation: f64,
    pub align: TextAlign,
}

crate::json_struct!(out LabelText {
    item,
    text,
    p,
    height,
    rotation,
    align
});

/// The texts, and how many labels were passed over and why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LabelTexts {
    pub texts: Vec<LabelText>,
    /// Out of their style's scale range.
    pub out_of_scale: usize,
    /// On an object smaller than the style's smallest feature, or of no size.
    pub small: usize,
    /// Touching an earlier label's cells.
    pub overlapping: usize,
}

crate::json_struct!(out LabelTexts {
    texts,
    out_of_scale => "outOfScale",
    small,
    overlapping
});

/// The labels' texts at 1:`scale`, in the given order (the drawing's: the
/// first of overlapping labels stays); `thin` passes over a label touching
/// an earlier one's cells. An empty text is no text and is counted nowhere.
pub fn label_texts(items: &[LabelItem], scale: f64, thin: bool) -> LabelTexts {
    let mut out = LabelTexts::default();
    if !(scale.is_finite() && scale > 0.0) {
        return out;
    }
    // CSS px per ground metre on the paper.
    let k = PX_PER_M / scale;
    let mut taken: HashSet<(i64, i64)> = HashSet::new();
    for (i, it) in items.iter().enumerate() {
        if it.text.is_empty() || !(it.p.x.is_finite() && it.p.y.is_finite() && it.em.is_finite()) {
            continue;
        }
        if it.min_scale.is_some_and(|m| k < m) || it.max_scale.is_some_and(|m| k > m) {
            out.out_of_scale += 1;
            continue;
        }
        if it.min_feature_px.is_some_and(|m| it.feature * k < m) {
            out.small += 1;
            continue;
        }
        let s = js_min(
            it.max_size.unwrap_or(it.size),
            it.size + it.grow.unwrap_or(0.0) * k,
        );
        if !(s > 0.0) {
            out.small += 1;
            continue;
        }
        // On the paper, px: x right, y down, from the drawing's origin.
        let w = it.em * s;
        let (x, y) = (it.p.x * k, -it.p.y * k);
        let (p, align, rotation, cells) = match it.placement {
            Placement::Center => (
                it.p,
                TextAlign::MiddleCenter,
                0.0,
                [x - w / 2.0, y - s / 2.0, x + w / 2.0, y + s / 2.0],
            ),
            Placement::Corner => (
                Vec2::new(it.p.x + 8.0 / k, it.p.y - 14.0 / k),
                TextAlign::MiddleLeft,
                0.0,
                [x + 8.0, y + 14.0 - s / 2.0, x + 8.0 + w, y + 14.0 + s / 2.0],
            ),
            Placement::Beside => (
                Vec2::new(it.p.x + 7.0 / k, it.p.y + 7.0 / k),
                TextAlign::MiddleLeft,
                0.0,
                [x + 7.0, y - 7.0 - s / 2.0, x + 7.0 + w, y - 7.0 + s / 2.0],
            ),
            Placement::Along => {
                let Some(q) = it.q.filter(|q| q.x.is_finite() && q.y.is_finite()) else {
                    continue;
                };
                // Past a right angle either way it turns half a turn, to read upright.
                let mut a = atan2(q.y - it.p.y, q.x - it.p.x);
                if a > PI / 2.0 || a < -PI / 2.0 {
                    a += PI;
                }
                let mut degrees = a * 180.0 / PI;
                if degrees > 180.0 {
                    degrees -= 360.0;
                }
                let (mx, my) = ((x + q.x * k) / 2.0, (y - q.y * k) / 2.0);
                let (c, sn) = (cos(a).abs(), sin(a).abs());
                let (hx, hy) = ((c * w + sn * s) / 2.0, (sn * w + c * s) / 2.0);
                (
                    Vec2::new((it.p.x + q.x) / 2.0, (it.p.y + q.y) / 2.0),
                    TextAlign::MiddleCenter,
                    degrees,
                    [mx - hx, my - hy, mx + hx, my + hy],
                )
            }
        };
        if thin {
            let cell = |v: f64| (v / CELL).floor() as i64;
            let (c0, r0, c1, r1) = (
                cell(cells[0]),
                cell(cells[1]),
                cell(cells[2]),
                cell(cells[3]),
            );
            let span = (c1 - c0 + 1).saturating_mul(r1 - r0 + 1);
            if (1..=MOST_CELLS).contains(&span) {
                if (c0..=c1).any(|c| (r0..=r1).any(|r| taken.contains(&(c, r)))) {
                    out.overlapping += 1;
                    continue;
                }
                for c in c0..=c1 {
                    for r in r0..=r1 {
                        taken.insert((c, r));
                    }
                }
            }
        }
        out.texts.push(LabelText {
            item: i,
            text: it.text.clone(),
            p,
            height: s / k,
            rotation,
            align,
        });
    }
    out
}

/// The label an object of `shape` with box `bounds` writes by style `look`:
/// its place by the placement (`spot`), its text by the template, the text's
/// width in `font`; none for an empty label or a shape with no place.
pub fn label_item(
    shape: &Shape,
    bounds: &Bounds,
    label: &str,
    look: &LabelLook,
    font: Font,
) -> Option<LabelItem> {
    if label.is_empty() {
        return None;
    }
    let (p, q) = match spot(shape, bounds, look.placement)? {
        Spot::At(p) => (p, None),
        Spot::Between(p, q) => (p, Some(q)),
    };
    let text = fill_template(look.template.as_deref(), label);
    let em = width_em(&text, font);
    Some(LabelItem {
        placement: look.placement,
        p,
        q,
        feature: js_min(bounds.max_x - bounds.min_x, bounds.max_y - bounds.min_y),
        text,
        em,
        size: look.size,
        grow: look.grow,
        max_size: look.max_size,
        min_scale: look.min_scale,
        max_scale: look.max_scale,
        min_feature_px: look.min_feature_px,
    })
}

/// One object's label as a text at 1:`scale`, thinned with nothing: what a
/// linked text keeps writing as its object changes (docs/adr/0175 §4). The
/// box is the object's own (the store's adds an insert's placed pieces).
/// None when the rule writes nothing: no label, no place, out of the
/// style's scale range, smaller than its smallest feature, of no size.
pub fn label_text_of(
    shape: &Shape,
    label: &str,
    look: &LabelLook,
    scale: f64,
    font: Font,
) -> Option<LabelText> {
    let bounds = entity_bounds_in(shape, font);
    let item = label_item(shape, &bounds, label, look, font)?;
    label_texts(&[item], scale, false).texts.into_iter().next()
}

pub(crate) static OPS: &[Op] = &[
    op!("labelTexts", |items: Vec<LabelItem>,
                       scale: f64,
                       thin: bool| label_texts(
        &items, scale, thin
    )),
    op!("labelTextOf", |entity: Entity,
                        label: String,
                        style: LabelLook,
                        scale: f64,
                        font: String| {
        label_text_of(&entity.shape, &label, &style, scale, Font::from_id(&font))
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_is_filled_literally_once() {
        assert_eq!(fill_template(None, "101"), "101");
        assert_eq!(fill_template(Some(""), "101"), "101");
        assert_eq!(fill_template(Some("No: {label}"), "101"), "No: 101");
        assert_eq!(fill_template(Some("{label}/{label}"), "7"), "7/{label}");
        assert_eq!(fill_template(Some("Ada"), "7"), "Ada");
        // JavaScript's replace would write the template's text for `$&`.
        assert_eq!(fill_template(Some("[{label}]"), "a$&b"), "[a$&b]");
    }

    #[test]
    fn a_label_along_a_multi_part_line_stays_on_its_longest_part() {
        let part = |pts: &[(f64, f64)]| crate::entity::Part {
            pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
            bulges: None,
            holes: None,
        };
        let line = Shape::Polyline {
            pts: vec![Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0)],
            bulges: None,
            holes: None,
            parts: Some(vec![part(&[
                (0.0, 10.0),
                (20.0, 10.0),
                (40.0, 10.0),
                (60.0, 10.0),
            ])]),
        };
        let b = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 60.0,
            max_y: 10.0,
        };
        // Four vertices of the longest part: the second segment, not one across the parts.
        assert_eq!(
            spot(&line, &b, Placement::Along),
            Some(Spot::Between(Vec2::new(20.0, 10.0), Vec2::new(40.0, 10.0)))
        );
        assert_eq!(
            spot(&line, &b, Placement::Corner),
            Some(Spot::At(Vec2::new(0.0, 10.0)))
        );
    }

    /// A linked text's rule is one object's label, thinned with nothing:
    /// the same text the store writes for it alone (docs/adr/0175 §4).
    #[test]
    fn one_objects_label_is_the_stores_for_it_alone() {
        let parcel = Shape::Polygon {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(30.0, 0.0),
                Vec2::new(30.0, 20.0),
                Vec2::new(0.0, 20.0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        };
        let look = LabelLook {
            placement: Placement::Corner,
            size: 10.0,
            grow: Some(1.0),
            max_size: Some(14.0),
            template: Some("Ada {label}".into()),
            min_feature_px: Some(26.0),
            min_scale: None,
            max_scale: None,
        };
        let font = Font::from_id("overpass");
        let mut store = crate::store::Store::new();
        store.set_font(font);
        store.put(1.0, "k", true, parcel.clone());
        let wanted = [crate::store::labels::LabelWanted {
            id: 1.0,
            label: "12".into(),
            style: look.clone(),
        }];
        let from_store = store.label_texts(&wanted, 1000.0, true);
        let alone = label_text_of(&parcel, "12", &look, 1000.0, font).expect("a text");
        assert_eq!(from_store.texts, [LabelText { item: 0, ..alone }]);
        // No label, and too small at 1:10000 (20 m × 0.38 px/m under 26 px): none.
        assert_eq!(label_text_of(&parcel, "", &look, 1000.0, font), None);
        assert_eq!(label_text_of(&parcel, "12", &look, 10000.0, font), None);
    }

    #[test]
    fn a_label_of_absurd_size_takes_no_cells_and_does_not_stall() {
        let item = |text: &str, size: f64| LabelItem {
            placement: Placement::Center,
            p: Vec2::new(0.0, 0.0),
            q: None,
            feature: 10.0,
            text: text.into(),
            em: 2.0,
            size,
            grow: None,
            max_size: None,
            min_scale: None,
            max_scale: None,
            min_feature_px: None,
        };
        let out = label_texts(&[item("dev", 1e12), item("101", 10.0)], 1000.0, true);
        assert_eq!(out.texts.len(), 2);
        assert_eq!(out.overlapping, 0);
        let none = label_texts(&[item("101", 10.0)], 0.0, true);
        assert_eq!(none, LabelTexts::default());
    }
}
