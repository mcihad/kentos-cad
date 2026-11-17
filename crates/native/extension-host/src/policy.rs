//! What the organisation allows and what has been withdrawn (plan/eklenti.md ADIM 17; D14, D15).
//!
//! The **policy** is IT's file in a system folder the user cannot write (`kentos.extension-policy` v1): sources,
//! publishers and extensions allowed, extensions and capabilities blocked, the least version of an extension, whether
//! data may go to an outside AI service, and the Python packages an extension may bring: those blocked, the only ones
//! allowed, the package indexes they may come from (D14; docs/adr/8057 §84). The user cannot loosen it; an installed
//! extension it now refuses is stopped with the reason, its data kept. Whether data may go to an outside AI service
//! binds the desktop's own AI agent too, with the addresses it may reach (docs/adr/8056 §7).
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
    /// Whether data may go to an outside AI service: an extension's (`network.sendsDataToAi`) and the drawing the
    /// desktop's AI agent reads; not written: yes. An agent on this computer sends nothing out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_data_to_ai: Option<bool>,
    /// When written, the only addresses the desktop's AI agent may reach besides this computer (the organisation's own
    /// server, `https://yz.ornek.bel.tr/v1`; a provider's): each one with what follows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_ai_addresses: Option<Vec<String>>,
    /// Whether a drawing's attribute values (an owner's name, an identity number, an address) may go to an AI service
    /// outside this computer that the policy otherwise allows (docs/adr/8057 §103, PY-AI-13); not written: yes. When
    /// not, the desktop's AI agent outside this computer reads the drawing as if every field were hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_attribute_values_to_ai: Option<bool>,
    /// Whether the desktop's AI agent may run Python it wrote, its workshop's extension in a trial or loaded over the
    /// installed one (docs/adr/8057 §104, PY-AI-11); not written: yes, as the user sets it for each agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_agent_python: Option<bool>,
    /// Python packages no extension may install, by name (compared as PEP 503 normalises them), or one version of
    /// one (`ultralytics==8.3.41`: a release taken over and withdrawn, the others allowed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_python_packages: Vec<String>,
    /// When written, the only Python packages an extension may install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_python_packages: Option<Vec<String>>,
    /// When written, the only package indexes a Python package may come from (`https://pypi.org/simple`, the
    /// organisation's mirror); a package from a repository or an address is then refused too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_package_indexes: Option<Vec<String>>,
    /// The organisation's own Python packages and the one index each may come from: a name, or a prefix ending in `*`
    /// (`belediye-*`), → the index (plan/eklenti.md PY-UV-26: "özel paket adı başka genel dizinden sessizce
    /// çözülmez"; docs/adr/8057 §118). A lock taking such a name from anywhere else is refused: the same name published
    /// on a public index is how a dependency confusion attack gets in, and an index list allowing both cannot tell.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pinned_package_indexes: std::collections::BTreeMap<String, String>,
    /// When written, the least days a Python package must have been on its index when it is installed (its newest
    /// file's `upload-time` in the lock): a malicious release is usually found and taken down within days (D18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_package_age_days: Option<u32>,
    /// Whether an extension's code may be sent from this computer to run on the KentOS server: an İşlemler tool of an
    /// extension run as a server job (docs/adr/8057 §96, PY-OPT-12); not written: yes. The user's choice of the place
    /// does not widen it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_extensions_on_server: Option<bool>,
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
    /// Why the policy keeps the İşlemler tool `tool` off the KentOS server, in a few words for the place's row
    /// (docs/adr/8057 §96): an extension's tool (`ext.…`) when extensions' code may not go there, or when the policy
    /// cannot be read (fail closed); none for KentOS's own tools and when it may.
    pub fn server_refusal(&self, tool: &str) -> Option<&'static str> {
        if !tool.starts_with("ext.") {
            return None;
        }
        if self.broken.is_some() {
            return Some("kurum ilkesi okunamıyor");
        }
        (self.allow_extensions_on_server == Some(false))
            .then_some("kurum ilkesi eklenti kodunun sunucuda çalışmasına izin vermiyor")
    }

    /// Why the policy keeps the desktop's AI agent `name` from sending a drawing to `address` (docs/adr/8056 §7); none
    /// when it may. An agent on this computer (`local`) sends nothing out and is never refused; a policy that cannot be
    /// read refuses every other.
    pub fn agent_refusal(&self, name: &str, address: &str, local: bool) -> Option<String> {
        if local {
            return None;
        }
        let elsewhere = "Bu bilgisayarda çalışan bir sunucuyla (Ollama, LM Studio) bir ajan tanımlayın ya da bilgi \
                         işlem biriminize danışın.";
        if self.broken.is_some() {
            return Some(format!(
                "Kurum ilkesi okunamıyor; ilke düzelene dek çizim bu bilgisayarın dışındaki yapay zekâ hizmetlerine \
                 gönderilmez (“{name}” {address} adresine bağlanıyor). {elsewhere}"
            ));
        }
        let listed = self
            .allowed_ai_addresses
            .as_ref()
            .map(|list| list.iter().any(|a| address_within(address, a)));
        if listed == Some(false) {
            let list = self
                .allowed_ai_addresses
                .as_deref()
                .unwrap_or_default()
                .join(", ");
            return Some(format!(
                "Kurum ilkesi yapay zekâ ajanlarının yalnız şu adreslere bağlanmasına izin veriyor: {list}. “{name}\
                 ” {address} adresine bağlanıyor. {elsewhere}"
            ));
        }
        if self.allow_data_to_ai == Some(false) && listed != Some(true) {
            return Some(format!(
                "Kurum ilkesi çizim verisinin bu bilgisayarın dışındaki yapay zekâ hizmetlerine gitmesine izin \
                 vermiyor; “{name}” {address} adresine bağlanıyor. {elsewhere}"
            ));
        }
        None
    }

    /// Why the policy lets no AI agent run the Python it wrote (docs/adr/8057 §104), wherever its model runs: the code
    /// runs here either way; none when it may. A policy that cannot be read refuses it.
    pub fn agent_python_refusal(&self) -> Option<String> {
        if self.broken.is_some() {
            return Some(
                "Kurum ilkesi okunamıyor; ilke düzelene dek yapay zekâ ajanları yazdıkları kodu çalıştıramaz."
                    .to_owned(),
            );
        }
        (self.allow_agent_python == Some(false)).then(|| {
            "Kurum ilkesi yapay zekâ ajanlarının yazdıkları kodu bu bilgisayarda çalıştırmasına izin vermiyor."
                .to_owned()
        })
    }

    /// Whether the policy keeps a drawing's attribute values from the desktop's AI agent (docs/adr/8057 §103): one
    /// outside this computer when `allowAttributeValuesToAi` is false; a policy that cannot be read keeps them too (it
    /// keeps that agent out altogether: [`Policy::agent_refusal`]).
    pub fn withholds_attribute_values(&self, local: bool) -> bool {
        !local && (self.broken.is_some() || self.allow_attribute_values_to_ai == Some(false))
    }

    /// Why the policy refuses a package that installs these Python packages (its lock's, by normalised name; docs/adr/
    /// 8057 §84); none when it allows them all. `installing`: the moment a package is reviewed or installed, when the
    /// packages' age is judged too (§85); an installed extension is not stopped by its packages' age.
    pub fn package_refusal(
        &self,
        packages: &std::collections::BTreeMap<String, kentos_extensions::release::LockedPackage>,
        installing: Option<std::time::SystemTime>,
    ) -> Option<String> {
        use kentos_extensions::python_names::normalized;
        if let Some(reason) = &self.broken {
            return Some(reason.clone());
        }
        let listed = |list: &[String], name: &str| list.iter().any(|n| normalized(n) == name);
        for (name, p) in packages {
            for blocked in &self.blocked_python_packages {
                let (who, version) = match blocked.split_once("==") {
                    Some((who, version)) => (who, Some(version.trim())),
                    None => (blocked.as_str(), None),
                };
                if normalized(who) != *name {
                    continue;
                }
                match version {
                    None => {
                        return Some(format!("Kurum ilkesi “{name}” Python paketini engelliyor."));
                    }
                    Some(v) if v == p.version => {
                        return Some(format!(
                            "Kurum ilkesi “{name}” Python paketinin {v} sürümünü engelliyor; eklentinin yazarı başka \
                             bir sürüme kilitlemeli."
                        ));
                    }
                    Some(_) => {}
                }
            }
            if let Some(allowed) = &self.allowed_python_packages
                && !listed(allowed, name)
            {
                return Some(format!(
                    "Kurum ilkesi yalnız izin verdiği Python paketlerini kurduruyor; “{name}” listede yok."
                ));
            }
            if let (Some(days), Some(now)) = (self.minimum_package_age_days, installing) {
                let now = now
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
                let at = p
                    .uploaded
                    .as_deref()
                    .and_then(kentos_extensions::release::seconds_of);
                match at {
                    None => {
                        return Some(format!(
                            "Kurum ilkesi Python paketlerinin en az {days} günlük olmasını istiyor; “{name}” paketinin \
                             yayımlanma zamanı kilitte yazmıyor, yaşı denetlenemiyor."
                        ));
                    }
                    Some(at) if now - at < i64::from(days) * 86_400 => {
                        return Some(format!(
                            "Kurum ilkesi {days} günden yeni Python paketi kurdurmuyor: “{name}” {} {} gün önce \
                             yayımlandı. Eklentinin yazarı daha eski bir sürüme kilitleyebilir ya da beklenir.",
                            p.version,
                            (now - at).max(0) / 86_400
                        ));
                    }
                    Some(_) => {}
                }
            }
            // The organisation's own names, from their own index only (§118).
            let pinned = self.pinned_package_indexes.iter().find(|(pattern, _)| {
                match pattern.strip_suffix('*') {
                    // `belediye-*` holds `belediye-cbs`, not `belediyeci`: the separator stays a separator.
                    Some(prefix) => {
                        let mut head = normalized(prefix);
                        if prefix.ends_with(['-', '_', '.']) {
                            head.push('-');
                        }
                        name.starts_with(&head)
                    }
                    None => normalized(pattern) == *name,
                }
            });
            if let Some((_, index)) = pinned {
                let from = p
                    .source
                    .strip_prefix("registry ")
                    .map(|url| url.trim_end_matches('/'));
                if from != Some(index.trim_end_matches('/')) {
                    return Some(format!(
                        "Kurum ilkesi “{name}” paketini yalnız kurumun deposundan ({index}) kurduruyor; kilit onu \
                         başka bir yerden alıyor ({}): aynı adla genel bir depoya konmuş başka bir paket olabilir \
                         (bağımlılık karışıklığı). Eklentinin yazarı paketi o depoya bağlamalı ([tool.uv.sources]).",
                        p.source
                    ));
                }
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

/// Whether `address` is `allowed` or under it: the same scheme and host (with its port), the path the same or below,
/// letters' case aside in the scheme and host (`https://yz.ornek.bel.tr` holds `https://YZ.ornek.bel.tr/v1`, never
/// `https://yz.ornek.bel.tr.baska.com`).
fn address_within(address: &str, allowed: &str) -> bool {
    fn parts(url: &str) -> Option<(String, String, String)> {
        let (scheme, rest) = url.trim().split_once("://")?;
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let path = path.trim_end_matches('/');
        Some((scheme.to_lowercase(), host.to_lowercase(), path.to_owned()))
    }
    let (Some((scheme, host, path)), Some((a_scheme, a_host, a_path))) =
        (parts(address), parts(allowed))
    else {
        return false;
    };
    scheme == a_scheme
        && host == a_host
        && (a_path.is_empty() || path == a_path || path.starts_with(&format!("{a_path}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(text: &str) -> Policy {
        Policy::read(text).expect("a policy")
    }

    /// `allowAgentPython: false` keeps every agent from running its own Python, an unreadable policy too; not written,
    /// the user decides (docs/adr/8057 §104).
    #[test]
    fn the_policy_says_whether_an_agent_runs_its_own_python() {
        let open = policy(r#"{"format": "kentos.extension-policy", "schemaVersion": 1}"#);
        assert_eq!(open.agent_python_refusal(), None);
        let closed = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowAgentPython": false}"#,
        );
        assert!(
            closed
                .agent_python_refusal()
                .is_some_and(|w| w.contains("izin vermiyor"))
        );
        let broken = Policy {
            broken: Some("okunamadı".into()),
            ..Policy::default()
        };
        assert!(broken.agent_python_refusal().is_some());
    }

    /// `allowAttributeValuesToAi: false` keeps the attribute values from an agent outside this computer, not from one
    /// on it; not written, they go; an unreadable policy keeps them (docs/adr/8057 §103).
    #[test]
    fn the_policy_says_whether_attribute_values_go_to_an_agent() {
        let open = policy(r#"{"format": "kentos.extension-policy", "schemaVersion": 1}"#);
        assert!(!open.withholds_attribute_values(false));
        let kept = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowAttributeValuesToAi": false}"#,
        );
        assert!(kept.withholds_attribute_values(false));
        assert!(!kept.withholds_attribute_values(true), "on this computer");
        assert_eq!(
            kept.agent_refusal("DeepSeek", "https://api.deepseek.com", false),
            None,
            "the agent itself still goes"
        );
        let broken = Policy {
            broken: Some("okunamadı".into()),
            ..Policy::default()
        };
        assert!(broken.withholds_attribute_values(false));
    }

    /// The agent on this computer always goes; outside, `allowDataToAi: false` keeps the drawing in, the addresses the
    /// organisation lists let it reach them alone; an unreadable policy keeps it in.
    #[test]
    fn the_policy_says_where_the_agent_may_send_a_drawing() {
        let open = policy(r#"{"format": "kentos.extension-policy", "schemaVersion": 1}"#);
        assert_eq!(
            open.agent_refusal("A", "https://api.deepseek.com", false),
            None
        );
        let closed = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowDataToAi": false}"#,
        );
        assert_eq!(
            closed.agent_refusal("Yerel", "http://localhost:11434/v1", true),
            None
        );
        let said = closed
            .agent_refusal("DeepSeek", "https://api.deepseek.com", false)
            .expect("refused");
        assert!(
            said.contains("bu bilgisayarın dışındaki")
                && said.contains("“DeepSeek” https://api.deepseek.com"),
            "{said}"
        );
        let own = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowDataToAi": false,
                "allowedAiAddresses": ["https://yz.ornek.bel.tr"]}"#,
        );
        assert_eq!(
            own.agent_refusal("Kurum", "https://YZ.ornek.bel.tr/v1", false),
            None
        );
        assert!(
            own.agent_refusal("DeepSeek", "https://api.deepseek.com", false)
                .is_some()
        );
        let listed = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1,
                "allowedAiAddresses": ["https://api.anthropic.com/"]}"#,
        );
        assert_eq!(
            listed.agent_refusal("Claude", "https://api.anthropic.com", false),
            None
        );
        let said = listed
            .agent_refusal("DeepSeek", "https://api.deepseek.com", false)
            .expect("not listed");
        assert!(
            said.contains("yalnız şu adreslere") && said.contains("https://api.anthropic.com/"),
            "{said}"
        );
        let broken = Policy::broken("Okunamadı.".into());
        assert!(
            broken
                .agent_refusal("A", "https://api.deepseek.com", false)
                .is_some()
        );
        assert_eq!(
            broken.agent_refusal("A", "http://127.0.0.1:1234/v1", true),
            None
        );
    }

    /// An address is under another only at a boundary: the host whole, the path by its parts.
    #[test]
    fn an_address_is_within_another_at_a_boundary() {
        assert!(address_within(
            "https://yz.ornek.bel.tr/v1",
            "https://yz.ornek.bel.tr"
        ));
        assert!(address_within(
            "https://yz.ornek.bel.tr/v1/",
            "https://yz.ornek.bel.tr/v1"
        ));
        assert!(!address_within(
            "https://yz.ornek.bel.tr.baska.com/v1",
            "https://yz.ornek.bel.tr"
        ));
        assert!(!address_within(
            "https://yz.ornek.bel.tr:8443/v1",
            "https://yz.ornek.bel.tr"
        ));
        assert!(!address_within(
            "http://yz.ornek.bel.tr/v1",
            "https://yz.ornek.bel.tr"
        ));
        assert!(!address_within(
            "https://yz.ornek.bel.tr/v10",
            "https://yz.ornek.bel.tr/v1"
        ));
        assert!(!address_within(
            "yz.ornek.bel.tr",
            "https://yz.ornek.bel.tr"
        ));
    }

    /// `allowExtensionsOnServer: false` keeps an extension's tool off the server, KentOS's own tools not; not written,
    /// they go; an unreadable policy keeps an extension's code here (docs/adr/8057 §96).
    #[test]
    fn the_policy_says_whether_an_extension_s_code_goes_to_the_server() {
        let open = policy(r#"{"format": "kentos.extension-policy", "schemaVersion": 1}"#);
        assert_eq!(open.server_refusal("ext.tr.ornek.vitrin.taks"), None);
        let closed = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowExtensionsOnServer": false}"#,
        );
        assert_eq!(
            closed.server_refusal("ext.tr.ornek.vitrin.taks"),
            Some("kurum ilkesi eklenti kodunun sunucuda çalışmasına izin vermiyor")
        );
        assert_eq!(closed.server_refusal("annotation.edgeLengths"), None);
        let allowed = policy(
            r#"{"format": "kentos.extension-policy", "schemaVersion": 1, "allowExtensionsOnServer": true}"#,
        );
        assert_eq!(allowed.server_refusal("ext.tr.ornek.vitrin.taks"), None);
        let broken = Policy::broken("okunamadı".to_owned());
        assert_eq!(
            broken.server_refusal("ext.tr.ornek.vitrin.taks"),
            Some("kurum ilkesi okunamıyor")
        );
        assert_eq!(broken.server_refusal("annotation.edgeLengths"), None);
    }
}
