//! `cad.zoning.symbolize` v1 (plan/map-cad.md MAP-09, docs/adr/4240): Sembolü karardan ver. A decision area's MPYY
//! symbol is chosen from its decision, never the other way: from its use (`Kullanim`, the profile `imar-plani`'s
//! `plan-kullanim` list) at its plan's level (the plan boundary's `Seviye`), by KentOS's table
//! (`catalog/v1/profiles/imar-plani-semboller.json`: the level's MPYY area symbol named as the use, only where exactly
//! one is) or by the user's choice among the level's area symbols. The command writes only the objects' library symbol;
//! the decision (the attributes) is never touched, so a symbol changed later changes no decision. The shared cases
//! are `fixtures/commands/v1/cad.zoning.symbolize.json`.
//!
//! docs/adr/5040: the project keeps the user's choices (`.kcad` schema 250, `settings.zoningPlan.symbols`). `keep`
//! makes the given choices the kept ones, in the same undo step; without it the kept ones are read after the given ones
//! and before KentOS's table (`from` `project`). Extra field: the command's version stays.

use serde::{Deserialize, Serialize};

use kentos_contracts::ZoningSymbolPick;
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ZONING_SYMBOLIZE: &str = "cad.zoning.symbolize";
pub const CAD_ZONING_SYMBOLIZE_VERSION: u32 = 1;
/// The format of KentOS's zoning symbol table.
pub const ZONING_SYMBOLS_FORMAT: &str = "kentos.zoning-symbols";

/// KentOS's zoning symbol table (`catalog/v1/profiles/imar-plani-semboller.json`): for each plan level its MPYY branch,
/// the uses whose symbol KentOS knows by name, and the area symbols a user may choose from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbols {
    pub format: String,
    pub version: u32,
    pub profile: String,
    pub basis: String,
    pub levels: Vec<ZoningSymbolLevel>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolLevel {
    pub level: String,
    pub library: String,
    pub uses: Vec<ZoningUseSymbol>,
    pub choices: Vec<ZoningSymbolChoice>,
}

/// A use and its symbol at a level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningUseSymbol {
    pub code: String,
    pub symbol: String,
}

/// An area symbol of a level's MPYY branch: its library id and name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolChoice {
    pub id: String,
    pub name: String,
}

/// KentOS's table, as it ships.
pub fn builtin_zoning_symbols() -> Option<ZoningSymbols> {
    serde_json::from_str(include_str!("../../../../catalog/v1/profiles/imar-plani-semboller.json")).ok()
}

impl ZoningSymbols {
    pub fn level(&self, level: &str) -> Option<&ZoningSymbolLevel> {
        self.levels.iter().find(|l| l.level == level)
    }
}

/// Input of `cad.zoning.symbolize` v1: the decision areas (none: every area on the profile's decision layer) and the
/// user's symbols, which go before the project's kept ones and KentOS's table; `keep` (docs/adr/5040): the given symbols
/// become the project's kept choices (none given: the project keeps none), and the old kept ones are not read.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`; `invalid_input` (a pick's level not one KentOS
/// has symbols for, its use not in `plan-kullanim`, a level and use given twice); `unknown_plan_symbol` (a pick's symbol
/// not one of the level's area symbols); `layer_not_found` (the drawing has no decision layer); for `uids[i]`:
/// `invalid_input` (given twice), `invalid_uid`, `entity_not_found`, `not_a_decision` (not an area on the decision
/// layer); `layer_locked` (an area whose symbol would change is on a locked layer: nothing is written).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolize {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ZoningSymbolPick>>"))]
    pub symbols: Vec<ZoningSymbolPick>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub keep: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A decision area as symbolized: its plan, the plan's level, its use; the symbol it takes and where that comes from
/// (`kentos`, `user` or, docs/adr/5040, `project`: the project's kept choice), the one it had; its state: `set` (its symbol changes), `same` (it has it already),
/// `unmapped` (no symbol for its use at its level), `noPlan` (its plan is not written, or not a plan boundary of the
/// drawing), `noLevel` (its plan has no level), `noUse` (no use), `unknownUse` (a use not in the list).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolRow {
    pub uid: String,
    pub plan: String,
    pub level: String,
    #[serde(rename = "use")]
    #[cfg_attr(feature = "ts", ts(rename = "use"))]
    pub usage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub was: Option<String>,
    pub state: String,
}

/// Output of `cad.zoning.symbolize` v1: every decision area in the drawing's order and the counts: symbols set, already
/// so, uses without a symbol, areas left (no plan, no level, no use or one not in the list); the revision after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningSymbolized {
    pub areas: Vec<ZoningSymbolRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub set: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub same: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unmapped: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    /// With `keep` (docs/adr/5040): the project's kept choices after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kept: Option<Vec<ZoningSymbolPick>>,
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table ships, reads, and each use's symbol is one of its level's choices.
    #[test]
    fn the_table_reads_and_its_symbols_are_choices() {
        let t = builtin_zoning_symbols().expect("the table");
        assert_eq!((t.format.as_str(), t.version, t.profile.as_str()), (ZONING_SYMBOLS_FORMAT, 1, crate::ZONING_PROFILE));
        for level in &t.levels {
            for u in &level.uses {
                assert!(level.choices.iter().any(|c| c.id == u.symbol), "{}: {}", level.level, u.symbol);
            }
        }
        assert!(t.level("uip").is_some() && t.level("nip").is_some());
    }
}
