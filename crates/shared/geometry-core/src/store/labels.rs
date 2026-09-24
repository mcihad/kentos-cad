//! Labels and grips from the store (`drawLabels`, `drawGrips`,
//! `apps/web/src/viewport/overlay.ts`): which texts, dimensions and labels a frame
//! draws and where, and the grips of selected objects. The overlay walked
//! every object each frame; the store asks its tree for the view and applies
//! the same tests (visible layer, box in view, text size on screen, the
//! label style's scale range and smallest feature). Strings and styles stay
//! with the overlay: it reads them from the object.

use super::Store;
use crate::api::json::Json;
use crate::entity::{Shape, dimension_geom, entity_anchor, entity_vertices};
use crate::geom::dimension::layout_dimension;
use crate::geometry::Bounds;
use crate::jsmath::js_min;
use crate::ops::grips::{entity_grips, mid_grip_segment};

/// Where a label sits (`LabelStyle.placement`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Center,
    Corner,
    Beside,
    Along,
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
    let placement = match v.get("placement") {
        Json::Str(s) if s == "center" => Placement::Center,
        Json::Str(s) if s == "corner" => Placement::Corner,
        Json::Str(s) if s == "beside" => Placement::Beside,
        Json::Str(s) if s == "along" => Placement::Along,
        _ => return Err("etiket yerleşimi center, corner, beside ya da along olmalı".into()),
    };
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

/// Numbers per label record.
pub const LABEL_STRIDE: usize = 8;

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

    /// What the overlay draws in `view` at `scale` px/m, in the document's
    /// order, `LABEL_STRIDE` numbers each: `id, what, x, y, a, b, c, d`.
    /// - dimension: x, y the value's place, a its angle (degrees), b the
    ///   value, c 1 for an angle (0 length), d the prefix (0 none, 1 "R ", 2 "Ø ");
    /// - text: x, y its insertion point, a its rotation;
    /// - centre and beside: x, y the anchor; corner: x, y the box's top left;
    /// - along: x, y and a, b the two vertices the label sits between.
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
                    let unit = if l.unit == "angle" { 1.0 } else { 0.0 };
                    let prefix = match l.prefix {
                        "R " => 1.0,
                        "Ø " => 2.0,
                        _ => 0.0,
                    };
                    out.extend([
                        it.id,
                        LABEL_DIMENSION,
                        l.text_at.x,
                        l.text_at.y,
                        l.rotation,
                        l.value,
                        unit,
                        prefix,
                    ]);
                    continue;
                }
                Shape::Text {
                    p,
                    height,
                    rotation,
                    ..
                } => {
                    let px = height * scale;
                    if px < 5.0 || px > 240.0 {
                        continue;
                    }
                    out.extend([it.id, LABEL_TEXT, p.x, p.y, *rotation, 0.0, 0.0, 0.0]);
                    continue;
                }
                _ => {}
            }
            if !it.label {
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
            match rule.placement {
                Placement::Center | Placement::Beside => {
                    // A path without vertices has no anchor (the TypeScript failed on it).
                    let Some(a) = entity_anchor(&it.shape) else {
                        continue;
                    };
                    let what = if rule.placement == Placement::Center {
                        LABEL_CENTER
                    } else {
                        LABEL_BESIDE
                    };
                    out.extend([it.id, what, a.x, a.y, 0.0, 0.0, 0.0, 0.0]);
                }
                Placement::Corner => {
                    out.extend([it.id, LABEL_CORNER, b.min_x, b.max_y, 0.0, 0.0, 0.0, 0.0])
                }
                Placement::Along => {
                    // A third of the way along, between two vertices.
                    let pts = entity_vertices(&it.shape);
                    if pts.len() < 2 {
                        continue;
                    }
                    let i = ((pts.len() as f64 * 0.35).floor() as usize).min(pts.len() - 2);
                    out.extend([
                        it.id,
                        LABEL_ALONG,
                        pts[i].x,
                        pts[i].y,
                        pts[i + 1].x,
                        pts[i + 1].y,
                        0.0,
                        0.0,
                    ]);
                }
            }
        }
        out
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
                Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts.len(),
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
        let parcel = [1.0, LABEL_CORNER, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];
        let street = [2.0, LABEL_ALONG, 10.0, 20.0, 20.0, 20.0, 0.0, 0.0];
        let text = [4.0, LABEL_TEXT, 1.0, 1.0, 30.0, 0.0, 0.0, 0.0];
        assert_eq!(s.labels(&view, 3.0, None), [parcel, street, text].concat());
        // The text being edited is left out.
        assert_eq!(s.labels(&view, 3.0, Some(4.0)), [parcel, street].concat());
        // At 1 px/m the street is below its scale range and the text below 5 px.
        assert_eq!(s.labels(&view, 1.0, None), parcel);
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
