//! Where a `project.audit` job takes its rule from (docs/adr/8057 §42): a published release of the extension catalog
//! (docs/adr/8059) and the Python environment the server prepared for it. The catalog is the host's (`kentosd`'s): the
//! worker asks through [`AuditSource`], which the host gives with its processor (`Processor::audits`).
//!
//! - **The release** ([`AuditSource::release`]): the organisation's catalog first, the public one only when the
//!   organisation's does not know the extension (the same id there may be another publisher's); the version asked when
//!   it may run (published, or retired after it was), else the newest published one. Its package bytes are the
//!   approved content's, checked by the catalog before they leave.
//! - **The interpreter** ([`AuditPythons`]): a mapping of an extension's version to the interpreter of the environment
//!   prepared for it (`extension@version` → an absolute path): the server setting's, under the file the server writes
//!   when it prepares one (docs/adr/8057 §66), the file winning. A version without one cannot run: the job says so
//!   (`tool_unavailable`) rather than running the rule with some other Python.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

/// A source call's future, boxed so the source can be a trait object.
pub type AuditFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A release a job runs: where it came from, its version, its content digest and its package's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditPackage {
    /// `tenant:{slug}` or `public`.
    pub catalog: String,
    pub version: String,
    /// `sha256:…`
    pub digest: String,
    pub bytes: Vec<u8>,
}

/// Where audit rules come from: the host's catalog and its prepared environments.
pub trait AuditSource: Send + Sync {
    /// The release of `extension` a job of the organisation `tenant` (its short name) runs: `version` when given and it
    /// may run, else the newest published one; why not, in a sentence the job says.
    fn release<'a>(
        &'a self,
        tenant: &'a str,
        extension: &'a str,
        version: Option<&'a str>,
    ) -> AuditFuture<'a, Result<AuditPackage, String>>;

    /// The interpreter of the environment prepared for `extension` at `version`; none when there is none.
    fn python(&self, extension: &str, version: &str) -> Option<String>;
}

/// The prepared environments' interpreters, by `extension@version`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditPythons(BTreeMap<String, String>);

impl AuditPythons {
    /// The mapping a server setting gives: a JSON object of `"extension@version": "/absolute/path/to/python"`. Refused,
    /// with where, when it is not one or a path is not absolute.
    pub fn parse(text: &str) -> Result<AuditPythons, String> {
        let map: BTreeMap<String, String> = serde_json::from_str(text)
            .map_err(|e| format!("Denetim ortamlarının listesi bir JSON nesnesi değil: {e}"))?;
        for (key, path) in &map {
            if key
                .split_once('@')
                .is_none_or(|(e, v)| e.is_empty() || v.is_empty())
            {
                return Err(format!(
                    "“{key}”: anahtar eklenti@sürüm olmalı (tr.ornek.denetim@1.0.0)."
                ));
            }
            if !std::path::Path::new(path).is_absolute() {
                return Err(format!(
                    "“{key}”: yorumlayıcının yolu mutlak olmalı ({path})."
                ));
            }
        }
        Ok(AuditPythons(map))
    }

    /// The interpreter prepared for `extension` at `version`.
    pub fn get(&self, extension: &str, version: &str) -> Option<String> {
        self.0.get(&format!("{extension}@{version}")).cloned()
    }

    /// These with `over`'s in their place where both name a version: the file the server writes over the setting.
    pub fn overlaid(mut self, over: &AuditPythons) -> AuditPythons {
        self.0
            .extend(over.0.iter().map(|(k, v)| (k.clone(), v.clone())));
        self
    }

    /// `extension` at `version` mapped to `python` (an absolute path), in place of what it was.
    pub fn insert(&mut self, extension: &str, version: &str, python: &str) {
        self.0
            .insert(format!("{extension}@{version}"), python.to_owned());
    }

    /// `extension` at `version` no longer mapped; whether it was.
    pub fn remove(&mut self, extension: &str, version: &str) -> bool {
        self.0.remove(&format!("{extension}@{version}")).is_some()
    }

    /// Every mapping, `extension@version` and its interpreter, in order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// As the setting and the file write it: a JSON object, in order, one mapping a line.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(&self.0).unwrap_or_else(|_| "{}".to_owned());
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The setting's mapping: by extension and version, absolute paths only.
    #[test]
    fn the_prepared_interpreters_are_found_by_extension_and_version() {
        let pythons = AuditPythons::parse(
            r#"{"tr.ornek.denetim@1.0.0": "/srv/kentos/python/tr.ornek.denetim-1.0.0/bin/python"}"#,
        )
        .unwrap();
        assert_eq!(
            pythons.get("tr.ornek.denetim", "1.0.0").as_deref(),
            Some("/srv/kentos/python/tr.ornek.denetim-1.0.0/bin/python")
        );
        assert_eq!(pythons.get("tr.ornek.denetim", "1.1.0"), None);
        assert!(AuditPythons::parse(r#"{"tr.ornek.denetim": "/p"}"#).is_err());
        assert!(AuditPythons::parse(r#"{"a@1": "python3"}"#).is_err());
        assert!(AuditPythons::parse("[]").is_err());
        assert_eq!(AuditPythons::parse("{}").unwrap(), AuditPythons::default());
    }

    /// The file over the setting: the file's mapping wins for the same version, the setting's others stay; what is
    /// written reads back the same.
    #[test]
    fn the_file_s_interpreters_win_over_the_setting_s() {
        let setting =
            AuditPythons::parse(r#"{"a@1": "/eski/bin/python", "b@1": "/b/bin/python"}"#).unwrap();
        let mut file = AuditPythons::default();
        file.insert("a", "1", "/yeni/bin/python");
        file.insert("c", "2", "/c/bin/python");
        let merged = setting.overlaid(&file);
        assert_eq!(merged.get("a", "1").as_deref(), Some("/yeni/bin/python"));
        assert_eq!(merged.get("b", "1").as_deref(), Some("/b/bin/python"));
        assert_eq!(merged.get("c", "2").as_deref(), Some("/c/bin/python"));
        assert_eq!(AuditPythons::parse(&merged.to_json()).unwrap(), merged);
        assert_eq!(merged.entries().count(), 3);
        file.remove("c", "2");
        assert!(!file.remove("c", "2"));
        assert_eq!(file.get("c", "2"), None);
    }
}
