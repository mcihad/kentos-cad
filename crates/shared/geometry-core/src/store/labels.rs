//! Labels and grips from the store (`drawLabels`, `drawGrips`,
//! `apps/web/src/viewport/overlay.ts`): which texts, dimensions and labels a frame
//! draws and where, and the grips of selected objects. The overlay walked
//! every object each frame; the store asks its tree for the view and applies
//! the same tests (visible layer, box in view, text size on screen, the
//! label style's scale range and smallest feature). Strings and styles stay
//! with the overlay: it reads them from the object.

use super::Store;
use crate::api::json::{FromJson, Json};
use crate::entity::{Shape, TextPlace, dimension_geom};
use crate::geom::dimension::layout_dimension;
use crate::geom::leader::note_place;
use crate::geometry::Bounds;
use crate::jsmath::js_min;
use crate::ops::grips::{entity_grips, mid_grip_segment};
use crate::ops::label_text::{LabelTexts, Spot, label_item, label_texts as write_labels, spot};

/// Where a label sits (`LabelStyle.placement`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Center,
    Corner,
    Beside,
    Along,
}

impl Placement {
    /// Its name in the contract's `LabelPlacement` (`center` …).
    pub fn name(self) -> &'static str {
        match self {
            Placement::Center => "center",
            Placement::Corner => "corner",
            Placement::Beside => "beside",
            Placement::Along => "along",
        }
    }

    /// The placement named `name`; none for any other name.
    pub fn from_name(name: &str) -> Option<Placement> {
        [
            Placement::Center,
            Placement::Corner,
            Placement::Beside,
            Placement::Along,
        ]
        .into_iter()
        .find(|p| p.name() == name)
    }
}

impl crate::api::json::FromJson for Placement {
    fn from_json(v: &Json) -> Result<Placement, String> {
        match v {
            Json::Str(s) => Placement::from_name(s)
                .ok_or_else(|| "etiket yerleşimi center, corner, beside ya da along olmalı".into()),
            _ => Err("etiket yerleşimi center, corner, beside ya da along olmalı".into()),
        }
    }
}

impl crate::api::json::ToJson for Placement {
    fn write_json(&self, out: &mut String) {
        crate::api::json::write_str(out, self.name());
    }
}

/// What of a `LabelStyle` decides whether and where a label is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelRule {
    pub placement: Placement,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
    pub min_feature_px: Option<f64>,
}

/// Reads `{ placement, minScale?, maxScale?, minFeaturePx? }`.
pub(crate) fn read_rule(v: &Json) -> Result<LabelRule, String> {
    let placement = Placement::from_json(v.get("placement"))?;
    let num = |k: &str| match v.get(k) {
        Json::Num(x) => Ok(Some(*x)),
        Json::Null => Ok(None),
        _ => Err(format!("{k} sayı olmalı")),
    };
    Ok(LabelRule {
        placement,
        min_scale: num("minScale")?,
        max_scale: num("maxScale")?,
        min_feature_px: num("minFeaturePx")?,
    })
}

/// A kind's index in the label defaults (the kinds the overlay has defaults for).
fn default_slot(s: &Shape) -> Option<usize> {
    match s {
        Shape::Polygon { .. } => Some(0),
        Shape::Circle { .. } => Some(1),
        Shape::Point { .. } => Some(2),
        Shape::Polyline { .. } => Some(3),
        Shape::Line { .. } => Some(4),
        _ => None,
    }
}

/// What a label record says (`labels`, second number).
pub const LABEL_DIMENSION: f64 = 0.0;
pub const LABEL_TEXT: f64 = 1.0;
pub const LABEL_CENTER: f64 = 2.0;
pub const LABEL_CORNER: f64 = 3.0;
pub const LABEL_BESIDE: f64 = 4.0;
pub const LABEL_ALONG: f64 = 5.0;
/// A text among a block's pieces (docs/adr/0144): x, y where its baseline
/// starts, a its rotation, b its height as placed, c the piece's place, d
/// its width factor, e its mask's width (as a text's).
pub const LABEL_PIECE_TEXT: f64 = 6.0;
/// A dimension among a block's pieces: x, y its value's place, a its angle,
/// b the value, c the piece's place, d its text height as placed, e 1 for a
/// mask (docs/adr/0147; its unit and prefix from its style, `dimension_measure`).
pub const LABEL_PIECE_DIMENSION: f64 = 7.0;
/// A leader's note (docs/adr/0146 §5), as a text's: x, y where its baseline
/// starts, a its rotation, b 1 (it has no width factor), c its mask's width.
pub const LABEL_LEADER: f64 = 8.0;
/// A leader's note among a block's pieces, as `LABEL_PIECE_TEXT`.
pub const LABEL_PIECE_LEADER: f64 = 9.0;

/// Numbers per label record.
pub const LABEL_STRIDE: usize = 9;

/// A dimension's unit as its label record says it (c): length, angle,
/// percent, coordinate (docs/adr/0147).
pub const DIMENSION_UNITS: [&str; 4] = ["length", "angle", "percent", "coordinate"];
/// A dimension's prefix as its label record says it (d).
pub const DIMENSION_PREFIXES: [&str; 7] = ["", "R ", "Ø ", "Y=", "X=", "t=", "%"];

fn code(of: &[&str], s: &str) -> f64 {
    of.iter().position(|x| *x == s).unwrap_or(0) as f64
}

/// 1 for a dimension with a mask, else 0.
fn masked(s: &Shape) -> f64 {
    match s {
        Shape::Dimension { mask: Some(true), .. } => 1.0,
        _ => 0.0,
    }
}

/// The label style's parts Etiketleri yazıya çevir reads (the contract's
/// `LabelStyle`; its weight and ink have no text counterpart, docs/adr/0175 §1).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelLook {
    pub placement: Placement,
    pub size: f64,
    pub grow: Option<f64>,
    pub max_size: Option<f64>,
    pub template: Option<String>,
    pub min_feature_px: Option<f64>,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
}

crate::json_struct!(LabelLook {
    placement,
    size,
    grow,
    max_size => "maxSize",
    template,
    min_feature_px => "minFeaturePx",
    min_scale => "minScale",
    max_scale => "maxScale",
});

/// A label to write as a text: the object, its label and its layer's (or
/// its kind's default) label style.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelWanted {
    pub id: f64,
    pub label: String,
    pub style: LabelLook,
}

crate::json_struct!(LabelWanted { id, label, style });

impl Store {
    /// Label defaults by kind when the layer has no label style
    /// (`DEFAULT_LABELS`): `{ polygon, circle, point, polyline, line }`.
    pub fn set_label_defaults_json(&mut self, text: &str) -> Result<(), String> {
        let v = Json::parse(text)?;
        let mut out = [None; 5];
        for (i, kind) in ["polygon", "circle", "point", "polyline", "line"]
            .iter()
            .enumerate()
        {
            let r = v.get(kind);
            if !matches!(r, Json::Null) {
                out[i] = Some(read_rule(r).map_err(|e| format!("{kind}: {e}"))?);
            }
        }
        self.label_defaults = out;
        Ok(())
    }

    /// The same defaults, typed: `[polygon, circle, point, polyline, line]`
    /// (the desktop's, which has no JSON on the way).
    pub fn set_label_defaults(&mut self, rules: [Option<LabelRule>; 5]) {
        self.label_defaults = rules;
    }

    /// What the overlay draws in `view` at `scale` px/m, in the document's
    /// order, `LABEL_STRIDE` numbers each: `id, what, x, y, a, b, c, d, e`.
    /// - dimension: x, y the value's place, a its angle (degrees), b the
    ///   value, c its unit (`DIMENSION_UNITS`: 0 length, 1 angle, 2 percent,
    ///   3 coordinate), d its prefix (`DIMENSION_PREFIXES`: 0 none, 1 "R ",
    ///   2 "Ø ", 3 "Y=", 4 "X=", 5 "t=", 6 "%"), e 1 for a mask (docs/adr/0147:
    ///   the overlay fills the value's measured box, as a text's);
    /// - text: x, y where its baseline starts (its `p` moved by its
    ///   alignment, docs/adr/0145), a its rotation, b its width factor (1
    ///   without one), c its mask's width, metres (0 without a mask: the
    ///   overlay fills `TextPlace::mask`'s box, from the origin c along and
    ///   a line and a tenth of the height around);
    /// - centre and beside: x, y the anchor; corner: x, y the box's top left;
    /// - along: x, y and a, b the two vertices the label sits between;
    /// - a leader's note (`LABEL_LEADER`), as a text's;
    /// - a block's text, dimension or leader pieces (`LABEL_PIECE_TEXT`,
    ///   `LABEL_PIECE_DIMENSION`, `LABEL_PIECE_LEADER`), before the insert's own label.
    ///
    /// `editing` is left out (the inline editor draws it).
    pub fn labels(&self, view: &Bounds, scale: f64, editing: Option<f64>) -> Vec<f64> {
        let mut out = Vec::new();
        for it in self.candidates(&super::padded(*view, 0.0)) {
            let flags = self.flags(it);
            if !flags.visible {
                continue;
            }
            let b = &it.bounds;
            if b.max_x < view.min_x
                || b.min_x > view.max_x
                || b.max_y < view.min_y
                || b.min_y > view.max_y
            {
                continue;
            }
            if Some(it.id) == editing {
                continue;
            }
            // A block's texts and dimension values, where its pieces are placed (docs/adr/0144).
            if let Some(x) = &it.expanded {
                for (i, s) in x.shapes.iter().enumerate() {
                    match s {
                        // An attribute's text that shows nothing has no label (docs/adr/0144 §7).
                        Shape::Text {
                            text,
                            height,
                            rotation,
                            ..
                        } => {
                            let px = height * scale;
                            if px < 5.0 || px > 240.0 || text.is_empty() {
                                continue;
                            }
                            let (o, factor, mask) = self.text_label(s);
                            out.extend([
                                it.id,
                                LABEL_PIECE_TEXT,
                                o.x,
                                o.y,
                                *rotation,
                                *height,
                                i as f64,
                                factor,
                                mask,
                            ]);
                        }
                        Shape::Leader {
                            height, rotation, ..
                        } => {
                            let px = height * scale;
                            let Some((o, mask)) = self.note_label(s) else {
                                continue;
                            };
                            if px < 5.0 || px > 240.0 {
                                continue;
                            }
                            out.extend([
                                it.id,
                                LABEL_PIECE_LEADER,
                                o.x,
                                o.y,
                                *rotation,
                                *height,
                                i as f64,
                                1.0,
                                mask,
                            ]);
                        }
                        Shape::Dimension { height, .. } => {
                            let px = height * scale;
                            let Some(l) = dimension_geom(s).and_then(|g| layout_dimension(&g))
                            else {
                                continue;
                            };
                            if px < 5.0 || px > 240.0 {
                                continue;
                            }
                            out.extend([
                                it.id,
                                LABEL_PIECE_DIMENSION,
                                l.text_at.x,
                                l.text_at.y,
                                l.rotation,
                                l.value,
                                i as f64,
                                *height,
                                masked(s),
                            ]);
                        }
                        _ => {}
                    }
                }
            }
            match &it.shape {
                Shape::Dimension { height, .. } => {
                    let px = height * scale;
                    let Some(l) = dimension_geom(&it.shape).and_then(|g| layout_dimension(&g))
                    else {
                        continue;
                    };
                    if px < 5.0 || px > 240.0 {
                        continue;
                    }
                    out.extend([
                        it.id,
                        LABEL_DIMENSION,
                        l.text_at.x,
                        l.text_at.y,
                        l.rotation,
                        l.value,
                        code(&DIMENSION_UNITS, l.unit),
                        code(&DIMENSION_PREFIXES, l.prefix),
                        masked(&it.shape),
                    ]);
                    continue;
                }
                Shape::Text {
                    height, rotation, ..
                } => {
                    let px = height * scale;
                    if px < 5.0 || px > 240.0 {
                        continue;
                    }
                    let (o, factor, mask) = self.text_label(&it.shape);
                    out.extend([
                        it.id, LABEL_TEXT, o.x, o.y, *rotation, factor, mask, 0.0, 0.0,
                    ]);
                    continue;
                }
                // Its note, as a text's; under 5 px it is not drawn, its line and arrowhead are (docs/adr/0146 §5).
                Shape::Leader {
                    height, rotation, ..
                } => {
                    let px = height * scale;
                    if let Some((o, mask)) = self.note_label(&it.shape)
                        && (5.0..=240.0).contains(&px)
                    {
                        out.extend([
                            it.id,
                            LABEL_LEADER,
                            o.x,
                            o.y,
                            *rotation,
                            1.0,
                            mask,
                            0.0,
                            0.0,
                        ]);
                    }
                    continue;
                }
                _ => {}
            }
            // A text writes its label (docs/adr/0175 §4).
            if !it.label || self.text_labelled.contains(&it.id.to_bits()) {
                continue;
            }
            let Some(rule) = flags
                .label
                .or_else(|| default_slot(&it.shape).and_then(|i| self.label_defaults[i]))
            else {
                continue;
            };
            if rule.min_scale.is_some_and(|m| scale < m)
                || rule.max_scale.is_some_and(|m| scale > m)
            {
                continue;
            }
            if rule
                .min_feature_px
                .is_some_and(|m| js_min(b.max_x - b.min_x, b.max_y - b.min_y) * scale < m)
            {
                continue;
            }
            // A path without vertices has no place (the TypeScript failed on it).
            match spot(&it.shape, b, rule.placement) {
                Some(Spot::At(a)) => {
                    let what = match rule.placement {
                        Placement::Center => LABEL_CENTER,
                        Placement::Beside => LABEL_BESIDE,
                        _ => LABEL_CORNER,
                    };
                    out.extend([it.id, what, a.x, a.y, 0.0, 0.0, 0.0, 0.0, 0.0]);
                }
                Some(Spot::Between(a, c)) => {
                    out.extend([it.id, LABEL_ALONG, a.x, a.y, c.x, c.y, 0.0, 0.0, 0.0]);
                }
                None => {}
            }
        }
        out
    }

    /// The texts Etiketleri yazıya çevir writes for `wanted` (in the
    /// drawing's order) at 1:`scale` (`ops::label_text`): each object's
    /// place by its style (`spot`), its text by the template, the text's
    /// width in the drawing's typeface. A text's `item` is its place in
    /// `wanted`; an empty label, an unknown id, a text, dimension or leader
    /// (they show no label) and a path without vertices give none and are
    /// counted nowhere.
    pub fn label_texts(&self, wanted: &[LabelWanted], scale: f64, thin: bool) -> LabelTexts {
        let mut items = Vec::new();
        let mut from = Vec::new();
        for (i, w) in wanted.iter().enumerate() {
            if w.label.is_empty() {
                continue;
            }
            let Some(it) = self.get(w.id) else {
                continue;
            };
            if matches!(
                it.shape,
                Shape::Text { .. } | Shape::Dimension { .. } | Shape::Leader { .. }
            ) {
                continue;
            }
            let Some(item) = label_item(&it.shape, &it.bounds, &w.label, &w.style, self.font)
            else {
                continue;
            };
            items.push(item);
            from.push(i);
        }
        let mut out = write_labels(&items, scale, thin);
        for t in &mut out.texts {
            t.item = from[t.item];
        }
        out
    }

    /// A leader's note (docs/adr/0146 §5): where its baseline starts and its
    /// mask's width (0 without a mask), in the drawing's typeface; none
    /// without a note.
    fn note_label(&self, s: &Shape) -> Option<(crate::vec2::Vec2, f64)> {
        let t = note_place(s)?;
        let masked = matches!(
            s,
            Shape::Leader {
                mask: Some(true),
                ..
            }
        );
        Some((
            t.origin(self.font),
            if masked { t.width(self.font) } else { 0.0 },
        ))
    }

    /// A text's label (docs/adr/0145): where its baseline starts, its width
    /// factor (1 without one) and its mask's width (0 without a mask), in the
    /// drawing's typeface.
    fn text_label(&self, s: &Shape) -> (crate::vec2::Vec2, f64, f64) {
        let Some(t) = TextPlace::of(s) else {
            return (crate::vec2::Vec2::new(0.0, 0.0), 1.0, 0.0);
        };
        let masked = matches!(
            s,
            Shape::Text {
                mask: Some(true),
                ..
            }
        );
        (
            t.origin(self.font),
            t.width_factor.unwrap_or(1.0),
            if masked { t.width(self.font) } else { 0.0 },
        )
    }

    /// Grips of these objects (`entityGrips`), known ids only, in the given
    /// order: `id, count, vertices`, then `x, y, segment` per grip, where
    /// `vertices` is a path's vertex count (0 otherwise) and `segment` the
    /// segment of a mid grip (−1 for other grips).
    pub fn grips(&self, ids: &[f64]) -> Vec<f64> {
        let mut out = Vec::new();
        for &id in ids {
            let Some(it) = self.get(id) else { continue };
            let grips = entity_grips(&it.shape);
            let vertices = match &it.shape {
                Shape::Polyline { pts, .. }
                | Shape::Polygon { pts, .. }
                | Shape::Leader { pts, .. } => pts.len(),
                _ => 0,
            };
            out.extend([id, grips.len() as f64, vertices as f64]);
            for (i, g) in grips.iter().enumerate() {
                let seg = mid_grip_segment(&it.shape, i).map_or(-1.0, |s| s as f64);
                out.extend([g.x, g.y, seg]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::label_text::LabelItem;
    use crate::text::width_em;

    #[test]
    fn places_labels_by_the_layer_rule_or_the_kinds_default() {
        let mut s = Store::new();
        s.put_json(
            r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
                {"id":2,"layerId":"yol","label":"Cadde","kind":"polyline","pts":[{"x":0,"y":20},{"x":10,"y":20},{"x":20,"y":20}]},
                {"id":3,"layerId":"yol","label":"","kind":"line","a":{"x":0,"y":30},"b":{"x":9,"y":30}},
                {"id":4,"layerId":"yazi","kind":"text","p":{"x":1,"y":1},"text":"Not","height":2,"rotation":30},
                {"id":5,"layerId":"parsel","label":"7","kind":"polygon","pts":[{"x":100,"y":100},{"x":101,"y":100},{"x":101,"y":101}]}]"#,
        )
        .unwrap();
        s.set_layers_json(
            r#"[{"id":"parsel","visible":true,"locked":false,"pickInterior":true,"label":{"placement":"corner","minFeaturePx":5}},
                {"id":"yol","visible":true,"locked":false,"pickInterior":true}]"#,
        )
        .unwrap();
        s.set_label_defaults_json(
            r#"{"polyline":{"placement":"along","minScale":1.6},"line":{"placement":"along"}}"#,
        )
        .unwrap();
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        // At 3 px/m: the parcel by its corner, the street a third of the way
        // along, the text; the unlabelled line and the 1 m parcel (3 px) not.
        let parcel = [1.0, LABEL_CORNER, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let street = [2.0, LABEL_ALONG, 10.0, 20.0, 20.0, 20.0, 0.0, 0.0, 0.0];
        // The text: its baseline's start (no alignment: its p), width factor 1, no mask.
        let text = [4.0, LABEL_TEXT, 1.0, 1.0, 30.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(s.labels(&view, 3.0, None), [parcel, street, text].concat());
        // The text being edited is left out.
        assert_eq!(s.labels(&view, 3.0, Some(4.0)), [parcel, street].concat());
        // At 1 px/m the street is below its scale range and the text below 5 px.
        assert_eq!(s.labels(&view, 1.0, None), parcel);
    }

    /// An object whose label a text writes shows none of its own (docs/adr/0175
    /// §4): the text is its label; the list is replaced whole and goes with `clear`.
    #[test]
    fn an_object_whose_label_a_text_writes_shows_none_of_its_own() {
        let mut s = Store::new();
        let objects = r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
                {"id":2,"layerId":"parsel","label":"13","kind":"polygon","pts":[{"x":10,"y":0},{"x":20,"y":0},{"x":20,"y":10},{"x":10,"y":10}]},
                {"id":3,"layerId":"yazi","kind":"text","p":{"x":5,"y":5},"text":"12","height":2,"rotation":0}]"#;
        s.put_json(objects).unwrap();
        s.set_layers_json(r#"[{"id":"parsel","visible":true,"locked":false,"pickInterior":true,"label":{"placement":"center"}}]"#)
            .unwrap();
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        let first = [1.0, LABEL_CENTER, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let second = [2.0, LABEL_CENTER, 15.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let text = [3.0, LABEL_TEXT, 5.0, 5.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(s.labels(&view, 3.0, None), [first, second, text].concat());
        s.set_text_labelled(&[1.0]);
        assert_eq!(s.labels(&view, 3.0, None), [second, text].concat());
        s.set_text_labelled(&[2.0]);
        assert_eq!(s.labels(&view, 3.0, None), [first, text].concat());
        s.clear();
        s.put_json(objects).unwrap();
        assert_eq!(s.labels(&view, 3.0, None), [first, second, text].concat());
    }

    /// A multi-part polyline's label along it sits on its longest part, a
    /// multi-part area's at its largest part (docs/adr/0175 §1): never
    /// between the end of one part and the start of the next.
    #[test]
    fn a_multi_part_objects_label_sits_on_its_main_part() {
        let mut s = Store::new();
        s.put_json(
            r#"[{"id":1,"layerId":"yol","label":"Dere","kind":"polyline","pts":[{"x":0,"y":0},{"x":5,"y":0}],
                 "parts":[{"pts":[{"x":0,"y":10},{"x":20,"y":10},{"x":40,"y":10},{"x":60,"y":10}]}]}]"#,
        )
        .unwrap();
        s.set_layers_json(r#"[{"id":"yol","visible":true,"locked":false,"pickInterior":true,"label":{"placement":"along"}}]"#)
            .unwrap();
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        assert_eq!(
            s.labels(&view, 3.0, None),
            [1.0, LABEL_ALONG, 20.0, 10.0, 40.0, 10.0, 0.0, 0.0, 0.0]
        );
    }

    /// Etiketleri yazıya çevir through the store (docs/adr/0175 §1): the
    /// objects by id in the given order, each placed by the style it comes
    /// with, the template filled, the text measured in the drawing's
    /// typeface; a text's `item` is its place among the wanted.
    #[test]
    fn labels_become_texts_by_id_style_and_template() {
        let mut s = Store::new();
        s.set_font(crate::text::Font::from_id("arimo"));
        s.put_json(
            r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":30,"y":0},{"x":30,"y":20},{"x":0,"y":20}]},
                {"id":2,"layerId":"yazi","kind":"text","p":{"x":1,"y":1},"text":"Not","height":2,"rotation":0},
                {"id":3,"layerId":"nokta","label":"P7","kind":"point","p":{"x":100,"y":50}}]"#,
        )
        .unwrap();
        let look = |placement: Placement, template: Option<&str>| LabelLook {
            placement,
            size: 10.0,
            grow: None,
            max_size: None,
            template: template.map(str::to_owned),
            min_feature_px: None,
            min_scale: None,
            max_scale: None,
        };
        let wanted = [
            LabelWanted {
                id: 3.0,
                label: "P7".into(),
                style: look(Placement::Beside, None),
            },
            LabelWanted {
                id: 2.0,
                label: "Not".into(),
                style: look(Placement::Center, None),
            },
            LabelWanted {
                id: 99.0,
                label: "yok".into(),
                style: look(Placement::Center, None),
            },
            LabelWanted {
                id: 1.0,
                label: "12".into(),
                style: look(Placement::Corner, Some("Parsel {label}")),
            },
        ];
        let out = s.label_texts(&wanted, 1000.0, true);
        let k = 96.0 / 0.0254 / 1000.0;
        assert_eq!(out.texts.len(), 2, "{out:?}");
        let (point, parcel) = (&out.texts[0], &out.texts[1]);
        assert_eq!((point.item, point.text.as_str()), (0, "P7"));
        assert!(
            (point.p.x - (100.0 + 7.0 / k)).abs() < 1e-9
                && (point.p.y - (50.0 + 7.0 / k)).abs() < 1e-9
        );
        assert_eq!((parcel.item, parcel.text.as_str()), (3, "Parsel 12"));
        assert!(
            (parcel.p.x - 8.0 / k).abs() < 1e-9 && (parcel.p.y - (20.0 - 14.0 / k)).abs() < 1e-9
        );
        assert!((parcel.height - 10.0 / k).abs() < 1e-12);
        // The same through the rule, with the text measured as the store measures it.
        let em = width_em("Parsel 12", crate::text::Font::from_id("arimo"));
        let alone = write_labels(
            &[LabelItem {
                placement: Placement::Corner,
                p: crate::vec2::Vec2::new(0.0, 20.0),
                q: None,
                feature: 20.0,
                text: "Parsel 12".into(),
                em,
                size: 10.0,
                grow: None,
                max_size: None,
                min_scale: None,
                max_scale: None,
                min_feature_px: None,
            }],
            1000.0,
            true,
        );
        assert_eq!(alone.texts[0].p, parcel.p);
        assert_eq!(out.overlapping + out.small + out.out_of_scale, 0);
    }

    /// A text's label starts where its alignment puts its baseline, with
    /// its width factor and its mask's width; it is picked where it is drawn
    /// (docs/adr/0145). By hand, at 90°: along the text is north, up from it
    /// west, so a top-right text's baseline starts its height east and its
    /// width south of its point.
    #[test]
    fn a_texts_label_starts_where_its_alignment_puts_it() {
        let mut s = Store::new();
        s.set_font(crate::text::Font::from_id("courier-prime"));
        s.put_json(
            r#"[{"id":7,"layerId":"yazi","kind":"text","p":{"x":10,"y":20},"text":"WW","height":2,"rotation":90,"align":"topRight","widthFactor":0.5,"mask":true}]"#,
        )
        .unwrap();
        let w =
            crate::text::width_em("WW", crate::text::Font::from_id("courier-prime")) * 2.0 * 0.5;
        let out = s.labels(
            &Bounds {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 40.0,
                max_y: 40.0,
            },
            20.0,
            None,
        );
        assert_eq!(out.len(), LABEL_STRIDE);
        assert_eq!(out[..2], [7.0, LABEL_TEXT]);
        assert!(
            (out[2] - 12.0).abs() < 1e-12 && (out[3] - (20.0 - w)).abs() < 1e-12,
            "{out:?}"
        );
        assert_eq!(out[4..], [90.0, 0.5, w, 0.0, 0.0]);
        // Its body: 9.7 … 12.46 east, 20 − w … 20 north; where it stood unaligned is empty.
        assert_eq!(s.hit(crate::vec2::Vec2::new(11.0, 19.9), 0.01), Some(7.0));
        assert_eq!(s.hit(crate::vec2::Vec2::new(8.0, 20.4), 0.01), None);
        // A plain text's label is its point, width factor 1, no mask.
        s.put_json(r#"[{"id":8,"layerId":"yazi","kind":"text","p":{"x":30,"y":30},"text":"A","height":2,"rotation":0}]"#)
            .unwrap();
        let out = s.labels(
            &Bounds {
                min_x: 25.0,
                min_y: 25.0,
                max_x: 40.0,
                max_y: 40.0,
            },
            20.0,
            None,
        );
        assert_eq!(out, [8.0, LABEL_TEXT, 30.0, 30.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    }

    /// A leader's note (docs/adr/0146 §2, §5): half its height under the
    /// landing's level, from h/2 past the landing's end; to the left, its
    /// width back from there. By hand, h = 2: the landing 4 m, the note 1 m
    /// on; to the right from (4, 3) its baseline starts at (9, 2), to the
    /// left from (26, 3) at (21 − w, 2).
    #[test]
    fn a_leaders_note_is_labelled_past_its_landing() {
        let mut s = Store::new();
        let font = crate::text::Font::from_id("courier-prime");
        s.set_font(font);
        s.put_json(
            r#"[{"id":3,"layerId":"k","kind":"leader","pts":[{"x":0,"y":0},{"x":4,"y":3}],"text":"Not","height":2,"rotation":0,"mask":true},
                {"id":4,"layerId":"k","kind":"leader","pts":[{"x":30,"y":0},{"x":26,"y":3}],"text":"Not","height":2,"rotation":0,"label":"K-1"},
                {"id":5,"layerId":"k","kind":"leader","pts":[{"x":40,"y":0},{"x":44,"y":3}],"height":2,"rotation":0}]"#,
        )
        .unwrap();
        let w = crate::text::width_em("Not", font) * 2.0;
        let view = Bounds {
            min_x: -10.0,
            min_y: -10.0,
            max_x: 60.0,
            max_y: 20.0,
        };
        // Masked: its width; its etiket is not drawn, a text's is not either; without a note nothing.
        let right = [3.0, LABEL_LEADER, 9.0, 2.0, 0.0, 1.0, w, 0.0, 0.0];
        let left = [4.0, LABEL_LEADER, 21.0 - w, 2.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(s.labels(&view, 3.0, None), [right, left].concat());
        // Under 5 px its note is not drawn.
        assert!(s.labels(&view, 2.0, None).is_empty());
        // It is picked on its note, on its landing and on its line; not past the note.
        let at = |x: f64, y: f64| s.hit(crate::vec2::Vec2::new(x, y), 0.01);
        assert_eq!(at(9.5, 2.5), Some(3.0));
        assert_eq!(at(6.0, 3.0), Some(3.0));
        assert_eq!(at(2.0, 1.5), Some(3.0));
        assert_eq!(at(9.0 + w + 1.0, 2.5), None);
    }

    #[test]
    fn lists_grips_with_their_mid_segments() {
        let mut s = Store::new();
        s.put_json(r#"[{"id":9,"layerId":"a","kind":"polyline","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10}]}]"#)
            .unwrap();
        let g = s.grips(&[9.0, 99.0]);
        assert_eq!(&g[..3], &[9.0, 5.0, 3.0]);
        assert_eq!(
            &g[3..],
            &[
                0.0, 0.0, -1.0, 10.0, 0.0, -1.0, 10.0, 10.0, -1.0, 5.0, 0.0, 0.0, 10.0, 5.0, 1.0
            ]
        );
    }
}
