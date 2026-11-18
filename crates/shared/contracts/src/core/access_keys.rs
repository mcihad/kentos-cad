//! Programs signing in to the cloud (docs/adr/9000 §12): the desktop, a mobile app or an AI agent. A program signs in
//! with an account's login and password and gets a session as a bearer token (`Authorization: Bearer kentos_s_…`),
//! the way a browser gets the session cookie. An account also makes personal access tokens
//! (`Authorization: Bearer kentos_pat_…`) for agents and scripts: named, read-only or read-write, expiring, revocable.
//! Every token is shown once and kept only as its SHA-256.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cloud::Me;

/// `POST /v1/auth/token`: a program signs in with a local account.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TokenRequest {
    pub login: String,
    pub password: String,
    /// What the program is: `desktop`, `mobile`, `agent`, `python`, … (lower-case letters, digits and `-`).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub client: Option<String>,
    /// The device or the program's name as its person knows it ("Ofis bilgisayarı").
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub label: Option<String>,
}

/// The session a program got: its token (once), when it ends at the latest, and who signed in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TokenIssued {
    /// `kentos_s_` and 64 hexadecimal digits; sent as `Authorization: Bearer …`.
    pub token: String,
    /// RFC 3339. Each use moves the idle limit; the session never outlives this.
    pub expires_at: String,
    pub me: Me,
}

/// One of the account's sessions: a browser's (cookie) or a program's (bearer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SessionView {
    pub id: String,
    /// `cookie` (a browser) or `bearer` (a program).
    pub kind: String,
    pub client: Option<String>,
    pub label: Option<String>,
    /// `local` or `oidc`.
    pub method: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
    /// The session this request came with.
    pub current: bool,
}

/// What a personal access token may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TokenScope {
    /// Reads only: `GET` and `HEAD`.
    Read,
    /// Reads and writes as its account may.
    Write,
}

/// `POST /v1/me/tokens`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApiTokenCreate {
    pub name: String,
    pub scope: TokenScope,
    /// 1–365 days.
    pub expires_in_days: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApiTokenView {
    pub id: String,
    pub name: String,
    /// The token's first characters, to recognise it by (`kentos_pat_3f9a…`).
    pub prefix: String,
    pub scope: TokenScope,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
}

/// A new personal access token: shown this once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApiTokenCreated {
    /// `kentos_pat_` and 64 hexadecimal digits.
    pub token: String,
    pub view: ApiTokenView,
}
