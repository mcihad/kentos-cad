//! The properties command of the product command catalog, `cad.entities.set`:
//! what objects are drawn with and what they carry — their layer, colour,
//! symbol, GIS attributes and label — set for objects named by their
//! persistent ids, as one undo step. Öznitelikler's Katman, Renk and Sembol
//! rows and its attribute rows, Sembol ver and Sembolü kaldır write through
//! it on the web (`apps/web/src/product`) and on the desktop
//! (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.
//!
//! A geometry is not a property: Öznitelikler's geometry rows and the
//! in-place text editor write through `cad.entities.edit`, operation
//! `properties`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Entity;

/// Sets the layer, colour, symbol, attributes or label of objects in one undo step.
pub const CAD_ENTITIES_SET: &str = "cad.entities.set";
pub const CAD_ENTITIES_SET_VERSION: u32 = 1;

/// What a change of properties is; it names the undo step, and nothing else:
/// it does not limit which properties the input sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PropertiesOperation {
    /// Katman ▾: the objects move to another layer. “Katman değiştir”.
    Layer,
    /// Renk ▾: their own colour, or their layer's. “Renk değiştir”.
    Color,
    /// Sembol ver and Sembolü kaldır: their own symbol, or their layer
    /// style's. “Sembol ata”; “Sembolü kaldır” when `symbol` is null.
    Symbol,
    /// An attribute row of Öznitelikler, with the label that shows the
    /// attribute (Parsel, Ada). “Değiştir”.
    Attributes,
    /// The label alone. “Etiket değiştir”.
    Label,
}

/// Input of `cad.entities.set` v1: properties set for objects named by their
/// persistent ids (docs/adr/0014), written as one undo step named after
/// `operation`. Everything the command depends on is here (TODOS.md CMD-07):
/// Öznitelikler fills `uids` from the selection and the property from the
/// row; the command reads no selection, library or view.
///
/// A property absent from the input stays as it is. `color`, `symbol` and
/// `label` are removed with null: the object is drawn in its layer's colour
/// and style again, and shows no label. An attribute is set by its name, or
/// removed with null; the attributes not named stay. What an object already
/// has is not a change: an object as the input asks is left alone and is
/// not in the output, and when no object changes nothing is written (no
/// undo step, the revision stays).
///
/// An object on a locked layer (by itself or a group above it) is not
/// changed, and no object moves to a locked layer: when one is named,
/// nothing is written and the answer is `layer_locked`. Objects move to a
/// hidden layer with the warning `layer_hidden`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `nothing_to_set` (no property given),
/// `invalid_attribute` (an attribute name empty or only white space), then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `layer_not_found` and
/// `not_a_layer` (the `layerId` given), `layer_locked` (each object's layer
/// in the input's order, then the `layerId` given). Warning: `layer_hidden`
/// (the `layerId` given is hidden and at least one object moves to it).
/// White space is Unicode's `White_Space` (Rust's `char::is_whitespace`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesSetProperties {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at
    /// least one; a repeated one counts once.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The layer they move to: a layer's id (`LayerNode.id`), not a group's.
    /// Absent: they stay on theirs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    /// Their own colour (`EntityBase.color`, as `#E5484D`); null: their
    /// layer's (katmana göre). Absent: unchanged.
    #[serde(default, with = "nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub color: Option<Option<String>>,
    /// Their own symbol, a library item's id (`EntityBase.symbol`), drawn
    /// instead of their layer's style; null: the layer's style. The id is
    /// not looked up: the libraries are the host's. Absent: unchanged.
    #[serde(default, with = "nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub symbol: Option<Option<String>>,
    /// Attributes by name, text in v1: a text sets the attribute (added when
    /// an object lacks it), null removes it. Absent or empty: none change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, Option<String>>>,
    /// The text shown beside them (`EntityBase.label`); null: none. Absent:
    /// unchanged.
    #[serde(default, with = "nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub label: Option<Option<String>>,
    /// What the change is; it names the undo step.
    pub operation: PropertiesOperation,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.set` v1: the objects that changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPropertiesSet {
    /// Their persistent ids, in the input's order (a repeated one once); an
    /// object that already was as asked is not among them.
    pub changed: Vec<String>,
    /// Their slots in the open document (`Entity.id`), in the same order;
    /// they mean nothing once the document is closed.
    pub ids: Vec<u32>,
    /// The document's revision after the write, as decimal text (the same
    /// when nothing changed). Inside an open transaction or group the write
    /// joins it, and the revision changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.set` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesSetPropertiesPlan {
    /// The objects that would change, each as execute would write it, with
    /// its own slot (`id`), in the input's order.
    pub changed: Vec<Entity>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A field that is absent (`None`: unchanged), null (`Some(None)`: removed)
/// or a value (`Some(Some(v))`). Plain serde reads null as absent.
mod nullable {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize>(
        value: &Option<Option<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // An absent field is skipped (`skip_serializing_if`) before it gets here.
        value.as_ref().and_then(Option::as_ref).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Absent, null and a value are three answers: unchanged, removed, set.
    #[test]
    fn absent_null_and_a_value_are_told_apart() {
        let input: EntitiesSetProperties = serde_json::from_value(json!({
            "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
            "color": null,
            "symbol": "agac",
            "attrs": { "Ada": "101", "Eski": null },
            "operation": "color"
        }))
        .expect("the input reads");
        assert_eq!(input.color, Some(None));
        assert_eq!(input.symbol, Some(Some("agac".into())));
        assert_eq!(input.label, None);
        assert_eq!(input.layer_id, None);
        let attrs = input.attrs.as_ref().expect("attrs");
        assert_eq!(attrs.get("Ada"), Some(&Some("101".into())));
        assert_eq!(attrs.get("Eski"), Some(&None));
        // Written back as it was read: null stays null, absent stays absent.
        assert_eq!(
            serde_json::to_value(&input).expect("the input writes"),
            json!({
                "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                "color": null,
                "symbol": "agac",
                "attrs": { "Ada": "101", "Eski": null },
                "operation": "color"
            })
        );
    }
}
