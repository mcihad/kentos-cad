//! A label class as the engine reads it (docs/adr/0212 §2): the contract's
//! `LabelStyle` (and a layer's `LayerLabels`) from their JSON, every value
//! the engine places by resolved to its default. Colours, the halo's and
//! the shadow's look stay with the hosts (they draw); the engine keeps what
//! decides where a label goes and how much room it takes.

use crate::api::json::Json;

/// Where a point's label goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointMode {
    Around,
    Center,
    Corner,
}

/// Where a line's label goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineMode {
    Parallel,
    Curved,
    Horizontal,
    Contour,
    Corner,
}

/// Where an area's label goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaMode {
    Horizontal,
    Free,
    Perimeter,
    Boundary,
    Parcel,
    Corner,
}

/// A line label's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    On,
    Above,
    Below,
    Sides,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlap {
    Never,
    IfNeeded,
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Cutting a label into lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub always: bool,
    pub chars: usize,
    /// The letters a line may end at.
    pub at: Vec<char>,
}

/// Shortening a label's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Abbreviate {
    pub always: bool,
    pub words: Vec<(String, String)>,
}

/// What one class decides (sizes CSS px, scales px per metre, angles radians).
#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    /// A rule's name; none for a single label.
    pub name: Option<String>,
    pub size: f64,
    pub grow: f64,
    pub max_size: Option<f64>,
    /// Weight 600 and over: the bold advances.
    pub bold: bool,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
    pub min_feature_px: Option<f64>,
    pub point: PointMode,
    pub line: LineMode,
    pub area: AreaMode,
    pub position: Position,
    pub distance: f64,
    pub repeat: Option<f64>,
    pub max_angle: f64,
    pub curved: bool,
    pub merge_lines: bool,
    pub inside: bool,
    pub outside: bool,
    /// The halo's width, px.
    pub halo: f64,
    /// A background's padding, px; none without a background.
    pub background: Option<f64>,
    /// A contour's mask under its label (no background of its own).
    pub mask: bool,
    pub stack: Option<Stack>,
    pub abbreviate: Option<Abbreviate>,
    pub shrink: f64,
    pub priority: u8,
    pub overlap: Overlap,
    pub duplicates: Option<f64>,
    /// A callout's shortest line, px; none without callouts.
    pub callout: Option<f64>,
    pub align: Align,
}

/// The halo a label has when its style names none, px.
pub const HALO: f64 = 1.5;
/// A background's padding when its style names none, px.
pub const PADDING: f64 = 2.0;
/// The room between two labels, px.
pub const SPACING: f64 = 1.0;
/// A line's height over the font size.
pub const LINE_HEIGHT: f64 = 1.2;
/// A contour's repetition when its style names none, px.
pub const CONTOUR_REPEAT: f64 = 400.0;
/// A boundary's repetition when its style names none, px.
pub const BOUNDARY_REPEAT: f64 = 300.0;
/// A callout's shortest line when its style names none, px.
pub const CALLOUT_MIN: f64 = 6.0;
/// The angle two neighbouring letters may turn when the style names none, degrees.
pub const MAX_ANGLE: f64 = 25.0;
/// The distance between a label and its object when the style names none, px.
pub const DISTANCE: f64 = 2.0;

impl Class {
    /// Its size at `scale` px per metre, before any shrinking.
    pub fn size_at(&self, scale: f64) -> f64 {
        let grown = self.size + self.grow * scale;
        match self.max_size {
            Some(m) if m < grown => m,
            _ => grown,
        }
    }

    /// Whether `scale` is inside its range.
    pub fn shown_at(&self, scale: f64) -> bool {
        !(self.min_scale.is_some_and(|m| scale < m) || self.max_scale.is_some_and(|m| scale > m))
    }

    /// The room round its letters a box takes: its halo or its background, and the spacing.
    pub fn pad(&self) -> f64 {
        let around = match self.background {
            Some(p) if p > self.halo => p,
            _ => self.halo,
        };
        around + SPACING
    }
}

/// How a layer is labelled, as the engine reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Labelling {
    /// Off: no labels (the obstacle may stay).
    pub off: bool,
    /// The classes in order: one for a single label (none: the kinds' defaults), the rules' for rules.
    pub classes: Vec<Class>,
    /// Its objects as obstacles: weight 1–10 and whether only an area's outline is.
    pub obstacle: Option<(u8, bool)>,
}

fn num(v: &Json, k: &str) -> Result<Option<f64>, String> {
    match v.get(k) {
        Json::Null => Ok(None),
        Json::Num(x) if x.is_finite() => Ok(Some(*x)),
        _ => Err(format!("{k} sayı olmalı")),
    }
}

fn flag(v: &Json, k: &str) -> Result<bool, String> {
    match v.get(k) {
        Json::Null => Ok(false),
        Json::Bool(b) => Ok(*b),
        _ => Err(format!("{k} doğru ya da yanlış olmalı")),
    }
}

fn name<'a>(v: &'a Json, k: &str) -> Result<Option<&'a str>, String> {
    match v.get(k) {
        Json::Null => Ok(None),
        Json::Str(s) => Ok(Some(s)),
        _ => Err(format!("{k} metin olmalı")),
    }
}

fn pick<T: Copy>(v: &Json, k: &str, of: &[(&str, T)]) -> Result<Option<T>, String> {
    match name(v, k)? {
        None => Ok(None),
        Some(s) => of
            .iter()
            .find(|(n, _)| *n == s)
            .map(|(_, t)| Some(*t))
            .ok_or_else(|| format!("{k}: “{s}” bilinmiyor")),
    }
}

/// A class from a `LabelStyle`'s JSON; `rule` its rule's name.
pub fn read_class(v: &Json, rule: Option<String>) -> Result<Class, String> {
    let placement = name(v, "placement")?.unwrap_or("center");
    // The old placements are the kinds' modes (docs/adr/0212 §3.3).
    let (point, line, area) = match placement {
        "center" => (
            PointMode::Center,
            LineMode::Horizontal,
            AreaMode::Horizontal,
        ),
        "corner" => (PointMode::Corner, LineMode::Corner, AreaMode::Corner),
        "beside" => (
            PointMode::Around,
            LineMode::Horizontal,
            AreaMode::Horizontal,
        ),
        "along" => (PointMode::Around, LineMode::Parallel, AreaMode::Perimeter),
        s => return Err(format!("placement: “{s}” bilinmiyor")),
    };
    let point = pick(
        v,
        "point",
        &[("around", PointMode::Around), ("center", PointMode::Center)],
    )?
    .unwrap_or(point);
    let line = pick(
        v,
        "line",
        &[
            ("parallel", LineMode::Parallel),
            ("curved", LineMode::Curved),
            ("horizontal", LineMode::Horizontal),
            ("contour", LineMode::Contour),
        ],
    )?
    .unwrap_or(line);
    let area = pick(
        v,
        "area",
        &[
            ("horizontal", AreaMode::Horizontal),
            ("free", AreaMode::Free),
            ("perimeter", AreaMode::Perimeter),
            ("boundary", AreaMode::Boundary),
            ("parcel", AreaMode::Parcel),
            ("corner", AreaMode::Corner),
        ],
    )?
    .unwrap_or(area);
    let position = pick(
        v,
        "position",
        &[
            ("on", Position::On),
            ("above", Position::Above),
            ("below", Position::Below),
            ("sides", Position::Sides),
        ],
    )?
    .unwrap_or(match area {
        // An outline's labels go inside unless the style says otherwise.
        AreaMode::Boundary => Position::Above,
        _ => Position::On,
    });
    let size = num(v, "size")?.ok_or("size gerekli")?;
    let halo = match v.get("halo") {
        Json::Null => HALO,
        h => num(h, "width")?.ok_or("halo.width gerekli")?,
    };
    let background = match v.get("background") {
        Json::Null => None,
        b => Some(num(b, "padding")?.unwrap_or(PADDING)),
    };
    let stack = match v.get("stack") {
        Json::Null => None,
        k => Some(Stack {
            always: name(k, "mode")? == Some("always"),
            chars: num(k, "chars")?.ok_or("stack.chars gerekli")? as usize,
            at: name(k, "at")?.unwrap_or(" ").chars().collect(),
        }),
    };
    let abbreviate = match v.get("abbreviate") {
        Json::Null => None,
        a => {
            let Json::Arr(list) = a.get("words") else {
                return Err("abbreviate.words dizi olmalı".into());
            };
            let mut words = Vec::with_capacity(list.len());
            for w in list {
                let (Some(word), Some(short)) = (name(w, "word")?, name(w, "short")?) else {
                    return Err("kısaltmanın sözcüğü ve kısası gerekli".into());
                };
                words.push((word.to_owned(), short.to_owned()));
            }
            Some(Abbreviate {
                always: flag(a, "always")?,
                words,
            })
        }
    };
    let callout = match v.get("callout") {
        Json::Null => None,
        c => Some(num(c, "minLength")?.unwrap_or(CALLOUT_MIN)),
    };
    let priority = num(v, "priority")?.unwrap_or(5.0);
    Ok(Class {
        name: rule,
        size,
        grow: num(v, "grow")?.unwrap_or(0.0),
        max_size: num(v, "maxSize")?,
        bold: num(v, "weight")?.is_some_and(|w| w >= 600.0),
        min_scale: num(v, "minScale")?,
        max_scale: num(v, "maxScale")?,
        min_feature_px: num(v, "minFeaturePx")?,
        point,
        line,
        area,
        position,
        distance: num(v, "distance")?.unwrap_or(DISTANCE),
        repeat: num(v, "repeat")?.or(match (line, area) {
            (LineMode::Contour, _) => Some(CONTOUR_REPEAT),
            _ => None,
        }),
        max_angle: num(v, "maxAngle")?.unwrap_or(MAX_ANGLE).to_radians(),
        curved: flag(v, "curved")?,
        merge_lines: flag(v, "mergeLines")?,
        inside: flag(v, "inside")? || area == AreaMode::Parcel,
        outside: flag(v, "outside")?,
        halo,
        mask: line == LineMode::Contour && background.is_none(),
        background,
        stack,
        abbreviate,
        shrink: num(v, "shrink")?.unwrap_or(1.0),
        priority: priority.clamp(0.0, 10.0) as u8,
        overlap: pick(
            v,
            "overlap",
            &[
                ("never", Overlap::Never),
                ("ifNeeded", Overlap::IfNeeded),
                ("always", Overlap::Always),
            ],
        )?
        .unwrap_or(Overlap::Never),
        duplicates: num(v, "duplicates")?,
        callout,
        align: pick(
            v,
            "align",
            &[
                ("left", Align::Left),
                ("center", Align::Center),
                ("right", Align::Right),
            ],
        )?
        .unwrap_or(Align::Center),
    })
}

impl Class {
    /// A boundary's repetition: its own, else the boundary mode's.
    pub fn outline_repeat(&self) -> Option<f64> {
        match (self.repeat, self.area) {
            (Some(r), _) => Some(r),
            (None, AreaMode::Boundary) => Some(BOUNDARY_REPEAT),
            _ => None,
        }
    }
}

/// A layer's labelling from `{ label?, labels? }` (its style's two fields).
pub fn read_labelling(v: &Json) -> Result<Labelling, String> {
    let labels = v.get("labels");
    let mode = match labels {
        Json::Null => "single",
        l => name(l, "mode")?.ok_or("labels.mode gerekli")?,
    };
    let obstacle = match labels.get("obstacle") {
        Json::Null => None,
        o => Some((
            num(o, "weight")?
                .ok_or("obstacle.weight gerekli")?
                .clamp(1.0, 10.0) as u8,
            name(o, "kind")? == Some("boundary"),
        )),
    };
    let classes = match mode {
        "off" => Vec::new(),
        "single" => match v.get("label") {
            Json::Null => Vec::new(),
            s => vec![read_class(s, None).map_err(|e| format!("label: {e}"))?],
        },
        "rules" => {
            let Json::Arr(list) = labels.get("classes") else {
                return Err("labels.classes dizi olmalı".into());
            };
            let mut out = Vec::with_capacity(list.len());
            for (i, c) in list.iter().enumerate() {
                let rule = name(c, "name")?.unwrap_or_default().to_owned();
                out.push(
                    read_class(c.get("style"), Some(rule))
                        .map_err(|e| format!("classes[{i}]: {e}"))?,
                );
            }
            out
        }
        m => return Err(format!("labels.mode: “{m}” bilinmiyor")),
    };
    Ok(Labelling {
        off: mode == "off",
        classes,
        obstacle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_style_reads_as_its_kinds_modes() {
        let c = read_class(
            &Json::parse(r#"{"placement":"along","size":10,"minScale":1.6}"#).unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(
            (c.point, c.line, c.area),
            (PointMode::Around, LineMode::Parallel, AreaMode::Perimeter)
        );
        assert_eq!((c.halo, c.priority, c.overlap), (HALO, 5, Overlap::Never));
        assert_eq!(c.min_scale, Some(1.6));
    }

    #[test]
    fn a_contour_repeats_and_masks_and_a_parcel_stays_inside() {
        let c = read_class(
            &Json::parse(r#"{"placement":"along","size":9,"line":"contour","area":"parcel"}"#)
                .unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(c.repeat, Some(CONTOUR_REPEAT));
        assert!(c.mask && c.inside);
        let l = read_labelling(&Json::parse(r#"{"labels":{"mode":"rules","classes":[{"name":"Ada","when":"x = 1","style":{"placement":"center","size":12,"priority":8}}],"obstacle":{"weight":7,"kind":"boundary"}}}"#).unwrap()).unwrap();
        assert_eq!(l.classes[0].name.as_deref(), Some("Ada"));
        assert_eq!((l.classes[0].priority, l.obstacle), (8, Some((7, true))));
    }
}
