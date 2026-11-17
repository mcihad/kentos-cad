//! Notifications that leave the portal (docs/adr/9000 §17): an e-mail or an SMS beside the bell. The platform's
//! administrator turns each channel on; each person chooses for themselves.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// `GET /v1/me/notification-settings`: how the account's notifications also reach it, and what reached it lately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NotificationSettings {
    /// The account's own choice: e-mail too (on unless turned off).
    pub email: bool,
    /// SMS too (off until turned on with a number).
    pub sms: bool,
    pub phone: Option<String>,
    /// Where the e-mails go: the account's address; none, no e-mail.
    pub email_address: Option<String>,
    /// Only a verified address gets e-mail: an unverified one is whoever typed it.
    pub email_verified: bool,
    /// Whether the platform sends e-mail at all.
    pub email_offered: bool,
    pub sms_offered: bool,
    /// The account's last deliveries, newest first.
    pub recent: Vec<DeliveryView>,
}

/// `PUT /v1/me/notification-settings`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NotificationSettingsEdit {
    pub email: bool,
    pub sms: bool,
    /// `+905321234567` or `05321234567`; spaces, dashes and brackets are dropped. Needed for SMS.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub phone: Option<String>,
}

/// One notification's way to the account by e-mail or SMS.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DeliveryView {
    /// `email` or `sms`.
    pub channel: String,
    pub address: String,
    /// `pending` (on its way or to be tried again), `sent` or `failed` (given up).
    pub state: String,
    pub title: String,
    pub attempts: u32,
    pub created_at: String,
    pub sent_at: Option<String>,
    pub last_error: Option<String>,
}

/// `PUT /v1/platform/settings`: what changes; a field left out stays as it is.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformSettingsEdit {
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub signup_open: Option<bool>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub email_notifications: Option<bool>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub sms_notifications: Option<bool>,
}
