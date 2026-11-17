//! How long the portal keeps personal data, and the runs of its periodic destruction (docs/adr/9000 §21; 6698 sayılı
//! KVKK and its regulation on deleting, destroying and anonymising personal data).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The periods, in days.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetentionSettings {
    /// A closed dossier moves to the archive after this (30–3650; it is kept, not deleted).
    pub archive_after_days: u32,
    /// A draft never sent and untouched this long is destroyed with its files (30–3650).
    pub draft_keep_days: u32,
    /// Notifications older than this are destroyed (30–3650).
    pub notification_keep_days: u32,
    /// E-mail / SMS deliveries done and signing requests that left no signature, older than this (7–3650).
    pub technical_keep_days: u32,
}

/// One run of the periodic destruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetentionRunView {
    pub id: String,
    /// RFC 3339.
    pub at: String,
    /// `schedule` (the server's daily run) or `manual`.
    pub trigger: String,
    pub actor_name: Option<String>,
    pub archived: u32,
    pub drafts_deleted: u32,
    pub files_deleted: u32,
    pub notifications_deleted: u32,
    pub deliveries_deleted: u32,
    pub signing_requests_deleted: u32,
    /// The periods it ran under.
    pub settings: RetentionSettings,
}

/// `GET /v1/platform/retention`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetentionPage {
    pub settings: RetentionSettings,
    /// The latest runs, newest first.
    pub runs: Vec<RetentionRunView>,
}
