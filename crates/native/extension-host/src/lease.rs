//! Leases and what an active extension adds (plan/eklenti.md ADIM 17; PY-RUN-09, 10).
//!
//! Every activation takes an [`ExtensionLease`]: the extension, its version and a generation no other activation in
//! this process shares. Everything it adds (a ribbon tab, a menu item, a data source, a form, a command, a map tool, an
//! İşlemler tool, an audit, an expression function, a panel) is registered under the lease, so revoking the lease
//! removes all of it at once and leaves nothing. A late callback that comes with an older generation is not applied.

use std::collections::BTreeMap;
use std::path::Path;

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
/// the ones left out (a capability not granted, Python not chosen). `titles` are the words of what the manifest names
/// only by a file, by its path in the package ([`file_titles`]): without them an İşlemler tool and a form are
/// titled by their ids.
pub fn contributions(
    manifest: &Manifest,
    grant: &Grant,
    lease: &ExtensionLease,
    titles: &BTreeMap<String, String>,
) -> (Vec<Registered>, Vec<Skipped>) {
    let granted = |c: Capability| grant.capabilities.contains(&c);
    let name = |local: &str| format!("ext.{}.{local}", manifest.id);
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    let mut add =
        |kind: ContributionKind, id: String, title: &str, needs: &[Capability], python: bool| {
            // Said by the capabilities' titles: a form's button shows it (docs/adr/8057 §122).
            let missing: Vec<String> = needs
                .iter()
                .filter(|c| !granted(**c))
                .map(|c| format!("“{}”", c.title()))
                .collect();
            if !missing.is_empty() {
                skipped.push(Skipped {
                    id,
                    reason: format!("Verilmeyen yetenek: {}.", missing.join(", ")),
                });
            } else if python && !grant.python {
                skipped.push(Skipped {
                    id,
                    reason: "Python katkıları kurulumda seçilmedi.".to_owned(),
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
        let title = titles.get(&form.definition).unwrap_or(&form.id);
        add(ContributionKind::Form, name(&form.id), title, &[], false);
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
        // Its own words, else those of what it shows: a menu item of a command read empty (kek dump, the MCP).
        let shown = || -> Option<String> {
            if let Some(c) = &item.command {
                return e
                    .commands
                    .iter()
                    .find(|x| x.id == *c)
                    .map(|x| x.title.clone());
            }
            if let Some(t) = &item.map_tool {
                return e
                    .map_tools
                    .iter()
                    .find(|x| x.id == *t)
                    .map(|x| x.title.clone());
            }
            if let Some(p) = &item.processing {
                let tool = e.processing.iter().find(|x| x.id == *p)?;
                return Some(titles.get(&tool.descriptor).unwrap_or(&tool.id).clone());
            }
            if let Some(a) = &item.audit {
                return e
                    .audits
                    .iter()
                    .find(|x| x.id == *a)
                    .map(|x| x.title.clone());
            }
            if let Some(p) = &item.panel {
                return e
                    .panels
                    .iter()
                    .find(|x| x.id == *p)
                    .map(|x| x.title.clone());
            }
            if let Some(s) = &item.data_source {
                return k
                    .data_sources
                    .iter()
                    .find(|x| x.id == *s)
                    .map(|x| x.title.clone());
            }
            None
        };
        let fallback = shown().unwrap_or_default();
        let title = item.label.as_deref().unwrap_or(&fallback);
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
            titles.get(&tool.descriptor).unwrap_or(&tool.id),
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

/// The words of the contributions a manifest names only by a file (docs/adr/8057 §24): an İşlemler tool's descriptor
/// `label`, a form package's `name`, read from the package folder `dir` (its files checked when it was installed), by
/// their paths. The desktop's ribbon, `kek dump` and the MCP then say the same; a file that does not read is left out
/// (its id stands).
pub fn file_titles(dir: &Path, manifest: &Manifest) -> BTreeMap<String, String> {
    let read = |path: &str, key: &str| -> Option<String> {
        if path.split('/').any(|p| p.is_empty() || p == "..") {
            return None;
        }
        let full = path.split('/').fold(dir.to_path_buf(), |d, p| d.join(p));
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(full).ok()?).ok()?;
        value
            .get(key)?
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
    };
    let e = &manifest.entrypoints;
    let tools = e.processing.iter().map(|t| (&t.descriptor, "label"));
    let forms = e.forms.iter().map(|f| (&f.definition, "name"));
    tools
        .chain(forms)
        .filter_map(|(path, key)| Some((path.clone(), read(path, key)?)))
        .collect()
}
