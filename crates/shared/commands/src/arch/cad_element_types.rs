//! The element type command (docs/adr/8106; plan/bim-plan.md BIM-12): the project's column, beam, slab, roof, railing
//! and stair types made (from scratch, as a copy or from a drawn element), changed, renamed and removed, and elements
//! tied to a type, freed from it, taken back to it, pushed into it or brought to its latest version
//! (`cad.elementtypes.edit`). A type's change goes to every element following it in the same undo step; one keeping its
//! own value of a field keeps it, one on a locked layer stays at the older version, one the change makes invalid is
//! refused (all or none) or, asked, left behind. The report says which.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{ElementType, ElementTypeKind};

pub const CAD_ELEMENT_TYPES_EDIT: &str = "cad.elementtypes.edit";
pub const CAD_ELEMENT_TYPES_EDIT_VERSION: u32 = 1;

/// When a type's change makes an element following it invalid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ElementTypeRefused {
    /// Nothing is written (`element_type_refused`).
    #[default]
    Stop,
    /// The others take it; those it does not fit stay at the older version and are reported.
    Skip,
}

/// What `cad.elementtypes.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ElementTypesOperation {
    /// Yeni tip: a new type `name` (its id `id`, else from its name) for `element` elements, version 1. Its values are
    /// `values`, over those of `copyOf` (a project type, by id or name) or of `from` (a drawn element: Tipe çevir,
    /// which then follows the new type in the same step) when given.
    Define {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The elements it serves; absent: those of `copyOf` or `from`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        element: Option<ElementTypeKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        copy_of: Option<String>,
        /// A drawn element's persistent id: its values become the type's and it follows the type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        from: Option<String>,
        /// The kind's type fields by their names on the wire, each as the element writes it.
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        values: Map<String, Value>,
    },
    /// Tipi değiştir: the type `type` (its id or name) given the values `values` (null takes a field away, the
    /// field's absence); its version goes up by one and its elements follow.
    Update {
        #[serde(rename = "type")]
        kind: String,
        values: Map<String, Value>,
    },
    /// Yeniden adlandır: the type's name; its id stays, and a column following it takes the name as its type.
    Rename {
        #[serde(rename = "type")]
        kind: String,
        name: String,
    },
    /// Sil: the type; its elements refuse it (`element_type_in_use`) unless `detach`, which leaves them with their
    /// values, following nothing.
    Remove {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        detach: bool,
    },
    /// Tipe bağla: the elements `uids` follow the type, taking all its values (their exceptions none).
    Assign {
        #[serde(rename = "type")]
        kind: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Tipten ayır: the elements keep their values and follow no type.
    Detach {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Tipe döndür: the elements give up their own values of `fields` (none: all) and take the type's latest.
    Reset {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fields: Vec<String>,
    },
    /// Tipe uygula: one element's own values of `fields` (none: all its exceptions) become its type's; the type's
    /// version goes up by one and its other elements follow.
    Push {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fields: Vec<String>,
    },
    /// Tipi yeniden uygula: the elements `uids` (none: every one behind) at an older version of their type take its
    /// latest, their exceptions kept.
    Refresh {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// Input of `cad.elementtypes.edit` v1: the project's element types changed as one undo step “Eleman tipi”, with the
/// elements following them. Its plan is the preview: the types and every element's fate, nothing written.
///
/// Refusals: `invalid_element_type`, `element_type_not_found`, `element_type_exists`, `element_type_in_use`,
/// `element_type_kind`, `element_not_typed`, `element_type_refused`, `nothing_to_set`, `no_entities`, `invalid_uid`,
/// `entity_not_found`, `layer_locked`, `invalid_revision`, `revision_conflict`. Warnings: `element_type_kept`,
/// `element_type_skipped`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElementTypesEdit {
    pub operation: ElementTypesOperation,
    /// When the change makes an element following the type invalid; absent: stop.
    #[serde(default)]
    pub on_refused: ElementTypeRefused,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An element a type's change kept its own value for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ElementKept {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The changed fields it keeps its own value of.
    pub fields: Vec<String>,
}

/// Why an element following a type did not take its change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ElementLeftWhy {
    /// On a locked layer.
    Locked,
    /// Its values would make no element of its kind.
    Invalid,
}

/// An element that did not take a type's change: it stays at the older version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ElementLeft {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub why: ElementLeftWhy,
    pub message: String,
}

/// Every element's fate in a type's change: those that took it, those that kept a changed field of their own, those
/// left at the older version (`skipped`: a locked layer; `refused`: it would be invalid).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ElementTypeReport {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub applied: Vec<String>,
    pub kept: Vec<ElementKept>,
    pub skipped: Vec<ElementLeft>,
    pub refused: Vec<ElementLeft>,
}

/// Output (and plan) of `cad.elementtypes.edit` v1: the project's types after it, the type it acted on (a new one's
/// id), every element's fate and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ElementTypesEdited {
    pub types: Vec<ElementType>,
    /// The type's id; absent for Tipten ayır and Tipi yeniden uygula, which may touch several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub report: ElementTypeReport,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
