//! `cad.profiles.compare` v1, the catalog's migration report (docs/adr/4940; plan/map-cad.md MAP-04): what moving the
//! drawing's data from one object catalog (a data profile) to another would do, said without changing anything. Each
//! class of the old catalog is followed to the new one by its persistent id, else by its field codes (renamed, split,
//! merged, retired); each field and each value list's code is compared; every row counts the drawing's objects it
//! touches and says whether it needs someone's decision. The reconcile decisions recorded as review notes
//! (docs/adr/4140) are followed too: a corrected class that is retired or split, a given field that is dropped.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::DataProfile;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_PROFILES_COMPARE: &str = "cad.profiles.compare";
pub const CAD_PROFILES_COMPARE_VERSION: u32 = 1;

/// The most object ids a row names; its `objects` says how many there are.
pub const MAX_COMPARE_UIDS: usize = 1000;

/// Input of `cad.profiles.compare` v1: the old catalog (`from`, a built-in or the project's profile by its name, or
/// `fromProfile`, the profile itself) and the new one (`to` or `toProfile`); the drawing's file for the project's
/// profiles. Reads only.
///
/// Refusals: `unknown_profile` (a catalog not named or not found; path `from` or `to`), `invalid_profile` (a
/// catalog that does not hold; path `fromProfile` or `toProfile`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCompare {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from_profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to_profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A catalog as the report names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ComparedCatalog {
    pub name: String,
    pub title: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
}

/// What became of an old class in the new catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogClassChangeKind {
    /// Aynı: the same id, layer, geometry, codes and fields.
    Same,
    /// Değişti: the same id; its layer, geometry, codes or fields differ.
    Changed,
    /// Yeniden adlandırıldı: another id holds its codes, and no other old class's.
    Renamed,
    /// Bölündü: its codes went to two classes or more.
    Split,
    /// Birleşti: its codes went to a class that holds another old class's too.
    Merged,
    /// Kalktı: no class of the new catalog has its id or one of its codes.
    Retired,
    /// Yeni: a class of the new catalog no old class leads to.
    Added,
}

/// A class's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogClassChange {
    /// The old class's id; none for an added class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    /// The new classes it leads to, in the new catalog's order (an added class: itself).
    pub to: Vec<String>,
    pub kind: CatalogClassChangeKind,
    /// The old class's title (an added class: the new one's).
    pub title: String,
    /// The old class's layer path, `A › B` (an added class: the new one's).
    pub layer: String,
    /// The new class's layer path when it leads to one; empty otherwise.
    pub to_layer: String,
    /// The old class's field codes (an added class: the new one's).
    pub codes: Vec<String>,
    /// The new classes' codes, in their order.
    pub to_codes: Vec<String>,
    /// The drawing's objects on the old class's layer (an added class: none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Those objects, in the drawing's order, at most 1000.
    pub uids: Vec<String>,
    /// Whether someone must decide before the data moves: a retired or split class, or one whose geometry changes,
    /// with objects.
    pub decide: bool,
    pub note: String,
}

/// What became of an old class's field in its one new class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogFieldChangeKind {
    /// Kalktı: the new class has no field of the name.
    Removed,
    /// Yeni: the new class's field the old one had not.
    Added,
    /// Türü değişti.
    KindChanged,
    /// Değer listesi değişti (another list, or none).
    ListChanged,
    /// Zorunlu oldu.
    NowRequired,
}

/// A field's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogFieldChange {
    /// The old class's id.
    pub class: String,
    /// The new class's id.
    pub to_class: String,
    pub field: String,
    pub kind: CatalogFieldChangeKind,
    /// The old field's kind and list (`metin`, `metin, sebeke`); empty for an added field.
    pub was: String,
    /// The new field's; empty for a removed field.
    pub becomes: String,
    /// The objects the change touches: a removed, retyped or relisted field's objects holding a value; an added or
    /// newly required field's objects without one.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub uids: Vec<String>,
    /// Whether someone must decide: values dropped or to be read anew, or required values missing.
    pub decide: bool,
    pub note: String,
}

/// What became of a value list's code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogCodeChangeKind {
    /// Kalktı: the new catalog's list (or the list itself) does not have it.
    Removed,
    /// Kullanımdan kalktı: the new list keeps it retired.
    Retired,
    /// Yeni.
    Added,
    /// Etiketi değişti.
    Relabelled,
}

/// A value list's code's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogCodeChange {
    /// The list's id.
    pub domain: String,
    pub code: String,
    pub kind: CatalogCodeChangeKind,
    /// The old label (an added code: the new one).
    pub label: String,
    /// The new label; empty for a removed code.
    pub to_label: String,
    /// The objects of the old catalog's classes whose field linked to the list holds the code.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub uids: Vec<String>,
    /// Whether someone must decide: a removed or retired code that objects hold.
    pub decide: bool,
    pub note: String,
}

/// What becomes of a recorded reconcile decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CatalogDecisionChangeKind {
    /// Korunur: its class is the same (or changed) and its fields are all there.
    Kept,
    /// Sınıfıyla gider: its class is renamed or merged; the decision holds under the new class.
    Follows,
    /// Alanı kalkar: a field it gave is removed.
    FieldDropped,
    /// Sınıfı kalkar.
    ClassRetired,
    /// Sınıfı bölünür: which new class it is, is a new decision.
    ClassSplit,
}

/// A recorded decision's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogDecisionChange {
    /// The review note's number.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    /// The candidate's persistent id.
    pub candidate: String,
    /// The asset's persistent id, when the decision wrote or tied one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The old class it concerns: its correction, else the class whose layer holds the asset.
    pub class: String,
    /// The new classes it leads to.
    pub to: Vec<String>,
    pub kind: CatalogDecisionChangeKind,
    /// The fields it gave that the new class drops.
    pub fields: Vec<String>,
    pub decide: bool,
    pub note: String,
}

/// Output (and plan) of `cad.profiles.compare` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCompared {
    pub from: ComparedCatalog,
    pub to: ComparedCatalog,
    /// The old catalog's classes in its order, then the new catalog's added classes in its order.
    pub classes: Vec<CatalogClassChange>,
    /// Per class row (in its order), the old fields' changes in the old class's order, then the added ones in the new
    /// class's order; unchanged fields are left out.
    pub fields: Vec<CatalogFieldChange>,
    /// Per old list (in its order), its codes' changes in its order, then the added codes; a list only the new catalog
    /// has, its codes as added.
    pub codes: Vec<CatalogCodeChange>,
    /// The reconcile decisions recorded as resolved review notes, by number.
    pub decisions: Vec<CatalogDecisionChange>,
    /// The rows that need someone's decision.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decide: u32,
    pub notes: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
