/// Each typed name (a command's alias or title, a method's alias) names one command or one method
/// (docs/adr/0281): the command line runs what it names, never the first of several.
#[test]
fn no_typed_name_names_two_things() {
    use std::collections::BTreeMap;
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // A name means one thing in its context: the sheet designer's commands apart from the drawing's.
    let key = |id: &str, name: String| {
        if id.starts_with("sheet.") {
            format!("pafta: {name}")
        } else {
            name
        }
    };
    for c in super::catalog().commands() {
        let mut own: Vec<String> = c.aliases.iter().map(|a| crate::app::fold(a)).collect();
        own.sort();
        own.dedup();
        for name in own {
            names
                .entry(key(c.id, name))
                .or_default()
                .push(c.id.to_owned());
        }
    }
    for tab in super::catalog().every_tab() {
        for panel in &tab.panels {
            for item in &panel.items {
                if let super::Item::Split { entries, .. } = item {
                    for e in entries.iter().filter(|e| e.option.is_some()) {
                        for a in e.aliases {
                            let at = format!("{} {}", e.id, e.option.unwrap_or(""));
                            let list = names.entry(crate::app::fold(a)).or_default();
                            if !list.contains(&at) {
                                list.push(at);
                            }
                        }
                    }
                }
            }
        }
    }
    // A tool and its own method by the method's name are one thing: the method is meant.
    let base = |id: &String| id.split(' ').next().unwrap_or("").to_owned();
    let clashes: Vec<String> = names
        .into_iter()
        .filter(|(_, ids)| ids.iter().any(|id| base(id) != base(&ids[0])))
        .map(|(name, ids)| format!("{name}: {}", ids.join(", ")))
        .collect();
    assert!(
        clashes.is_empty(),
        "{} names name several:\n{}",
        clashes.len(),
        clashes.join("\n")
    );
}

use super::*;
use std::path::PathBuf;

#[test]
fn the_embedded_inventory_reads_and_every_ribbon_command_is_known() {
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the web inventory parses");
    assert!(
        catalog.commands().len() > 100,
        "the web has more commands than this"
    );
    assert!(catalog.tabs().count() >= 5);
    for tab in catalog.tabs() {
        for panel in &tab.panels {
            for item in &panel.items {
                let ids = match item {
                    Item::Command { id, .. } => vec![*id],
                    Item::Split { entries, .. } => entries.iter().map(|e| e.id).collect(),
                    Item::Menu { ids, .. } => ids.clone(),
                    Item::Builtin(_) => vec![],
                };
                for id in ids {
                    assert!(
                        catalog.get(id).is_some(),
                        "{} › {}: {id} is not a command",
                        tab.label,
                        panel.label
                    );
                }
            }
        }
    }
}

/// A tool's methods keep their names and options (Daire ▾: 2 nokta is
/// the circle tool, then 2N), a family's tools their titles (docs/adr/0032).
#[test]
fn split_entries_keep_their_methods() {
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the web inventory parses");
    // Whichever project type's ribbon shows them (CBS has no Dikdörtgen).
    let splits: Vec<&Vec<Entry>> = catalog
        .every_tab()
        .flat_map(|tab| tab.panels.iter())
        .flat_map(|panel| panel.items.iter())
        .filter_map(|item| match item {
            Item::Split { entries, .. } => Some(entries),
            _ => None,
        })
        .collect();
    let of = |id: &str| {
        let entries = splits
            .iter()
            .find(|entries| entries[0].id == id)
            .unwrap_or_else(|| panic!("a split button starts with {id}"));
        entries
            .iter()
            .map(|e| (e.id, e.label, e.option))
            .collect::<Vec<_>>()
    };
    let circle = "tool.circle";
    assert_eq!(
        of(circle),
        [
            (circle, "Merkez, yarıçap", None),
            (circle, "2 nokta", Some("2N")),
            (circle, "3 nokta", Some("3N")),
            (circle, "Teğet, teğet, yarıçap", Some("TTY")),
            (circle, "Teğet, teğet, teğet", Some("TTT")),
            // The additions' (docs/adr/0287).
            (circle, "Merkez, çap", Some("MÇ")),
        ]
    );
    let arc = "tool.arc";
    assert_eq!(
        of(arc),
        [
            (arc, "3 nokta", None),
            (arc, "Merkez, başlangıç, bitiş", Some("M")),
            (arc, "Devam", Some("D")),
            (arc, "Başlangıç, merkez, bitiş", Some("BMS")),
            (arc, "Başlangıç, merkez, açı", Some("BMA")),
            (arc, "Başlangıç, merkez, uzunluk", Some("BMU")),
            (arc, "Başlangıç, bitiş, açı", Some("BSA")),
            (arc, "Başlangıç, bitiş, yön", Some("BSY")),
            (arc, "Başlangıç, bitiş, yarıçap", Some("BSR")),
            (arc, "Merkez, başlangıç, açı", Some("MBA")),
            (arc, "Merkez, başlangıç, uzunluk", Some("MBU")),
        ]
    );
    assert_eq!(
        of("tool.rectangle"),
        [
            ("tool.rectangle", "Dikdörtgen", None),
            ("tool.rectangle", "Dikdörtgen, boyutlarla", Some("BOYUT")),
            ("tool.rectangle", "Dikdörtgen, alanla", Some("ALAN")),
            ("tool.rectangle3", "Döndürülmüş dikdörtgen", None),
            ("tool.regularPolygon", "Düzgün çokgen", None),
            (
                "tool.regularPolygon",
                "Düzgün çokgen, çember içinde",
                Some("İÇ")
            ),
            (
                "tool.regularPolygon",
                "Düzgün çokgen, çember dışında",
                Some("DIŞ")
            ),
            ("tool.regularPolygon", "Düzgün çokgen, kenardan", Some("K")),
        ]
    );
    // A tool's plain button becomes a split of its methods (docs/adr/0287).
    let ellipse = "tool.ellipse";
    assert_eq!(
        of(ellipse),
        [
            (ellipse, "Eksen, uç", None),
            (ellipse, "Merkez", Some("M")),
            (ellipse, "Eliptik yay", Some("Y")),
        ]
    );
    let cloud = "tool.revcloud";
    assert_eq!(
        of(cloud),
        [
            (cloud, "Revizyon bulutu", None),
            (cloud, "Dikdörtgen", Some("D")),
            (cloud, "Çokgen", Some("Ç")),
            (cloud, "Nesne", Some("N")),
        ]
    );
}

/// Every method the additions give (docs/adr/0287) names an option its tool takes before its first
/// point: the tool, started as the ribbon starts it, answers it; a method in a ▾ list only (Yardımcı
/// çizgi's) is still known.
#[test]
fn every_added_method_is_an_option_its_tool_takes() {
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the catalog loads");
    assert!(catalog.added_methods().len() >= 20);
    for m in catalog.added_methods() {
        let Some(option) = m.option else { continue };
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.run_method(m.id, option, m.label);
        let tool = m.id.strip_prefix("tool.").unwrap_or(m.id);
        assert_eq!(app.session.tool_id(), tool, "{} › {}", m.id, m.label);
        assert!(
            !app.log
                .lines()
                .any(|l| l.level == kentos_interaction::Level::Warn),
            "{} › {} ({option}) is taken",
            m.id,
            m.label
        );
    }
}

/// The catalog's additions (catalog/v1/additions.json, docs/adr/0228 §4,
/// docs/adr/0229 §15) hold against the web's inventory by the contract's
/// rules; every element of their icons is drawn here; their commands and
/// places are in the catalog of each project type.
#[test]
fn the_additions_hold_against_the_inventory() {
    use std::collections::HashSet;
    let raw: serde_json::Value = serde_json::from_str(INVENTORY).expect("the inventory parses");
    let added: kentos_commands::CatalogAdditions =
        serde_json::from_str(ADDITIONS).expect("the additions parse");
    let commands: HashSet<&str> = raw["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .filter_map(|c| c["id"].as_str())
        .collect();
    let icon_names: HashSet<&str> = raw["icons"]
        .as_object()
        .expect("icons")
        .keys()
        .map(String::as_str)
        .collect();
    let panels = |mode: &str, tab: &str| -> Option<Vec<String>> {
        raw["layout"]["ribbonByMode"][mode]
            .as_array()?
            .iter()
            .find(|t| t["id"] == tab)
            .map(|t| {
                t["panels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["label"].as_str().map(str::to_owned))
                    .collect()
            })
    };
    let command = |id: &str| commands.contains(id);
    let icon = |name: &str| icon_names.contains(name);
    assert_eq!(
        added.problem(&kentos_commands::KnownCatalog {
            command: &command,
            icon: &icon,
            panels: &panels,
        }),
        None
    );
    for (name, markup) in &added.icons {
        assert!(
            !markup.contains("<g"),
            "{name}: the icon reader here has no groups"
        );
        let tags: usize = ["<path", "<circle", "<rect", "<ellipse"]
            .iter()
            .map(|t| markup.matches(t).count())
            .sum();
        assert_eq!(
            kentos_ui::icon::svg_elements(markup),
            tags,
            "{name}: every element is drawn"
        );
    }
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the catalog loads");
    for c in &added.commands {
        let got = catalog
            .get(&c.id)
            .unwrap_or_else(|| panic!("{} is in the catalog", c.id));
        assert!(matches!(got.icon, Icon::Svg(_)), "{} takes its icon", c.id);
        assert_eq!(
            got.standing == Standing::Ported,
            c.status == kentos_commands::AddedStatus::Implemented
        );
    }
    // Adres (docs/adr/0622 §3): Adres ara in CBS's Harita › Sorgu and CAD's Ekle › Adres, the
    // address data in CBS's Analiz › Adres.
    use kentos_commands::Workspace;
    let holds = |mode: Workspace, tab: &str, panel: &str, id: &str| {
        catalog
            .tabs_in(mode)
            .find(|t| t.id == tab)
            .and_then(|t| t.panels.iter().find(|p| p.label == panel))
            .is_some_and(|p| {
                p.items
                    .iter()
                    .any(|i| matches!(i, Item::Command { id: c, .. } if *c == id))
            })
    };
    assert!(holds(Workspace::Gis, "map", "Sorgu", "address.search"));
    assert!(holds(Workspace::Cad, "insert", "Adres", "address.search"));
    assert!(holds(Workspace::Gis, "analysis", "Adres", "address.data"));
    // A project not asked its type shows CBS's ribbon.
    assert!(catalog.tabs().any(|t| t.id == "map"));
}

/// The desktop's ribbon (docs/adr/0622 §3) loses nothing the web's ribbon and the additions placed
/// that the desktop runs: each such command is on a tab, a contextual tab, a split, a menu or a ▾
/// list. İşlemler's tools stay in the toolbox (and their contextual tabs); the type switches are
/// gone with the switching (§1); what does not run here is left out.
#[test]
fn the_ribbon_keeps_every_command_the_old_one_placed() {
    use std::collections::BTreeSet;
    let raw: serde_json::Value = serde_json::from_str(INVENTORY).expect("the inventory parses");
    let added: serde_json::Value = serde_json::from_str(ADDITIONS).expect("the additions parse");
    let mut old: BTreeSet<String> = BTreeSet::new();
    fn walk(items: &serde_json::Value, out: &mut BTreeSet<String>) {
        for item in items.as_array().into_iter().flatten() {
            if let Some(id) = item.as_str() {
                out.insert(id.to_owned());
            } else if let Some(id) = item["command"].as_str() {
                out.insert(id.to_owned());
            } else if let Some(entries) = item["split"].as_array() {
                for e in entries {
                    out.extend(e["command"].as_str().map(str::to_owned));
                }
            } else if let Some(blocks) = item["blocks"].as_array() {
                for b in blocks {
                    walk(&b["items"], out);
                }
            } else if item["items"].is_array() {
                walk(&item["items"], out);
            }
        }
    }
    for tabs in raw["layout"]["ribbonByMode"].as_object().expect("the modes").values() {
        for tab in tabs.as_array().into_iter().flatten() {
            for panel in tab["panels"].as_array().into_iter().flatten() {
                walk(&panel["items"], &mut old);
                walk(&panel["overflow"], &mut old);
            }
        }
    }
    for place in added["ribbon"].as_array().into_iter().flatten() {
        walk(&place["items"], &mut old);
        walk(&place["overflow"], &mut old);
    }
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the catalog loads");
    let mut now: BTreeSet<&str> = BTreeSet::new();
    for tab in catalog
        .mode_tabs
        .iter()
        .flat_map(|(_, tabs)| tabs.iter())
    {
        for panel in &tab.panels {
            now.extend(panel.overflow.iter().copied());
            if let Some(Launcher {
                target: LauncherTarget::Command(id),
                ..
            }) = &panel.launcher
            {
                now.insert(id);
            }
            for item in &panel.items {
                match item {
                    Item::Command { id, .. } => {
                        now.insert(id);
                    }
                    Item::Split { entries, .. } => now.extend(entries.iter().map(|e| e.id)),
                    Item::Menu { ids, .. } => now.extend(ids.iter().copied()),
                    Item::Builtin(_) => {}
                }
            }
        }
    }
    // The drafting aids are the status bar's lamps; Yardım's commands its ? menu and Komut ara.
    now.extend(crate::view::STATUS_AIDS.iter().map(|(id, _)| *id));
    now.extend(catalog.menu("help").iter().flatten().copied());
    now.insert("view.commandSearch");
    let lost: Vec<&String> = old
        .iter()
        .filter(|id| !now.contains(id.as_str()))
        .filter(|id| !id.starts_with("processing.run.") && !id.starts_with("workspace."))
        .filter(|id| catalog.get(id).is_some_and(|c| c.standing == Standing::Ported))
        .collect();
    assert!(lost.is_empty(), "the ribbon lost {lost:?}");
    // And every İşlemler tool the old ribbon placed is a tool the toolbox lists.
    let app = crate::files_testing::app_with_drawing();
    let tools: BTreeSet<String> = app
        .gis
        .processing
        .registry
        .tools()
        .iter()
        .map(|t| t.id.to_string())
        .collect();
    let gone: Vec<&String> = old
        .iter()
        .filter_map(|id| id.strip_prefix("processing.run.").map(|t| (id, t)))
        .filter(|(id, t)| !tools.contains(*t) && catalog.get(id).is_some_and(|c| c.standing == Standing::Ported))
        .map(|(id, _)| id)
        .collect();
    assert!(gone.is_empty(), "not in the toolbox: {gone:?}");
}

#[test]
fn every_ported_command_is_a_web_command() {
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the web inventory parses");
    for id in PORTED {
        assert!(
            catalog.get(id).is_some(),
            "{id} is not in the web inventory"
        );
    }
}

/// The chords that pass a text field are the web's `allowInInput`
/// bindings, read from the inventory: Ctrl+S and the F-keys, not Ctrl+Z
/// (the text field's own), each also a shortcut of its command.
#[test]
fn chords_that_work_in_text_fields_come_from_the_web() {
    let catalog = Catalog::load(INVENTORY, ADDITIONS, RIBBON).expect("the web inventory parses");
    let in_input: Vec<&str> = catalog
        .commands()
        .iter()
        .flat_map(|c| c.shortcuts_in_input.iter().copied())
        .collect();
    for chord in ["Ctrl+S", "Ctrl+O", "F8", "Ctrl+,"] {
        assert!(in_input.contains(&chord), "{chord} works in a text field");
    }
    assert!(!in_input.contains(&"Ctrl+Z"));
    for c in catalog.commands() {
        for chord in c.shortcuts_in_input {
            assert!(c.shortcuts.contains(chord), "{}: {chord}", c.id);
        }
    }
}

/// `apps/desktop/ported.json` tells the inventory which commands the
/// desktop runs; `KENTOS_WRITE_PORTED=1` rewrites it.
#[test]
fn ported_file_is_current() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ported.json");
    let text = format!(
        "{}\n",
        serde_json::to_string_pretty(&serde_json::json!({
            "format": "kentos.desktop-ported",
            "version": 1,
            "commands": PORTED,
        }))
        .expect("serializes")
    );
    if std::env::var("KENTOS_WRITE_PORTED").as_deref() == Ok("1") {
        std::fs::write(&path, &text).expect("writes ported.json");
        return;
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_default(),
        text,
        "apps/desktop/ported.json is out of date: KENTOS_WRITE_PORTED=1 cargo test -p kentos-desktop ported"
    );
}
