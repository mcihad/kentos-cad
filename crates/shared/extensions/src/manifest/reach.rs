//! Whether the user can reach a panel (docs/adr/8057 §3): a ribbon button (a vertical extension's) or a menu item
//! opens it, or a command's code does (`ctx.open_panel`). An extension without commands whose panel no button and no
//! menu item opens has a panel nobody can open; Kapı 1 says so, as a warning.

use std::collections::BTreeSet;

use super::{Checker, Manifest};

/// Warns of each panel nothing opens, in an extension without commands.
pub(super) fn check(m: &Manifest, c: &mut Checker<'_>) {
    if !m.entrypoints.commands.is_empty() {
        return;
    }
    let contributions = &m.contributions;
    let opened: BTreeSet<&str> = contributions
        .ribbon_tabs
        .iter()
        .flat_map(|tab| tab.panels.iter().flat_map(|panel| panel.items.iter()))
        .filter_map(|item| item.panel.as_deref())
        .chain(
            contributions
                .context_menus
                .iter()
                .filter_map(|item| item.panel.as_deref()),
        )
        .collect();
    for (i, panel) in m.entrypoints.panels.iter().enumerate() {
        if !opened.contains(panel.id.as_str()) {
            c.warn(
                "manifest_panel_unopened",
                format!("/entrypoints/panels/{i}"),
                format!(
                    "“{}” panelini hiçbir şey açmıyor: sağ tık menüsüne (contextMenus) ya da şerit sekmesine (dikey \
                     eklentide) “panel”: “{}” olan bir öğe koyun.",
                    panel.id, panel.id
                ),
            );
        }
    }
}
