//! Etiketleri yazıya çevir (docs/adr/0175, 0212 §4): the labels the label
//! engine places at 1:N, as the sheet writes them, written as objects. A
//! label's size in CSS px on the paper (25.4/96 mm each) is its text's
//! height on the ground; a one-line label is a text centred where the
//! engine put its line; a stacked one a multi-line text whose lines are the
//! label's; a curved one a text along a curve through its letters' middles;
//! a callout a line. A label with no free place is passed over (counted)
//! unless every label is wanted, then written at its best place. The
//! independent reference of the placing is `scripts/fixtures/label_engine_cases.py`.

use crate::api::Op;
use crate::api::json::Json;
use crate::entity::{Entity, Shape};
use crate::jsmath::{cos, js_max, js_min, sin};
use crate::labels::engine::{CURVED, UNPLACED};
use crate::op;
use crate::store::Store;
use crate::store::labels::LABEL_STRIDE;
use crate::store::placing::{
    LABEL_PLACED, LABEL_PLACED_CALLOUT, LABEL_PLACED_LETTER, LABEL_PLACED_LINE, ObjectLabels,
    PlaceOptions, Shown,
};
use crate::text::{Font, TextAlign};
use crate::vec2::Vec2;

/// CSS px per metre of paper: 96 px an inch.
pub const PX_PER_M: f64 = 96.0 / 0.0254;
/// A multi-line label's lines are 1.2 sizes apart; a multi-line text's in 5/3 of its height.
const LINE_SPACING: f64 = 1.2 / (5.0 / 3.0);

/// A label's text: the template's first `{label}` the label, literally (no
/// `$&` and such, as JavaScript's `replace` has); without a template, or
/// with an empty one, the label.
pub fn fill_template(template: Option<&str>, label: &str) -> String {
    match template {
        Some(t) if !t.is_empty() => t.replacen("{label}", label, 1),
        _ => label.to_owned(),
    }
}

/// A label written as a text object.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelText {
    /// The asked object it was made from (its place in the input).
    pub item: usize,
    /// The label's class (its layer's rule's place; 0 a single label's).
    pub class: u32,
    pub text: String,
    pub p: Vec2,
    /// On the ground, metres.
    pub height: f64,
    /// Degrees counter-clockwise from east.
    pub rotation: f64,
    pub align: TextAlign,
    /// A multi-line text's line spacing (in 5/3 of its height); none for one line.
    pub line_spacing: Option<f64>,
    /// A curved label's curve: its vertices after `p` in the text's frame (its rotation 0), metres.
    pub path: Option<Vec<Vec2>>,
}

crate::json_struct!(out LabelText {
    item,
    class,
    text,
    p,
    height,
    rotation,
    align,
    line_spacing => "lineSpacing",
    path
});

/// A label's callout written as a line: from by the label to its object.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelCallout {
    pub item: usize,
    pub class: u32,
    pub from: Vec2,
    pub to: Vec2,
}

crate::json_struct!(out LabelCallout { item, class, from, to });

/// The texts and callouts, and how many labels were passed over and why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LabelTexts {
    pub texts: Vec<LabelText>,
    pub callouts: Vec<LabelCallout>,
    /// Out of their class's scale range.
    pub out_of_scale: usize,
    /// On an object smaller than the class's smallest feature.
    pub small: usize,
    /// With no free place (written only when every label is wanted).
    pub overlapping: usize,
}

crate::json_struct!(out LabelTexts {
    texts,
    callouts,
    out_of_scale => "outOfScale",
    small,
    overlapping
});

/// One placed label's records: its frame and its parts.
struct Placed<'r> {
    frame: &'r [f64],
    parts: Vec<&'r [f64]>,
}

/// The label records of a window grouped by label (frames and their parts); other records left out.
fn placed(records: &[f64]) -> Vec<Placed<'_>> {
    let mut out: Vec<Placed<'_>> = Vec::new();
    for r in records.chunks_exact(LABEL_STRIDE) {
        let what = r[1];
        if what == LABEL_PLACED {
            out.push(Placed {
                frame: r,
                parts: Vec::new(),
            });
        } else if (what == LABEL_PLACED_LINE
            || what == LABEL_PLACED_LETTER
            || what == LABEL_PLACED_CALLOUT)
            && let Some(last) = out.last_mut()
            && last.frame[0] == r[0]
        {
            last.parts.push(r);
        }
    }
    out
}

/// The objects a window's labels make, of the labels whose object `item_of`
/// names; `k` px per metre; `every` writes the unplaced ones too.
pub fn texts_of(
    shown: &Shown,
    item_of: impl Fn(f64) -> Option<usize>,
    k: f64,
    every: bool,
    out: &mut LabelTexts,
) {
    for label in placed(&shown.records) {
        let f = label.frame;
        let Some(item) = item_of(f[0]) else {
            continue;
        };
        let (class, state) = (f[7] as u32, f[8] as u32);
        if state & UNPLACED != 0 && !every {
            out.overlapping += 1;
            continue;
        }
        let (mid, angle, w) = (Vec2::new(f[2], f[3]), f[4], f[5]);
        let lines: Vec<&[f64]> = label
            .parts
            .iter()
            .copied()
            .filter(|r| r[1] == LABEL_PLACED_LINE)
            .collect();
        let letters: Vec<&[f64]> = label
            .parts
            .iter()
            .copied()
            .filter(|r| r[1] == LABEL_PLACED_LETTER)
            .collect();
        let text_of = |i: f64| shown.texts.get(i as usize).cloned().unwrap_or_default();
        if state & CURVED != 0 && !letters.is_empty() {
            // Through the letters' middles, from half the first letter's advance before it to half the last's after.
            let size = letters[0][5];
            let dir = |r: &[f64], sign: f64| {
                let a = r[4].to_radians();
                let half = sign * r[8] / 2.0 / k;
                Vec2::new(r[2] + half * cos(a), r[3] + half * sin(a))
            };
            let start = dir(letters[0], -1.0);
            let mut pts: Vec<Vec2> = letters
                .iter()
                .map(|r| Vec2::new(r[2] - start.x, r[3] - start.y))
                .collect();
            let end = dir(letters[letters.len() - 1], 1.0);
            pts.push(Vec2::new(end.x - start.x, end.y - start.y));
            out.texts.push(LabelText {
                item,
                class,
                text: text_of(letters[0][6]),
                p: start,
                height: size / k,
                rotation: 0.0,
                align: TextAlign::MiddleLeft,
                line_spacing: None,
                path: Some(pts),
            });
        } else if lines.len() == 1 {
            let r = lines[0];
            out.texts.push(LabelText {
                item,
                class,
                text: text_of(r[6]),
                p: Vec2::new(r[2], r[3]),
                height: r[5] / k,
                rotation: r[4],
                align: TextAlign::MiddleCenter,
                line_spacing: None,
                path: None,
            });
        } else if lines.len() > 1 {
            // The lines' alignment from where the first stands in the block: its middle off the block's by half the gap.
            let (c, s) = (cos(angle.to_radians()), sin(angle.to_radians()));
            let r = lines[0];
            let along = ((r[2] - mid.x) * c + (r[3] - mid.y) * s) * k;
            let gap = (w - r[7]) / 2.0;
            let (align, shift) = if gap > 1e-6 && (along + gap).abs() < 1e-6 {
                (TextAlign::MiddleLeft, -w / 2.0)
            } else if gap > 1e-6 && (along - gap).abs() < 1e-6 {
                (TextAlign::MiddleRight, w / 2.0)
            } else {
                (TextAlign::MiddleCenter, 0.0)
            };
            let text = lines
                .iter()
                .map(|r| text_of(r[6]))
                .collect::<Vec<_>>()
                .join("\n");
            out.texts.push(LabelText {
                item,
                class,
                text,
                p: Vec2::new(mid.x + shift / k * c, mid.y + shift / k * s),
                height: r[5] / k,
                rotation: angle,
                align,
                line_spacing: Some(LINE_SPACING),
                path: None,
            });
        }
        for r in label.parts.iter().filter(|r| r[1] == LABEL_PLACED_CALLOUT) {
            out.callouts.push(LabelCallout {
                item,
                class,
                from: Vec2::new(r[2], r[3]),
                to: Vec2::new(r[4], r[5]),
            });
        }
    }
}

/// One object's label as a text at 1:`scale`, placed alone (nothing else on
/// the page): what a linked text keeps writing as its object changes
/// (docs/adr/0175 §4, 0212 §4). `style` is the contract's `LabelStyle`,
/// `point` its layer's point symbol's size (px). None when the engine
/// writes nothing: no text, out of the style's scale range, smaller than its
/// smallest feature, no place inside an area that wants one.
pub fn label_text_of(
    shape: &Shape,
    text: &str,
    style: &Json,
    scale: f64,
    point: f64,
    font: Font,
) -> Option<LabelText> {
    if text.is_empty() || !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    let mut store = Store::new();
    store.set_font(font);
    store.put(1.0, "l", true, shape.clone());
    let mut row = String::from("[{\"id\":\"l\",\"rank\":0,\"point\":");
    crate::api::json::ToJson::write_json(&point, &mut row);
    row.push_str(",\"label\":");
    crate::api::json::ToJson::write_json(style, &mut row);
    row.push_str("}]");
    store.set_label_layers_json(&row).ok()?;
    store.set_object_labels(
        1.0,
        ObjectLabels {
            texts: vec![(0, text.to_owned())],
            z: f64::NAN,
        },
    );
    store
        .label_texts(&[1.0], scale, true)
        .texts
        .into_iter()
        .next()
}

impl Store {
    /// Etiketleri yazıya çevir (docs/adr/0212 §4): the labels of the objects
    /// `ids` as the engine places the window round them at 1:`scale` (with
    /// every other object's label and the drawing's texts in it, as the
    /// sheet does), written as objects; `every` writes the labels with no
    /// free place too. A text's `item` is its object's place in `ids`.
    pub fn label_texts(&self, ids: &[f64], scale: f64, every: bool) -> LabelTexts {
        let mut out = LabelTexts::default();
        if !(scale.is_finite() && scale > 0.0) {
            return out;
        }
        let k = PX_PER_M / scale;
        let mut items: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        let mut window: Option<crate::geometry::Bounds> = None;
        for (i, &id) in ids.iter().enumerate() {
            let Some(it) = self.get(id) else {
                continue;
            };
            items.entry(id.to_bits()).or_insert(i);
            let b = it.bounds;
            window = Some(match window {
                None => b,
                Some(w) => crate::geometry::Bounds {
                    min_x: js_min(w.min_x, b.min_x),
                    min_y: js_min(w.min_y, b.min_y),
                    max_x: js_max(w.max_x, b.max_x),
                    max_y: js_max(w.max_y, b.max_y),
                },
            });
            self.count_unwritten(id, k, &mut out);
        }
        let Some(window) = window else {
            return out;
        };
        let shown = self.labels(
            &window,
            k,
            None,
            PlaceOptions {
                unplaced: true,
                hidden: false,
            },
        );
        texts_of(
            &shown,
            |id| items.get(&id.to_bits()).copied(),
            k,
            every,
            &mut out,
        );
        out
    }
}

pub(crate) static OPS: &[Op] =
    &[op!("labelTextOf", |entity: Entity,
                          label: String,
                          style: Json,
                          scale: f64,
                          point: f64,
                          font: String| {
        label_text_of(
            &entity.shape,
            &label,
            &style,
            scale,
            point,
            Font::from_id(&font),
        )
    })];

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

    /// A linked text's place is the object's label placed alone: the same text the store writes for it alone
    /// (docs/adr/0175 §4).
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
        let style = Json::parse(r#"{"placement":"center","size":10,"grow":1,"maxSize":14,"template":"Ada {label}","minFeaturePx":26}"#).unwrap();
        let font = Font::from_id("overpass");
        let alone = label_text_of(&parcel, "Ada 12", &style, 1000.0, 0.0, font).expect("a text");
        // At 1:1000 (3.78 px/m) the size is 10 + 3.78, its height on the ground 13.78 / 3.78 m; the pole is the middle.
        let k = PX_PER_M / 1000.0;
        assert!((alone.height - (10.0 + k) / k).abs() < 1e-9);
        assert!(
            (alone.p.x - 15.0).abs() < 0.5 && (alone.p.y - 10.0).abs() < 0.5,
            "{:?}",
            alone.p
        );
        assert_eq!(
            (alone.text.as_str(), alone.align, alone.rotation),
            ("Ada 12", TextAlign::MiddleCenter, 0.0)
        );
        // No text, and too small at 1:10000 (20 m × 0.38 px/m under 26 px): none.
        assert_eq!(label_text_of(&parcel, "", &style, 1000.0, 0.0, font), None);
        assert_eq!(
            label_text_of(&parcel, "Ada 12", &style, 10000.0, 0.0, font),
            None
        );
    }
}
