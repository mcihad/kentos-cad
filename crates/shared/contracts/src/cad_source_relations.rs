//! `cad.sources.relate` (docs/adr/0532 §2): how a project relation whose child field holds several parents' ids (a
//! many-to-many reference, docs/adr/0409) is kept in a bound GeoPackage. Both layers bound to tables of one file, the
//! reference can be a GeoPackage related-tables relation (OGC 18-000) with a mapping table; until it is, the field is a
//! text column of ids joined with `; `. The command says which, maps it, or takes the mapping out again. The shared
//! cases are `fixtures/commands/v1/cad.sources.relate.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SOURCES_RELATE: &str = "cad.sources.relate";
pub const CAD_SOURCES_RELATE_VERSION: u32 = 1;

/// What `cad.sources.relate` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RelateOp {
    /// Says how the relation is kept now; writes nothing.
    Check,
    /// Makes the relation's mapping table in the file, filled from the layers' references.
    Map,
    /// Takes the relation's mapping table out of the file; the references stay in the layers.
    Unmap,
}

/// How a project relation is kept in its sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RelationState {
    /// A layer of the relation is not bound to a source.
    Unbound,
    /// The layers are bound to different files or databases: one transaction cannot hold both.
    Split,
    /// The child's field holds one id (a one-to-many reference): a plain column, nothing to map.
    Single,
    /// A many-to-many reference kept as a text column of ids joined with `; `.
    Text,
    /// A many-to-many reference kept as KentOS's related-tables relation.
    Mapped,
    /// The file relates the two tables in another representation: it is not rewritten, and not read as text.
    Foreign,
    /// The file's relation between the two tables is not as the profile says.
    Broken,
}

/// Input of `cad.sources.relate` v1.
///
/// Refusals, checked in this order: `unknown_relation`; `source_gone` (the file is not there). `check` answers for what it
/// finds (`unbound`, `split`, `single`, `text`, `mapped`, `foreign`, `broken`); `map` and `unmap` refuse them:
/// `relation_unbound` (a layer of the relation is not bound), `transaction_split` (the layers are not in one GeoPackage
/// file: the mapping needs both in one transaction), `relation_not_multiple` (`map`) and `relation_not_mapped` (`unmap`) for a
/// field that holds one id, `source_readonly`, `source_changed` (status `conflict`: the file changed since the layers last
/// matched it); then for `map`: `relation_mapped` (already mapped), `relation_unsupported` (the file relates the two tables
/// another way, or its mapping table is not one), `invalid_mapping` (the table's name is not a name, or the file has a table
/// of it), `relation_dangling` (a reference to an object the table does not hold); for `unmap`: `relation_not_mapped`.
/// `file_unwritable` when the file cannot be written (nothing is).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesRelate {
    /// The project relation's id.
    pub relation: String,
    pub action: RelateOp,
    /// `map`: the mapping table's name (letters, digits and `_`, 1–64, not starting with a digit or `gpkg`);
    /// none: the child's table and the parent's joined with `_`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<String>,
}

/// Output of `cad.sources.relate` v1, and its plan: how the relation is kept (after the op; a plan says what it
/// would be), its mapping table and rows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesRelated {
    pub relation: String,
    pub state: RelationState,
    /// The mapping table (mapped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mapping: Option<String>,
    /// The mapping rows (mapped; `map`: those written).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    /// The child's table and the parent's, when both are bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub child: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<String>,
    /// Why a foreign or broken relation is not rewritten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
    /// The file's SHA-256 after (plan, check: now); none for an unbound or split relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    pub revision: String,
}
