//! Labels in the file (docs/specs/kcad-v2.md §6.5, §6.6; docs/adr/0212 §2):
//! a label style (its first fields every schema reads; the label engine's
//! only schema 36), a layer style's `labels` and an object's `labelPins`
//! (schema 36), read field by field and checked whole by the contract's
//! rules (`style_problem`, `layer_labels_problem`, `pins_problem`), as the
//! writers check them.

use kentos_contracts::{
    AreaLabelMode, CalloutKind, LabelAbbreviate, LabelAlign, LabelBackground, LabelCallout,
    LabelClass, LabelHalo, LabelInk, LabelObstacle, LabelOverlap, LabelPin, LabelPlacement,
    LabelPosition, LabelShadow, LabelShape, LabelStack, LabelStyle, LabelWord, LabelsMode,
    LayerLabels, LineLabelMode, ObstacleKind, PointLabelMode, StackMode, layer_labels_problem,
    pins_problem, style_problem,
};

use super::{list, map, named, point, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A label style; the label engine's fields only when `engine` (schema 36).
pub(super) fn label_style(r: &mut Reader<'_>, engine: bool) -> Result<LabelStyle, KcadError> {
    let at = r.position();
    let mut s = LabelStyle::default();
    let (mut size, mut placement) = (None, None);
    map(r, |r, key| {
        match key {
            "ink" => {
                s.ink = Some(named(
                    r,
                    &[
                        ("fg", LabelInk::Fg),
                        ("fg-dim", LabelInk::FgDim),
                        ("label", LabelInk::Label),
                    ],
                )?)
            }
            "grow" => s.grow = Some(r.float()?),
            "size" => size = Some(r.float()?),
            "weight" => s.weight = Some(r.uint(u64::from(u16::MAX))? as u16),
            "maxSize" => s.max_size = Some(r.float()?),
            "maxScale" => s.max_scale = Some(r.float()?),
            "minScale" => s.min_scale = Some(r.float()?),
            "template" => s.template = Some(text(r)?),
            "placement" => {
                placement = Some(named(
                    r,
                    &[
                        ("center", LabelPlacement::Center),
                        ("corner", LabelPlacement::Corner),
                        ("beside", LabelPlacement::Beside),
                        ("along", LabelPlacement::Along),
                    ],
                )?)
            }
            "minFeaturePx" => s.min_feature_px = Some(r.float()?),
            // The label engine's (schema 36).
            "text" if engine => s.text = Some(text(r)?),
            "color" if engine => s.color = Some(text(r)?),
            "italic" if engine => s.italic = Some(r.bool()?),
            "align" if engine => {
                s.align = Some(named(
                    r,
                    &[
                        ("left", LabelAlign::Left),
                        ("center", LabelAlign::Center),
                        ("right", LabelAlign::Right),
                    ],
                )?)
            }
            "point" if engine => {
                s.point = Some(named(
                    r,
                    &[
                        ("around", PointLabelMode::Around),
                        ("center", PointLabelMode::Center),
                    ],
                )?)
            }
            "line" if engine => {
                s.line = Some(named(
                    r,
                    &[
                        ("parallel", LineLabelMode::Parallel),
                        ("curved", LineLabelMode::Curved),
                        ("horizontal", LineLabelMode::Horizontal),
                        ("contour", LineLabelMode::Contour),
                    ],
                )?)
            }
            "area" if engine => {
                s.area = Some(named(
                    r,
                    &[
                        ("horizontal", AreaLabelMode::Horizontal),
                        ("free", AreaLabelMode::Free),
                        ("perimeter", AreaLabelMode::Perimeter),
                        ("boundary", AreaLabelMode::Boundary),
                        ("parcel", AreaLabelMode::Parcel),
                        ("corner", AreaLabelMode::Corner),
                    ],
                )?)
            }
            "position" if engine => {
                s.position = Some(named(
                    r,
                    &[
                        ("on", LabelPosition::On),
                        ("above", LabelPosition::Above),
                        ("below", LabelPosition::Below),
                        ("sides", LabelPosition::Sides),
                    ],
                )?)
            }
            "distance" if engine => s.distance = Some(r.float()?),
            "repeat" if engine => s.repeat = Some(r.float()?),
            "maxAngle" if engine => s.max_angle = Some(r.float()?),
            "curved" if engine => s.curved = Some(r.bool()?),
            "mergeLines" if engine => s.merge_lines = Some(r.bool()?),
            "inside" if engine => s.inside = Some(r.bool()?),
            "outside" if engine => s.outside = Some(r.bool()?),
            "halo" if engine => s.halo = Some(halo(r)?),
            "background" if engine => s.background = Some(background(r)?),
            "shadow" if engine => s.shadow = Some(shadow(r)?),
            "callout" if engine => s.callout = Some(callout(r)?),
            "stack" if engine => s.stack = Some(stack(r)?),
            "abbreviate" if engine => s.abbreviate = Some(abbreviate(r)?),
            "shrink" if engine => s.shrink = Some(r.float()?),
            "priority" if engine => s.priority = Some(r.uint(10)? as u8),
            "overlap" if engine => {
                s.overlap = Some(named(
                    r,
                    &[
                        ("never", LabelOverlap::Never),
                        ("ifNeeded", LabelOverlap::IfNeeded),
                        ("always", LabelOverlap::Always),
                    ],
                )?)
            }
            "duplicates" if engine => s.duplicates = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    s.placement = required(r, placement, "placement")?;
    s.size = required(r, size, "size")?;
    // The old fields keep the old readers' leniency; the engine's are checked whole.
    if engine && let Some(problem) = style_problem(&s) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(s)
}

fn halo(r: &mut Reader<'_>) -> Result<LabelHalo, KcadError> {
    let (mut width, mut color) = (None, None);
    map(r, |r, key| {
        match key {
            "color" => color = Some(text(r)?),
            "width" => width = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelHalo {
        width: required(r, width, "width")?,
        color,
    })
}

fn background(r: &mut Reader<'_>) -> Result<LabelBackground, KcadError> {
    let (mut shape, mut fill, mut stroke, mut padding) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "fill" => fill = Some(text(r)?),
            "shape" => {
                shape = Some(named(
                    r,
                    &[
                        ("rect", LabelShape::Rect),
                        ("round", LabelShape::Round),
                        ("ellipse", LabelShape::Ellipse),
                    ],
                )?)
            }
            "stroke" => stroke = Some(text(r)?),
            "padding" => padding = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelBackground {
        shape: required(r, shape, "shape")?,
        fill,
        stroke,
        padding,
    })
}

fn shadow(r: &mut Reader<'_>) -> Result<LabelShadow, KcadError> {
    let (mut dx, mut dy, mut color, mut opacity) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "dx" => dx = Some(r.float()?),
            "dy" => dy = Some(r.float()?),
            "color" => color = Some(text(r)?),
            "opacity" => opacity = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelShadow {
        dx: required(r, dx, "dx")?,
        dy: required(r, dy, "dy")?,
        color,
        opacity,
    })
}

fn callout(r: &mut Reader<'_>) -> Result<LabelCallout, KcadError> {
    let (mut kind, mut color, mut width, mut min_length) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                kind = Some(named(
                    r,
                    &[
                        ("straight", CalloutKind::Straight),
                        ("manhattan", CalloutKind::Manhattan),
                    ],
                )?)
            }
            "color" => color = Some(text(r)?),
            "width" => width = Some(r.float()?),
            "minLength" => min_length = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelCallout {
        kind: required(r, kind, "kind")?,
        color,
        width,
        min_length,
    })
}

fn stack(r: &mut Reader<'_>) -> Result<LabelStack, KcadError> {
    let (mut mode, mut chars, mut at) = (None, None, None);
    map(r, |r, key| {
        match key {
            "at" => at = Some(text(r)?),
            "mode" => {
                mode = Some(named(
                    r,
                    &[
                        ("ifNeeded", StackMode::IfNeeded),
                        ("always", StackMode::Always),
                    ],
                )?)
            }
            "chars" => chars = Some(r.uint(u64::from(u32::MAX))? as u32),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelStack {
        mode: required(r, mode, "mode")?,
        chars: required(r, chars, "chars")?,
        at,
    })
}

fn abbreviate(r: &mut Reader<'_>) -> Result<LabelAbbreviate, KcadError> {
    let (mut always, mut words) = (None, None);
    map(r, |r, key| {
        match key {
            "words" => {
                words = Some(list(r, |r, _| {
                    let (mut word, mut short) = (None, None);
                    map(r, |r, key| {
                        match key {
                            "word" => word = Some(text(r)?),
                            "short" => short = Some(text(r)?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(LabelWord {
                        word: required(r, word, "word")?,
                        short: required(r, short, "short")?,
                    })
                })?)
            }
            "always" => always = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LabelAbbreviate {
        always,
        words: required(r, words, "words")?,
    })
}

/// A layer style's `labels` (schema 36).
pub(super) fn layer_labels(r: &mut Reader<'_>) -> Result<LayerLabels, KcadError> {
    let at = r.position();
    let (mut mode, mut classes, mut obstacle) = (None, None, None);
    map(r, |r, key| {
        match key {
            "mode" => {
                mode = Some(named(
                    r,
                    &[
                        ("single", LabelsMode::Single),
                        ("rules", LabelsMode::Rules),
                        ("off", LabelsMode::Off),
                    ],
                )?)
            }
            "classes" => {
                let list_at = r.position();
                let all = list(r, |r, _| {
                    let (mut name, mut when, mut style) = (None, None, None);
                    map(r, |r, key| {
                        match key {
                            "name" => name = Some(text(r)?),
                            "when" => when = Some(text(r)?),
                            "style" => style = Some(label_style(r, true)?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(LabelClass {
                        name: required(r, name, "name")?,
                        when,
                        style: required(r, style, "style")?,
                    })
                })?;
                // An empty list is not written (the field is left out).
                if all.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        list_at,
                        "sınıf listesi boş; boş liste yazılmaz",
                    ));
                }
                classes = Some(all);
            }
            "obstacle" => {
                let (mut weight, mut kind) = (None, None);
                map(r, |r, key| {
                    match key {
                        "kind" => {
                            kind = Some(named(
                                r,
                                &[
                                    ("interior", ObstacleKind::Interior),
                                    ("boundary", ObstacleKind::Boundary),
                                ],
                            )?)
                        }
                        "weight" => weight = Some(r.uint(u64::from(u8::MAX))? as u8),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                obstacle = Some(LabelObstacle {
                    weight: required(r, weight, "weight")?,
                    kind,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let labels = LayerLabels {
        mode: required(r, mode, "mode")?,
        classes: classes.unwrap_or_default(),
        obstacle,
    };
    match layer_labels_problem(&labels) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(labels),
    }
}

/// An object's `labelPins` (schema 36).
pub(super) fn label_pins(r: &mut Reader<'_>) -> Result<Vec<LabelPin>, KcadError> {
    let at = r.position();
    let pins = list(r, |r, _| {
        let mut p = LabelPin::default();
        map(r, |r, key| {
            match key {
                "at" => p.at = Some(point(r)?),
                "class" => p.class = Some(text(r)?),
                "hidden" => p.hidden = Some(r.bool()?),
                "rotation" => p.rotation = Some(r.float()?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(p)
    })?;
    if pins.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "etiket iğneleri boş; boş liste yazılmaz",
        ));
    }
    match pins_problem(&pins) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(pins),
    }
}
