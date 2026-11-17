//! The installed extensions as the agents' common catalog reads them (plan/eklenti.md ADIM 21 › Üretim, PY-AI-01;
//! docs/adr/8057 §109): each with what it is, where it stands, the capabilities it asks and was given, the Python it
//! needs and whether that is ready, and its contributions with how an agent uses each. The link's `extensions`
//! request (Asistan's and an agent's on the desktop link alike); `catalog.search` and `catalog.describe` read it.

use kentos_extension_host::ContributionKind;
use serde_json::{Value, json};

use crate::app::App;

/// A contribution's kind as the catalog names it and how an agent uses it; none for the interface's own parts (a tab,
/// a menu item), which the user meets with the contributions they hold.
pub(crate) fn kind_words(kind: ContributionKind) -> Option<(&'static str, &'static str)> {
    Some(match kind {
        ContributionKind::Processing => (
            "tool",
            "İşlemler aracı: processing.describe tanımlar, processing.run çalıştırır (bir modelin adımı da olur).",
        ),
        ContributionKind::Audit => (
            "rule",
            "Denetim kuralı: audit.run çalıştırır, audit.apply_fix bulgusunu onarır.",
        ),
        ContributionKind::Expression => (
            "expression",
            "İfade işlevi: süzgeç ve hesap ifadelerinde adıyla çağrılır.",
        ),
        ContributionKind::Command => ("command", "Eklenti komutu: kullanıcı şeritten çalıştırır."),
        ContributionKind::Panel => ("panel", "Panel: kullanıcının ekranında açılır."),
        ContributionKind::Form => ("form", "Form: kullanıcı nesnenin formunda doldurur."),
        ContributionKind::MapTool => ("mapTool", "Harita aracı: kullanıcı haritada kullanır."),
        ContributionKind::DataSource => (
            "dataSource",
            "Veri kaynağı: kullanıcı Veri ekle'den bağlar.",
        ),
        ContributionKind::RibbonTab | ContributionKind::ContextMenuItem => return None,
    })
}

impl App {
    /// The link's `extensions`: every installed extension as the catalog reads it; why there are none when the
    /// extension store is not open.
    pub(crate) fn ext_catalog(&self) -> Value {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return json!({ "extensions": [], "why": self.extensions.missing });
        };
        let contributions = registry.contributions();
        let list: Vec<Value> = registry
            .extensions()
            .into_iter()
            .map(|view| {
                let manifest = registry.manifest(&view.id);
                let grant = registry.grant(&view.id);
                let capabilities: Vec<Value> = manifest
                    .map(|m| {
                        m.requested_capabilities
                            .iter()
                            .map(|c| {
                                // Its words as the install dialog says them: an agent left with the id alone
                                // said `audit.provider` as "eğitim" (the real window, 17 Nov).
                                json!({
                                    "id": c.id,
                                    "title": c.id.title(),
                                    "reason": c.reason,
                                    "granted": grant.is_some_and(|g| g.capabilities.contains(&c.id)),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let python = manifest.and_then(|m| m.python.as_ref()).map(|project| {
                    let requirements = registry
                        .package_file(&view.id, &project.project_file)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .map(|text| kentos_extensions::declared_requirements(&text))
                        .unwrap_or_default();
                    let why = self.extensions.python_reasons.get(&view.id).cloned().flatten();
                    json!({
                        "requirements": requirements,
                        "allowed": grant.is_some_and(|g| g.python),
                        "ready": why.is_none(),
                        "why": why,
                    })
                });
                let held: Vec<Value> = contributions
                    .iter()
                    .filter(|c| c.lease.extension == view.id)
                    .filter_map(|c| {
                        let (kind, use_words) = kind_words(c.kind)?;
                        Some(json!({
                            "kind": kind,
                            "id": c.id,
                            "title": c.title,
                            "python": c.needs_python,
                            "use": use_words,
                        }))
                    })
                    .collect();
                json!({
                    "id": view.id,
                    "name": view.name,
                    "version": view.version,
                    "summary": manifest.map(|m| m.summary.clone()),
                    "publisher": manifest.and_then(|m| m.publisher_id.clone()),
                    "keywords": manifest.map(|m| m.keywords.clone()).unwrap_or_default(),
                    "state": view.state_label,
                    "enabled": view.enabled,
                    "trust": view.trust_label,
                    "source": view.source.kind.words(),
                    "capabilities": capabilities,
                    "python": python,
                    "contributions": held,
                    "problem": view.problem.as_ref().map(|r| r.message.clone()),
                })
            })
            .collect();
        json!({ "extensions": list })
    }
}
