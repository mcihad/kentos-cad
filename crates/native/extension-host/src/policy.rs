//! What the organisation allows and what has been withdrawn (plan/eklenti.md ADIM 17; D14, D15).
//!
//! The **policy** is IT's file in a system folder the user cannot write (`kentos.extension-policy` v1): sources,
//! publishers and extensions allowed, extensions and capabilities blocked, the least version of an extension, whether
//! data may go to an outside AI service. The user cannot loosen it; an installed extension it now refuses is stopped
//! with the reason, its data kept.
//!
//! The **revocation list** (`kentos.extension-revocations` v1) names versions withdrawn for a reason (malware, a
//! dangerous bug); syncing it stops them, never deletes the user's data. Its signature is checked by the catalog's
//! trust root before it gets here (ADIM 22A); offline, the last list applied holds.

use kentos_extensions::Problem;
use kentos_extensions::manifest::{Capability, Manifest};
use kentos_extensions::version::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::store::{SourceKind, problem};

pub const POLICY_FORMAT: &str = "kentos.extension-policy";
pub const REVOCATIONS_FORMAT: &str = "kentos.extension-revocations";

/// The organisation's extension policy. Every list not written allows all.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub format: String,
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_sources: Option<Vec<SourceKind>>,
    /// Publishers by `publisherId` (verified by the catalog).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_publishers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_capabilities: Vec<Capability>,
    /// An extension's least version: older ones are stopped (a known bug).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub minimum_versions: std::collections::BTreeMap<String, String>,
    /// Whether an extension may send data to an outside AI service (`network.sendsDataToAi`); not written: yes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_data_to_ai: Option<bool>,
    /// The policy file is there but cannot be read: everything is refused with this reason (fail closed).
    #[serde(skip)]
    pub broken: Option<String>,
}

impl Policy {
    /// Reads the policy file.
    pub fn read(text: &str) -> Result<Policy, Problem> {
        let policy: Policy = serde_json::from_str(text).map_err(|e| {
            problem(
                "policy_invalid",
                format!("Kurum eklenti ilkesi okunamadı ({e}); bilgi işlem birimine bildirin. Eklenti kurulmaz."),
            )
        })?;
        if policy.format != POLICY_FORMAT || policy.schema_version != 1 {
            return Err(problem(
                "policy_invalid",
                format!("Kurum eklenti ilkesi “{POLICY_FORMAT}” sürüm 1 değil; eklenti kurulmaz."),
            ));
        }
        Ok(policy)
    }

    /// Why the policy refuses this package from this source; none when it allows it.
    pub fn refusal(&self, manifest: &Manifest, source: SourceKind) -> Option<String> {
        if let Some(reason) = &self.broken {
            return Some(reason.clone());
        }
        let id = &manifest.id;
        if self.blocked_extensions.iter().any(|b| b == id) {
            return Some(format!("Kurum ilkesi “{id}” eklentisini engelliyor."));
        }
        if let Some(allowed) = &self.allowed_extensions
            && !allowed.iter().any(|a| a == id)
        {
            return Some(format!(
                "Kurum ilkesi yalnız izin verdiği eklentileri kurduruyor; “{id}” listede yok."
            ));
        }
        if let Some(sources) = &self.allowed_sources
            && !sources.contains(&source)
        {
            return Some(format!(
                "Kurum ilkesi {} kaynağından eklenti kurdurmuyor.",
                source.words()
            ));
        }
        if let Some(publishers) = &self.allowed_publishers
            && !manifest
                .publisher_id
                .as_ref()
                .is_some_and(|p| publishers.contains(p))
        {
            return Some(format!(
                "Kurum ilkesi yalnız izin verdiği yayımcıların eklentilerini kurduruyor; bu paketin yayımcısı “{}”.",
                manifest.publisher_id.as_deref().unwrap_or("belirtilmemiş")
            ));
        }
        if let Some(capability) = manifest
            .requested_capabilities
            .iter()
            .map(|r| r.id)
            .find(|c| self.blocked_capabilities.contains(c))
        {
            return Some(format!(
                "Kurum ilkesi bu eklentinin istediği bir yeteneği yasaklıyor: {} ({}).",
                capability.title(),
                capability.id()
            ));
        }
        if self.allow_data_to_ai == Some(false)
            && manifest
                .requested_capabilities
                .iter()
                .any(|r| r.sends_data_to_ai == Some(true))
        {
            return Some(
                "Kurum ilkesi dış yapay zekâ servislerine veri gönderen eklentilere izin vermiyor."
                    .to_owned(),
            );
        }
        if let Some(least) = self
            .minimum_versions
            .get(id)
            .and_then(|v| Version::parse(v))
            && manifest.parsed_version().is_some_and(|v| v < least)
        {
            return Some(format!(
                "Kurum ilkesi “{id}” için en az {least} sürümünü istiyor."
            ));
        }
        None
    }
}

impl Policy {
    /// A policy file that is there but cannot be read: it refuses everything, saying why (an unreadable policy must not
    /// mean "no policy").
    pub fn broken(reason: String) -> Policy {
        Policy {
            format: POLICY_FORMAT.to_owned(),
            schema_version: 1,
            broken: Some(format!(
                "{reason} Bilgi işlem birimine bildirin; eklentiler bu arada çalışmaz."
            )),
            ..Policy::default()
        }
    }
}

/// A withdrawn range of an extension's versions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Revoked {
    pub id: String,
    /// A range: `<1.4.2`, `=1.4.0`.
    pub versions: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Revocations {
    pub format: String,
    pub schema_version: u32,
    #[serde(default)]
    pub entries: Vec<Revoked>,
}

impl Revocations {
    pub fn read(text: &str) -> Result<Revocations, Problem> {
        let list: Revocations = serde_json::from_str(text).map_err(|e| {
            problem(
                "revocations_invalid",
                format!("Geri çekme listesi okunamadı ({e})."),
            )
        })?;
        if list.format != REVOCATIONS_FORMAT || list.schema_version != 1 {
            return Err(problem(
                "revocations_invalid",
                format!("Geri çekme listesi “{REVOCATIONS_FORMAT}” sürüm 1 değil."),
            ));
        }
        for entry in &list.entries {
            if VersionReq::parse(&entry.versions).is_none() {
                return Err(problem(
                    "revocations_invalid",
                    format!(
                        "Geri çekme listesinde “{}” için sürüm aralığı okunamadı: {}.",
                        entry.id, entry.versions
                    ),
                ));
            }
        }
        Ok(list)
    }

    /// Why this version is withdrawn; none when it is not. A preview inside a withdrawn range is withdrawn too.
    pub fn reason(&self, id: &str, version: &str) -> Option<&str> {
        let version = Version::parse(version)?;
        self.entries
            .iter()
            .filter(|e| e.id == id)
            .find(|e| VersionReq::parse(&e.versions).is_some_and(|req| req.contains(&version)))
            .map(|e| e.reason.as_str())
    }
}
