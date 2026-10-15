//! Özellik kopyala's semantic command of the product command catalog (docs/adr/0304; AutoCAD's MATCHPROP): the
//! properties chosen of one object given to others, the values read from the source by the command rather than given.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// Gives one object's properties to others.
pub const CAD_ENTITIES_MATCH: &str = "cad.entities.match";
pub const CAD_ENTITIES_MATCH_VERSION: u32 = 1;

/// A property Özellik kopyala gives (AutoCAD's MATCHPROP settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MatchProperty {
    /// The layer the objects move to.
    Layer,
    /// Their own colour, or their layer's.
    Color,
    /// Their own line type, or their layer's (`ObjectLook.line_type`).
    LineType,
    /// Their line type's own factor.
    LineTypeScale,
    /// Their own line weight, or their layer's: only between objects drawn with lines.
    LineWeight,
    /// Their own transparency.
    Transparency,
    /// Their own symbol, or their layer's style.
    Symbol,
    /// A text's face (its style, font, bold, italic, slant), height and width factor, to texts.
    Text,
    /// A dimension's look (its style, arrowheads, sizes, value's writing), value height and background, to dimensions.
    Dimension,
    /// A hatch's pattern (its name, scale, angle, lines or gradient), to hatches; their regions stay.
    Hatch,
    /// A leader's arrowhead, its size, the note's height and background, to leaders.
    Leader,
}

impl MatchProperty {
    /// Every property, in AutoCAD's settings' order: what an input that names none gives.
    pub const ALL: [MatchProperty; 11] = [
        MatchProperty::Layer,
        MatchProperty::Color,
        MatchProperty::LineType,
        MatchProperty::LineTypeScale,
        MatchProperty::LineWeight,
        MatchProperty::Transparency,
        MatchProperty::Symbol,
        MatchProperty::Text,
        MatchProperty::Dimension,
        MatchProperty::Hatch,
        MatchProperty::Leader,
    ];
}

/// Input of `cad.entities.match` v1: the `properties` of the object `source` given to the objects `uids`, as one
/// undo step “Özellik kopyala”. A property the source has none of (its layer's colour, no symbol) clears the
/// target's own. A line weight goes only between objects drawn with lines; the kind's properties (`text`,
/// `dimension`, `hatch`, `leader`) only to objects of the source's kind; the rest to every object. The source among
/// the targets is left out. An object on a locked layer is left out with a warning (`layer_locked`).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid` (the source, then the targets), `no_entities`,
/// `no_properties` (an empty list), `invalid_revision`, `revision_conflict` (status `conflict`), `entity_not_found`
/// (the source, then the targets), `no_entities` (the source alone), `layer_locked` (every target locked, then the
/// source's layer when the layer is given). Warning: `layer_hidden` (objects moved to a hidden layer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesMatch {
    /// The object whose properties are given: its persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub source: String,
    /// The objects that take them: their persistent ids, at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The properties given, each once; absent: every one (`MatchProperty::ALL`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub properties: Option<Vec<MatchProperty>>,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
