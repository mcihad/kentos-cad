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
    /// An area's mask (AutoCAD's WIPEOUT, docs/adr/0264 §3, 0310): it fills with the drawing's background and covers
    /// what is drawn before it; only an area has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wipeout: Option<Wipeout>,
}

/// An area's mask (docs/adr/0310 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Wipeout {
    /// Whether its boundary is drawn (AutoCAD's WIPEOUTFRAME, per mask).
    pub frame: bool,
}

impl ObjectLook {
    /// Whether it says nothing (an object as before docs/adr/0260).
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What is wrong with a mask on an object of `kind` (its name in the contract, `polygon` …), said so a reader can
/// fix it; none: it is right (docs/adr/0310 §1). Only an area masks.
pub fn wipeout_problem(look: &ObjectLook, kind: &str) -> Option<String> {
    (look.wipeout.is_some() && kind != "polygon").then(|| {
        format!("Maske yalnız alanda olur; bu nesne bir {kind}. Kapalı alanı maske yapın ya da Maske aracıyla çizin.")
    })
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

// ── Where a part of the look comes from (docs/adr/0312 §4) ──────────────────

/// Where a part of a drawn object's look comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LookSource {
    /// The object's own value.
    Own,
    /// Its layer's (in a block: a `byLayer` part, its own layer's).
    Layer,
    /// In a block: the insert's, as the insert draws it (BYBLOCK).
    Block,
}

/// The look an object is drawn with, each part with its source.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedLook {
    pub color: String,
    pub line_type: LineType,
    pub line_type_scale: f64,
    pub line_weight: f64,
    pub transparency: f64,
    /// Colour, line type, scale, weight and transparency's sources, in that order.
    pub from: [LookSource; 5],
}

/// An object's own parts: its colour and line weight (on its base) and its look.
#[derive(Clone, Copy, Debug)]
pub struct OwnLook<'a> {
    pub color: Option<&'a str>,
    pub line_weight: Option<f64>,
    pub look: &'a ObjectLook,
}

/// The look an object is drawn with: each part its own, else in a block (`insert`: the insert it is drawn through,
/// resolved) its own layer's for a `byLayer` part and the insert's for the rest, else its layer's. A layer has no
/// transparency and no pattern scale.
pub fn resolve_look(
    own: OwnLook<'_>,
    layer: &crate::LayerStyle,
    insert: Option<&ResolvedLook>,
) -> ResolvedLook {
    let follows = |p: LookPart| own.look.by_layer.contains(&p);
    let source = |has: bool, part: Option<LookPart>| match (has, insert) {
        (true, _) => LookSource::Own,
        (false, Some(_)) if part.is_none_or(|p| !follows(p)) => LookSource::Block,
        _ => LookSource::Layer,
    };
    let from = [
        source(own.color.is_some(), Some(LookPart::Color)),
        source(own.look.line_type.is_some(), Some(LookPart::LineType)),
        source(own.look.line_type_scale.is_some(), None),
        source(own.line_weight.is_some(), Some(LookPart::LineWeight)),
        source(
            own.look.transparency.is_some(),
            Some(LookPart::Transparency),
        ),
    ];
    let pick = |i: usize| from[i];
    ResolvedLook {
        color: match pick(0) {
            LookSource::Own => own.color.unwrap_or_default().to_owned(),
            LookSource::Block => insert.map_or_else(|| layer.color.clone(), |r| r.color.clone()),
            LookSource::Layer => layer.color.clone(),
        },
        line_type: match pick(1) {
            LookSource::Own => own.look.line_type.unwrap_or(layer.line_type),
            LookSource::Block => insert.map_or(layer.line_type, |r| r.line_type),
            LookSource::Layer => layer.line_type,
        },
        line_type_scale: match pick(2) {
            LookSource::Own => own.look.line_type_scale.unwrap_or(1.0),
            LookSource::Block => insert.map_or(1.0, |r| r.line_type_scale),
            LookSource::Layer => 1.0,
        },
        line_weight: match pick(3) {
            LookSource::Own => own.line_weight.unwrap_or(layer.line_weight),
            LookSource::Block => insert.map_or(layer.line_weight, |r| r.line_weight),
            LookSource::Layer => layer.line_weight,
        },
        transparency: match pick(4) {
            LookSource::Own => own.look.transparency.unwrap_or(0.0),
            LookSource::Block => insert.map_or(0.0, |r| r.transparency),
            LookSource::Layer => 0.0,
        },
        from,
    }
}

// ── The colours on paper (docs/adr/0312 §5) ─────────────────────────────────

/// The paper's colours (DESIGN.md §9a): what a theme colour is on a sheet, in its PDF and its legend, whatever the
/// interface's theme. A true colour is drawn as it is.
pub mod paper {
    pub const PAPER: &str = "#ffffff";
    pub const INK: &str = "#000000";
    pub const FG: &str = "#111111";
    pub const FG_DIM: &str = "#555555";
    /// The drawing's text.
    pub const LABEL: &str = FG;
    /// The halo round the drawing's text.
    pub const LABEL_HALO: &str = PAPER;
}

/// A colour as paper takes it: a theme colour (`fg`, `fg-dim`, `ink`, `paper`, `label`) the paper's, any other as
/// it is.
pub fn print_color(color: &str) -> &str {
    match color {
        "paper" => paper::PAPER,
        "ink" => paper::INK,
        "fg" => paper::FG,
        "label" => paper::LABEL,
        "fg-dim" => paper::FG_DIM,
        other => other,
    }
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
            wipeout: None,
        };
        assert_eq!(look_problem(&ok, false, false, false), None);
        // A mask is an area's only (docs/adr/0310).
        let mask = ObjectLook {
            wipeout: Some(Wipeout { frame: true }),
            ..ObjectLook::default()
        };
        assert_eq!(wipeout_problem(&mask, "polygon"), None);
        assert!(wipeout_problem(&mask, "polyline").is_some());
        assert_eq!(wipeout_problem(&ObjectLook::default(), "line"), None);
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

    fn fixture(name: &str) -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/look/v1")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture"))
            .expect("it reads")
    }

    #[test]
    fn the_look_s_sources_follow_the_shared_cases() {
        let doc = fixture("cases.json");
        let layers = &doc["layers"];
        for case in doc["cases"].as_array().expect("cases") {
            let name = case["name"].as_str().unwrap_or_default();
            let mut resolved: Option<ResolvedLook> = None;
            for link in case["chain"].as_array().expect("a chain") {
                let layer: crate::LayerStyle = serde_json::from_value(
                    layers[link["layerId"].as_str().expect("a layer")].clone(),
                )
                .expect("a layer style");
                let look: ObjectLook = serde_json::from_value(link.clone()).expect("a look");
                let own = OwnLook {
                    color: link["color"].as_str(),
                    line_weight: link["lineWeight"].as_f64(),
                    look: &look,
                };
                resolved = Some(resolve_look(own, &layer, resolved.as_ref()));
            }
            let r = resolved.expect("a look");
            let want = &case["expect"];
            let from = |s: LookSource| match s {
                LookSource::Own => "own",
                LookSource::Layer => "layer",
                LookSource::Block => "block",
            };
            let got = serde_json::json!({
                "color": r.color,
                "lineType": r.line_type,
                "lineWeight": r.line_weight,
                "transparency": r.transparency,
                "lineTypeScale": r.line_type_scale,
                "from": {
                    "color": from(r.from[0]),
                    "lineType": from(r.from[1]),
                    "lineTypeScale": from(r.from[2]),
                    "lineWeight": from(r.from[3]),
                    "transparency": from(r.from[4]),
                },
            });
            assert_eq!(&got, want, "{name}");
        }
    }

    #[test]
    fn the_colours_on_paper_follow_the_shared_cases() {
        for case in fixture("print.json")["cases"].as_array().expect("cases") {
            let color = case["color"].as_str().expect("a colour");
            assert_eq!(
                print_color(color),
                case["paper"].as_str().expect("paper"),
                "{color}"
            );
        }
    }
}
