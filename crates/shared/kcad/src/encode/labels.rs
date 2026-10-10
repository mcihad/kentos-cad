//! Labels in the file (docs/specs/kcad-v2.md §6.5, §6.6; docs/adr/0212 §2):
//! a label style (its first fields as every schema wrote them, the label
//! engine's after them), a layer style's `labels` and an object's
//! `labelPins`, checked by the contract's rules as the readers check them,
//! each map's keys sorted as RFC 8949 sorts them. A style without the
//! engine's fields is written byte for byte as before.

use kentos_contracts::{
    AreaLabelMode, CalloutKind, LabelAbbreviate, LabelAlign, LabelBackground, LabelCallout,
    LabelHalo, LabelOverlap, LabelPin, LabelPosition, LabelShadow, LabelShape, LabelStack,
    LabelStyle, LabelsMode, LayerLabels, LineLabelMode, ObstacleKind, PointLabelMode, StackMode,
    layer_labels_problem, pins_problem, style_problem,
};

use super::Encoder;
use super::names::{label_ink, label_placement};
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

/// One value of a label map.
enum V<'d> {
    Float(f64),
    Text(&'d str),
    Name(&'static str),
    Bool(bool),
    Uint(u64),
    Halo(&'d LabelHalo),
    Background(&'d LabelBackground),
    Shadow(&'d LabelShadow),
    Callout(&'d LabelCallout),
    Stack(&'d LabelStack),
    Abbreviate(&'d LabelAbbreviate),
}

/// Whether a style has any of the label engine's fields (schema 36).
pub(super) fn has_engine_fields(l: &LabelStyle) -> bool {
    l.has_engine_fields()
}

impl<'d> Encoder<'d> {
    /// A map of these fields, its keys sorted.
    fn label_map(&mut self, mut f: Vec<(&'static str, V<'d>)>) -> Result<(), KcadError> {
        f.sort_by(|a, b| key_order(a.0, b.0));
        self.open(f.len(), true)?;
        for (k, v) in f {
            self.key(k);
            self.at(Seg::Name(k), |e| e.label_value(v))?;
        }
        self.close();
        Ok(())
    }

    fn label_value(&mut self, v: V<'d>) -> Result<(), KcadError> {
        match v {
            V::Float(x) => self.float(x),
            V::Text(t) => self.text(t),
            V::Name(n) => {
                self.w.text(n);
                Ok(())
            }
            V::Bool(b) => {
                self.w.bool(b);
                Ok(())
            }
            V::Uint(u) => {
                self.w.uint(u);
                Ok(())
            }
            V::Halo(h) => {
                let mut f = vec![("width", V::Float(h.width))];
                if let Some(c) = &h.color {
                    f.push(("color", V::Text(c)));
                }
                self.label_map(f)
            }
            V::Background(b) => {
                let mut f = vec![(
                    "shape",
                    V::Name(match b.shape {
                        LabelShape::Rect => "rect",
                        LabelShape::Round => "round",
                        LabelShape::Ellipse => "ellipse",
                    }),
                )];
                if let Some(c) = &b.fill {
                    f.push(("fill", V::Text(c)));
                }
                if let Some(c) = &b.stroke {
                    f.push(("stroke", V::Text(c)));
                }
                if let Some(p) = b.padding {
                    f.push(("padding", V::Float(p)));
                }
                self.label_map(f)
            }
            V::Shadow(h) => {
                let mut f = vec![("dx", V::Float(h.dx)), ("dy", V::Float(h.dy))];
                if let Some(c) = &h.color {
                    f.push(("color", V::Text(c)));
                }
                if let Some(o) = h.opacity {
                    f.push(("opacity", V::Float(o)));
                }
                self.label_map(f)
            }
            V::Callout(c) => {
                let mut f = vec![(
                    "kind",
                    V::Name(match c.kind {
                        CalloutKind::Straight => "straight",
                        CalloutKind::Manhattan => "manhattan",
                    }),
                )];
                if let Some(x) = &c.color {
                    f.push(("color", V::Text(x)));
                }
                if let Some(w) = c.width {
                    f.push(("width", V::Float(w)));
                }
                if let Some(m) = c.min_length {
                    f.push(("minLength", V::Float(m)));
                }
                self.label_map(f)
            }
            V::Stack(s) => {
                let mut f = vec![
                    (
                        "mode",
                        V::Name(match s.mode {
                            StackMode::IfNeeded => "ifNeeded",
                            StackMode::Always => "always",
                        }),
                    ),
                    ("chars", V::Uint(u64::from(s.chars))),
                ];
                if let Some(a) = &s.at {
                    f.push(("at", V::Text(a)));
                }
                self.label_map(f)
            }
            V::Abbreviate(a) => {
                let n = 1 + usize::from(a.always.is_some());
                self.open(n, true)?;
                // words (5), always (6).
                self.key("words");
                self.at(Seg::Name("words"), |e| {
                    e.open(a.words.len(), false)?;
                    for (i, w) in a.words.iter().enumerate() {
                        e.at(Seg::Index(i), |e| {
                            e.label_map(vec![
                                ("word", V::Text(&w.word)),
                                ("short", V::Text(&w.short)),
                            ])
                        })?;
                    }
                    e.close();
                    Ok(())
                })?;
                if let Some(b) = a.always {
                    self.key("always");
                    self.w.bool(b);
                }
                self.close();
                Ok(())
            }
        }
    }

    /// A label style: its fields in the file's order; the label engine's checked whole (schema 36).
    pub(super) fn label_style(&mut self, l: &'d LabelStyle) -> Result<(), KcadError> {
        if has_engine_fields(l)
            && let Some(problem) = style_problem(l)
        {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let mut f: Vec<(&'static str, V<'d>)> = vec![
            ("placement", V::Name(label_placement(l.placement))),
            ("size", V::Float(l.size)),
        ];
        let mut put = |k, v: Option<V<'d>>| {
            if let Some(v) = v {
                f.push((k, v));
            }
        };
        put("ink", l.ink.map(|i| V::Name(label_ink(i))));
        put("grow", l.grow.map(V::Float));
        put("weight", l.weight.map(|w| V::Uint(u64::from(w))));
        put("maxSize", l.max_size.map(V::Float));
        put("maxScale", l.max_scale.map(V::Float));
        put("minScale", l.min_scale.map(V::Float));
        put("template", l.template.as_deref().map(V::Text));
        put("minFeaturePx", l.min_feature_px.map(V::Float));
        put("text", l.text.as_deref().map(V::Text));
        put("color", l.color.as_deref().map(V::Text));
        put("italic", l.italic.map(V::Bool));
        put(
            "align",
            l.align.map(|a| {
                V::Name(match a {
                    LabelAlign::Left => "left",
                    LabelAlign::Center => "center",
                    LabelAlign::Right => "right",
                })
            }),
        );
        put(
            "point",
            l.point.map(|p| {
                V::Name(match p {
                    PointLabelMode::Around => "around",
                    PointLabelMode::Center => "center",
                })
            }),
        );
        put(
            "line",
            l.line.map(|m| {
                V::Name(match m {
                    LineLabelMode::Parallel => "parallel",
                    LineLabelMode::Curved => "curved",
                    LineLabelMode::Horizontal => "horizontal",
                    LineLabelMode::Contour => "contour",
                })
            }),
        );
        put(
            "area",
            l.area.map(|m| {
                V::Name(match m {
                    AreaLabelMode::Horizontal => "horizontal",
                    AreaLabelMode::Free => "free",
                    AreaLabelMode::Perimeter => "perimeter",
                    AreaLabelMode::Boundary => "boundary",
                    AreaLabelMode::Parcel => "parcel",
                    AreaLabelMode::Corner => "corner",
                })
            }),
        );
        put(
            "position",
            l.position.map(|p| {
                V::Name(match p {
                    LabelPosition::On => "on",
                    LabelPosition::Above => "above",
                    LabelPosition::Below => "below",
                    LabelPosition::Sides => "sides",
                })
            }),
        );
        put("distance", l.distance.map(V::Float));
        put("repeat", l.repeat.map(V::Float));
        put("maxAngle", l.max_angle.map(V::Float));
        put("curved", l.curved.map(V::Bool));
        put("mergeLines", l.merge_lines.map(V::Bool));
        put("inside", l.inside.map(V::Bool));
        put("outside", l.outside.map(V::Bool));
        put("halo", l.halo.as_ref().map(V::Halo));
        put("background", l.background.as_ref().map(V::Background));
        put("shadow", l.shadow.as_ref().map(V::Shadow));
        put("callout", l.callout.as_ref().map(V::Callout));
        put("stack", l.stack.as_ref().map(V::Stack));
        put("abbreviate", l.abbreviate.as_ref().map(V::Abbreviate));
        put("shrink", l.shrink.map(V::Float));
        put("priority", l.priority.map(|p| V::Uint(u64::from(p))));
        put(
            "overlap",
            l.overlap.map(|o| {
                V::Name(match o {
                    LabelOverlap::Never => "never",
                    LabelOverlap::IfNeeded => "ifNeeded",
                    LabelOverlap::Always => "always",
                })
            }),
        );
        put("duplicates", l.duplicates.map(V::Float));
        self.label_map(f)
    }

    /// A layer style's `labels` (schema 36).
    pub(super) fn layer_labels(&mut self, l: &'d LayerLabels) -> Result<(), KcadError> {
        if let Some(problem) = layer_labels_problem(l) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let n = 1 + usize::from(!l.classes.is_empty()) + usize::from(l.obstacle.is_some());
        self.open(n, true)?;
        // mode (4), classes obstacle (7).
        self.key("mode");
        self.w.text(match l.mode {
            LabelsMode::Single => "single",
            LabelsMode::Rules => "rules",
            LabelsMode::Off => "off",
        });
        if !l.classes.is_empty() {
            self.key("classes");
            self.at(Seg::Name("classes"), |e| {
                e.open(l.classes.len(), false)?;
                for (i, c) in l.classes.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(2 + usize::from(c.when.is_some()), true)?;
                        // name when (4), style (5).
                        e.key("name");
                        e.at(Seg::Name("name"), |e| e.text(&c.name))?;
                        if let Some(w) = &c.when {
                            e.key("when");
                            e.at(Seg::Name("when"), |e| e.text(w))?;
                        }
                        e.key("style");
                        e.at(Seg::Name("style"), |e| e.label_style(&c.style))?;
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        if let Some(o) = &l.obstacle {
            self.key("obstacle");
            self.at(Seg::Name("obstacle"), |e| {
                e.open(1 + usize::from(o.kind.is_some()), true)?;
                // kind (4), weight (6).
                if let Some(k) = o.kind {
                    e.key("kind");
                    e.w.text(match k {
                        ObstacleKind::Interior => "interior",
                        ObstacleKind::Boundary => "boundary",
                    });
                }
                e.key("weight");
                e.w.uint(u64::from(o.weight));
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// An object's `labelPins` (schema 36).
    pub(super) fn label_pins(&mut self, pins: &'d [LabelPin]) -> Result<(), KcadError> {
        if let Some(problem) = pins_problem(pins) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(pins.len(), false)?;
        for (i, p) in pins.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let n = [
                    p.at.is_some(),
                    p.class.is_some(),
                    p.hidden.is_some(),
                    p.rotation.is_some(),
                ]
                .iter()
                .filter(|b| **b)
                .count();
                e.open(n, true)?;
                // at (2), class (5), hidden (6), rotation (8).
                if let Some(a) = &p.at {
                    e.key("at");
                    e.at(Seg::Name("at"), |e| e.point(a))?;
                }
                if let Some(c) = &p.class {
                    e.key("class");
                    e.at(Seg::Name("class"), |e| e.text(c))?;
                }
                if let Some(h) = p.hidden {
                    e.key("hidden");
                    e.w.bool(h);
                }
                if let Some(r) = p.rotation {
                    e.key("rotation");
                    e.at(Seg::Name("rotation"), |e| e.float(r))?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }
}
