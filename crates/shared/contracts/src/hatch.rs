//! Hatches' patterns and their tie to their boundary (docs/adr/0186): solid,
//! user-defined lines and crossed lines, a pattern of line families (the
//! library's or a file's, its definition carried with it), a gradient; and,
//! for a hatch made inside a closed object, the objects its region follows.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{EntityId, Vec2};

/// The most line families a pattern may have, and dashes a family.
pub const MAX_PATTERN_LINES: usize = 64;
pub const MAX_PATTERN_DASHES: usize = 16;
/// The longest a pattern's name may be.
pub const MAX_PATTERN_NAME: usize = 64;
/// The largest number a pattern's definition may hold, and its scale.
pub const MAX_PATTERN_SIZE: f64 = 1.0e6;
/// The most objects a hatch's tie may name.
pub const MAX_ASSOC_OBJECTS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchPatternType {
    Solid,
    /// User-defined lines at `angle`, `spacing` apart.
    Lines,
    /// User-defined lines and the same turned 90°.
    Cross,
    /// A pattern of line families (`name`, `scale`, `lines`).
    Pattern,
    /// A gradient from the hatch's colour to `gradient.color2`.
    Gradient,
}

/// One family of a pattern's lines (docs/adr/0186 §1), in the pattern's
/// units, before the pattern turns: lines at `angle` (degrees) through
/// `origin`, each the next `offset` on (`[along, across]` the line), drawn
/// as `dashes` say (plus drawn, minus a gap, 0 a dot; none: whole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternLine {
    pub angle: f64,
    pub origin: [f64; 2],
    pub offset: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    pub dashes: Vec<f64>,
}

/// How a gradient runs over its hatch (docs/adr/0186 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GradientShape {
    /// From one side of the hatch to the other along `angle`.
    Linear,
    /// The second colour along the middle, the first at both sides.
    Cylinder,
    /// The second colour in the middle, the first at the farthest corner.
    Spherical,
}

/// A gradient: its shape, whether it runs the other way, its second colour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchGradient {
    pub shape: GradientShape,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub inverted: bool,
    /// `#RRGGBB`.
    pub color2: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPattern {
    #[serde(rename = "type")]
    pub kind: HatchPatternType,
    /// Degrees, counter-clockwise from east: the lines' (lines, cross), the
    /// pattern's turn (pattern), the gradient's direction (gradient).
    pub angle: f64,
    /// Metres between the lines (lines, cross); 1 and unread for the others.
    pub spacing: f64,
    /// A pattern's name (`ANSI31`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// A pattern's metres per unit of its definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scale: Option<f64>,
    /// A pattern's line families.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lines: Option<Vec<PatternLine>>,
    /// A gradient's shape and second colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gradient: Option<HatchGradient>,
}

/// The objects a hatch made inside a closed object follows (docs/adr/0186
/// §6): the closed object, its islands, the texts and inserts left open, and
/// the point clicked inside, which picks the region's part.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchAssoc {
    pub outer: EntityId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub islands: Vec<EntityId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub cutouts: Vec<EntityId>,
    pub seed: Vec2,
}

fn finite_all(xs: &[f64]) -> bool {
    xs.iter()
        .all(|x| x.is_finite() && x.abs() <= MAX_PATTERN_SIZE)
}

/// Whether `color` is `#RRGGBB`.
pub fn is_hex_colour(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl HatchPattern {
    /// A user-defined pattern: solid, lines or crossed lines.
    pub fn user(kind: HatchPatternType, angle: f64, spacing: f64) -> Self {
        Self {
            kind,
            angle,
            spacing,
            name: None,
            scale: None,
            lines: None,
            gradient: None,
        }
    }

    /// Whether it is more than the first three kinds wrote: a pattern, a
    /// gradient or one of their fields (`.kcad` schema 23).
    pub fn has_definition(&self) -> bool {
        matches!(
            self.kind,
            HatchPatternType::Pattern | HatchPatternType::Gradient
        ) || self.name.is_some()
            || self.scale.is_some()
            || self.lines.is_some()
            || self.gradient.is_some()
    }

    /// What is wrong with it (docs/adr/0186 §1): the field and the refusal's
    /// words; none when it may be written. Each kind its own fields only: a
    /// pattern's name, scale and families within their bounds, each family's
    /// lines apart; a gradient's second colour.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        let kind = self.kind;
        let pattern = kind == HatchPatternType::Pattern;
        if !pattern && (self.name.is_some() || self.scale.is_some() || self.lines.is_some()) {
            return Some((
                "pattern",
                "Yalnız desen türündeki taramanın adı, ölçeği ve çizgi aileleri olur.".to_owned(),
            ));
        }
        if kind != HatchPatternType::Gradient && self.gradient.is_some() {
            return Some((
                "pattern.gradient",
                "Yalnız degrade taramanın ikinci rengi ve biçimi olur.".to_owned(),
            ));
        }
        if matches!(kind, HatchPatternType::Lines | HatchPatternType::Cross)
            && !(self.spacing > 0.0 && self.spacing <= MAX_PATTERN_SIZE)
        {
            return Some((
                "pattern.spacing",
                format!(
                    "Taramanın çizgi aralığı {}; sıfırdan büyük ve sonlu olmalı.",
                    self.spacing
                ),
            ));
        }
        if pattern {
            let name = self.name.as_deref().unwrap_or("");
            if name.trim().is_empty() || name.chars().count() > MAX_PATTERN_NAME {
                return Some((
                    "pattern.name",
                    format!("Desenin adı boş olamaz ve en çok {MAX_PATTERN_NAME} harf olabilir."),
                ));
            }
            let scale = self.scale.unwrap_or(f64::NAN);
            if !(scale > 0.0 && scale <= MAX_PATTERN_SIZE) {
                return Some((
                    "pattern.scale",
                    format!("Desenin ölçeği {scale}; sıfırdan büyük ve sonlu olmalı."),
                ));
            }
            let lines = self.lines.as_deref().unwrap_or_default();
            if lines.is_empty() || lines.len() > MAX_PATTERN_LINES {
                return Some((
                    "pattern.lines",
                    format!(
                        "Desenin {} çizgi ailesi var; en az 1, en çok {MAX_PATTERN_LINES} olmalı.",
                        lines.len()
                    ),
                ));
            }
            for (k, l) in lines.iter().enumerate() {
                let n = k + 1;
                if !finite_all(&[l.angle, l.origin[0], l.origin[1], l.offset[0], l.offset[1]])
                    || !finite_all(&l.dashes)
                {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinde sonlu olmayan ya da çok büyük bir sayı var."
                        ),
                    ));
                }
                if l.offset[1] == 0.0 {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinin çizgileri arası 0; aileler sıfırdan büyük aralıklı olmalı."
                        ),
                    ));
                }
                if l.dashes.len() > MAX_PATTERN_DASHES {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinde {} kesik var; en çok {MAX_PATTERN_DASHES} olmalı.",
                            l.dashes.len()
                        ),
                    ));
                }
                if !l.dashes.is_empty() && l.dashes.iter().all(|d| *d == 0.0) {
                    return Some((
                        "pattern.lines",
                        format!(
                            "Desenin {n}. çizgi ailesinin kesiklerinin hepsi 0; en az birinin uzunluğu olmalı."
                        ),
                    ));
                }
            }
        }
        if kind == HatchPatternType::Gradient {
            let Some(g) = &self.gradient else {
                return Some((
                    "pattern.gradient",
                    "Degrade taramanın ikinci rengi ve biçimi verilmeli.".to_owned(),
                ));
            };
            if !is_hex_colour(&g.color2) {
                return Some((
                    "pattern.gradient.color2",
                    format!(
                        "Degradenin ikinci rengi “{}”; #RRGGBB biçiminde olmalı.",
                        g.color2
                    ),
                ));
            }
        }
        None
    }
}

impl HatchAssoc {
    /// What is wrong with it: its seed finite, its lists within their bounds,
    /// no object named twice.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if !(self.seed.x.is_finite() && self.seed.y.is_finite()) {
            return Some((
                "assoc.seed",
                "Taramanın tohum noktası sonlu olmalı.".to_owned(),
            ));
        }
        let n = self.islands.len() + self.cutouts.len();
        if n > MAX_ASSOC_OBJECTS {
            return Some((
                "assoc",
                format!(
                    "Taramanın ilişkisi {n} nesne gösteriyor; en çok {MAX_ASSOC_OBJECTS} olmalı."
                ),
            ));
        }
        let mut seen = std::collections::HashSet::with_capacity(n + 1);
        seen.insert(self.outer);
        for id in self.islands.iter().chain(&self.cutouts) {
            if !seen.insert(*id) {
                return Some((
                    "assoc",
                    "Taramanın ilişkisi bir nesneyi iki kez gösteriyor; her nesne bir kez gösterilmeli."
                        .to_owned(),
                ));
            }
        }
        None
    }
}
