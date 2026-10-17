//! Annotative objects (docs/adr/0323): a text, a dimension or a leader drawn
//! at the paper height it was given at every scale it takes part in. Its own
//! geometry is its representation at `at` (the project's plot scale when it
//! was made annotative, or when the plot scale last changed); at another of
//! its `scales` it is drawn `scale / at` times as high, with what that scale
//! sets apart (a text's or a note's place and direction, a dimension's line
//! and value); at a scale it does not take part in it is not drawn. A sheet's
//! map draws each object as its own scale says (`at_scale`); the drawing area
//! draws the object itself. Overrides are absolute values, so a map's
//! representation changes only when its own scale's entry does.
//!
//! The independent reference is `scripts/fixtures/annotative_cases.py`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{Entity, TextShift, Vec2};

/// The most scales an object takes part in besides its own.
pub const MAX_ANNOTATIVE_SCALES: usize = 32;
/// The largest scale's denominator (1:1 000 000).
pub const MAX_ANNOTATIVE_SCALE: f64 = 1.0e6;

/// An object's scales (docs/adr/0323 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Annotative {
    /// The scale (1:at) its own geometry is drawn at.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub at: f64,
    /// The other scales it takes part in, ascending, each with what it sets apart.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AnnotativeScale>>", optional))]
    pub scales: Vec<AnnotativeScale>,
}

/// One of an object's other scales and what it sets apart from the object's
/// own values; a field not given is the object's own (`textShift` scaled with
/// the height).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotativeScale {
    /// 1:scale.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: f64,
    /// A text's point; a leader's landing start (its last point: the points after the arrow's tip move with it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub point: Option<Vec2>,
    /// A text's or a leader's note's direction, degrees counter-clockwise from east.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: Option<f64>,
    /// A dimension's line offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    /// A dimension's value moved from its place (0 and 0: in its place).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_shift: Option<TextShift>,
}

impl AnnotativeScale {
    /// A scale that sets nothing apart.
    pub fn plain(scale: f64) -> Self {
        Self {
            scale,
            point: None,
            rotation: None,
            offset: None,
            text_shift: None,
        }
    }

    /// Whether it sets nothing apart.
    pub fn is_plain(&self) -> bool {
        self.point.is_none()
            && self.rotation.is_none()
            && self.offset.is_none()
            && self.text_shift.is_none()
    }
}

impl Annotative {
    /// Whether it takes part in `scale`.
    pub fn takes(&self, scale: f64) -> bool {
        same_scale(self.at, scale) || self.scales.iter().any(|s| same_scale(s.scale, scale))
    }

    /// Its entry for another scale.
    pub fn entry(&self, scale: f64) -> Option<&AnnotativeScale> {
        self.scales.iter().find(|s| same_scale(s.scale, scale))
    }

    /// Every scale it takes part in, ascending.
    pub fn every_scale(&self) -> Vec<f64> {
        let mut all: Vec<f64> = self.scales.iter().map(|s| s.scale).collect();
        all.push(self.at);
        all.sort_by(f64::total_cmp);
        all
    }
}

/// Whether two scales are the same: equal within a billionth (a scale typed
/// as 100 and one computed as 1000/10).
pub fn same_scale(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
}

/// Whether `scale` may be a scale: finite, over 0, at most `MAX_ANNOTATIVE_SCALE`.
pub fn annotative_scale_holds(scale: f64) -> bool {
    scale.is_finite() && scale > 0.0 && scale <= MAX_ANNOTATIVE_SCALE
}

/// An object's scales, if it is annotative.
pub fn annotative_of(e: &Entity) -> Option<&Annotative> {
    match e {
        Entity::Text(t) => t.annotative.as_deref(),
        Entity::Dimension(d) => d.annotative.as_deref(),
        Entity::Leader(l) => l.annotative.as_deref(),
        Entity::Hatch(h) => h.annotative.as_deref(),
        _ => None,
    }
}

/// The field an object keeps its scales in; none for a kind that has none.
pub fn annotative_field(e: &mut Entity) -> Option<&mut Option<Box<Annotative>>> {
    match e {
        Entity::Text(t) => Some(&mut t.annotative),
        Entity::Dimension(d) => Some(&mut d.annotative),
        Entity::Leader(l) => Some(&mut l.annotative),
        Entity::Hatch(h) => Some(&mut h.annotative),
        _ => None,
    }
}

/// Why an object cannot be annotative; none when it can (docs/adr/0323 §1):
/// texts, dimensions and leaders, but a text along a curve (its curve is drawn
/// in its own frame at its height) and a text linked to an object (it follows
/// its own label scale, docs/adr/0175).
pub fn why_not_annotative(e: &Entity) -> Option<&'static str> {
    match e {
        Entity::Text(t) if t.path.is_some() => {
            Some("Eğri boyunca yazı ölçekli olamaz: harfleri eğrisinin üstündedir.")
        }
        Entity::Text(t) if t.label_of.is_some() => {
            Some("Nesneye bağlı yazı ölçekli olamaz: kendi etiket ölçeğini izler; önce Bağı kopar.")
        }
        Entity::Hatch(h)
            if matches!(
                h.pattern.kind,
                crate::HatchPatternType::Solid | crate::HatchPatternType::Gradient
            ) =>
        {
            Some("Dolu ve degrade taramanın ölçeklenecek deseni yok; yalnız çizgili ve desenli tarama ölçekli olur.")
        }
        Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_) | Entity::Hatch(_) => None,
        _ => Some("Yalnız yazı, ölçü, kılavuz ve tarama ölçekli olur."),
    }
}

/// What is wrong with an object's scales; none when they hold (or it has none).
pub fn annotative_problem(e: &Entity) -> Option<String> {
    let a = annotative_of(e)?;
    if let Some(why) = why_not_annotative(e) {
        return Some(why.to_owned());
    }
    if !annotative_scale_holds(a.at) {
        return Some(format!(
            "Ölçekli nesnenin kendi ölçeği 1:{} olamaz: sıfırdan büyük, en çok 1:{MAX_ANNOTATIVE_SCALE} olmalı.",
            a.at
        ));
    }
    if a.scales.len() > MAX_ANNOTATIVE_SCALES {
        return Some(format!(
            "Ölçekli nesne kendi ölçeğinden başka en çok {MAX_ANNOTATIVE_SCALES} ölçekte olur; {} verildi.",
            a.scales.len()
        ));
    }
    let mut last: Option<f64> = None;
    for s in &a.scales {
        if !annotative_scale_holds(s.scale) {
            return Some(format!(
                "Ölçek 1:{} olamaz: sıfırdan büyük, en çok 1:{MAX_ANNOTATIVE_SCALE} olmalı.",
                s.scale
            ));
        }
        if same_scale(s.scale, a.at) {
            return Some(format!(
                "1:{} nesnenin kendi ölçeği; diğer ölçeklerde yinelenmez.",
                s.scale
            ));
        }
        if last.is_some_and(|l| s.scale <= l || same_scale(s.scale, l)) {
            return Some("Ölçekler küçükten büyüğe ve birer kez yazılır.".to_owned());
        }
        last = Some(s.scale);
        if let Some(why) = entry_problem(e, s) {
            return Some(why);
        }
    }
    None
}

/// What is wrong with one scale's overrides for `e`'s kind.
pub fn entry_problem(e: &Entity, s: &AnnotativeScale) -> Option<String> {
    let finite = s.point.is_none_or(|p| p.x.is_finite() && p.y.is_finite())
        && s.rotation.is_none_or(f64::is_finite)
        && s.offset.is_none_or(f64::is_finite)
        && s.text_shift
            .is_none_or(|t| t.along.is_finite() && t.across.is_finite());
    if !finite {
        return Some(format!(
            "1:{} ölçeğinin değerleri sonlu sayılar olmalı.",
            s.scale
        ));
    }
    let dimension = matches!(e, Entity::Dimension(_));
    if dimension && (s.point.is_some() || s.rotation.is_some()) {
        return Some(
            "Ölçünün ölçeği yalnız çizgisinin uzaklığını (offset) ve değerinin yerini (textShift) değiştirir.".to_owned(),
        );
    }
    if !dimension && (s.offset.is_some() || s.text_shift.is_some()) {
        return Some(
            "Yazının ve kılavuzun ölçeği yalnız yerini (point) ve doğrultusunu (rotation) değiştirir.".to_owned(),
        );
    }
    None
}

/// How an object is drawn at a scale.
#[derive(Clone, Debug, PartialEq)]
pub enum AtScale {
    /// As it is: not annotative, or the scale is its own.
    Own,
    /// Not at all: it does not take part in the scale.
    Hidden,
    /// As this.
    Drawn(Box<Entity>),
}

/// How `e` is drawn at 1:`scale` (docs/adr/0323 §2): scaled by `scale / at`
/// about its own place (a text's height and box, a dimension's height and
/// its value's shift, a leader's note), with what the scale sets apart.
pub fn at_scale(e: &Entity, scale: f64) -> AtScale {
    let Some(a) = annotative_of(e) else {
        return AtScale::Own;
    };
    if same_scale(a.at, scale) {
        return AtScale::Own;
    }
    let Some(entry) = a.entry(scale) else {
        return AtScale::Hidden;
    };
    AtScale::Drawn(Box::new(represented(e, a.at, entry)))
}

/// `e` (its own at `at`) drawn at `entry`'s scale.
fn represented(e: &Entity, at: f64, entry: &AnnotativeScale) -> Entity {
    let f = entry.scale / at;
    match e {
        Entity::Text(t) => {
            let mut out = t.clone();
            out.height = t.height * f;
            out.paragraph.box_width = t.paragraph.box_width.map(|w| w * f);
            out.p = entry.point.unwrap_or(t.p);
            out.rotation = entry.rotation.unwrap_or(t.rotation);
            Entity::Text(out)
        }
        Entity::Leader(l) => {
            let mut out = l.clone();
            out.height = l.height * f;
            if let (Some(p), Some(last)) = (entry.point, l.pts.last()) {
                let (dx, dy) = (p.x - last.x, p.y - last.y);
                for q in out.pts.iter_mut().skip(1) {
                    q.x += dx;
                    q.y += dy;
                }
            }
            out.rotation = entry.rotation.unwrap_or(l.rotation);
            Entity::Leader(out)
        }
        Entity::Dimension(d) => {
            let mut out = d.clone();
            out.height = d.height * f;
            out.offset = entry.offset.unwrap_or(d.offset);
            out.text_shift = match entry.text_shift {
                Some(t) if t.along == 0.0 && t.across == 0.0 => None,
                Some(t) => Some(t),
                None => d.text_shift.map(|t| TextShift {
                    along: t.along * f,
                    across: t.across * f,
                }),
            };
            Entity::Dimension(out)
        }
        // A hatch's pattern as large on paper at the scale as at its own: its lines' spacing and its families' scale
        // times the ratio; its region and its origin stay (docs/adr/0331 §3).
        Entity::Hatch(h) => {
            let mut out = h.clone();
            match out.pattern.kind {
                crate::HatchPatternType::Lines | crate::HatchPatternType::Cross => {
                    out.pattern.spacing = h.pattern.spacing * f;
                }
                crate::HatchPatternType::Pattern => {
                    out.pattern.scale = h.pattern.scale.map(|s| s * f);
                }
                _ => {}
            }
            Entity::Hatch(out)
        }
        other => other.clone(),
    }
}

/// The values a scale may set apart, as an object is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Values {
    point: Option<Vec2>,
    rotation: Option<f64>,
    offset: Option<f64>,
    shift: Option<TextShift>,
}

fn values(e: &Entity) -> Values {
    let none = Values {
        point: None,
        rotation: None,
        offset: None,
        shift: None,
    };
    match e {
        Entity::Text(t) => Values {
            point: Some(t.p),
            rotation: Some(t.rotation),
            ..none
        },
        Entity::Leader(l) => Values {
            point: l.pts.last().copied(),
            rotation: Some(l.rotation),
            ..none
        },
        Entity::Dimension(d) => Values {
            offset: Some(d.offset),
            shift: Some(d.text_shift.unwrap_or(TextShift {
                along: 0.0,
                across: 0.0,
            })),
            ..none
        },
        _ => none,
    }
}

/// Equal within a billionth of the larger, or both within a nanometre of 0.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// The entry for `scale` that draws `base` (its own at `at`) as `wanted` is drawn: the fields
/// that differ, as overrides.
fn entry_for(base: &Entity, at: f64, scale: f64, wanted: &Entity) -> AnnotativeScale {
    let plain = AnnotativeScale::plain(scale);
    let shown = values(&represented(base, at, &plain));
    let want = values(wanted);
    AnnotativeScale {
        scale,
        point: match (shown.point, want.point) {
            (Some(s), Some(w)) if !(near(s.x, w.x) && near(s.y, w.y)) => Some(w),
            _ => None,
        },
        rotation: match (shown.rotation, want.rotation) {
            (Some(s), Some(w)) if !near(s, w) => Some(w),
            _ => None,
        },
        offset: match (shown.offset, want.offset) {
            (Some(s), Some(w)) if !near(s, w) => Some(w),
            _ => None,
        },
        text_shift: match (shown.shift, want.shift) {
            (Some(s), Some(w)) if !(near(s.along, w.along) && near(s.across, w.across)) => Some(w),
            _ => None,
        },
    }
}

/// `e` with its own geometry the representation at 1:`scale` (docs/adr/0323 §3): the scale is
/// added when it was not one of its; every other scale, its old own one too, keeps drawing as it
/// did. None when `e` is not annotative or `scale` is its own already.
pub fn rebase_annotative(e: &Entity, scale: f64) -> Option<Entity> {
    let a = annotative_of(e)?;
    if same_scale(a.at, scale) {
        return None;
    }
    let plain = AnnotativeScale::plain(scale);
    let mut base = represented(e, a.at, a.entry(scale).unwrap_or(&plain));
    // Every scale it takes part in, drawn as before.
    let mut wanted: Vec<(f64, Entity)> = vec![(a.at, e.clone())];
    for s in &a.scales {
        if !same_scale(s.scale, scale) {
            wanted.push((s.scale, represented(e, a.at, s)));
        }
    }
    let mut scales: Vec<AnnotativeScale> = wanted
        .iter()
        .map(|(s, w)| entry_for(&base, scale, *s, w))
        .collect();
    scales.sort_by(|x, y| x.scale.total_cmp(&y.scale));
    if let Some(field) = annotative_field(&mut base) {
        *field = Some(Box::new(Annotative { at: scale, scales }));
    }
    Some(base)
}

/// `e` taking part in `scale` too; none when it does already or is not annotative.
pub fn with_scale(e: &Entity, scale: f64) -> Option<Entity> {
    let a = annotative_of(e)?;
    if a.takes(scale) {
        return None;
    }
    let mut scales = a.scales.clone();
    scales.push(AnnotativeScale::plain(scale));
    scales.sort_by(|x, y| x.scale.total_cmp(&y.scale));
    let mut out = e.clone();
    *annotative_field(&mut out)? = Some(Box::new(Annotative { at: a.at, scales }));
    Some(out)
}

/// `e` without another scale; none when it is not one of its other scales.
pub fn without_scale(e: &Entity, scale: f64) -> Option<Entity> {
    let a = annotative_of(e)?;
    a.entry(scale)?;
    let scales = a
        .scales
        .iter()
        .filter(|s| !same_scale(s.scale, scale))
        .cloned()
        .collect();
    let mut out = e.clone();
    *annotative_field(&mut out)? = Some(Box::new(Annotative { at: a.at, scales }));
    Some(out)
}
