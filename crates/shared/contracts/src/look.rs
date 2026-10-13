//! An object's look beyond its colour and line weight (docs/adr/0260): its
//! own line type and the scale of its pattern, its transparency, its place in
//! its layer's drawing order and, on a block's object, the parts of its look
//! that follow its own layer rather than the insert's (AutoCAD's BYLAYER in a
//! block). Absent, each part is its source's: the layer's in the drawing, the
//! insert's in a block (docs/adr/0139, 0144).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::layer::LineType;

/// The most transparent an object may be: AutoCAD's 90 %.
pub const MAX_TRANSPARENCY: f64 = 0.9;
/// The largest line type scale.
pub const MAX_LINE_TYPE_SCALE: f64 = 1000.0;

/// A part of an object's look whose source can be its layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LookPart {
    Color,
    LineType,
    LineWeight,
    Transparency,
}

impl LookPart {
    /// Every part, in the contract's order.
    pub const ALL: [LookPart; 4] = [
        LookPart::Color,
        LookPart::LineType,
        LookPart::LineWeight,
        LookPart::Transparency,
    ];

    /// Its name in the contract and the file (`lineType`).
    pub fn name(self) -> &'static str {
        match self {
            LookPart::Color => "color",
            LookPart::LineType => "lineType",
            LookPart::LineWeight => "lineWeight",
            LookPart::Transparency => "transparency",
        }
    }

    /// The part of `name`; none for another word.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// An object's line type, its scale, transparency, drawing order and, on a
/// block's object, the parts that follow its own layer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ObjectLook {
    /// Its own line type; absent: its source's (the layer's, or the insert's in a block).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<LineType>,
    /// Its pattern's own factor, above 0, at most 1000; absent: 1. Always the object's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub line_type_scale: Option<f64>,
    /// How transparent it is, 0 to 0.9; absent: its source's (none for a layer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 0.9)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub transparency: Option<f64>,
    /// Its place in its layer's drawing order: lower first, equals in the
    /// document's order; absent: 0; a 32-bit integer. Only a drawing's object has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub order: Option<i32>,
    /// On a block's object, the parts without a value of their own that
    /// follow the object's layer instead of the insert; in order, once each,
    /// never one with a value of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LookPart>>", optional))]
    pub by_layer: Vec<LookPart>,
}

impl ObjectLook {
    /// Whether it says nothing (an object as before docs/adr/0260).
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What is wrong with an object's look, said so a reader can fix it; none:
/// it is right. `color`, `line_weight`: whether the object has its own;
/// `in_block`: whether it is a block's object.
pub fn look_problem(
    look: &ObjectLook,
    has_color: bool,
    has_line_weight: bool,
    in_block: bool,
) -> Option<String> {
    if let Some(s) = look.line_type_scale
        && !(s.is_finite() && s > 0.0 && s <= MAX_LINE_TYPE_SCALE)
    {
        return Some(format!(
            "Çizgi tipi ölçeği 0'dan büyük ve en çok {MAX_LINE_TYPE_SCALE} olmalı; {s} verildi."
        ));
    }
    if let Some(t) = look.transparency
        && !(t.is_finite() && (0.0..=MAX_TRANSPARENCY).contains(&t))
    {
        return Some(format!(
            "Saydamlık 0 ile {MAX_TRANSPARENCY} arasında olmalı (%0–90); {t} verildi."
        ));
    }
    if in_block && look.order.is_some() {
        return Some("Bloğun nesnesinin çizim sırası olmaz: tanımın sırasıyla çizilir.".into());
    }
    if !in_block && !look.by_layer.is_empty() {
        return Some(
            "Katmana göre parçalar yalnız bloğun nesnesinde olur; çizimdeki nesne zaten katmanına göredir.".into(),
        );
    }
    if look.by_layer.windows(2).any(|w| w[1] <= w[0]) {
        return Some("Katmana göre parçalar sırayla ve birer kez yazılır (renk, çizgi tipi, kalınlık, saydamlık).".into());
    }
    for part in &look.by_layer {
        let own = match part {
            LookPart::Color => has_color,
            LookPart::LineType => look.line_type.is_some(),
            LookPart::LineWeight => has_line_weight,
            LookPart::Transparency => look.transparency.is_some(),
        };
        if own {
            return Some(format!(
                "{} nesnenin kendi değeri; katmana göre olamaz.",
                match part {
                    LookPart::Color => "Renk",
                    LookPart::LineType => "Çizgi tipi",
                    LookPart::LineWeight => "Çizgi kalınlığı",
                    LookPart::Transparency => "Saydamlık",
                }
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_is_checked_by_its_rules() {
        let ok = ObjectLook {
            line_type: Some(LineType::Dashed),
            line_type_scale: Some(2.5),
            transparency: Some(0.5),
            order: Some(-3),
            by_layer: vec![],
        };
        assert_eq!(look_problem(&ok, false, false, false), None);
        let scale = ObjectLook {
            line_type_scale: Some(0.0),
            ..ObjectLook::default()
        };
        assert!(look_problem(&scale, false, false, false).is_some());
        let clear = ObjectLook {
            transparency: Some(0.95),
            ..ObjectLook::default()
        };
        assert!(look_problem(&clear, false, false, false).is_some());
        let layered = ObjectLook {
            by_layer: vec![LookPart::Color, LookPart::LineWeight],
            ..ObjectLook::default()
        };
        assert_eq!(look_problem(&layered, false, false, true), None);
        assert!(look_problem(&layered, false, false, false).is_some());
        assert!(look_problem(&layered, true, false, true).is_some());
        let unsorted = ObjectLook {
            by_layer: vec![LookPart::LineWeight, LookPart::Color],
            ..ObjectLook::default()
        };
        assert!(look_problem(&unsorted, false, false, true).is_some());
        let ordered = ObjectLook {
            order: Some(1),
            ..ObjectLook::default()
        };
        assert!(look_problem(&ordered, false, false, true).is_some());
    }

    #[test]
    fn an_empty_look_writes_nothing() {
        let json = serde_json::to_string(&ObjectLook::default()).unwrap();
        assert_eq!(json, "{}");
    }
}
