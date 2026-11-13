//! What the extensions add to the ribbon (docs/adr/8057 §3; plan/eklenti.md ADIM 18).
//!
//! A vertical extension's tab, registered under its lease when the user granted `ui.ribbonTab`, joins the ribbon of
//! its own program only: KentOS GIS's tab is not offered in KentOS Mimari (kentos-cad, which runs every program's
//! commands, shows it in the work mode of its program). Its buttons do what needs no extension code: a KentOS command
//! (`builtin`), the extension's data source (opened as Kaynaklar's services are) and its form (imported as Formu içe
//! aktar does). A button that runs the extension's Python is shown, not pressable, with the reason, until its
//! environment is ready (ADIM 18A, 19). The tabs are rebuilt from the registry whenever it changes, so a disabled or
//! removed extension leaves no tab and no button behind.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use kentos_extension_host::ContributionKind;
use kentos_extensions::manifest::{ItemSize, Manifest, MenuTarget, RibbonItem};
use kentos_launch::Module;
use kentos_ui::icon::Icon;
use kentos_ui::widget::ribbon::{Button, Group};
use kentos_ui::widget::{Menu, Tip};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::catalog::catalog;

/// A string kept for the program's lifetime, once per distinct value: tab ids and command ids are `&'static str` in
/// the ribbon's messages. The extensions' ids are few and kept as long as the program runs.
pub(crate) fn intern(text: &str) -> &'static str {
    static KEPT: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut kept = KEPT
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(found) = kept.get(text) {
        return found;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    kept.insert(leaked);
    leaked
}

/// What a button of an extension's tab does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    /// A KentOS command.
    Builtin(&'static str),
    /// The extension's data source: (extension, its local id).
    Source(String, String),
    /// The extension's form: (extension, its local id).
    Form(String, String),
    /// Shown, not pressable: why.
    Unavailable(String),
}

#[derive(Clone, Debug)]
pub(crate) struct ExtItem {
    pub(crate) label: String,
    pub(crate) icon: Icon,
    pub(crate) large: bool,
    pub(crate) action: Action,
}

#[derive(Clone, Debug)]
pub(crate) struct ExtPanel {
    pub(crate) label: String,
    pub(crate) items: Vec<ExtItem>,
}

/// An extension's ribbon tab.
#[derive(Clone, Debug)]
pub(crate) struct ExtTab {
    /// `ext.<extension>.<tab>`, kept for the ribbon's messages.
    pub(crate) id: &'static str,
    pub(crate) label: String,
    pub(crate) module: Module,
    pub(crate) extension: String,
    pub(crate) panels: Vec<ExtPanel>,
}

/// A right-click menu's item an extension adds (`ui.contextMenu`): which menu, and what it does.
#[derive(Clone, Debug)]
pub(crate) struct ExtMenuItem {
    pub(crate) target: MenuTarget,
    pub(crate) item: ExtItem,
    pub(crate) modules: Vec<Module>,
}

/// The program a manifest's module names.
fn module_of(m: kentos_extensions::manifest::Module) -> Option<Module> {
    Module::ALL.into_iter().find(|p| p.id() == m.id())
}

/// An icon a manifest names: a built-in one by its name, or a 20×20 SVG file in the package, checked when the package
/// was installed (kentos_extensions::icon) and with the package's files before it ran; drawn in the theme's colour.
/// A package file that cannot be read now falls back to `fallback`.
pub(crate) fn icon_of(
    registry: &kentos_extension_host::Registry,
    ext: &str,
    name: &str,
    fallback: Icon,
) -> Icon {
    if !name.contains('/') && !name.contains('.') {
        return crate::icons::from_web(Some(name));
    }
    registry
        .package_file(ext, name)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map_or(fallback, |text| Icon::Svg(intern(&text)))
}

/// What an item points at, its words and icon, and what pressing it does.
fn item_of(
    registry: &kentos_extension_host::Registry,
    manifest: &Manifest,
    item: &RibbonItem,
    registered: &[&kentos_extension_host::Registered],
) -> ExtItem {
    let ext = manifest.id.as_str();
    let e = &manifest.entrypoints;
    let has = |kind: ContributionKind, local: &str| {
        let id = format!("ext.{ext}.{local}");
        registered.iter().any(|r| r.kind == kind && r.id == id)
    };
    let not_granted = || {
        Action::Unavailable(
            "Bu katkının istediği yetenek kurulurken verilmedi; Eklentiler penceresinden yeniden kurup izin verin."
                .to_owned(),
        )
    };
    let (label, icon, action): (String, Icon, Action) = if let Some(id) = &item.builtin {
        match catalog().get(id) {
            Some(command) => (
                command.short.to_owned(),
                command.icon,
                Action::Builtin(command.id),
            ),
            None => (
                id.clone(),
                Icon::Button,
                Action::Unavailable(format!("“{id}” bu KentOS'ta bir komut değil.")),
            ),
        }
    } else if let Some(local) = &item.data_source {
        let title = manifest
            .contributions
            .data_sources
            .iter()
            .find(|s| s.id == *local)
            .map_or_else(|| local.clone(), |s| s.title.clone());
        let action = if has(ContributionKind::DataSource, local) {
            Action::Source(ext.to_owned(), local.clone())
        } else {
            not_granted()
        };
        (
            title,
            crate::icons::from_web(Some("serviceSettings")),
            action,
        )
    } else if let Some(local) = &item.form {
        let action = if has(ContributionKind::Form, local) {
            Action::Form(ext.to_owned(), local.clone())
        } else {
            not_granted()
        };
        (
            local.clone(),
            crate::icons::from_web(Some("featureTable")),
            action,
        )
    } else {
        // The extension's code: a command, a map tool, an İşlemler tool, an audit or a panel.
        let (kind, local, title, icon) = if let Some(local) = &item.command {
            let c = e.commands.iter().find(|c| c.id == *local);
            (
                ContributionKind::Command,
                local,
                c.map(|c| c.title.clone()),
                c.and_then(|c| c.icon.clone()),
            )
        } else if let Some(local) = &item.map_tool {
            let t = e.map_tools.iter().find(|t| t.id == *local);
            (
                ContributionKind::MapTool,
                local,
                t.map(|t| t.title.clone()),
                t.and_then(|t| t.icon.clone()),
            )
        } else if let Some(local) = &item.processing {
            (
                ContributionKind::Processing,
                local,
                None,
                Some("processing".to_owned()),
            )
        } else if let Some(local) = &item.audit {
            let a = e.audits.iter().find(|a| a.id == *local);
            (
                ContributionKind::Audit,
                local,
                a.map(|a| a.title.clone()),
                Some("standardsCheck".to_owned()),
            )
        } else if let Some(local) = &item.panel {
            let p = e.panels.iter().find(|p| p.id == *local);
            (
                ContributionKind::Panel,
                local,
                p.map(|p| p.title.clone()),
                Some("panelRight".to_owned()),
            )
        } else {
            return ExtItem {
                label: "?".to_owned(),
                icon: Icon::Button,
                large: false,
                action: Action::Unavailable("Öğe bir katkı göstermiyor.".to_owned()),
            };
        };
        let title = title.unwrap_or_else(|| local.clone());
        let action = if has(kind, local) {
            Action::Unavailable(format!(
                "“{title}” eklentinin Python işlevini çalıştırır; Python ortamı hazırlanınca çalışacak."
            ))
        } else {
            not_granted()
        };
        let fallback = crate::icons::from_web(Some("processing"));
        let icon = icon.map_or(fallback, |name| icon_of(registry, ext, &name, fallback));
        (title, icon, action)
    };
    let icon = match item.icon.as_deref() {
        Some(name) => icon_of(registry, ext, name, icon),
        None => icon,
    };
    ExtItem {
        label: item.label.clone().unwrap_or(label),
        icon,
        large: item.size == ItemSize::Large,
        action,
    }
}

impl App {
    /// The extensions' tabs follow the registry's contributions.
    pub(crate) fn extension_tabs_changed(&mut self) {
        let mut tabs = Vec::new();
        if let Some(registry) = self.extensions.registry.as_ref() {
            let contributions = registry.contributions();
            for tab_contribution in contributions
                .iter()
                .filter(|c| c.kind == ContributionKind::RibbonTab)
            {
                let ext = &tab_contribution.lease.extension;
                let Some(manifest) = registry.manifest(ext) else {
                    continue;
                };
                let registered: Vec<&kentos_extension_host::Registered> = contributions
                    .iter()
                    .filter(|c| c.lease == tab_contribution.lease)
                    .collect();
                let prefix = format!("ext.{ext}.");
                let local = tab_contribution
                    .id
                    .strip_prefix(&prefix)
                    .unwrap_or_default();
                let Some(tab) = manifest
                    .contributions
                    .ribbon_tabs
                    .iter()
                    .find(|t| t.id == local)
                else {
                    continue;
                };
                let Some(module) = module_of(tab.module) else {
                    continue;
                };
                tabs.push(ExtTab {
                    id: intern(&tab_contribution.id),
                    label: tab.label.clone(),
                    module,
                    extension: ext.clone(),
                    panels: tab
                        .panels
                        .iter()
                        .map(|p| ExtPanel {
                            label: p.label.clone(),
                            items: p
                                .items
                                .iter()
                                .map(|i| item_of(registry, manifest, i, &registered))
                                .collect(),
                        })
                        .collect(),
                });
            }
        }
        self.extensions.tabs = tabs;
        // The right-click menus' items, under the same leases.
        let mut menus = Vec::new();
        if let Some(registry) = self.extensions.registry.as_ref() {
            let contributions = registry.contributions();
            for c in contributions
                .iter()
                .filter(|c| c.kind == ContributionKind::ContextMenuItem)
            {
                let ext = &c.lease.extension;
                let Some(manifest) = registry.manifest(ext) else {
                    continue;
                };
                let Some(n) =
                    c.id.rsplit('.')
                        .next()
                        .and_then(|n| n.parse::<usize>().ok())
                else {
                    continue;
                };
                let Some(entry) = manifest.contributions.context_menus.get(n) else {
                    continue;
                };
                let registered: Vec<&kentos_extension_host::Registered> = contributions
                    .iter()
                    .filter(|r| r.lease == c.lease)
                    .collect();
                // The menu item points as a ribbon button does.
                let as_button = RibbonItem {
                    command: entry.command.clone(),
                    map_tool: entry.map_tool.clone(),
                    processing: entry.processing.clone(),
                    audit: entry.audit.clone(),
                    panel: entry.panel.clone(),
                    form: entry.form.clone(),
                    data_source: entry.data_source.clone(),
                    builtin: entry.builtin.clone(),
                    size: ItemSize::Small,
                    label: entry.label.clone(),
                    icon: entry.icon.clone(),
                    when: entry.when.clone(),
                };
                menus.push(ExtMenuItem {
                    target: entry.target,
                    item: item_of(registry, manifest, &as_button, &registered),
                    modules: manifest
                        .modules
                        .iter()
                        .filter_map(|m| module_of(*m))
                        .collect(),
                });
            }
        }
        self.extensions.menus = menus;
    }

    /// The extensions' items of a right-click menu, after a line, where this program shows their extension.
    pub(crate) fn extension_menu(&self, target: MenuTarget, menu: Menu<Message>) -> Menu<Message> {
        let items: Vec<&ExtMenuItem> = self
            .extensions
            .menus
            .iter()
            .filter(|m| {
                m.target == target
                    && m.modules
                        .iter()
                        .any(|module| self.extension_tab_here(*module))
            })
            .collect();
        if items.is_empty() {
            return menu;
        }
        items.into_iter().fold(menu.separator(), |menu, m| {
            let on = self.item_message(&m.item.action);
            menu.item(m.item.label.clone(), on).icon(m.item.icon)
        })
    }

    /// What pressing an item sends: none when it cannot be pressed here.
    fn item_message(&self, action: &Action) -> Option<Message> {
        match action {
            Action::Builtin(id) => self.offers(id).then_some(Message::Run(id)),
            Action::Source(ext, local) => Some(msg(Event::OpenSource(ext.clone(), local.clone()))),
            Action::Form(ext, local) => Some(msg(Event::OpenForm(ext.clone(), local.clone()))),
            Action::Unavailable(_) => None,
        }
    }

    /// An extension's tab's key tips' controls: its buttons that can be pressed, in reading order, with their names
    /// (ribbon_keys.rs).
    pub(crate) fn extension_tip_controls(
        &self,
        tab: &str,
    ) -> Vec<(
        crate::ribbon_keys::TipKey,
        String,
        crate::ribbon_keys::TipAction,
    )> {
        let Some(tab) = self.extension_tab(tab) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (p, panel) in tab.panels.iter().enumerate() {
            for (i, item) in panel.items.iter().enumerate() {
                if let Some(message) = self.item_message(&item.action) {
                    out.push((
                        crate::ribbon_keys::TipKey::ExtItem(tab.id, p, i),
                        item.label.clone(),
                        crate::ribbon_keys::TipAction::Run(message),
                    ));
                }
            }
        }
        out
    }

    /// Whether an extension's tab of this program shows here: its own program's, or in kentos-cad the work mode of
    /// its program.
    fn extension_tab_here(&self, module: Module) -> bool {
        match self.program {
            Some(program) => program == module,
            None => crate::program::type_of(module) == self.work_mode(),
        }
    }

    /// Whether this program shows what an extension of a manifest's module adds.
    pub(crate) fn extension_module_here(
        &self,
        module: kentos_extensions::manifest::Module,
    ) -> bool {
        module_of(module).is_some_and(|m| self.extension_tab_here(m))
    }

    /// The extensions' tabs this program shows.
    pub(crate) fn extension_tabs(&self) -> impl Iterator<Item = &ExtTab> {
        self.extensions
            .tabs
            .iter()
            .filter(|t| self.extension_tab_here(t.module))
    }

    /// The extension's tab of this id, when this program shows it.
    pub(crate) fn extension_tab(&self, id: &str) -> Option<&ExtTab> {
        self.extension_tabs().find(|t| t.id == id)
    }

    /// An extension's tab's panels, as the ribbon draws them.
    pub(crate) fn extension_groups(&self, tab: &ExtTab) -> Vec<Group<'static, Message>> {
        tab.panels
            .iter()
            .enumerate()
            .map(|(p, panel)| {
                let mut group = Group::new(panel.label.clone());
                if let Some(first) = panel.items.first() {
                    group = group.icon(first.icon);
                }
                for (i, item) in panel.items.iter().enumerate() {
                    // Komut ara's reveal (search.rs).
                    let flash = self.extensions.flash
                        == Some(super::ExtHit {
                            tab: tab.id,
                            panel: p,
                            item: i,
                        });
                    let button = if item.large {
                        Button::large(item.icon, item.label.clone())
                    } else {
                        Button::small(item.icon, item.label.clone())
                    };
                    let on = self.item_message(&item.action);
                    let tip = match &item.action {
                        Action::Builtin(id) => {
                            catalog().get(id).map_or_else(String::new, |c| c.description.to_owned())
                        }
                        Action::Source(..) => {
                            "Eklentinin veri kaynağını haritaya ekler (Kaynaklar'ın servis yolundan).".to_owned()
                        }
                        Action::Form(..) => {
                            "Eklentinin formunu projeye alır (Formu içe aktar gibi).".to_owned()
                        }
                        Action::Unavailable(why) => why.clone(),
                    };
                    let key_tip =
                        self.key_tip(&crate::ribbon_keys::TipKey::ExtItem(tab.id, p, i));
                    group = group.tool(
                        button
                            .on_press_maybe(on)
                            .flash(flash)
                            .key_tips(key_tip, None)
                            .tip(Tip::new(item.label.clone()).body(format!("{tip} ({})", tab.extension))),
                    );
                }
                group
            })
            .collect()
    }
}
