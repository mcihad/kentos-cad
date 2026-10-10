//! The label engine's commands of the product command catalog (docs/adr/0212
//! §5): `cad.layers.labels` writes how a layer is labelled (its single label
//! style, its rule-based classes, its objects as obstacles) and
//! `cad.labels.pin` moves, turns, hides or frees objects' labels by hand.
//! Each is one undo step. The Etiketler window and the label tools write
//! through them on the web (`apps/web/src/product`) and on the desktop
//! (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::labels::{LabelPin, LayerLabels};
use crate::layer::{LabelStyle, LayerNode};

/// Writes how a layer is labelled.
pub const CAD_LAYERS_LABELS: &str = "cad.layers.labels";
pub const CAD_LAYERS_LABELS_VERSION: u32 = 1;

/// Input of `cad.layers.labels` v1: a layer's labelling written as one undo
/// step “Etiketler”: its single label's style (`label`, the layer style's
/// `label`) and how it is labelled (`labels`: a single label, rule-based
/// classes or none, and its objects as obstacles). Each one absent is left
/// as it is; null takes it away (no `label`: the kinds' default labels; no
/// `labels`: a single label). A layer that already has them as given is
/// left as it is (`changed` false, no step).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_labels`
/// (the style's or the labelling's rules, `labels::style_problem` and
/// `labels::layer_labels_problem`), `invalid_expression` (a label's text or
/// a class's condition does not compile, or reads `$sıra` or `$ölçek`; the
/// message says which); then `invalid_revision`, `revision_conflict` (status
/// `conflict`), `layer_not_found`, `not_a_layer` (a group), `service_layer`
/// (a layer drawn from a service: it has no objects to label).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersLabels {
    /// The layer's id.
    pub layer: String,
    /// Its single label's style; null: the kinds' default labels. Absent: unchanged.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<LabelStyle>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<LabelStyle>"))]
    pub label: Option<Option<LabelStyle>>,
    /// How it is labelled; null: a single label. Absent: unchanged.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<LayerLabels>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<LayerLabels>"))]
    pub labels: Option<Option<LayerLabels>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.labels` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersLabelled {
    pub layer: String,
    /// Whether anything differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.labels` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersLabelsPlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Moves, turns, hides or frees objects' labels.
pub const CAD_LABELS_PIN: &str = "cad.labels.pin";
pub const CAD_LABELS_PIN_VERSION: u32 = 1;

/// One label pinned by hand, or freed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelPinChange {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Which of its labels: a rule-based layer's class (`LabelClass.name`);
    /// absent, its first label (a single label's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    /// Where the label stays (its middle from the object's anchor, metres),
    /// turned (degrees, counter-clockwise), or hidden; its own `class` is the
    /// change's (absent or the same). Null frees the label: it is placed again.
    pub pin: Option<LabelPin>,
}

/// Input of `cad.labels.pin` v1: labels moved, turned or hidden by hand, or
/// freed, as one undo step “Etiket”. A change that leaves a label's pin as
/// it is changes nothing; when none changes, nothing is written (`changed`
/// 0).
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities` (no
/// change), `invalid_uid`, `invalid_pin` (a pin against its rules, a pin
/// whose `class` is not its change's, a label changed twice, the pins an
/// object would have against their rules: `labels::pins_problem`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found`, `layer_locked`, `unknown_class` (a class its object's
/// layer does not label with: a rule-based layer's names it, a single
/// label's names none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelsPin {
    /// The changes, at least one.
    pub pins: Vec<LabelPinChange>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.labels.pin` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelsPinned {
    /// The objects whose pins changed (an undo step was written when any did).
    pub changed: u32,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// An object's pins as `cad.labels.pin` would leave them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PinnedObject {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub label_pins: Vec<LabelPin>,
}

/// What `cad.labels.pin` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelsPinPlan {
    /// The objects whose pins would change, in the input's order.
    pub objects: Vec<PinnedObject>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Absent, null and a value are three answers: unchanged, removed, set.
    #[test]
    fn a_labelling_absent_null_and_given_are_told_apart() {
        let input: LayersLabels = serde_json::from_value(json!({
            "layer": "parsel",
            "label": null,
            "labels": { "mode": "off", "obstacle": { "weight": 7 } }
        }))
        .expect("the input reads");
        assert_eq!(input.label, Some(None));
        assert!(matches!(input.labels, Some(Some(_))));
        let back = serde_json::to_value(&input).expect("writes");
        assert_eq!(back["label"], serde_json::Value::Null);
        let input: LayersLabels =
            serde_json::from_value(json!({ "layer": "parsel" })).expect("reads");
        assert_eq!((input.label, input.labels), (None, None));
        let pin: LabelsPin = serde_json::from_value(json!({
            "pins": [{ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "pin": null }]
        }))
        .expect("a freed label reads");
        assert_eq!(pin.pins[0].pin, None);
    }
}
