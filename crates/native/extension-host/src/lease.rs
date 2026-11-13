//! Leases and what an active extension adds (plan/eklenti.md ADIM 17; PY-RUN-09, 10).
//!
//! Every activation takes an [`ExtensionLease`]: the extension, its version and a generation no other activation in
//! this process shares. Everything it adds (a ribbon tab, a menu item, a data source, a form, a command, a map tool, an
//! İşlemler tool, an audit, an expression function, a panel) is registered under the lease, so revoking the lease
//! removes all of it at once and leaves nothing. A late callback that comes with an older generation is not applied.

use kentos_extensions::manifest::{Capability, Manifest};
use serde::Serialize;

use crate::store::Grant;

/// An activation of an extension's version.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionLease {
    pub extension: String,
    pub version: String,
    pub generation: u64,
}

/// What a contribution is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContributionKind {
    RibbonTab,
    ContextMenuItem,
    DataSource,
    Form,
    Command,
    MapTool,
    Processing,
    Audit,
    Expression,
    Panel,
}

/// A contribution of an active extension, in the host's namespace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Registered {
    pub lease: ExtensionLease,
    pub kind: ContributionKind,
    /// `ext.<extension>.<local id>` (a menu item: `ext.<extension>.menu.<n>`).
    pub id: String,
    pub title: String,
    /// Runs Python when called: the process starts then (D6), the environment is ADIM 18A's.
    pub needs_python: bool,
}

/// A contribution left out at activation, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

/// What an extension's manifest adds under `lease`, given what the user granted: the registered contributions, and
/// the ones left out (a capability not granted, Python not chosen).
pub fn contributions(
    manifest: &Manifest,
    grant: &Grant,
    lease: &ExtensionLease,
) -> (Vec<Registered>, Vec<Skipped>) {
    let granted = |c: Capability| grant.capabilities.contains(&c);
    let name = |local: &str| format!("ext.{}.{local}", manifest.id);
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    let mut add =
        |kind: ContributionKind, id: String, title: &str, needs: &[Capability], python: bool| {
            let missing: Vec<&str> = needs
                .iter()
                .filter(|c| !granted(**c))
                .map(|c| c.id())
                .collect();
            if !missing.is_empty() {
                skipped.push(Skipped {
                    id,
                    reason: format!("İzin verilmeyen yetenek: {}.", missing.join(", ")),
                });
            } else if python && !grant.python {
                skipped.push(Skipped {
                    id,
                    reason: "Python katkıları seçilmedi; ortam hazırlanmadı.".to_owned(),
                });
            } else {
                out.push(Registered {
                    lease: lease.clone(),
                    kind,
                    id,
                    title: title.to_owned(),
                    needs_python: python,
                });
            }
        };
    let e = &manifest.entrypoints;
    let k = &manifest.contributions;
    let has_python = manifest.python.is_some();
    for source in &k.data_sources {
        let mut needs = vec![Capability::DataSource];
        if source.url.is_some() {
            needs.push(Capability::Network);
        }
        add(
            ContributionKind::DataSource,
            name(&source.id),
            &source.title,
            &needs,
            false,
        );
    }
    for form in &e.forms {
        add(ContributionKind::Form, name(&form.id), &form.id, &[], false);
    }
    for tab in &k.ribbon_tabs {
        add(
            ContributionKind::RibbonTab,
            name(&tab.id),
            &tab.label,
            &[Capability::UiRibbonTab],
            false,
        );
    }
    for (i, item) in k.context_menus.iter().enumerate() {
        let title = item.label.as_deref().unwrap_or("");
        add(
            ContributionKind::ContextMenuItem,
            name(&format!("menu.{i}")),
            title,
            &[Capability::UiContextMenu],
            false,
        );
    }
    for panel in &e.panels {
        add(
            ContributionKind::Panel,
            name(&panel.id),
            &panel.title,
            &[Capability::UiPanel],
            has_python,
        );
    }
    for command in &e.commands {
        add(
            ContributionKind::Command,
            name(&command.id),
            &command.title,
            &[],
            true,
        );
    }
    for tool in &e.map_tools {
        add(
            ContributionKind::MapTool,
            name(&tool.id),
            &tool.title,
            &[Capability::MapTool],
            true,
        );
    }
    for tool in &e.processing {
        add(
            ContributionKind::Processing,
            name(&tool.id),
            &tool.id,
            &[Capability::ProcessingProvider],
            true,
        );
    }
    for audit in &e.audits {
        add(
            ContributionKind::Audit,
            name(&audit.id),
            &audit.title,
            &[Capability::AuditProvider],
            true,
        );
    }
    for expression in &e.expressions {
        let title = expression.title.as_deref().unwrap_or(&expression.id);
        add(
            ContributionKind::Expression,
            name(&expression.id),
            title,
            &[Capability::ExpressionProvider],
            true,
        );
    }
    (out, skipped)
}
