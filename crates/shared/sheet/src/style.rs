//! How lines and letters look on the paper: strokes, text styles, alignment
//! and colours. Colours are `#rrggbb` or `#rrggbbaa` (design §2): the paper
//! is always white, so no theme token reaches it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::units::Um;

pub(crate) fn black() -> String {
    "#000000".to_owned()
}

pub(crate) fn white() -> String {
    "#ffffff".to_owned()
}

/// Whether `s` is `#rrggbb` or `#rrggbbaa`.
pub fn is_color(s: &str) -> bool {
    match s.strip_prefix('#') {
        Some(h) => (h.len() == 6 || h.len() == 8) && h.bytes().all(|b| b.is_ascii_hexdigit()),
        None => false,
    }
}

/// A colour's opacity, 0–255 (255 without an alpha part).
pub fn alpha(s: &str) -> u8 {
    s.strip_prefix('#')
        .filter(|h| h.len() == 8)
        .and_then(|h| h.get(6..8))
        .and_then(|a| u8::from_str_radix(a, 16).ok())
        .unwrap_or(255)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// A line's look: colour, width on paper, dash pattern (empty: solid), ends and corners.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Stroke {
    #[serde(default = "black")]
    pub color: String,
    /// Width in micrometres (ISO 128 pens: 180, 250, 350, 500, 700).
    pub width: Um,
    /// Dash and gap lengths in micrometres, alternating; empty: a solid line.
    #[serde(default)]
    pub dash: Vec<Um>,
    #[serde(default)]
    pub cap: LineCap,
    #[serde(default)]
    pub join: LineJoin,
}

impl Stroke {
    pub fn solid(color: &str, width: Um) -> Stroke {
        Stroke {
            color: color.to_owned(),
            width,
            dash: Vec::new(),
            cap: LineCap::Butt,
            join: LineJoin::Miter,
        }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke::solid("#000000", 250)
    }
}

/// Horizontal alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

pub(crate) fn default_font() -> String {
    "barlow".to_owned()
}

fn w400() -> u16 {
    400
}

/// Letters: one of the drawing's typefaces (ADR 0055, `DRAWING_FONTS` ids:
/// barlow, arimo, overpass, quicksand, architects-daughter, courier-prime,
/// plex-mono), its size on paper, weight, slant and colour.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TextStyle {
    #[serde(default = "default_font")]
    pub font: String,
    /// The em size in micrometres (2.5 mm = 2 500; 1 pt = 352.8 µm).
    pub size: Um,
    /// 400, 500, 600 or 700; a face that lacks it is matched as CSS does.
    #[serde(default = "w400")]
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
    #[serde(default = "black")]
    pub color: String,
}

impl TextStyle {
    pub fn new(size: Um, weight: u16) -> TextStyle {
        TextStyle {
            font: default_font(),
            size,
            weight,
            italic: false,
            color: black(),
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle::new(2_000, 400)
    }
}
