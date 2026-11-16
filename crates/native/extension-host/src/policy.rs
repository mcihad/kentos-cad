//! What the organisation allows and what has been withdrawn (plan/eklenti.md ADIM 17; D14, D15).
//!
//! The **policy** is IT's file in a system folder the user cannot write (`kentos.extension-policy` v1): sources,
//! publishers and extensions allowed, extensions and capabilities blocked, the least version of an extension, whether
//! data may go to an outside AI service, and the Python packages an extension may bring: those blocked, the only ones
//! allowed, the package indexes they may come from (D14; docs/adr/8057 §84). The user cannot loosen it; an installed
//! extension it now refuses is stopped with the reason, its data kept.
//!
//! The **revocation list** (`kentos.extension-revocations` v1) names versions withdrawn for a reason (malware, a
//! dangerous bug); syncing it stops them, never deletes the user's data. Its signature is checked by the catalog's
//! trust root before it gets here (ADIM 22A); offline, the last list applied holds. Its format is the shared crate's,
//! the same the catalog writes the list with (`kentos_extensions::revocations`); it is named here as before.

use kentos_extensions::Problem;
use kentos_extensions::manifest::{Capability, Manifest};
pub use kentos_extensions::revocations::{REVOCATIONS_FORMAT, Revocations, Revoked};
use kentos_extensions::version::Version;
use serde::{Deserialize, Serialize};

use crate::store::{SourceKind, problem};

pub const POLICY_FORMAT: &str = "kentos.extension-policy";

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
    /// Python packages no extension may install, by name (compared as PEP 503 normalises them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_python_packages: Vec<String>,
    /// When written, the only Python packages an extension may install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_python_packages: Option<Vec<String>>,
    /// When written, the only package indexes a Python package may come from (`https://pypi.org/simple`, the
    /// organisation's mirror); a package from a repository or an address is then refused too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_package_indexes: Option<Vec<String>>,
    /// The policy file is there but cannot be read: everything is refused with this reason (fail closed).
    #[serde(skip)]
    pub broken: Option<String>,
}

impl Policy {
    /// Reads the policy file.
    pub fn read(text: &str) -> Result<Policy, Problem> {
        let policy: Policy = serde_json::from_str(text).map_err(|e| {
            let said = e.to_string();
            // A field this KentOS does not know: the policy was written for a newer one (a rule added later, such as
            // the Python packages', docs/adr/8057 §84). Said so in Turkish, not in the reader's English; still closed.
            let unknown = said
                .strip_prefix("unknown field `")
                .and_then(|rest| rest.split('`').next());
            let why = match unknown {
                Some(field) => format!(
                    "bu KentOS'un tanımadığı “{field}” alanı var; ilke daha yeni bir KentOS sürümü için yazılmış \
                     olabilir"
                ),
                None => format!("satır {}, sütun {}: JSON olarak okunamadı", e.line(), e.column()),
            };
            problem(
                "policy_invalid",
                format!("Kurum eklenti ilkesi okunamadı ({why}); bilgi işlem birimine bildirin. Eklenti kurulmaz."),
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
    /// Why the policy refuses a package that installs these Python packages (its lock's, by normalised name; docs/adr/
    /// 8057 §84); none when it allows them all.
    pub fn package_refusal(
        &self,
        packages: &std::collections::BTreeMap<String, kentos_extensions::release::LockedPackage>,
    ) -> Option<String> {
        use kentos_extensions::python_names::normalized;
        if let Some(reason) = &self.broken {
            return Some(reason.clone());
        }
        let listed = |list: &[String], name: &str| list.iter().any(|n| normalized(n) == name);
        for (name, p) in packages {
            if listed(&self.blocked_python_packages, name) {
                return Some(format!("Kurum ilkesi “{name}” Python paketini engelliyor."));
            }
            if let Some(allowed) = &self.allowed_python_packages
                && !listed(allowed, name)
            {
                return Some(format!(
                    "Kurum ilkesi yalnız izin verdiği Python paketlerini kurduruyor; “{name}” listede yok."
                ));
            }
            if let Some(indexes) = &self.allowed_package_indexes {
                let from = p
                    .source
                    .strip_prefix("registry ")
                    .map(|url| url.trim_end_matches('/'));
                let allowed =
                    from.is_some_and(|url| indexes.iter().any(|i| i.trim_end_matches('/') == url));
                if !allowed {
                    return Some(format!(
                        "Kurum ilkesi “{name}” paketinin geldiği yere ({}) izin vermiyor; izinli paket depoları: {}.",
                        p.source,
                        indexes.join(", ")
                    ));
                }
            }
        }
        None
    }

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
