//! Pafta seti (docs/adr/0347 §1; plan/cad.md `CAD-ARCH-43`): the project's sheet book as a drawing set — its sheets by
//! series from their names' numbers, each map frame's ground box and the view marks that point at it — and, given objects,
//! the sheets their change affects. `cad.sheets.set` reads; it writes nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;

pub const CAD_SHEETS_SET: &str = "cad.sheets.set";
pub const CAD_SHEETS_SET_VERSION: u32 = 1;

/// The set's series titles: an `A` sheet's series by its number's first digit (the template's order).
pub const SERIES_TITLES: &[(&str, &str)] = &[
    ("A-0", "Genel"),
    ("A-1", "Planlar"),
    ("A-2", "Görünüşler"),
    ("A-3", "Kesitler"),
    ("A-4", "Büyük ölçekli çizimler"),
    ("A-5", "Detaylar"),
];

/// The series of a sheet without a number.
pub const OTHER_SERIES: &str = "Diğer";

/// A sheet's series from its name: the name's leading letters (ASCII, upper-cased), an optional `-`, `.` or space, and
/// the first digit of the number after them (`A-101`, `a101`, `A.205 Görünüş` …); [`OTHER_SERIES`] for a name without.
pub fn series_of(name: &str) -> String {
    let t = name.trim();
    let letters: String = t.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    if letters.is_empty() {
        return OTHER_SERIES.to_owned();
    }
    let mut rest = t[letters.len()..].chars();
    let mut c = rest.next();
    if matches!(c, Some('-' | '.' | ' ')) {
        c = rest.next();
    }
    match c {
        Some(d) if d.is_ascii_digit() => format!("{}-{d}", letters.to_ascii_uppercase()),
        _ => OTHER_SERIES.to_owned(),
    }
}

/// A series' title; empty for one KentOS does not name.
pub fn series_title(series: &str) -> &'static str {
    SERIES_TITLES
        .iter()
        .find(|(s, _)| *s == series)
        .map_or("", |(_, t)| t)
}

/// Input of `cad.sheets.set` v1 (“Pafta seti”): the set; with `uids`, the sheets those objects' change affects.
///
/// Refusals, in this order: `entity_not_found` (at `uids[i]`), `book_unreadable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsSet {
    /// The changed objects' persistent ids; absent: no affected list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
    /// Include the canonical book and complete system templates for automation.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub details: bool,
    /// Include embedded picture bytes (base64), explicitly opt in to the larger response.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub pictures: bool,

}

/// A series of the set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetSeries {
    /// `A-1` …, or `Diğer`.
    pub series: String,
    /// Its title (`Planlar` …); empty for a series KentOS does not name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub title: String,
    /// Its sheets' ids, in the book's order.
    pub sheets: Vec<String>,
}

/// A map frame of the set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SetFrame {
    /// The item's id.
    pub item: String,
    pub name: String,
    /// Its scale (1:N); none for a map that follows something else (an atlas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<f64>,
    /// The ground its content shows, its four corners clockwise from the content's top left (east, north, metres); empty
    /// for a map without a fixed view and centre (`unknown`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<Vec2>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub corners: Vec<Vec2>,
    /// The corners' box: west, south, east, north (metres); none with no corners.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bounds: Option<[f64; 4]>,
    /// The view marks that point at it, by persistent id, in the drawing's order.
    pub marks: Vec<String>,
    /// The named view it is linked to, by its stable id (docs/adr/0599); none: its own camera or a snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<String>,
    /// Its linked view is gone (`missing_named_view`): it draws nothing until linked again or detached.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub missing: bool,
    /// The level it draws (its view's recipe's, else its own; docs/adr/0716, 0774 §3), by id; none: every level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// Its view's recipe's version (docs/adr/0774 §1); none without a recipe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub recipe: Option<u32>,
}

/// A sheet of the set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SetSheet {
    pub id: String,
    pub name: String,
    pub series: String,
    /// Its map frames, in its drawing order.
    pub frames: Vec<SetFrame>,
}

/// Why a frame is affected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AffectReason {
    /// An object's box meets the frame's ground box, and the frame draws it: on its level (or on none), its category
    /// and phase not left out by its view's recipe (docs/adr/0774 §4).
    Shows,
    /// One of the objects is a view mark pointing at the frame.
    Marked,
    /// The frame's ground is not known (no fixed view): it may show them.
    Unknown,
}

/// A frame the objects' change affects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AffectedFrame {
    pub sheet: String,
    pub item: String,
    /// Its reasons, in this order: shows, marked, unknown.
    pub reasons: Vec<AffectReason>,
    /// The objects that affect it, by persistent id, in the input's order (none for `unknown` alone).
    pub uids: Vec<String>,
}

/// Output of `cad.sheets.set` v1 (its plan's too).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsSetResult {
    /// The series, sorted by their letters and digit; `Diğer` last.
    pub series: Vec<SheetSeries>,
    /// The sheets, in the book's order.
    pub sheets: Vec<SetSheet>,
    /// With `uids`: the frames affected, in the book's order; their sheets are the ones to print again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub affected: Option<Vec<AffectedFrame>>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<kentos_sheet::model::SheetBook>"))]
    pub book: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Array<unknown>"))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<kentos_sheet::template::Template>>"))]
    pub templates: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pictures: Option<Vec<crate::SheetPicture>>,
    // With details: CAD preset ratios, using the same table as the viewport inspector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub standard_ratios: Option<Vec<kentos_contracts::PaperScaleRatio>>,
    /// With details: the same current named-view registry the desktop source picker uses.
    #[serde(default,skip_serializing_if="Option::is_none")]
    #[cfg_attr(feature="ts",ts(optional))]
    pub named_views:Option<Vec<kentos_contracts::NamedView>>,
    /// Available source annotation representations, ordered; with details.
    #[serde(default,skip_serializing_if="Option::is_none")]
    #[cfg_attr(feature="ts",ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="scale")))]
    pub annotation_scales:Option<Vec<f64>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_gives_its_series() {
        for (name, series) in [
            ("A-101 Zemin kat planı", "A-1"),
            ("a301", "A-3"),
            ("A.205 Görünüş", "A-2"),
            ("S 401", "S-4"),
            ("Kapak", "Diğer"),
            ("A-", "Diğer"),
            ("101", "Diğer"),
            ("", "Diğer"),
            ("Ü-101", "Diğer"),
        ] {
            assert_eq!(series_of(name), series, "{name}");
        }
        assert_eq!(series_title("A-3"), "Kesitler");
        assert_eq!(series_title("S-4"), "");
    }
}
