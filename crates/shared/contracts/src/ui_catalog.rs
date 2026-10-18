//! The interface catalog's additions (docs/adr/0228 §4, docs/adr/0229 §15):
//! `catalog/v1/additions.json`, the commands added while the web is paused
//! with their titles, icons, descriptions and aliases, their icons' markup
//! and their places in each project type's ribbon. The desktop embeds it
//! beside the web's frozen inventory (`docs/inventory/web.json`); the web and
//! a mobile shell will read the same file. Its shape is a subset of the
//! inventory's; the rules here are the ones the desktop's test checks it by.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CATALOG_ADDITIONS_FORMAT: &str = "kentos.catalog-additions";
pub const CATALOG_ADDITIONS_VERSION: u32 = 1;

/// The project types a ribbon place names.
pub const CATALOG_MODES: [&str; 2] = ["cad", "gis"];

/// The sizes a ribbon item may be drawn at (the inventory's).
pub const CATALOG_SIZES: [&str; 3] = ["large", "lead", "small"];

/// The file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogAdditions {
    pub format: String,
    pub version: u32,
    pub commands: Vec<AddedCommand>,
    /// Icon name → its SVG elements on the web set's 20 × 20 grid, without the `<svg>` element: the
    /// inventory's `icons`' form (the stroke, 1.4 units, rounded, is the drawer's).
    #[serde(default)]
    pub icons: BTreeMap<String, String>,
    /// New tabs of a type's ribbon (docs/adr/0339 §6: CAD's Mimari), each after a tab it has; their panels come in
    /// `ribbon` as any other tab's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AddedTab>>", optional))]
    pub tabs: Vec<AddedTab>,
    #[serde(default)]
    pub ribbon: Vec<RibbonAddition>,
    /// A tool's methods added to its ribbon button (docs/adr/0287): its split's
    /// entries, or a split made of its button; each starts the tool and gives it
    /// its option as if typed. One without an option names the tool's default
    /// method, the first entry of a split made of a button.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AddedMethod>>", optional))]
    pub methods: Vec<AddedMethod>,
}

/// A method of a tool (the inventory's split entry, `SplitEntry`): the tool's
/// command, the entry's name, the option it gives the tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddedMethod {
    /// The tool's command (`tool.arc`).
    pub command: String,
    /// Its name in the menu (`Başlangıç, merkez, açı`).
    pub label: String,
    /// What it gives the tool once it runs, as if typed; none: the tool's default method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub option: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// Typed names that start the tool with this method.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub icon: Option<String>,
}

/// A command of the interface, as the inventory lists one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddedCommand {
    pub id: String,
    pub title: String,
    /// A ribbon button's name when the title is long.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub short: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub icon: Option<String>,
    pub description: String,
    /// Command line spellings: the first is the shown name.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Key chords as the web writes them (`Ctrl+S`).
    #[serde(default)]
    pub shortcuts: Vec<String>,
    /// The category Komut ara says (“Harita”, “Analiz”).
    pub category: String,
    pub status: AddedStatus,
    /// Why a pending command does not run yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pending_note: Option<String>,
}

/// Whether a command runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AddedStatus {
    Implemented,
    Pending,
}

/// A new tab of a ribbon: its id, its name and the tab it goes after (none: last).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddedTab {
    /// The project types whose ribbons take it (`cad`, `gis`).
    pub modes: Vec<String>,
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub after: Option<String>,
}

/// A place in a ribbon: a new panel of a tab (after a panel, else last), or
/// commands added to the end of a panel the tab has (`into`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RibbonAddition {
    /// The project types whose ribbons take it (`cad`, `gis`).
    pub modes: Vec<String>,
    /// The tab's id (`map`, `insert`).
    pub tab: String,
    /// The new panel's name; none when the items go `into` a panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The new panel's icon (the button it folds into on a narrow window).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub icon: Option<String>,
    /// The panel the new one goes after; none: last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub after: Option<String>,
    /// The panel the items are added to, by its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub into: Option<String>,
    pub items: Vec<AddedItem>,
    /// Seldom used commands, under the ▾ beside the panel's title (the inventory's `overflow`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub overflow: Vec<String>,
}

/// A ribbon button: a command and its size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddedItem {
    pub command: String,
    pub size: String,
}

/// What the rules ask of the catalog the file adds to (the web's inventory).
pub struct KnownCatalog<'a> {
    /// Whether the inventory has a command of this id.
    pub command: &'a dyn Fn(&str) -> bool,
    /// Whether the inventory's icon set has an icon of this name.
    pub icon: &'a dyn Fn(&str) -> bool,
    /// The names of a type's tab's panels, in order; none when the type's ribbon has no such tab.
    pub panels: &'a dyn Fn(&str, &str) -> Option<Vec<String>>,
}

impl CatalogAdditions {
    /// Whether a tab of `mode`'s ribbon is one of the file's own.
    fn added_tab(&self, mode: &str, tab: &str) -> bool {
        self.tabs
            .iter()
            .any(|t| t.id == tab && t.modes.iter().any(|m| m == mode))
    }
}

impl CatalogAdditions {
    /// What is wrong with the file against the catalog it adds to; none when it holds.
    pub fn problem(&self, known: &KnownCatalog<'_>) -> Option<String> {
        if self.format != CATALOG_ADDITIONS_FORMAT || self.version != CATALOG_ADDITIONS_VERSION {
            return Some(format!(
                "biçimi “{}” {}; {CATALOG_ADDITIONS_FORMAT} {CATALOG_ADDITIONS_VERSION} olmalı",
                self.format, self.version
            ));
        }
        let icon = |name: &str| self.icons.contains_key(name) || (known.icon)(name);
        for (i, c) in self.commands.iter().enumerate() {
            if c.id.trim().is_empty() || c.id.trim() != c.id {
                return Some(format!("{}. komutun kimliği boş ya da kırpılmamış", i + 1));
            }
            if self.commands[..i].iter().any(|o| o.id == c.id) {
                return Some(format!("“{}” komutu iki kez var", c.id));
            }
            if (known.command)(&c.id) {
                return Some(format!(
                    "“{}” komutu web'in envanterinde zaten var; ek dosya yalnız yeni komutları yazar",
                    c.id
                ));
            }
            if c.title.trim().is_empty() || c.description.trim().is_empty() {
                return Some(format!("“{}” komutunun başlığı ya da açıklaması boş", c.id));
            }
            if let Some(i) = c.icon.as_deref().filter(|i| !icon(i)) {
                return Some(format!("“{}” komutunun ikonu “{i}” yok", c.id));
            }
            if c.status == AddedStatus::Implemented && c.pending_note.is_some() {
                return Some(format!(
                    "“{}” komutu çalışıyor; bekleme notu yazılmaz",
                    c.id
                ));
            }
        }
        for (name, markup) in &self.icons {
            if (known.icon)(name) {
                return Some(format!(
                    "“{name}” ikonu web'in envanterinde zaten var; ek dosya yalnız yeni ikonları yazar"
                ));
            }
            let m = markup.trim();
            if m.is_empty() || !m.starts_with('<') || m.contains("<svg") {
                return Some(format!(
                    "“{name}” ikonu envanterin biçiminde değil: 20 × 20'lik ızgaranın öğeleri yazılır, <svg> öğesi yazılmaz"
                ));
            }
        }
        let command = |id: &str| (known.command)(id) || self.commands.iter().any(|c| c.id == id);
        for (i, t) in self.tabs.iter().enumerate() {
            let at = format!("{}. sekme", i + 1);
            if t.id.trim().is_empty() || t.label.trim().is_empty() {
                return Some(format!("{at}: kimliği ya da adı boş"));
            }
            if t.modes.is_empty() {
                return Some(format!("{at}: proje türü yok"));
            }
            for mode in &t.modes {
                if !CATALOG_MODES.contains(&mode.as_str()) {
                    return Some(format!("{at}: “{mode}” türü yok; cad ya da gis olmalı"));
                }
                if (known.panels)(mode, &t.id).is_some()
                    || self.tabs[..i]
                        .iter()
                        .any(|o| o.id == t.id && o.modes.contains(mode))
                {
                    return Some(format!(
                        "{at}: {mode} şeridinde “{}” sekmesi zaten var",
                        t.id
                    ));
                }
                if let Some(after) = t
                    .after
                    .as_deref()
                    .filter(|a| (known.panels)(mode, a).is_none() && !self.added_tab(mode, a))
                {
                    return Some(format!("{at}: {mode} şeridinde “{after}” sekmesi yok"));
                }
            }
        }
        let mut added: Vec<(String, String, String)> = Vec::new();
        for (i, r) in self.ribbon.iter().enumerate() {
            let at = format!("şeridin {}. eki", i + 1);
            if r.modes.is_empty() {
                return Some(format!("{at}: proje türü yok"));
            }
            if r.items.is_empty() {
                return Some(format!("{at}: düğme yok"));
            }
            for item in &r.items {
                if !command(&item.command) {
                    return Some(format!("{at}: “{}” komutu yok", item.command));
                }
                if !CATALOG_SIZES.contains(&item.size.as_str()) {
                    return Some(format!(
                        "{at}: “{}” boyu “{}”; large, lead ya da small olmalı",
                        item.command, item.size
                    ));
                }
            }
            for (k, id) in r.overflow.iter().enumerate() {
                if !command(id) {
                    return Some(format!("{at}: ▾ listesinde “{id}” komutu yok"));
                }
                if r.items.iter().any(|i| i.command == *id) || r.overflow[..k].contains(id) {
                    return Some(format!("{at}: “{id}” komutu iki kez var"));
                }
            }
            for mode in &r.modes {
                if !CATALOG_MODES.contains(&mode.as_str()) {
                    return Some(format!("{at}: “{mode}” türü yok; cad ya da gis olmalı"));
                }
                let own = self.added_tab(mode, &r.tab).then(Vec::new);
                let Some(mut panels) = (known.panels)(mode, &r.tab).or(own) else {
                    return Some(format!("{at}: {mode} şeridinde “{}” sekmesi yok", r.tab));
                };
                panels.extend(
                    added
                        .iter()
                        .filter(|(m, t, _)| m == mode && *t == r.tab)
                        .map(|(_, _, p)| p.clone()),
                );
                match (&r.label, &r.into) {
                    (Some(label), None) => {
                        if label.trim().is_empty() {
                            return Some(format!("{at}: yeni panelin adı boş"));
                        }
                        if panels.contains(label) {
                            return Some(format!(
                                "{at}: {mode} şeridinin “{}” sekmesinde “{label}” paneli zaten var",
                                r.tab
                            ));
                        }
                        if let Some(after) = r
                            .after
                            .as_deref()
                            .filter(|a| !panels.iter().any(|p| p == a))
                        {
                            return Some(format!(
                                "{at}: {mode} şeridinin “{}” sekmesinde “{after}” paneli yok",
                                r.tab
                            ));
                        }
                        if let Some(i) = r.icon.as_deref().filter(|i| !icon(i)) {
                            return Some(format!("{at}: panelin ikonu “{i}” yok"));
                        }
                        added.push((mode.clone(), r.tab.clone(), label.clone()));
                    }
                    (None, Some(into)) => {
                        if !panels.contains(into) {
                            return Some(format!(
                                "{at}: {mode} şeridinin “{}” sekmesinde “{into}” paneli yok",
                                r.tab
                            ));
                        }
                        if r.after.is_some() || r.icon.is_some() {
                            return Some(format!(
                                "{at}: var olan panele eklemede yer ya da ikon yazılmaz"
                            ));
                        }
                    }
                    _ => {
                        return Some(format!(
                            "{at}: ya yeni panelin adı (label) ya da eklenecek panel (into) yazılır"
                        ));
                    }
                }
            }
        }
        for (i, m) in self.methods.iter().enumerate() {
            let at = format!("{}. yöntem", i + 1);
            if !m.command.starts_with("tool.") || !command(&m.command) {
                return Some(format!("{at}: “{}” bir araç komutu değil", m.command));
            }
            if m.label.trim().is_empty() {
                return Some(format!("{at}: adı boş"));
            }
            if m.option
                .as_deref()
                .is_some_and(|o| o.trim().is_empty() || o.trim() != o)
            {
                return Some(format!("{at}: seçeneği boş ya da kırpılmamış"));
            }
            if self.methods[..i]
                .iter()
                .any(|o| o.command == m.command && (o.option == m.option || o.label == m.label))
            {
                return Some(format!(
                    "{at}: “{}” aracının aynı seçenekli ya da aynı adlı yöntemi iki kez var",
                    m.command
                ));
            }
            if let Some(name) = m.icon.as_deref().filter(|i| !icon(i)) {
                return Some(format!("{at}: ikonu “{name}” yok"));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> CatalogAdditions {
        CatalogAdditions {
            format: CATALOG_ADDITIONS_FORMAT.into(),
            version: 1,
            commands: vec![AddedCommand {
                id: "address.search".into(),
                title: "Adres ara".into(),
                short: None,
                icon: Some("addressSearch".into()),
                description: "Adres arar.".into(),
                aliases: vec!["ADRES".into()],
                shortcuts: Vec::new(),
                category: "Harita".into(),
                status: AddedStatus::Implemented,
                pending_note: None,
            }],
            icons: [(
                "addressSearch".to_owned(),
                "<circle cx=\"8.5\" cy=\"8.5\" r=\"5\"/>".to_owned(),
            )]
            .into_iter()
            .collect(),
            tabs: Vec::new(),
            ribbon: vec![RibbonAddition {
                modes: vec!["gis".into()],
                tab: "map".into(),
                label: Some("Adres".into()),
                icon: Some("addressSearch".into()),
                after: Some("Altlık".into()),
                into: None,
                items: vec![AddedItem {
                    command: "address.search".into(),
                    size: "large".into(),
                }],
                overflow: vec!["file.open".into()],
            }],
            methods: vec![AddedMethod {
                command: "tool.arc".into(),
                label: "Başlangıç, merkez, açı".into(),
                option: Some("BMA".into()),
                description: None,
                aliases: Vec::new(),
                icon: Some("addressSearch".into()),
            }],
        }
    }

    #[test]
    fn methods_name_a_tool_once_each() {
        let ok = check(&file());
        assert_eq!(ok, None);
        let mut f = file();
        f.methods[0].command = "address.search".into();
        assert!(check(&f).is_some_and(|p| p.contains("araç komutu değil")));
        let mut f = file();
        f.methods.push(f.methods[0].clone());
        assert!(check(&f).is_some_and(|p| p.contains("iki kez")));
        let mut f = file();
        f.methods[0].icon = Some("yok".into());
        assert!(check(&f).is_some_and(|p| p.contains("ikonu")));
    }

    fn check(f: &CatalogAdditions) -> Option<String> {
        let command = |id: &str| id == "file.open" || id == "tool.arc";
        let icon = |name: &str| name == "basemap";
        let panels = |mode: &str, tab: &str| {
            (mode == "gis" && tab == "map").then(|| vec!["Altlık".to_owned(), "Parsel".to_owned()])
        };
        f.problem(&KnownCatalog {
            command: &command,
            icon: &icon,
            panels: &panels,
        })
    }

    /// A tab of its own (docs/adr/0339 §6): after a tab the ribbon has, its panels then as any tab's.
    #[test]
    fn a_tab_of_its_own_takes_panels() {
        let mut f = file();
        f.tabs.push(AddedTab {
            modes: vec!["gis".into()],
            id: "arch".into(),
            label: "Mimari".into(),
            after: Some("map".into()),
        });
        f.ribbon[0].tab = "arch".into();
        f.ribbon[0].after = None;
        assert_eq!(check(&f), None);
        let mut g = f.clone();
        g.tabs[0].id = "map".into();
        assert!(check(&g).unwrap().contains("sekmesi zaten var"));
        let mut g = f.clone();
        g.tabs[0].after = Some("raster".into());
        assert!(check(&g).unwrap().contains("“raster” sekmesi yok"));
        let mut g = f;
        g.tabs[0].modes = vec!["cad".into()];
        assert!(
            check(&g).unwrap().contains("sekmesi yok"),
            "a cad tab is no gis ribbon's"
        );
    }

    #[test]
    fn a_file_that_adds_a_command_and_its_panel_holds() {
        assert_eq!(check(&file()), None);
    }

    #[test]
    fn the_rules_say_what_is_wrong() {
        let mut f = file();
        f.commands[0].id = "file.open".into();
        assert!(check(&f).unwrap().contains("zaten var"));
        let mut f = file();
        f.commands[0].icon = Some("yok".into());
        assert!(check(&f).unwrap().contains("ikonu"));
        let mut f = file();
        f.ribbon[0].after = Some("Ölçme".into());
        assert!(check(&f).unwrap().contains("paneli yok"));
        let mut f = file();
        f.ribbon[0].tab = "raster".into();
        assert!(check(&f).unwrap().contains("sekmesi yok"));
        let mut f = file();
        f.ribbon[0].label = Some("Parsel".into());
        assert!(check(&f).unwrap().contains("zaten var"));
        let mut f = file();
        f.ribbon[0].items[0].size = "huge".into();
        assert!(check(&f).unwrap().contains("boyu"));
        let mut f = file();
        f.icons.insert(
            "x".into(),
            "<svg viewBox=\"0 0 20 20\"><path d=\"M1 1h2\"/></svg>".into(),
        );
        assert!(check(&f).unwrap().contains("envanterin biçiminde"));
        let mut f = file();
        f.icons
            .insert("basemap".into(), "<path d=\"M1 1h2\"/>".into());
        assert!(check(&f).unwrap().contains("ikonu web'in envanterinde"));
        // The ▾ list: known commands, each once in the place.
        let mut f = file();
        f.ribbon[0].overflow = vec!["file.close".into()];
        assert!(check(&f).unwrap().contains("▾ listesinde"));
        let mut f = file();
        f.ribbon[0].overflow = vec!["address.search".into()];
        assert!(check(&f).unwrap().contains("iki kez"));
        let mut f = file();
        f.ribbon[0].overflow = vec!["file.open".into(), "file.open".into()];
        assert!(check(&f).unwrap().contains("iki kez"));
        let mut f = file();
        f.ribbon[0].label = None;
        f.ribbon[0].icon = None;
        f.ribbon[0].after = None;
        f.ribbon[0].into = Some("Parsel".into());
        assert_eq!(check(&f), None);
    }
}
