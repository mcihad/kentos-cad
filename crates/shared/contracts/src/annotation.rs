//! Named text and dimension styles (docs/adr/0183). A style is a preset the
//! project keeps (`ProjectSettings::text_styles`, `::dimension_styles`); a
//! text or a dimension follows one by its id and carries the style's values
//! in its own fields (`TextFace`, `DimensionLook`), so whatever draws, picks
//! or writes it reads the object alone. Applying a style sets those fields
//! (`apply_text_style`, `apply_dimension_style`); changing a style moves the
//! objects whose fields still hold its old values (`follow_text_style`,
//! `follow_dimension_style`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DrawingFont, DrawingUnit, LineType, width_factor_ok};

/// The steepest a text's letters may lean, degrees either way (AutoCAD's
/// bound); never reached.
pub const MAX_OBLIQUE: f64 = 85.0;
/// A dimension's sizes when it names none, times its value's height: the look
/// dimensions always had (docs/adr/0147).
pub const DEFAULT_TICK: f64 = 0.6;
pub const DEFAULT_ARROW: f64 = 1.0;
pub const DEFAULT_EXT_OFFSET: f64 = 0.5;
pub const DEFAULT_EXT_BEYOND: f64 = 0.5;
pub const DEFAULT_TEXT_GAP: f64 = 0.35;
/// The largest a dimension's size may be, times its value's height.
pub const MAX_DIMENSION_RATIO: f64 = 100.0;
/// A dimension's value height on paper when it follows no style (the tools' 2.5 mm).
pub const STANDARD_DIMENSION_HEIGHT_MM: f64 = 2.5;
/// The most decimals a dimension's value may show.
pub const MAX_DIMENSION_DECIMALS: u32 = 8;
/// The most letters a dimension's prefix or suffix may have.
pub const MAX_AFFIX: usize = 32;
/// The most letters a style's name may have.
pub const MAX_STYLE_NAME: usize = 64;
/// The largest paper size a style may name, mm.
pub const MAX_STYLE_MM: f64 = 1000.0;
/// The name the styleless look goes by; no style takes it.
pub const STANDARD_STYLE: &str = "Standart";

impl DrawingFont {
    /// Every typeface, in the contract's order (the typed columns and the
    /// core's metrics number them so).
    pub const ALL: [DrawingFont; 7] = [
        DrawingFont::Barlow,
        DrawingFont::Arimo,
        DrawingFont::Overpass,
        DrawingFont::Quicksand,
        DrawingFont::ArchitectsDaughter,
        DrawingFont::CourierPrime,
        DrawingFont::PlexMono,
    ];

    /// Its name in the contract and the file (`courier-prime`).
    pub fn id(self) -> &'static str {
        match self {
            DrawingFont::Barlow => "barlow",
            DrawingFont::Arimo => "arimo",
            DrawingFont::Overpass => "overpass",
            DrawingFont::Quicksand => "quicksand",
            DrawingFont::ArchitectsDaughter => "architects-daughter",
            DrawingFont::CourierPrime => "courier-prime",
            DrawingFont::PlexMono => "plex-mono",
        }
    }

    /// The typeface named `id`; none for any other name.
    pub fn from_id(id: &str) -> Option<DrawingFont> {
        Self::ALL.into_iter().find(|f| f.id() == id)
    }

    /// Its name as the interface writes it (`Courier Prime`).
    pub fn label(self) -> &'static str {
        match self {
            DrawingFont::Barlow => "Barlow",
            DrawingFont::Arimo => "Arimo",
            DrawingFont::Overpass => "Overpass",
            DrawingFont::Quicksand => "Quicksand",
            DrawingFont::ArchitectsDaughter => "Architects Daughter",
            DrawingFont::CourierPrime => "Courier Prime",
            DrawingFont::PlexMono => "IBM Plex Mono",
        }
    }
}

/// A text's look from its style (docs/adr/0183 §2): the style it follows and
/// the face it is drawn in. With a typeface the text is drawn upright at 400
/// (600 bold, its italic face when italic), its letters leaning `oblique`
/// degrees; without one it is drawn as texts always were, in the project's
/// typeface (docs/adr/0055), and has no bold, italic or slant.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextFace {
    /// The project's text style it follows, by its id; absent: Standart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_style: Option<String>,
    /// Its typeface; absent: the project's, in the look of docs/adr/0055.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub font: Option<DrawingFont>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub italic: bool,
    /// How far its letters lean, degrees, positive with their tops to the
    /// right; within ±`MAX_OBLIQUE`, never 0 (upright is the field's absence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = -85.0, max = 85.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub oblique: Option<f64>,
    /// The fields set apart from its style by hand (docs/adr/0322 §5), by name (`TEXT_OVERRIDES`), as a dimension's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub overrides: Vec<String>,
}

/// A dimension's fields a style sets, by their names on the wire (docs/adr/0322 §5): its look's and its height.
pub const DIMENSION_OVERRIDES: [&str; 20] = [
    "arrow",
    "arrowSize",
    "extOffset",
    "extBeyond",
    "textGap",
    "textPlace",
    "decimals",
    "unit",
    "prefix",
    "suffix",
    "font",
    "dimLineColor",
    "dimLineWeight",
    "dimLineType",
    "extColor",
    "extWeight",
    "extLineType",
    "textColor",
    "options",
    "height",
];

/// A text's fields a style sets, by their names on the wire (docs/adr/0322 §5).
pub const TEXT_OVERRIDES: [&str; 6] =
    ["font", "bold", "italic", "oblique", "widthFactor", "height"];

/// What is wrong with an override list: a name not among `known`, or one named twice.
fn overrides_problem(list: &[String], known: &[&str]) -> Option<String> {
    for (i, name) in list.iter().enumerate() {
        if !known.contains(&name.as_str()) {
            return Some(format!(
                "“{name}” stilin bir alanı değil; geçersiz kılınan alanlar: {}.",
                known.join(", ")
            ));
        }
        if list[..i].contains(name) {
            return Some(format!("“{name}” geçersiz kılınanlarda iki kez var."));
        }
    }
    None
}

/// The fields of `after` set apart from its style `style` (docs/adr/0322 §5), from each field's value, the style's
/// and the object's before the step, and the overrides the step left it with (`own`: a copied object's as they were,
/// none when the step took them away): a field the step changed is the object's own when it differs from the
/// style's value, and follows again when it equals it; one the step did not change is as `own` says.
fn overrides_after(
    names: &[&'static str],
    was: &serde_json::Map<String, serde_json::Value>,
    own: &[String],
    after: &serde_json::Map<String, serde_json::Value>,
    style: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            if was.get(**name) != after.get(**name) {
                after.get(**name) != style.get(**name)
            } else {
                own.iter().any(|o| o == **name)
            }
        })
        .map(|name| (*name).to_owned())
        .collect()
}

fn as_map(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    }
}

/// A dimension's style fields as values: its look's (none absent) and its height.
fn dimension_fields(
    look: &DimensionLook,
    height: f64,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = as_map(serde_json::to_value(look).unwrap_or_default());
    m.insert("height".into(), serde_json::json!(height));
    m
}

/// A text's style fields as values; its width factor 1 when it has none.
fn text_fields(look: &TextLook) -> serde_json::Map<String, serde_json::Value> {
    let f = &look.face;
    as_map(serde_json::json!({
        "font": f.font,
        "bold": f.bold,
        "italic": f.italic,
        "oblique": f.oblique,
        "widthFactor": look.width_factor.unwrap_or(1.0),
        "height": look.height,
    }))
}

/// A dimension of the style `style`'s overrides after a step that changed it (docs/adr/0322 §5): `before` its look and
/// height before the step, of the same style.
pub fn dimension_overrides(
    before: (&DimensionLook, f64),
    after: (&DimensionLook, f64),
    style: &DimensionStyleDef,
    plot_scale: f64,
) -> Vec<String> {
    overrides_after(
        &DIMENSION_OVERRIDES,
        &dimension_fields(before.0, before.1),
        &after.0.overrides,
        &dimension_fields(after.0, after.1),
        &dimension_fields(&style.look(), style.height_at(plot_scale)),
    )
}

/// A text of the style `style`'s overrides after a step, as `dimension_overrides`. A style that fixes no height
/// leaves the text's height its own business: never an override.
pub fn text_overrides(
    before: &TextLook,
    after: &TextLook,
    style: &TextStyleDef,
    plot_scale: f64,
) -> Vec<String> {
    let styled = TextLook {
        face: style.face(),
        width_factor: style.width_factor,
        height: style.height_at(plot_scale).unwrap_or(after.height),
    };
    let mut own = overrides_after(
        &TEXT_OVERRIDES,
        &text_fields(before),
        &after.face.overrides,
        &text_fields(after),
        &text_fields(&styled),
    );
    if style.height.is_none() {
        own.retain(|n| n != "height");
    }
    own
}

/// Whether `o` may be a text's slant: finite, not 0, less than `MAX_OBLIQUE` either way.
pub fn oblique_holds(o: f64) -> bool {
    o.is_finite() && o != 0.0 && o.abs() < MAX_OBLIQUE
}

impl TextFace {
    /// No style and no face of its own: a text as it always was.
    pub fn is_plain(&self) -> bool {
        *self == TextFace::default()
    }

    /// What is wrong with it (docs/adr/0183 §2): the field and the refusal's
    /// words. An empty style id; bold, italic or a slant without a typeface;
    /// a slant out of its bounds. None when it may be written.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if self.text_style.as_deref().is_some_and(str::is_empty) {
            return Some((
                "textStyle",
                "Yazının stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart)."
                    .to_owned(),
            ));
        }
        if self.font.is_none() {
            let field = if self.bold {
                Some("bold")
            } else if self.italic {
                Some("italic")
            } else if self.oblique.is_some() {
                Some("oblique")
            } else {
                None
            };
            if let Some(field) = field {
                return Some((
                    field,
                    "Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok. Yazıya bir yazı tipi verin ya da bu alanları kaldırın (yazı projenin yazı tipiyle çizilir).".to_owned(),
                ));
            }
        }
        if let Some(o) = self.oblique
            && !oblique_holds(o)
        {
            return Some((
                "oblique",
                format!(
                    "Yazının eğikliği −{MAX_OBLIQUE} ile {MAX_OBLIQUE} derece arasında ve sıfırdan farklı olmalı; {o} verildi. Eğikliği bu aralıkta verin ya da alanı kaldırın (dik)."
                ),
            ));
        }
        if let Some(words) = overrides_problem(&self.overrides, &TEXT_OVERRIDES) {
            return Some(("overrides", words));
        }
        None
    }
}

/// A dimension's arrowheads other than the oblique tick, which is no value but
/// the field's absence (docs/adr/0183 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionArrow {
    /// A filled triangle, its tip on the dimension line's end (Dolu ok).
    Closed,
    /// The triangle's two sides (Açık ok).
    Open,
    /// A filled circle about the end (Nokta).
    Dot,
    /// Nothing at the ends (Yok).
    None,
}

impl DimensionArrow {
    /// Every arrowhead, in the contract's order (the typed columns number them so).
    pub const ALL: [DimensionArrow; 4] = [
        DimensionArrow::Closed,
        DimensionArrow::Open,
        DimensionArrow::Dot,
        DimensionArrow::None,
    ];

    /// Its name in the contract and the file (`closed`).
    pub fn name(self) -> &'static str {
        match self {
            DimensionArrow::Closed => "closed",
            DimensionArrow::Open => "open",
            DimensionArrow::Dot => "dot",
            DimensionArrow::None => "none",
        }
    }

    /// The arrowhead named `name`; none for any other name.
    pub fn from_name(name: &str) -> Option<DimensionArrow> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }
}

/// Where a dimension's value stands other than over its line (docs/adr/0183 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionTextPlace {
    /// Its middle on the line, the line hidden under it.
    Centre,
}

/// A dimension's look from its style (docs/adr/0183 §3): the style it follows,
/// its arrowheads, its parts' sizes (times its value's height), where its
/// value stands and how the value is written. All absent: the look dimensions
/// always had, the value in the project's unit and decimals.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionLook {
    /// The project's dimension style it follows, by its id; absent: Standart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dim_style: Option<String>,
    /// Its arrowheads; absent: oblique ticks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arrow: Option<DimensionArrow>,
    /// The arrowhead's (or tick's) length; absent: `DEFAULT_TICK` for ticks,
    /// `DEFAULT_ARROW` for the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub arrow_size: Option<f64>,
    /// The extension lines' gap from the measured points; absent: `DEFAULT_EXT_OFFSET`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub ext_offset: Option<f64>,
    /// How far the extension lines pass the dimension line; absent: `DEFAULT_EXT_BEYOND`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub ext_beyond: Option<f64>,
    /// The value's baseline over the dimension line; absent: `DEFAULT_TEXT_GAP`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub text_gap: Option<f64>,
    /// Where the value stands; absent: over the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_place: Option<DimensionTextPlace>,
    /// A length's, a coordinate's or a slope's decimals; absent: the
    /// project's length decimals (a slope's 2). Angles keep the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 8)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
    /// The unit lengths and coordinates are written in; absent: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    /// Written before the value (and its kind's own prefix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prefix: Option<String>,
    /// Written after the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub suffix: Option<String>,
    /// The value's typeface; absent: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub font: Option<DrawingFont>,
    /// The dimension line's and its arrowheads' colour, `#RRGGBB`; absent:
    /// the object's (docs/adr/0205 §6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dim_line_color: Option<String>,
    /// The dimension line's weight, paper mm as an object's; absent: a hairline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub dim_line_weight: Option<f64>,
    /// The dimension line's type; absent: continuous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dim_line_type: Option<LineType>,
    /// The extension lines' colour; absent: the object's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ext_color: Option<String>,
    /// The extension lines' weight, paper mm; absent: a hairline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub ext_weight: Option<f64>,
    /// The extension lines' type; absent: continuous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ext_line_type: Option<LineType>,
    /// The value's colour; absent: the object's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_color: Option<String>,
    /// Where the value goes when it does not fit and how it is written (docs/adr/0321 §1, §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub options: Option<Box<crate::DimensionOptions>>,
    /// The fields set apart from its style by hand (docs/adr/0322 §5), by name (`DIMENSION_OVERRIDES`): they keep
    /// their value when the style changes. Empty: each field follows while it holds the style's old value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub overrides: Vec<String>,
}

/// Whether a dimension line's weight may be written: finite, 0 (the thinnest
/// line) to `MAX_LINE_WEIGHT` mm, as an object's own (docs/adr/0139).
pub fn line_weight_holds(w: f64) -> bool {
    w.is_finite() && (0.0..=crate::MAX_LINE_WEIGHT).contains(&w)
}

/// What is wrong with a dimension's line colours and weights (docs/adr/0205
/// §6): the field and the refusal's words; none when they hold.
fn line_problem(
    colors: [(&'static str, &'static str, &Option<String>); 3],
    weights: [(&'static str, &'static str, Option<f64>); 2],
) -> Option<(&'static str, String)> {
    for (field, what, c) in colors {
        if let Some(c) = c
            && !crate::is_hex_colour(c)
        {
            return Some((
                field,
                format!(
                    "Ölçünün {what} #RRGGBB biçiminde olmalı; “{c}” verildi. Rengi #RRGGBB olarak verin ya da alanı kaldırın (nesnenin rengi)."
                ),
            ));
        }
    }
    for (field, what, w) in weights {
        if let Some(w) = w
            && !line_weight_holds(w)
        {
            return Some((
                field,
                format!(
                    "Ölçünün {what} kâğıtta 0 ile {} mm arasında olmalı; {w} verildi. Bu aralıkta verin ya da alanı kaldırın (kılcal).",
                    crate::MAX_LINE_WEIGHT
                ),
            ));
        }
    }
    None
}

/// Why a prefix or a suffix may not be written: empty, too long, or with a
/// line break or another control character; none when it may.
fn affix_problem(s: &str) -> Option<String> {
    let n = s.chars().count();
    if n == 0 {
        return Some("boş".to_owned());
    }
    if n > MAX_AFFIX {
        return Some(format!("{n} harf; en çok {MAX_AFFIX}"));
    }
    s.chars()
        .any(char::is_control)
        .then(|| "satır sonu ya da denetim karakteri var".to_owned())
}

impl DimensionLook {
    /// No style and no look of its own: a dimension as it always was.
    pub fn is_plain(&self) -> bool {
        *self == DimensionLook::default()
    }

    /// Its arrowheads' length, times its height.
    pub fn arrow_size_or_default(&self) -> f64 {
        self.arrow_size.unwrap_or(match self.arrow {
            None => DEFAULT_TICK,
            Some(_) => DEFAULT_ARROW,
        })
    }

    pub fn ext_offset_or_default(&self) -> f64 {
        self.ext_offset.unwrap_or(DEFAULT_EXT_OFFSET)
    }

    pub fn ext_beyond_or_default(&self) -> f64 {
        self.ext_beyond.unwrap_or(DEFAULT_EXT_BEYOND)
    }

    pub fn text_gap_or_default(&self) -> f64 {
        self.text_gap.unwrap_or(DEFAULT_TEXT_GAP)
    }

    /// What is wrong with it (docs/adr/0183 §3): the field and the refusal's
    /// words. An empty style id; a size not finite, below 0 (an arrowhead's
    /// not over 0) or over `MAX_DIMENSION_RATIO`; more than 8 decimals; a
    /// prefix or a suffix empty, too long or with a line break. None when it
    /// may be written.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if let Some(words) = self
            .options
            .as_deref()
            .and_then(crate::DimensionOptions::problem)
        {
            return Some(("options", words));
        }
        if let Some(words) = overrides_problem(&self.overrides, &DIMENSION_OVERRIDES) {
            return Some(("overrides", words));
        }
        if self.dim_style.as_deref().is_some_and(str::is_empty) {
            return Some((
                "dimStyle",
                "Ölçünün stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart)."
                    .to_owned(),
            ));
        }
        let sizes = [
            ("arrowSize", "ok boyu", self.arrow_size, true),
            (
                "extOffset",
                "uzatma çizgisinin boşluğu",
                self.ext_offset,
                false,
            ),
            (
                "extBeyond",
                "uzatma çizgisinin aşması",
                self.ext_beyond,
                false,
            ),
            (
                "textGap",
                "değerin çizgiden yüksekliği",
                self.text_gap,
                false,
            ),
        ];
        for (field, what, v, positive) in sizes {
            if let Some(v) = v
                && !(v.is_finite()
                    && v <= MAX_DIMENSION_RATIO
                    && if positive { v > 0.0 } else { v >= 0.0 })
            {
                let floor = if positive {
                    "sıfırdan büyük"
                } else {
                    "0 ya da büyük"
                };
                return Some((
                    field,
                    format!(
                        "Ölçünün {what} değer yüksekliğinin katıdır: {floor}, en çok {MAX_DIMENSION_RATIO} olmalı; {v} verildi. Bu aralıkta verin ya da alanı kaldırın (Standart'ınki)."
                    ),
                ));
            }
        }
        if let Some(d) = self.decimals
            && d > MAX_DIMENSION_DECIMALS
        {
            return Some((
                "decimals",
                format!(
                    "Ölçünün basamak sayısı en çok {MAX_DIMENSION_DECIMALS}; {d} verildi. Daha az basamak verin ya da alanı kaldırın (projenin basamakları)."
                ),
            ));
        }
        for (field, what, v) in [
            ("prefix", "öneki", &self.prefix),
            ("suffix", "soneki", &self.suffix),
        ] {
            if let Some(why) = v.as_deref().and_then(affix_problem) {
                return Some((
                    field,
                    format!(
                        "Ölçünün {what} yazılamaz: {why}. Tek satır, en çok {MAX_AFFIX} harf verin ya da alanı kaldırın."
                    ),
                ));
            }
        }
        line_problem(
            [
                (
                    "dimLineColor",
                    "ölçü çizgisinin rengi",
                    &self.dim_line_color,
                ),
                ("extColor", "uzatma çizgilerinin rengi", &self.ext_color),
                ("textColor", "değerinin rengi", &self.text_color),
            ],
            [
                (
                    "dimLineWeight",
                    "ölçü çizgisinin kalınlığı",
                    self.dim_line_weight,
                ),
                (
                    "extWeight",
                    "uzatma çizgilerinin kalınlığı",
                    self.ext_weight,
                ),
            ],
        )
    }

    /// Whether it has one of the line fields (`.kcad` schema 30, docs/adr/0205 §6).
    pub fn has_lines(&self) -> bool {
        self.dim_line_color.is_some()
            || self.dim_line_weight.is_some()
            || self.dim_line_type.is_some()
            || self.ext_color.is_some()
            || self.ext_weight.is_some()
            || self.ext_line_type.is_some()
            || self.text_color.is_some()
    }
}

/// A named text style (docs/adr/0183 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextStyleDef {
    /// One of its kind among the project's text styles (a UUIDv7).
    pub id: String,
    /// Its name: trimmed, one of its kind whatever its letters' case, never
    /// `STANDARD_STYLE`.
    pub name: String,
    pub font: DrawingFont,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub italic: bool,
    /// As a text's (`TextFace::oblique`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub oblique: Option<f64>,
    /// The texts' height on paper, mm; absent: the tool's height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    /// As a text's; absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub width_factor: Option<f64>,
    /// The typeface file a DXF named for it, written back as it came.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub font_file: Option<String>,
}

/// A named leader style (docs/adr/0326 §4): its arrowhead, the arrowhead's length in the note's height, the note's
/// height on paper (mm) and its mask.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeaderStyleDef {
    /// One of its kind among the project's leader styles (a UUIDv7).
    pub id: String,
    /// As a text style's.
    pub name: String,
    /// The note's height on paper, mm.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: f64,
    /// Absent: a filled arrow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arrow: Option<crate::LeaderArrow>,
    /// Times the note's height; absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub arrow_size: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
}

impl LeaderStyleDef {
    /// What is wrong with it: the field and the refusal's words (as the other styles say theirs).
    pub fn problem(&self) -> Option<(&'static str, String)> {
        let name = &self.name;
        if !mm_holds(self.height, true) {
            return Some((
                "height",
                format!(
                    "“{name}” kılavuz stilinin not yüksekliği {} mm; sıfırdan büyük, en çok {MAX_STYLE_MM} olmalı",
                    self.height
                ),
            ));
        }
        if let Some(s) = self.arrow_size
            && !crate::leader_arrow_holds(s)
        {
            return Some((
                "arrowSize",
                format!(
                    "“{name}” kılavuz stilinin ok boyu {s}; notun yüksekliğinin {} ile {} katı olmalı",
                    crate::MIN_LEADER_ARROW,
                    crate::MAX_LEADER_ARROW
                ),
            ));
        }
        None
    }
}

/// A named dimension style (docs/adr/0183 §3): sizes in paper mm.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionStyleDef {
    /// One of its kind among the project's dimension styles (a UUIDv7).
    pub id: String,
    /// As a text style's.
    pub name: String,
    /// The value's height on paper, mm.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arrow: Option<DimensionArrow>,
    /// The arrowhead's length, mm; absent: the arrowhead's default times the height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub arrow_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub ext_offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub ext_beyond: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub text_gap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_place: Option<DimensionTextPlace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 8)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub font: Option<DrawingFont>,
    /// The line fields as a dimension's (docs/adr/0205 §6); weights in paper mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dim_line_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub dim_line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dim_line_type: Option<LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ext_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub ext_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ext_line_type: Option<LineType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_color: Option<String>,
    /// The value's fit and writing (docs/adr/0321 §1, §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub options: Option<Box<crate::DimensionOptions>>,
}

/// The name rule both tables share: what is wrong with `names` (each its id
/// and name), in the words of `kind` (“yazı stili”); none when they hold.
fn names_problem<'a>(
    kind: &str,
    names: impl Iterator<Item = (&'a str, &'a str)>,
) -> Option<String> {
    let mut seen: Vec<(&str, String)> = Vec::new();
    for (id, name) in names {
        if id.is_empty() {
            return Some(format!("{kind} kimliği boş"));
        }
        if seen.iter().any(|(i, _)| *i == id) {
            return Some(format!("“{id}” kimlikli {kind} iki kez var"));
        }
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Some(format!("{kind} adı boş"));
        }
        if trimmed != name {
            return Some(format!(
                "“{name}” {kind} adının başında ya da sonunda boşluk var"
            ));
        }
        if trimmed.chars().count() > MAX_STYLE_NAME {
            return Some(format!(
                "“{trimmed}” {kind} adı {MAX_STYLE_NAME} harften uzun"
            ));
        }
        if trimmed.chars().any(char::is_control) {
            return Some(format!(
                "“{trimmed}” {kind} adında satır sonu ya da denetim karakteri var"
            ));
        }
        let folded = trimmed.to_lowercase();
        if folded == STANDARD_STYLE.to_lowercase() {
            return Some(format!(
                "“{trimmed}” adı Standart'ındır; {kind} başka bir ad almalı"
            ));
        }
        if seen.iter().any(|(_, n)| *n == folded) {
            return Some(format!("“{trimmed}” adlı {kind} iki kez var"));
        }
        seen.push((id, folded));
    }
    None
}

/// Whether `mm` may be a style's paper size: finite, at most `MAX_STYLE_MM`,
/// over 0 when `positive`, else 0 or more.
fn mm_holds(mm: f64, positive: bool) -> bool {
    mm.is_finite() && mm <= MAX_STYLE_MM && if positive { mm > 0.0 } else { mm >= 0.0 }
}

impl TextStyleDef {
    /// What is wrong with its values (its name aside); none when they hold.
    pub fn problem(&self) -> Option<String> {
        let name = &self.name;
        if let Some(o) = self.oblique
            && !oblique_holds(o)
        {
            return Some(format!(
                "“{name}” yazı stilinin eğikliği {o}; −{MAX_OBLIQUE} ile {MAX_OBLIQUE} arasında ve sıfırdan farklı olmalı"
            ));
        }
        if let Some(h) = self.height
            && !mm_holds(h, true)
        {
            return Some(format!(
                "“{name}” yazı stilinin yüksekliği {h} mm; sıfırdan büyük, en çok {MAX_STYLE_MM} olmalı"
            ));
        }
        if let Some(w) = self.width_factor
            && !(width_factor_ok(w) && w != 1.0)
        {
            return Some(format!(
                "“{name}” yazı stilinin genişlik çarpanı {w}; sıfırdan büyük, en çok 100 ve 1'den farklı olmalı (1 yazılmaz)"
            ));
        }
        if self.font_file.as_deref().is_some_and(str::is_empty) {
            return Some(format!("“{name}” yazı stilinin yazı tipi dosyası boş"));
        }
        None
    }

    /// The face a text in this style has.
    pub fn face(&self) -> TextFace {
        TextFace {
            overrides: Vec::new(),
            text_style: Some(self.id.clone()),
            font: Some(self.font),
            bold: self.bold,
            italic: self.italic,
            oblique: self.oblique,
        }
    }

    /// A text's height in this style at `1:plot_scale`, metres, when the style fixes one.
    pub fn height_at(&self, plot_scale: f64) -> Option<f64> {
        self.height.map(|mm| mm / 1000.0 * plot_scale)
    }
}

/// The ratio a style's paper size `mm` gives against its height `of`; none
/// for no size and for the default `standard` (one spelling).
fn ratio(mm: Option<f64>, of: f64, standard: f64) -> Option<f64> {
    mm.map(|mm| mm / of).filter(|r| *r != standard)
}

impl DimensionStyleDef {
    /// What is wrong with its values (its name aside); none when they hold.
    pub fn problem(&self) -> Option<String> {
        let name = &self.name;
        if let Some(words) = self
            .options
            .as_deref()
            .and_then(crate::DimensionOptions::problem)
        {
            return Some(format!("“{name}” ölçü stili: {words}"));
        }
        if !mm_holds(self.height, true) {
            return Some(format!(
                "“{name}” ölçü stilinin değer yüksekliği {} mm; sıfırdan büyük, en çok {MAX_STYLE_MM} olmalı",
                self.height
            ));
        }
        for (what, v, positive) in [
            ("ok boyu", self.arrow_size, true),
            ("uzatma çizgisinin boşluğu", self.ext_offset, false),
            ("uzatma çizgisinin aşması", self.ext_beyond, false),
            ("değerin çizgiden yüksekliği", self.text_gap, false),
        ] {
            if let Some(v) = v
                && !(mm_holds(v, positive) && v / self.height <= MAX_DIMENSION_RATIO)
            {
                return Some(format!(
                    "“{name}” ölçü stilinin {what} {v} mm; {}, en çok {MAX_STYLE_MM} ve değer yüksekliğinin {MAX_DIMENSION_RATIO} katı olmalı",
                    if positive {
                        "sıfırdan büyük"
                    } else {
                        "0 ya da büyük"
                    }
                ));
            }
        }
        if let Some(d) = self.decimals
            && d > MAX_DIMENSION_DECIMALS
        {
            return Some(format!(
                "“{name}” ölçü stilinin basamak sayısı {d}; en çok {MAX_DIMENSION_DECIMALS} olmalı"
            ));
        }
        for (what, v) in [("öneki", &self.prefix), ("soneki", &self.suffix)] {
            if let Some(why) = v.as_deref().and_then(affix_problem) {
                return Some(format!("“{name}” ölçü stilinin {what} yazılamaz: {why}"));
            }
        }
        line_problem(
            [
                (
                    "dimLineColor",
                    "ölçü çizgisinin rengi",
                    &self.dim_line_color,
                ),
                ("extColor", "uzatma çizgilerinin rengi", &self.ext_color),
                ("textColor", "değerinin rengi", &self.text_color),
            ],
            [
                (
                    "dimLineWeight",
                    "ölçü çizgisinin kalınlığı",
                    self.dim_line_weight,
                ),
                (
                    "extWeight",
                    "uzatma çizgilerinin kalınlığı",
                    self.ext_weight,
                ),
            ],
        )
        .map(|(_, why)| format!("“{name}” ölçü stili: {why}"))
    }

    /// Whether it has one of the line fields (`.kcad` schema 30).
    pub fn has_lines(&self) -> bool {
        self.look().has_lines()
    }

    /// The look a dimension in this style has: its sizes as ratios of its height.
    pub fn look(&self) -> DimensionLook {
        let tick = self.arrow.is_none();
        DimensionLook {
            overrides: Vec::new(),
            dim_style: Some(self.id.clone()),
            arrow: self.arrow,
            arrow_size: ratio(
                self.arrow_size,
                self.height,
                if tick { DEFAULT_TICK } else { DEFAULT_ARROW },
            ),
            ext_offset: ratio(self.ext_offset, self.height, DEFAULT_EXT_OFFSET),
            ext_beyond: ratio(self.ext_beyond, self.height, DEFAULT_EXT_BEYOND),
            text_gap: ratio(self.text_gap, self.height, DEFAULT_TEXT_GAP),
            text_place: self.text_place,
            decimals: self.decimals,
            unit: self.unit,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            font: self.font,
            dim_line_color: self.dim_line_color.clone(),
            dim_line_weight: self.dim_line_weight,
            dim_line_type: self.dim_line_type,
            ext_color: self.ext_color.clone(),
            ext_weight: self.ext_weight,
            ext_line_type: self.ext_line_type,
            text_color: self.text_color.clone(),
            options: self.options.clone(),
        }
    }

    /// A dimension's value height in this style at `1:plot_scale`, metres.
    pub fn height_at(&self, plot_scale: f64) -> f64 {
        self.height / 1000.0 * plot_scale
    }
}

/// What is wrong with a project's text styles: the name rule, then each
/// style's values; none when they hold.
pub fn text_styles_problem(styles: &[TextStyleDef]) -> Option<String> {
    names_problem(
        "yazı stili",
        styles.iter().map(|s| (s.id.as_str(), s.name.as_str())),
    )
    .or_else(|| styles.iter().find_map(TextStyleDef::problem))
}

/// What is wrong with a project's dimension styles, as `text_styles_problem`.
pub fn leader_styles_problem(styles: &[LeaderStyleDef]) -> Option<String> {
    names_problem(
        "kılavuz stili",
        styles.iter().map(|s| (s.id.as_str(), s.name.as_str())),
    )
    .or_else(|| styles.iter().find_map(|s| s.problem().map(|(_, words)| words)))
}

/// A leader's look as leader style `now` gives it where it held `old`'s values (docs/adr/0326 §4): each of its
/// arrowhead, arrowhead size, note height (the style's paper mm at the plot scale) and mask that was the old style's
/// takes the new one's; one set apart stays.
pub fn follow_leader_style(old: &LeaderStyleDef, now: &LeaderStyleDef, l: &crate::LeaderEntity, plot_scale: f64) -> crate::LeaderEntity {
    let mut out = l.clone();
    let height = |s: &LeaderStyleDef| s.height / 1000.0 * plot_scale;
    if l.arrow == old.arrow {
        out.arrow = now.arrow;
    }
    if l.arrow_size == old.arrow_size {
        out.arrow_size = now.arrow_size;
    }
    if (l.height - height(old)).abs() <= 1e-9 * height(old).abs().max(1.0) {
        out.height = height(now);
    }
    if l.mask == old.mask {
        out.mask = now.mask;
    }
    out
}

/// The leader as leader style `s` writes it, its other fields kept.
pub fn apply_leader_style(s: &LeaderStyleDef, l: &crate::LeaderEntity, plot_scale: f64) -> crate::LeaderEntity {
    crate::LeaderEntity {
        arrow: s.arrow,
        arrow_size: s.arrow_size,
        height: s.height / 1000.0 * plot_scale,
        mask: s.mask,
        leader_style: Some(s.id.clone()),
        ..l.clone()
    }
}

pub fn dimension_styles_problem(styles: &[DimensionStyleDef]) -> Option<String> {
    names_problem(
        "ölçü stili",
        styles.iter().map(|s| (s.id.as_str(), s.name.as_str())),
    )
    .or_else(|| styles.iter().find_map(DimensionStyleDef::problem))
}

/// The text styles as a project keeps them: of those breaking the name rule
/// against the ones before (or alone) none, and none whose values do not hold.
pub fn sanitized_text_styles(styles: Vec<TextStyleDef>) -> Vec<TextStyleDef> {
    let mut out: Vec<TextStyleDef> = Vec::with_capacity(styles.len());
    for s in styles {
        let mut with = out.clone();
        with.push(s.clone());
        if text_styles_problem(&with).is_none() {
            out.push(s);
        }
    }
    out
}

/// The dimension styles as a project keeps them, as `sanitized_text_styles`.
pub fn sanitized_dimension_styles(styles: Vec<DimensionStyleDef>) -> Vec<DimensionStyleDef> {
    let mut out: Vec<DimensionStyleDef> = Vec::with_capacity(styles.len());
    for s in styles {
        let mut with = out.clone();
        with.push(s.clone());
        if dimension_styles_problem(&with).is_none() {
            out.push(s);
        }
    }
    out
}

/// A text's fields a style sets: its face, width factor and height.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLook {
    pub face: TextFace,
    pub width_factor: Option<f64>,
    pub height: f64,
}

/// A text after `style` is applied (none: Standart), at `1:plot_scale`
/// (docs/adr/0183 §2): the style's face and width factor; its height when
/// the style fixes one, else the text's own. Standart takes the face and the
/// width factor away and keeps the height.
pub fn apply_text_style(
    style: Option<&TextStyleDef>,
    look: &TextLook,
    plot_scale: f64,
) -> TextLook {
    match style {
        Some(s) => TextLook {
            face: s.face(),
            width_factor: s.width_factor,
            height: s.height_at(plot_scale).unwrap_or(look.height),
        },
        None => TextLook {
            face: TextFace::default(),
            width_factor: None,
            height: look.height,
        },
    }
}

/// A text of `old` after the style became `new` (the same style, its values
/// changed), at `1:plot_scale` (docs/adr/0183 §1): each field still holding
/// the old style's value takes the new one; a field of its own stays. The
/// height moves when it is the old style's and the new style fixes one.
pub fn follow_text_style(
    old: &TextStyleDef,
    new: &TextStyleDef,
    look: &TextLook,
    plot_scale: f64,
) -> TextLook {
    let f = &look.face;
    // An override keeps its value (docs/adr/0322 §5).
    let free = |name: &str| !f.overrides.iter().any(|o| o == name);
    let face = TextFace {
        text_style: f.text_style.clone(),
        font: if free("font") && f.font == Some(old.font) {
            Some(new.font)
        } else {
            f.font
        },
        bold: if free("bold") && f.bold == old.bold {
            new.bold
        } else {
            f.bold
        },
        italic: if free("italic") && f.italic == old.italic {
            new.italic
        } else {
            f.italic
        },
        oblique: if free("oblique") && f.oblique == old.oblique {
            new.oblique
        } else {
            f.oblique
        },
        overrides: f.overrides.clone(),
    };
    let width_factor = if free("widthFactor")
        && look.width_factor.unwrap_or(1.0) == old.width_factor.unwrap_or(1.0)
    {
        new.width_factor
    } else {
        look.width_factor
    };
    let height = match (old.height_at(plot_scale), new.height_at(plot_scale)) {
        (Some(was), Some(now)) if free("height") && look.height == was => now,
        _ => look.height,
    };
    TextLook {
        face,
        width_factor,
        height,
    }
}

/// A dimension after `style` is applied (none: Standart), at `1:plot_scale`
/// (docs/adr/0183 §3): the style's look and value height. Standart takes the
/// look away; its height is the project's dimension height `standard_mm`
/// (docs/adr/0205 §1; `STANDARD_DIMENSION_HEIGHT_MM` when it names none).
pub fn apply_dimension_style(
    style: Option<&DimensionStyleDef>,
    plot_scale: f64,
    standard_mm: f64,
) -> (DimensionLook, f64) {
    match style {
        Some(s) => (s.look(), s.height_at(plot_scale)),
        None => (DimensionLook::default(), standard_mm / 1000.0 * plot_scale),
    }
}

/// A dimension of `old` after the style became `new`, as `follow_text_style`:
/// each field of its look still holding the old style's value takes the new
/// one, its height too.
pub fn follow_dimension_style(
    old: &DimensionStyleDef,
    new: &DimensionStyleDef,
    look: &DimensionLook,
    height: f64,
    plot_scale: f64,
) -> (DimensionLook, f64) {
    let (was, now) = (old.look(), new.look());
    // An override keeps its value (docs/adr/0322 §5).
    let own = &look.overrides;
    fn pick<T: PartialEq + Clone>(free: bool, mine: &T, was: &T, now: &T) -> T {
        if free && mine == was {
            now.clone()
        } else {
            mine.clone()
        }
    }
    let free = |name: &str| !own.iter().any(|o| o == name);
    let out = DimensionLook {
        dim_style: look.dim_style.clone(),
        arrow: pick(free("arrow"), &look.arrow, &was.arrow, &now.arrow),
        arrow_size: pick(
            free("arrowSize"),
            &look.arrow_size,
            &was.arrow_size,
            &now.arrow_size,
        ),
        ext_offset: pick(
            free("extOffset"),
            &look.ext_offset,
            &was.ext_offset,
            &now.ext_offset,
        ),
        ext_beyond: pick(
            free("extBeyond"),
            &look.ext_beyond,
            &was.ext_beyond,
            &now.ext_beyond,
        ),
        text_gap: pick(
            free("textGap"),
            &look.text_gap,
            &was.text_gap,
            &now.text_gap,
        ),
        text_place: pick(
            free("textPlace"),
            &look.text_place,
            &was.text_place,
            &now.text_place,
        ),
        decimals: pick(
            free("decimals"),
            &look.decimals,
            &was.decimals,
            &now.decimals,
        ),
        unit: pick(free("unit"), &look.unit, &was.unit, &now.unit),
        prefix: pick(free("prefix"), &look.prefix, &was.prefix, &now.prefix),
        suffix: pick(free("suffix"), &look.suffix, &was.suffix, &now.suffix),
        font: pick(free("font"), &look.font, &was.font, &now.font),
        dim_line_color: pick(
            free("dimLineColor"),
            &look.dim_line_color,
            &was.dim_line_color,
            &now.dim_line_color,
        ),
        dim_line_weight: pick(
            free("dimLineWeight"),
            &look.dim_line_weight,
            &was.dim_line_weight,
            &now.dim_line_weight,
        ),
        dim_line_type: pick(
            free("dimLineType"),
            &look.dim_line_type,
            &was.dim_line_type,
            &now.dim_line_type,
        ),
        ext_color: pick(
            free("extColor"),
            &look.ext_color,
            &was.ext_color,
            &now.ext_color,
        ),
        ext_weight: pick(
            free("extWeight"),
            &look.ext_weight,
            &was.ext_weight,
            &now.ext_weight,
        ),
        ext_line_type: pick(
            free("extLineType"),
            &look.ext_line_type,
            &was.ext_line_type,
            &now.ext_line_type,
        ),
        text_color: pick(
            free("textColor"),
            &look.text_color,
            &was.text_color,
            &now.text_color,
        ),
        options: pick(free("options"), &look.options, &was.options, &now.options),
        overrides: look.overrides.clone(),
    };
    let height = if free("height") && height == old.height_at(plot_scale) {
        new.height_at(plot_scale)
    } else {
        height
    };
    (out, height)
}
