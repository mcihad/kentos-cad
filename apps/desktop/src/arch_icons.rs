//! Mimari's icons (M16, docs/adr/3016; plan ADV-10): every command of the Mimari tab and of the building's context
//! tabs has a picture of its own. No two of the tab's commands share one, and none of the building's own commands
//! borrows a drawing tool's (Duvar ölçüleri had Ölçülendirme's, Açıklık dağıt Dizi's, Duvarı kaydır Paralel
//! kaydır's). A selection filter's icon is its kind's on purpose (Duvarları seç shows the wall), so it is no borrower.
//!
//! The table of them, at the menu's 16 px, 20 px and the ribbon's large 28 px, light and dark, for the owner:
//!
//! ```text
//! cargo test -p kentos-desktop --lib arch_icons::icon_table -- --ignored --nocapture
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_ui::icon::Icon;

use crate::catalog::{Tab, catalog};

/// The building's context tabs (`scripts/ui/ribbon.py`'s `ARCH_TABS`).
const CONTEXT_TABS: [&str; 5] = [
    "ctx.wall",
    "ctx.opening",
    "ctx.space",
    "ctx.slab",
    "ctx.arch",
];

/// The CAD ribbon's Mimari tab.
fn mimari() -> &'static Tab {
    catalog()
        .tabs_in(kentos_commands::Workspace::Cad)
        .find(|t| t.id == "arch")
        .expect("CAD's ribbon has Mimari")
}

/// The building's context tabs, as the CAD ribbon has them.
fn context_tabs() -> Vec<&'static Tab> {
    catalog()
        .contextual_in(kentos_commands::Workspace::Cad)
        .filter(|t| CONTEXT_TABS.contains(&t.id))
        .collect()
}

/// Every command of the Mimari tab, then of the building's context tabs, once, with the place it first stands:
/// “Duvar”, “Düzenle ▾”, “Duvar sekmesi › Düzenle”.
fn places() -> Vec<(String, &'static str)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut take = |place: String, id: &'static str| {
        if seen.insert(id) {
            out.push((place, id));
        }
    };
    for panel in &mimari().panels {
        let shown = panel.commands();
        for id in shown {
            let place = if panel.overflow.contains(&id) {
                format!("{} ▾", panel.label)
            } else {
                panel.label.to_owned()
            };
            take(place, id);
        }
    }
    for tab in context_tabs() {
        for panel in &tab.panels {
            for id in panel.commands() {
                take(format!("{} sekmesi › {}", tab.label, panel.label), id);
            }
        }
    }
    out
}

/// The building's own tools and commands: on its tabs and on no other tab of any project type (Taşı and Kopyala are
/// every tab's; Duvar ölçüleri and Açıklık dağıt are Mimari's; Nesne özellikleri is a panel, not a tool).
fn own() -> BTreeSet<&'static str> {
    let elsewhere: BTreeSet<&str> = catalog()
        .every_tab()
        .filter(|t| t.id != "arch")
        .chain(
            [
                kentos_commands::Workspace::Cad,
                kentos_commands::Workspace::Gis,
            ]
            .into_iter()
            .flat_map(|mode| catalog().contextual_in(mode))
            .filter(|t| !CONTEXT_TABS.contains(&t.id)),
        )
        .flat_map(|t| t.panels.iter().flat_map(|p| p.commands()))
        .collect();
    places()
        .into_iter()
        .map(|(_, id)| id)
        .filter(|id| {
            (id.starts_with("tool.") || id.starts_with("arch.")) && !elsewhere.contains(id)
        })
        .collect()
}

fn icon_of(id: &str) -> Icon {
    catalog()
        .get(id)
        .unwrap_or_else(|| panic!("{id} is in the catalog"))
        .icon
}

fn title_of(id: &str) -> &'static str {
    catalog().get(id).map_or("?", |c| c.title)
}

/// The icon sets' names, by their markup: the inventory's and the additions'.
fn icon_names() -> HashMap<String, String> {
    let mut names = HashMap::new();
    for (file, key) in [
        (include_str!("../../../docs/inventory/web.json"), "icons"),
        (include_str!("../../../catalog/v1/additions.json"), "icons"),
    ] {
        let doc: serde_json::Value = serde_json::from_str(file).expect("an icon set");
        if let Some(icons) = doc.get(key).and_then(|v| v.as_object()) {
            for (name, markup) in icons {
                if let Some(markup) = markup.as_str() {
                    names.insert(markup.to_owned(), name.clone());
                }
            }
        }
    }
    names
}

fn name_of(icon: Icon, names: &HashMap<String, String>) -> String {
    match icon {
        Icon::Svg(markup) => names.get(markup).cloned().unwrap_or_else(|| "?".to_owned()),
        other => format!("{other:?}"),
    }
}

#[test]
fn no_two_of_mimari_s_commands_share_an_icon() {
    let names = icon_names();
    let mut by_icon: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for panel in &mimari().panels {
        for id in panel.commands() {
            let list = by_icon.entry(name_of(icon_of(id), &names)).or_default();
            if !list.contains(&id) {
                list.push(id);
            }
        }
    }
    let shared: Vec<String> = by_icon
        .iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(icon, ids)| {
            let titles: Vec<&str> = ids.iter().map(|id| title_of(id)).collect();
            format!("{icon}: {}", titles.join(", "))
        })
        .collect();
    assert!(
        shared.is_empty(),
        "Mimari'de aynı ikonu taşıyan komutlar:\n{}",
        shared.join("\n")
    );
}

/// A folded panel shows its icon (its first command's, DESIGN.md §6): Duvar and Düzenle both showed the wall.
#[test]
fn no_two_of_mimari_s_panels_share_an_icon() {
    let names = icon_names();
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    for panel in &mimari().panels {
        if let Some(other) = seen.insert(name_of(panel.icon, &names), panel.label) {
            panic!("{other} ve {} aynı ikonla katlanıyor", panel.label);
        }
    }
}

#[test]
fn the_building_s_own_commands_borrow_no_other_tool_s_icon() {
    let names = icon_names();
    let own = own();
    let ours: BTreeSet<&str> = places().into_iter().map(|(_, id)| id).collect();
    let mut borrowed = Vec::new();
    for id in &own {
        let icon = icon_of(id);
        let others: Vec<&str> = catalog()
            .commands()
            .iter()
            // Another drawing tool's: Dizi, Ölçülendirme, Paralel kaydır. A selection filter shows its kind's icon on
            // purpose (Duvarları seç: the wall), and the sheet's Detay paftaları the mark's (docs/adr/1112).
            .filter(|c| c.icon == icon && !ours.contains(c.id) && c.id.starts_with("tool."))
            .map(|c| c.title)
            .collect();
        if !others.is_empty() {
            borrowed.push(format!(
                "{} ({}): {}",
                title_of(id),
                name_of(icon, &names),
                others.join(", ")
            ));
        }
    }
    assert!(
        borrowed.is_empty(),
        "Başka bir aracın ikonunu taşıyan mimari komutlar:\n{}",
        borrowed.join("\n")
    );
}

/// The table: one row a command, its place, its icon at 16, 20 and the ribbon's 28 px (the large button's fixed
/// 1.55 px line), its title and its icon's name; in columns, light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn icon_table() {
    use iced::widget::{Space, column, container, row};
    use iced::{Length, Size};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/shots/mimari-ikonlar");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    let names = icon_names();
    let places = places();
    let own = own();
    let rows_per_column = 34;
    let columns: Vec<Vec<(String, &'static str)>> = places
        .chunks(rows_per_column)
        .map(<[(String, &'static str)]>::to_vec)
        .collect();
    let width = 640.0 * columns.len() as f32;
    let height = 34.0 * rows_per_column as f32 + 70.0;
    for (mode, suffix) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let theme = app.theme();
        let table = row(columns.iter().map(|list| {
            column(list.iter().map(|(place, id)| {
                let icon = icon_of(id);
                let mark = if own.contains(id) { "" } else { "  (genel)" };
                row![
                    container(kentos_ui::label::caption(place.clone())).width(Length::Fixed(150.0)),
                    container(kentos_ui::icon::icon(icon).size(16.0)).width(Length::Fixed(28.0)),
                    container(kentos_ui::icon::icon(icon).size(20.0)).width(Length::Fixed(32.0)),
                    container(kentos_ui::icon::icon(icon).size(28.0).weight(1.55))
                        .width(Length::Fixed(42.0)),
                    container(kentos_ui::label::body(format!("{}{mark}", title_of(id))))
                        .width(Length::Fixed(230.0)),
                    kentos_ui::label::caption(name_of(icon, &names)),
                ]
                .spacing(6)
                .height(Length::Fixed(34.0))
                .align_y(iced::alignment::Vertical::Center)
                .into()
            }))
            .width(Length::Fixed(640.0))
            .into()
        }));
        let page = container(column![
            kentos_ui::label::strong(format!(
                "Mimari ikonları — {} komut (Mimari sekmesi ve bina bağlam sekmeleri); 16 · 20 · 28 px",
                places.len()
            )),
            Space::new().height(Length::Fixed(10.0)),
            table,
        ])
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(kentos_ui::style::container::surface);
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let element: iced::Element<'_, crate::app::Message> = page.into();
        let file = out.join(format!("mimari-ikonlar-{suffix}.png"));
        snapshot
            .render(element, &theme)
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
