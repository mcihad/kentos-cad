//! The drawing order command of the product command catalog (docs/adr/0260
//! §5): Öne getir and Arkaya gönder for objects named by their persistent
//! ids, as one undo step. An object's place in its layer's drawing order is
//! its `order` (absent 0), equals in the document's order; the command gives
//! the objects orders past every other object of their layer, keeping their
//! own order among themselves. The drawing order never crosses a layer.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// Brings objects to the front of their layers or sends them to the back.
pub const CAD_ENTITIES_ORDER: &str = "cad.entities.order";
pub const CAD_ENTITIES_ORDER_VERSION: u32 = 1;

/// Where the objects go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OrderTo {
    /// Öne getir: drawn after every other object of their layer. “Öne getir”.
    Front,
    /// Arkaya gönder: drawn before every other object of their layer. “Arkaya gönder”.
    Back,
}

/// Input of `cad.entities.order` v1: the objects' persistent ids and where
/// they go, as one undo step. For each layer the objects are on, the others'
/// highest order (to the front) or lowest (to the back) is found, absent
/// counting 0; the objects take the next orders past it, in their own
/// drawing order (order, then the document's). A layer whose objects
/// already are all in front of (or behind) its others does not change; when
/// nothing changes, nothing is written. The command reads no selection
/// (TODOS.md CMD-07): Sırala ▾ gives the selection's ids.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid`, `invalid_revision`, `revision_conflict` (status
/// `conflict`), `entity_not_found`, `layer_locked` (every object on a locked
/// layer: the first one's, in the input's order; when only some are, they are
/// left out with the warning `layer_locked`, docs/adr/0278), `order_limit` (a
/// layer's orders at the 32-bit bound, −2³¹ or 2³¹ − 1: no room past them).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOrder {
    /// The objects' persistent ids, at least one; a repeated one counts once.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// To the front or the back.
    pub to: OrderTo,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.order` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOrdered {
    /// The objects whose order changed, in the input's order.
    pub changed: Vec<String>,
    /// Their slots in the open document (`Entity.id`), in the same order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The document's revision after the write (the same when nothing changed; in a plan, now).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.order` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesOrderPlan {
    /// The objects that would change, each as execute would write it, with
    /// its own slot (`id`), in the input's order.
    pub changed: Vec<crate::entity::Entity>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
