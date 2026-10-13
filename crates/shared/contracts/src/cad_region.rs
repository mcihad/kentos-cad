//! The semantic area and hatch commands of the product command catalog
//! (docs/adr/0254 §6, §7; plan/cad.md CAD-CMD-01, 05): a region found by a
//! point inside it (or a closed object) made an area or hatched, named by
//! the point, the boundaries and the pattern rather than the rings.
//!
//! The regions are the shared geometry core's (`FaceIndex` over the line
//! work, the smallest closed object around a point, its islands, the texts'
//! boxes left open; the rules İçine tıklayarak alan, Tarama and Çoklu tara
//! have always used), so the tools, Python and an agent asking the same get
//! the same rings. Their output and plan are `cad.entities.create`'s
//! (`EntitiesCreated`, `EntitiesCreatePlan`). Coordinates are metres on the
//! wire (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_trim::TrimBoundaries;
use crate::entity::Vec2;

/// Makes the region around a point an area.
pub const CAD_ENTITIES_BOUNDARY: &str = "cad.entities.boundary";
pub const CAD_ENTITIES_BOUNDARY_VERSION: u32 = 1;
/// Hatches regions.
pub const CAD_ENTITIES_HATCH: &str = "cad.entities.hatch";
pub const CAD_ENTITIES_HATCH_VERSION: u32 = 1;

/// How new objects look and what they carry, as `cad.entities.create`'s
/// objects do (`NewObject`): the tools give the current colour and weight
/// and an object template's attributes, label and symbol. Absent: the
/// layer's look, nothing carried.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewLook {
    /// Colour override, `#RRGGBB`. Absent: the layer's colour (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its own line weight, paper mm (0 the thinnest, at most 100), for what
    /// is drawn with lines. Absent: the layer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The text shown beside it. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Its own symbol, a library item's id. Absent: the layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
}

/// Input of `cad.entities.boundary` v1: the region around `at` that the
/// boundaries' line work closes becomes an area on `layerId`, closed groups
/// inside it holes unless `islands` is false. One undo step “Alan oluştur”,
/// written by `cad.entities.create`'s rules.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite`
/// (`at`, then the rectangle's corners), `no_entities` (`boundaries.uids`
/// empty), `invalid_uid` (at `boundaries.uids[i]`), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found` (at
/// `boundaries.uids[i]`), `layer_not_found` (`boundaryLayer`),
/// `not_enclosed` (no region closes around `at`), then the layer's and the
/// look's as `cad.entities.create` checks them (`layer_not_found`,
/// `not_a_layer`, `layer_locked`, `invalid_line_weight`); a hidden layer
/// gives a `layer_hidden` warning; on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesBoundary {
    /// A point inside the region.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The layer the area goes to.
    pub layer_id: String,
    /// Whose line work closes the region; absent: every shown object (`all`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundaries: Option<TrimBoundaries>,
    /// Sınır katmanı: only this layer's line work counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary_layer: Option<String>,
    /// Adalar: closed groups inside the region are holes; absent: true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub islands: Option<bool>,
    /// How the area looks and what it carries.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: NewLook,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// How Tarama finds the region around a point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchWay {
    /// Kapalı nesne (Netcad's): the smallest closed object around the point,
    /// the smaller closed objects inside or across its edge its islands; the
    /// hatch can follow them (`associative`).
    #[default]
    Object,
    /// Çizgiler (AutoCAD's): the face the boundaries' line work closes
    /// around the point, closed groups inside it its islands; it follows nothing.
    Lines,
}

/// Where Tarama hatches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchRegion {
    /// The region around `at`.
    Point {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        /// How the region is found; absent: `object`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        way: Option<HatchWay>,
        /// `lines`: whose line work closes the region; absent: every shown object.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        boundaries: Option<TrimBoundaries>,
        /// `lines`: only this layer's line work counts (Sınır katmanı).
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "boundaryLayer"
        )]
        #[cfg_attr(feature = "ts", ts(optional))]
        boundary_layer: Option<String>,
        /// Adalar: the islands are left unhatched; absent: true.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        islands: Option<bool>,
        /// Yazılar: the texts and inserts in the region are left open, each
        /// its box a little wider; absent: false.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        texts: Option<bool>,
        /// İlişkili (`object` only): the hatch follows its objects; absent: true.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        associative: Option<bool>,
    },
    /// Çoklu tara: each closed object (each part of a multi-part area)
    /// hatched by itself, from a point inside it.
    Objects {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        /// Adalar; absent: true.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        islands: Option<bool>,
        /// Yazılar; absent: false.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        texts: Option<bool>,
        /// İlişkili; absent: true.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        associative: Option<bool>,
    },
}

/// The pattern by its name, as Tarama's Desen lists it, and its options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPatternSpec {
    /// The pattern's name: Çizgili, Çapraz, Dolu, a library pattern (ANSI31,
    /// ISO02W100 …) or a gradient (Degrade doğrusal, silindir, küre); its
    /// case aside, Turkish, and a gradient by its shape's word too.
    pub name: String,
    /// Ölçek: on paper, times the project's plot scale; absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    /// Açı, degrees; absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    /// İkinci renk of a gradient, `#RRGGBB` or a colour's name; absent: white.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_color: Option<String>,
    /// Ters: a gradient's colours swapped; absent: false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub inverted: Option<bool>,
}

/// Input of `cad.entities.hatch` v1: the regions of `region` hatched with
/// `pattern` on `layerId`, one hatch each, as one undo step “Tarama”,
/// written by `cad.entities.create`'s rules. Çoklu tara's objects that are
/// not closed are left out with a `not_closed` warning at `region.uids[i]`.
///
/// Refusals (`CommandError.code`), checked in this order: the pattern's
/// (`unknown_pattern` at `pattern.name`, `invalid_scale` at `pattern.scale`
/// when not over 0, not finite or over 10 000, `not_finite` at
/// `pattern.angle`, `invalid_color` at `pattern.secondColor`), then the
/// region's (`not_finite` at `region.at` and the rectangle's corners;
/// `no_entities` when `region.uids` or `boundaries.uids` is empty,
/// `invalid_uid`), `invalid_revision`, `revision_conflict` (status
/// `conflict`), `entity_not_found`, `layer_not_found` (`boundaryLayer`),
/// `not_enclosed` (no region around `at`; Çoklu tara: the islands and the
/// texts cover every object), `not_closed` (none of the objects closed),
/// `pattern_too_dense` (the pattern too dense for a region), then the
/// layer's and the look's as `cad.entities.create` checks them; on the
/// desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesHatch {
    /// Where to hatch.
    pub region: HatchRegion,
    /// With what.
    pub pattern: HatchPatternSpec,
    /// The layer the hatches go to.
    pub layer_id: String,
    /// How the hatches look and what they carry.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: NewLook,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_region_reads_by_its_kind_and_the_look_flat() {
        let input: EntitiesHatch = serde_json::from_value(json!({
            "region": { "kind": "point", "at": { "x": 1.0, "y": 2.0 }, "way": "lines", "boundaryLayer": "sinir" },
            "pattern": { "name": "ANSI31", "scale": 2.0 },
            "layerId": "tarama", "color": "#E5484D"
        }))
        .expect("reads");
        assert!(matches!(
            &input.region,
            HatchRegion::Point { way: Some(HatchWay::Lines), boundary_layer: Some(l), .. } if l == "sinir"
        ));
        assert_eq!(input.look.color.as_deref(), Some("#E5484D"));
        let back = serde_json::to_value(&input).expect("writes");
        assert_eq!(back["region"]["boundaryLayer"], "sinir");
        assert_eq!(back["color"], "#E5484D");
    }
}
