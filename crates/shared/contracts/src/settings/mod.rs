//! Typed settings (TODOS.md §7 `SET-01..05`, docs/adr/0023): one schema for
//! every host. Each setting has a stable key, a type and domain, a default, a
//! unit, the scope it lives in, the hosts that use it, a version, Turkish
//! texts, a sensitive flag and how a change takes effect. The web app (in
//! TypeScript, from the generated `settingsSchema.json`) and the desktop app
//! (natively, from [`settings_schema`]) validate, resolve and store settings
//! by the same rules and report the same error codes; both run the shared
//! cases in `fixtures/settings/v1`.
//!
//! What is here and what is not:
//! - the schema ([`settings_schema`]), validation ([`SettingDescriptor::validate`],
//!   [`check_write`]), resolution ([`resolve`]) and the stored document
//!   ([`SettingsFile`]) are data and pure functions, with no storage;
//! - where a host keeps its layers (the browser's localStorage, the desktop's
//!   file), migrations from its older stores and what it can do (the GPU's
//!   sample counts) are the host's (docs/adr/0023).
//!
//! `cargo test -p kentos-contracts` compares the schema with
//! `apps/web/src/contracts/generated/settingsSchema.json` (and the settings
//! file's JSON Schema with `settingsFile.schema.json`); after a deliberate
//! change, `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings`
//! rewrites them (read the diff before committing).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

mod rules;
mod schema;

pub use rules::*;
pub use schema::settings_schema;

/// The stored settings document (`SettingsFile`): the web's localStorage
/// entry, the desktop's file and the export are all this.
pub const SETTINGS_FORMAT: &str = "kentos.settings";
pub const SETTINGS_VERSION: u32 = 1;
/// The schema document (`settingsSchema.json`).
pub const SETTINGS_SCHEMA_FORMAT: &str = "kentos.settings-schema";
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

/// A layer of values by key, as stored: what a layer holds may be invalid
/// until it is read through [`check_write`] or [`SettingsFile::from_json`].
pub type SettingsLayer = BTreeMap<String, Value>;

/// Where a setting lives and who may change it: the scope table of TODOS.md §7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingScope {
    /// Fixed by this version of the app: only the schema's default.
    Default,
    /// Set only by the organisation's policy; a lock or a bound on the other layers.
    Organization,
    /// The user's preference on this machine (an account-wide copy comes later).
    User,
    /// This device only (the GPU, the window); a project never carries it to another device.
    Device,
    /// Saved with the project (`.kcad`, cloud revision); no preference overrides it.
    Project,
    /// Until the app closes; never saved.
    Session,
}

/// The value's type. `enum` values are text from [`SettingDescriptor::choices`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingType {
    Boolean,
    Integer,
    Number,
    Enum,
}

/// How a change takes effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingApply {
    /// At once, nothing is rebuilt.
    Live,
    /// At once, by rebuilding resources (GPU targets, a backend); nothing is reopened.
    Recreate,
    /// When the app starts again.
    Restart,
}

/// An app that uses a setting. A file keeps the values other hosts use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingHost {
    Web,
    Desktop,
}

/// One value a setting may take, with its Turkish label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingChoice {
    #[cfg_attr(feature = "ts", ts(type = "boolean | number | string"))]
    pub value: Value,
    pub label: String,
}

/// One setting (TODOS.md SET-01).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingDescriptor {
    /// Stable dotted name, `group.name` in camelCase (`graphics.msaa`); never reused.
    pub key: String,
    #[serde(rename = "type")]
    pub kind: SettingType,
    #[cfg_attr(feature = "ts", ts(type = "boolean | number | string"))]
    pub default: Value,
    /// Smallest value of a number, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<f64>,
    /// Largest value of a number, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<f64>,
    /// The only values allowed, in the order they are offered: every enum
    /// has them; a number has them when only some steps make sense.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<SettingChoice>,
    /// Unit of a number: `px`, `deg`, `sample`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    pub scope: SettingScope,
    pub hosts: Vec<SettingHost>,
    /// Version of this setting's meaning and domain; an incompatible change is
    /// a new version with a migration of the stored values.
    pub version: u32,
    /// The group it is shown in ([`SettingsSchema::groups`]).
    pub group: String,
    /// Turkish name, as the settings window shows it.
    pub title: String,
    /// One or two Turkish sentences: what it changes.
    pub description: String,
    /// Never written to a settings file or an export (passwords, tokens; SET-08).
    pub sensitive: bool,
    pub apply: SettingApply,
}

/// A titled group of settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingGroup {
    pub id: String,
    pub title: String,
    pub description: String,
}

/// A named set of values (TODOS.md SET-05): choosing it only fills these
/// values, each stays changeable on its own; no setting stores the preset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsPreset {
    pub id: String,
    /// The group whose values it fills.
    pub group: String,
    pub title: String,
    pub description: String,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, boolean | number | string>"))]
    pub values: BTreeMap<String, Value>,
}

/// Every setting, as [`settings_schema`] builds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsSchema {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.settings-schema\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    pub groups: Vec<SettingGroup>,
    pub settings: Vec<SettingDescriptor>,
    pub presets: Vec<SettingsPreset>,
}

impl SettingsSchema {
    pub fn get(&self, key: &str) -> Option<&SettingDescriptor> {
        self.settings.iter().find(|s| s.key == key)
    }

    pub fn preset(&self, id: &str) -> Option<&SettingsPreset> {
        self.presets.iter().find(|p| p.id == id)
    }
}

/// Why a value was not taken, or why the effective value differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SettingErrorCode {
    /// No setting has this key.
    UnknownKey,
    /// Not the setting's type (text for a switch, a number for a choice).
    WrongType,
    /// A fraction where a whole number belongs.
    NotInteger,
    /// Below the minimum or above the maximum.
    OutOfRange,
    /// Not one of the setting's choices.
    NotAllowed,
    /// The setting does not live in this layer (a project setting in a
    /// preference, a device setting in the user layer).
    WrongScope,
    /// A sensitive value in a file: never read from or written to one.
    Sensitive,
    /// The text is not JSON.
    NotJson,
    /// JSON, but not a settings document (`format` ≠ `kentos.settings`).
    NotSettings,
    /// A settings document of a version this app does not read.
    UnsupportedVersion,
}

impl SettingErrorCode {
    /// What went wrong and how to fix it, in Turkish (CLAUDE.md §8).
    pub fn message(self) -> &'static str {
        match self {
            Self::UnknownKey => "Bu sürümde böyle bir ayar yok; değer saklanır ama kullanılmaz.",
            Self::WrongType => "Değerin türü yanlış; ayarın türünde bir değer girin.",
            Self::NotInteger => "Tam sayı olmalı; ondalık kısmı kaldırın.",
            Self::OutOfRange => "Değer izin verilen aralığın dışında; aralıktaki bir değer seçin.",
            Self::NotAllowed => "Bu değer seçeneklerden biri değil; listeden seçin.",
            Self::WrongScope => {
                "Bu ayar bu katmanda saklanmaz (örneğin proje ayarı kullanıcı tercihinde)."
            }
            Self::Sensitive => {
                "Gizli değerler ayar dosyasına yazılmaz ve dosyadan okunmaz; yeniden girin."
            }
            Self::NotJson => "Dosya JSON değil; KentOS'tan dışa aktarılmış bir ayar dosyası seçin.",
            Self::NotSettings => {
                "Bu bir KentOS ayar dosyası değil (format ≠ kentos.settings); doğru dosyayı seçin."
            }
            Self::UnsupportedVersion => {
                "Ayar dosyası bu uygulamanın okuyamadığı bir sürümde; uygulamayı güncelleyin."
            }
        }
    }
}

/// A value a layer holds but resolution did not use, or a problem in a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingDiagnostic {
    /// The layer it was found in.
    pub layer: SettingScope,
    pub key: String,
    pub code: SettingErrorCode,
}

/// One migration from an older store into the typed one (TODOS.md SET-04).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsMigration {
    /// The older store: `localStorage kentos.prefs.v1`, `~/.config/kentos-cad/ayarlar`.
    pub from: String,
    /// When, as RFC 3339 UTC text.
    pub at: String,
    /// Values taken over.
    pub moved: u32,
    /// Values left behind, and why; the older store keeps them untouched.
    pub dropped: Vec<SettingDiagnostic>,
}

/// The stored settings (TODOS.md SET-04): the layers a host keeps, and the
/// migrations that filled them. Session values are never stored; project
/// values live in the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsFile {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.settings\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    /// Preferences: user-scope settings.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub user: SettingsLayer,
    /// This device: device-scope settings, and user ones overridden here.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub device: SettingsLayer,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrations: Vec<SettingsMigration>,
}

impl Default for SettingsFile {
    fn default() -> Self {
        Self {
            format: SETTINGS_FORMAT.into(),
            version: SETTINGS_VERSION,
            user: SettingsLayer::new(),
            device: SettingsLayer::new(),
            migrations: Vec::new(),
        }
    }
}

/// A rule of the organisation's policy for one setting. No server sends
/// policies yet: hosts take one as an explicit input (docs/adr/0023).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PolicyRule {
    /// The value everyone gets; the control is shown locked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "boolean | number | string"))]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<f64>,
    /// The values that may be used (a subset of the setting's own).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(type = "Array<boolean | number | string>"))]
    pub allowed: Vec<Value>,
}

/// The organisation's policy: locks and upper bounds over every other layer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsPolicy {
    /// Who set it, as the interface names it ("Kurum politikası").
    pub source: String,
    pub rules: BTreeMap<String, PolicyRule>,
}

/// Why the effective value is not the requested one (TODOS.md SET-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ResolveReason {
    /// The policy fixes the value.
    OrganizationLocked,
    /// The policy bounds the value.
    OrganizationLimit,
    /// The device cannot use the value (8× MSAA where it has 4×, WebGPU in a browser without it).
    DeviceUnsupported,
    /// The device refused the value when it was applied; the last working one is in use.
    DeviceFailed,
}

/// What a host can use for one setting (TODOS.md AA-01): values outside
/// `allowed` fall to the nearest allowed one below (numbers) or the first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingConstraint {
    #[cfg_attr(feature = "ts", ts(type = "Array<boolean | number | string>"))]
    pub allowed: Vec<Value>,
    pub reason: ResolveReason,
    /// The reason in Turkish, for the interface ("Bu aygıt en çok 4× destekliyor.").
    pub detail: String,
}

/// A setting as resolved (TODOS.md SET-02, SET-03).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ResolvedSetting {
    pub key: String,
    /// What was asked for: the value of the most specific layer that holds a
    /// valid one. The preference is never lost when it cannot be used.
    #[cfg_attr(feature = "ts", ts(type = "boolean | number | string"))]
    pub requested: Value,
    /// The layer `requested` came from (`default` when none holds one).
    pub source: SettingScope,
    /// What is used.
    #[cfg_attr(feature = "ts", ts(type = "boolean | number | string"))]
    pub effective: Value,
    /// Why `effective` differs from `requested`; none when they are equal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<ResolveReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub detail: Option<String>,
    /// The policy fixes it: the control is shown locked.
    pub locked: bool,
}
