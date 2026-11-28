//! `cad.delivery.edit` v1 (“Teslim kaydı”, docs/adr/8099; plan/bim-plan.md BIM-15): the drawing's delivery record
//! (`settings.arch.deliveries`). A package's delivery is added (its EK-2 revision, its publication's manifest, the
//! authority, the channel, the day), the authority's answer given (accepted, or returned with its reason and the review
//! notes that bind the reasons to objects), or a mistaken record taken away. One undo step “Teslim kaydı”; the
//! drawing's geometry never changes.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{Delivery, DeliveryChannel, DeliveryOutcome, DeliveryPackage};

/// The command's name.
pub const CAD_DELIVERY_EDIT: &str = "cad.delivery.edit";
/// Its version.
pub const CAD_DELIVERY_EDIT_VERSION: u32 = 1;

/// One change of the record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeliveryChange {
    /// A package went: its record, numbered one more than the largest; `id` given or made.
    #[serde(rename_all = "camelCase")]
    Add {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        revision: String,
        package: DeliveryPackage,
        to: String,
        #[serde(default)]
        channel: DeliveryChannel,
        at: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        receipt: Option<String>,
    },
    /// The authority's answer, in place of the one it has: accepted, or returned with its reason; the review notes
    /// (by id) that bind the reasons to objects.
    #[serde(rename_all = "camelCase")]
    Respond {
        id: String,
        outcome: DeliveryOutcome,
        at: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        notes: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        receipt: Option<String>,
    },
    /// A mistaken record taken away (its review notes stay).
    #[serde(rename_all = "camelCase")]
    Remove { id: String },
}

/// Input of `cad.delivery.edit` v1.
///
/// Refusals, in this order: `invalid_uid` (an id not a lowercase hyphenated UUID), `delivery_not_found`,
/// `duplicate_delivery` (an added id the drawing has), `review_note_not_found` (a response's note), `invalid_delivery`
/// (the record's checks: a revision of three digits, the manifest's SHA-256, the authority and days said, a return's
/// reason, an answer before the delivery, the texts' lengths; its field's path), `too_many_deliveries`,
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryEdit {
    pub change: DeliveryChange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.delivery.edit` v1, and its plan: the record changed (its id and number), the drawing's record then,
/// the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DeliveryEdited {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub deliveries: Vec<Delivery>,
    pub revision: String,
}
