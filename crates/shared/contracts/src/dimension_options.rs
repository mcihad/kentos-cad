//! A dimension's options past its style's look (docs/adr/0321; plan/cad.md CAD-ANNO-08, CAD-ANNO-09): where its value
//! goes when it does not fit, how its number is written (an increment, zeros, feet and inches, an alternate unit, a
//! tolerance) and how wide its breaks are; and the dimension's own moved value and breaks. A style keeps the options as
//! a dimension does; the value's writing is `Format::dimension_value`, the layout's the core's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DrawingUnit, LengthStyle};

/// The most decimals a tolerance or an alternate unit is written with.
pub const MAX_OPTION_DECIMALS: u32 = 8;
/// The most breaks a dimension keeps.
pub const MAX_BREAKS: usize = 64;
/// The largest increment, tolerance or shift a dimension takes, metres.
pub const MAX_OPTION_LENGTH: f64 = 1.0e6;

/// Where the value goes when it does not fit between the extension lines (AutoCAD's DIMATFIT and DIMTIX): absent,
/// always between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionFit {
    /// Beyond the second extension line when the value and the two arrowheads are longer than the line between them.
    Auto,
    /// Always beyond the second extension line, the arrowheads outside.
    Outside,
}

/// Which zeros the number leaves out (AutoCAD's DIMZIN).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionZeros {
    /// The zero before the point: `0.50` → `.50`.
    Leading,
    /// The decimals' trailing zeros, and a bare point: `2.500` → `2.5`, `4.000` → `4`.
    Trailing,
    Both,
}

impl DimensionZeros {
    pub fn name(self) -> &'static str {
        match self {
            DimensionZeros::Leading => "leading",
            DimensionZeros::Trailing => "trailing",
            DimensionZeros::Both => "both",
        }
    }
}

/// An alternate unit written after the value in brackets (AutoCAD's DIMALT): `2.500 [2500]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AltUnits {
    pub unit: DrawingUnit,
    /// Its decimals; absent: the project's length decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
}

/// How a tolerance is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ToleranceKind {
    /// ` ±upper`.
    Symmetric,
    /// ` +upper -lower`.
    Deviation,
    /// The largest and the smallest value instead of the number: `max/min`.
    Limits,
}

/// A tolerance (AutoCAD's DIMTOL and DIMLIM), metres; lengths and coordinates only.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionTolerance {
    pub kind: ToleranceKind,
    /// Above the value, 0 or more.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub upper: f64,
    /// Below the value, 0 or more; a deviation's and limits' (absent: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub lower: Option<f64>,
    /// Its decimals; absent: the value's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
}

/// A dimension's options (docs/adr/0321 §1, §2): all absent, the dimension as it always was.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fit: Option<DimensionFit>,
    /// Lengths and coordinates rounded to this increment, metres, over 0 (AutoCAD's DIMRND).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub round: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zeros: Option<DimensionZeros>,
    /// Feet and inches in an inch or foot unit (AutoCAD's DIMLUNIT, docs/adr/0262 §2); absent: the project's, unless
    /// the dimension names its own unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub length_style: Option<LengthStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alt: Option<AltUnits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tolerance: Option<DimensionTolerance>,
    /// A break's width, times the value's height, over 0; absent: 1 (AutoCAD's DIMBREAK size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub break_size: Option<f64>,
}

/// A break's default width, times the value's height.
pub const DEFAULT_BREAK_SIZE: f64 = 1.0;

/// The value moved from its place (AutoCAD's text moved, DIMTMOVE 2): metres along the value's own direction and
/// across it (left positive).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextShift {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub along: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub across: f64,
}

/// The most a Kot bound to a level may stand off its level, metres (docs/adr/0721 §2).
pub const MAX_LEVEL_OFFSET: f64 = 1000.0;

/// A Kot (an elevation mark) bound to a level of the project (docs/adr/0721 §2): its value is the level's floor
/// elevation plus `offset` (a wet room's −0.02), written again whenever the level's elevation changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionLevel {
    /// The level, by its id among the project's levels.
    pub id: String,
    /// Metres added to the level's elevation; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: Option<f64>,
}

impl DimensionLevel {
    /// What is wrong with it, in a refusal's words; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if self.id.trim() != self.id || self.id.is_empty() || self.id.chars().any(char::is_control) {
            return Some("Kot işaretinin katı boş, kenarı boşluklu ya da denetim karakterli olamaz.".into());
        }
        if self.offset.is_some_and(|o| !(o.is_finite() && o.abs() <= MAX_LEVEL_OFFSET)) {
            return Some(format!(
                "Kot işaretinin kata göre farkı sonlu ve en çok {MAX_LEVEL_OFFSET} m olmalı."
            ));
        }
        None
    }

    /// Its value among `levels`: the level's elevation plus the offset; none when the project lacks the level.
    pub fn value(&self, levels: &[crate::ArchLevel]) -> Option<f64> {
        levels
            .iter()
            .find(|l| l.id == self.id)
            .map(|l| l.elevation + self.offset.unwrap_or(0.0))
    }
}

/// Which of a dimension's lines a break is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BreakLine {
    /// The dimension line, from its first end (0) to its second (1).
    Dimension,
    /// The first extension line, from the measured point (0) to its far end (1).
    First,
    /// The second.
    Second,
}

/// A gap in one of a dimension's lines where another object crosses it (AutoCAD's DIMBREAK), at the fraction `t` of
/// the line; its width the options' `breakSize`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionBreak {
    pub line: BreakLine,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub t: f64,
}

fn length_holds(v: f64, positive: bool) -> bool {
    v.is_finite() && v.abs() <= MAX_OPTION_LENGTH && if positive { v > 0.0 } else { v >= 0.0 }
}

impl DimensionOptions {
    /// Whether it says nothing (then it is not written).
    pub fn is_empty(&self) -> bool {
        *self == DimensionOptions::default()
    }

    /// What is wrong with it: the refusal's words; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if self.is_empty() {
            return Some("Ölçünün seçenekleri boş: hiçbiri yoksa alan yazılmaz.".into());
        }
        if let Some(r) = self.round
            && !length_holds(r, true)
        {
            return Some(format!(
                "Ölçünün yuvarlama artışı sıfırdan büyük, en çok {MAX_OPTION_LENGTH} m olmalı; {r} verildi."
            ));
        }
        if let Some(s) = self.break_size
            && !(s.is_finite() && s > 0.0 && s <= crate::MAX_DIMENSION_RATIO)
        {
            return Some(format!(
                "Ölçünün kırma genişliği değer yüksekliğinin katıdır: sıfırdan büyük, en çok {} olmalı; {s} verildi.",
                crate::MAX_DIMENSION_RATIO
            ));
        }
        if let Some(a) = self.alt
            && a.decimals.is_some_and(|d| d > MAX_OPTION_DECIMALS)
        {
            return Some(format!(
                "Alternatif birimin basamağı en çok {MAX_OPTION_DECIMALS} olmalı."
            ));
        }
        if let Some(t) = self.tolerance {
            if !length_holds(t.upper, false) || t.lower.is_some_and(|l| !length_holds(l, false)) {
                return Some(format!(
                    "Toleransın üst ve alt değeri 0 ya da büyük, en çok {MAX_OPTION_LENGTH} m olmalı."
                ));
            }
            if t.kind == ToleranceKind::Symmetric && t.lower.is_some() {
                return Some("Simetrik toleransın alt değeri yoktur (üstü iki yana); lower'ı kaldırın.".into());
            }
            if t.decimals.is_some_and(|d| d > MAX_OPTION_DECIMALS) {
                return Some(format!(
                    "Toleransın basamağı en çok {MAX_OPTION_DECIMALS} olmalı."
                ));
            }
        }
        None
    }
}

impl TextShift {
    pub fn problem(&self) -> Option<String> {
        (!(self.along.is_finite()
            && self.across.is_finite()
            && self.along.abs() <= MAX_OPTION_LENGTH
            && self.across.abs() <= MAX_OPTION_LENGTH))
            .then(|| format!("Değerin kayması sonlu ve en çok {MAX_OPTION_LENGTH} m olmalı."))
    }
}

/// What is wrong with a dimension's breaks; none when they hold.
pub fn breaks_problem(breaks: &[DimensionBreak]) -> Option<String> {
    if breaks.len() > MAX_BREAKS {
        return Some(format!(
            "Ölçünün {} kırması var; en çok {MAX_BREAKS} olur.",
            breaks.len()
        ));
    }
    breaks
        .iter()
        .any(|b| !(0.0..=1.0).contains(&b.t))
        .then(|| "Kırmanın yeri (t) 0 ile 1 arasında olmalı.".into())
}
