//! The revocation list, `kentos.extension-revocations` v1 (plan/eklenti.md ADIM 17 and 22A; D15): the versions taken
//! back for a reason (malware, a dangerous bug). The catalog writes it from its releases
//! ([`crate::release::revocation_list`]); the extension host reads it and stops what it names, never deleting the
//! user's data. One format in one place, so the writer and the reader cannot drift apart.
//!
//! Unknown fields are refused, so a field added to the list is a new format version: every installed KentOS would
//! refuse a list that carried one. The list's signature is not a field either: the catalog's trust root checks it
//! before the list is read (ADIM 22A).

use serde::{Deserialize, Serialize};

use crate::problem::Problem;
use crate::version::{Version, VersionReq};

pub const REVOCATIONS_FORMAT: &str = "kentos.extension-revocations";
/// The `schemaVersion` this build reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

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
            Problem::new(
                "revocations_invalid",
                format!("Geri çekme listesi okunamadı ({e})."),
            )
        })?;
        if list.format != REVOCATIONS_FORMAT || list.schema_version != SCHEMA_VERSION {
            return Err(Problem::new(
                "revocations_invalid",
                format!("Geri çekme listesi “{REVOCATIONS_FORMAT}” sürüm 1 değil."),
            ));
        }
        for entry in &list.entries {
            if VersionReq::parse(&entry.versions).is_none() {
                return Err(Problem::new(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn list(entries: serde_json::Value) -> String {
        serde_json::json!({ "format": REVOCATIONS_FORMAT, "schemaVersion": 1, "entries": entries })
            .to_string()
    }

    #[test]
    fn a_range_withdraws_the_versions_in_it_and_nothing_else() {
        let text = list(serde_json::json!([
            { "id": "tr.ornek.a", "versions": "<1.4.2", "reason": "veri kaybına yol açıyor" },
            { "id": "tr.ornek.b", "versions": "=2.0.0", "reason": "kötü amaçlı kod" },
        ]));
        let revocations = Revocations::read(&text).unwrap();
        assert_eq!(
            revocations.reason("tr.ornek.a", "1.4.1"),
            Some("veri kaybına yol açıyor")
        );
        assert_eq!(revocations.reason("tr.ornek.a", "1.4.2"), None);
        assert_eq!(
            revocations.reason("tr.ornek.b", "2.0.0"),
            Some("kötü amaçlı kod")
        );
        assert_eq!(revocations.reason("tr.ornek.b", "2.0.1"), None);
        assert_eq!(revocations.reason("tr.ornek.c", "1.0.0"), None);
        assert_eq!(revocations.reason("tr.ornek.a", "sürüm değil"), None);
    }

    #[test]
    fn another_format_an_unknown_field_or_a_bad_range_is_refused() {
        let other =
            serde_json::json!({ "format": REVOCATIONS_FORMAT, "schemaVersion": 2, "entries": [] });
        let with_extra = serde_json::json!({ "format": REVOCATIONS_FORMAT, "schemaVersion": 1, "entries": [],
            "generatedAt": "2026-11-15T10:00:00Z" });
        let bad_range =
            list(serde_json::json!([{ "id": "tr.ornek.a", "versions": "yeni", "reason": "x" }]));
        for text in [other.to_string(), with_extra.to_string(), bad_range] {
            assert_eq!(
                Revocations::read(&text).unwrap_err().code,
                "revocations_invalid",
                "{text}"
            );
        }
    }
}
