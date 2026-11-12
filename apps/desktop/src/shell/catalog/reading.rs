//! The catalog read from the embedded inventory, layout and additions: commands placed, methods added,
//! split buttons resolved, entries flattened.

use super::*;

impl Catalog {
    /// Reads the embedded inventory and its additions. The strings live as
    /// long as the program (the catalog is built once), which the command
    /// line's borrowed suggestions need.
    pub(super) fn load(text: &str, additions: &str, ribbon: &str) -> Result<Self, String> {
        let mut raw: RawInventory =
            serde_json::from_str(text).map_err(|e| format!("web envanteri okunamadı: {e}"))?;
        let added: kentos_commands::CatalogAdditions = serde_json::from_str(additions)
            .map_err(|e| format!("katalog ekleri okunamadı: {e}"))?;
        // Before the commands and panels take their icons; the additions' icons are new names.
        icons::keep_web_set(
            std::mem::take(&mut raw.icons)
                .into_iter()
                .chain(added.icons.clone())
                .map(|(name, markup)| (leak(name), leak(markup)))
                .collect(),
        );

        let commands: Vec<Command> = raw
            .commands
            .into_iter()
            .map(|c| {
                let id = leak(c.id);
                let title = DESKTOP_TITLES
                    .iter()
                    .find(|(desktop, _)| *desktop == id)
                    .map_or_else(|| leak(c.title), |(_, t)| *t);
                let standing = if PORTED.contains(&id) {
                    Standing::Ported
                } else if c.status == "pending" {
                    Standing::Pending
                } else {
                    Standing::OnTheWeb
                };
                let description = DESKTOP_DESCRIPTIONS
                    .iter()
                    .find(|(desktop, _)| *desktop == id)
                    .map_or_else(|| leak(c.description.unwrap_or_default()), |(_, d)| *d);
                let pending_note = c.pending_note.map(leak);
                // The sheet mode's keys are its own (docs/adr/0164): the web binds them only
                // while a sheet is in front, and the desktop's sheet mode takes them itself
                // (sheets.rs). In the drawing's key table they took Esc, F2, H, V and Home.
                let sheet_keys = id.starts_with("sheet.");
                Command {
                    id,
                    title,
                    short: c.short.map_or(title, leak),
                    description,
                    line_note: line_note(standing, pending_note, description),
                    aliases: leak_list(c.aliases),
                    shortcuts: if sheet_keys {
                        &[]
                    } else {
                        leak_list(c.shortcuts)
                    },
                    shortcuts_in_input: if sheet_keys {
                        &[]
                    } else {
                        leak_list(c.shortcuts_in_input)
                    },
                    icon: icons::from_web(
                        DESKTOP_ICONS
                            .iter()
                            .find(|(desktop, _)| *desktop == id)
                            .map(|(_, i)| *i)
                            .or(c.icon.as_deref()),
                    ),
                    standing,
                    pending_note,
                    category: leak(c.category.unwrap_or_default()),
                }
            })
            .chain(desktop_commands())
            .chain(added.commands.iter().map(added_command))
            .collect();
        let by_id = commands
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        // The tools' methods the additions give (docs/adr/0287), as split entries.
        let methods: Vec<Entry> = added
            .methods
            .iter()
            .map(|m| {
                let title = commands
                    .iter()
                    .find(|c| c.id == m.command)
                    .map_or("", |c| c.title);
                Entry {
                    id: leak(m.command.clone()),
                    title,
                    label: leak(m.label.clone()),
                    option: m.option.clone().map(leak),
                    description: m.description.clone().map(leak),
                    aliases: leak_list(m.aliases.clone()),
                    icon: m.icon.clone().map(leak),
                }
            })
            .collect();
        // The inventory's split buttons by key: the desktop's ribbon names them (docs/adr/0622 §3).
        let mut splits: HashMap<String, RawItem> = HashMap::new();
        for raw_tab in raw
            .layout
            .ribbon
            .iter()
            .chain(raw.layout.ribbon_by_mode.values().flatten())
        {
            for raw_item in raw_tab.panels.iter().flat_map(|p| p.items.iter()) {
                if let RawItem::Split { key, .. } = raw_item
                    && !key.is_empty()
                {
                    splits.entry(key.clone()).or_insert_with(|| raw_item.clone());
                }
            }
        }
        let ribbon: RawRibbon =
            serde_json::from_str(ribbon).map_err(|e| format!("şerit okunamadı: {e}"))?;
        if ribbon.format != "kentos.ribbon" || ribbon.version != 1 {
            return Err(format!(
                "şeridin biçimi tanınmadı: {} sürüm {}",
                ribbon.format, ribbon.version
            ));
        }
        let mut mode_tabs: Vec<(kentos_commands::Workspace, Vec<Tab>)> = Vec::new();
        let mut work_tabs: Vec<(kentos_commands::CadWork, Vec<Tab>)> = Vec::new();
        for (name, raw_tabs) in &ribbon.tabs {
            let Ok(mode) = serde_json::from_value(serde_json::Value::String(name.clone())) else {
                continue;
            };
            // A tab or panel shows for a work its `works` name, or for every work without them (docs/adr/0801);
            // so do its buttons, a split's entries, menu blocks and ▾ commands (docs/adr/0802), and in a project
            // type its `modes` name.
            let shown =
                |p: &RawPanel, work: Option<&str>| shows(&p.works, work) && in_mode(&p.modes, name);
            let build = |work: Option<&str>| {
                let mut tabs: Vec<Tab> = raw_tabs
                    .iter()
                    .filter(|t| shows(&t.works, work))
                    .map(|t| {
                        let panels = t
                            .panels
                            .iter()
                            .filter(|p| shown(p, work))
                            .map(|p| cut(p, work, name, &splits))
                            .collect();
                        tab(
                            t.id.clone(),
                            t.label.clone(),
                            false,
                            Vec::new(),
                            panels,
                            &splits,
                        )
                    })
                    .collect();
                for c in ribbon
                    .contextual
                    .iter()
                    .filter(|c| c.modes.is_empty() || c.modes.contains(name))
                {
                    tabs.push(tab(
                        c.id.clone(),
                        c.label.clone(),
                        true,
                        c.kinds.clone(),
                        c.panels
                            .iter()
                            .filter(|p| shown(p, work))
                            .map(|p| cut(p, work, name, &splits))
                            .collect(),
                        &splits,
                    ));
                }
                place_methods(&mut tabs, &methods);
                tabs
            };
            mode_tabs.push((mode, build(None)));
            if mode == kentos_commands::Workspace::Cad {
                for work in kentos_commands::CadWork::ALL {
                    work_tabs.push((work, build(Some(work.id()))));
                }
            }
        }
        // The ribbon a project not asked its type shows is CBS's.
        let tabs = mode_tabs
            .iter()
            .find(|(m, _)| *m == kentos_commands::Workspace::Gis)
            .map(|(_, tabs)| tabs.clone())
            .unwrap_or_default();
        fn tab(
            id: String,
            label: String,
            contextual: bool,
            kinds: Vec<String>,
            panels: Vec<RawPanel>,
            splits: &HashMap<String, RawItem>,
        ) -> Tab {
            Tab {
                id: leak(id),
                label: leak(label),
                contextual,
                kinds: leak_list(kinds),
                panels: panels
                    .into_iter()
                    .map(|panel| Panel {
                        label: leak(panel.label),
                        icon: icons::from_web(panel.icon.as_deref()),
                        items: panel
                            .items
                            .into_iter()
                            .map(|raw| item(resolve(raw, splits)))
                            .collect(),
                        overflow: panel
                            .overflow
                            .into_iter()
                            .map(|o| leak(o.command()))
                            .collect(),
                        keep: panel.keep,
                        roles: leak_list(panel.roles),
                        launcher: panel.launcher.map(|l| Launcher {
                            title: leak(l.title),
                            target: match (l.tab, l.command) {
                                (Some(tab), _) => LauncherTarget::Tab(leak(tab)),
                                (None, command) => {
                                    LauncherTarget::Command(leak(command.unwrap_or_default()))
                                }
                            },
                        }),
                    })
                    .collect(),
            }
        }

        // The web's order (the contract's): the inventory lists them by id.
        let mut modes: Vec<Mode> = raw
            .workspaces
            .into_iter()
            .filter_map(|w| {
                Some(Mode {
                    id: serde_json::from_value(serde_json::Value::String(w.id)).ok()?,
                    label: leak(w.label),
                    title: leak(w.title),
                    description: leak(w.description.unwrap_or_default()),
                    highlights: leak_list(w.highlights),
                    ready: w.status == "implemented",
                })
            })
            .collect();
        modes.sort_by_key(|m| m.id as u8);

        // What each module's program offers (docs/adr/0802): its type's ribbon cut to its works, contextual tabs too,
        // and the commands the ribbon file gives it beyond them; every command some ribbon holds, and those the ribbon
        // file gives only some modules (`only`): none of them is the shell's.
        let only: Vec<(&'static str, &[String])> = ribbon
            .modules
            .only
            .iter()
            .map(|(id, modules)| (leak(id.clone()), modules.as_slice()))
            .collect();
        let ribbon_commands: HashSet<&'static str> = mode_tabs
            .iter()
            .flat_map(|(_, tabs)| tabs.iter())
            .chain(work_tabs.iter().flat_map(|(_, tabs)| tabs.iter()))
            .flat_map(Tab::commands)
            .chain(only.iter().map(|(id, _)| *id))
            .collect();
        let everywhere: Vec<&'static str> = ribbon
            .modules
            .everywhere
            .iter()
            .map(|id| leak(id.clone()))
            .collect();
        let map_only: Vec<&'static str> =
            ribbon.modules.map.iter().map(|id| leak(id.clone())).collect();
        let module_commands = kentos_launch::Module::ALL
            .into_iter()
            .map(|module| {
                let tabs: Vec<&Tab> = match module.project_type() {
                    "gis" => mode_tabs
                        .iter()
                        .filter(|(m, _)| *m == kentos_commands::Workspace::Gis)
                        .flat_map(|(_, tabs)| tabs.iter())
                        .collect(),
                    _ => work_tabs
                        .iter()
                        .filter(|(w, _)| module.cad_works().contains(&w.id()))
                        .flat_map(|(_, tabs)| tabs.iter())
                        .collect(),
                };
                let mut ids: HashSet<&'static str> =
                    tabs.into_iter().flat_map(Tab::commands).collect();
                ids.extend(everywhere.iter().copied());
                // KentOS Yol works on a map's coordinates and layers (plan/road.md ROAD-MOD): a layer's system,
                // GIS files, sources, labels and typed layers are a road's too.
                if matches!(
                    module,
                    kentos_launch::Module::Map | kentos_launch::Module::Road
                ) {
                    ids.extend(map_only.iter().copied());
                }
                ids.extend(
                    only.iter()
                        .filter(|(_, modules)| modules.iter().any(|m| m == module.id()))
                        .map(|(id, _)| *id),
                );
                (module, ids)
            })
            .collect();

        Ok(Self {
            commands,
            by_id,
            tabs,
            mode_tabs,
            work_tabs,
            module_commands,
            ribbon_commands,
            quick: raw.layout.quick_access.into_iter().map(leak).collect(),
            menus: raw
                .layout
                .menus
                .into_iter()
                .map(|menu| {
                    let blocks = menu
                        .blocks
                        .into_iter()
                        .map(|block| flatten(vec![block]))
                        .collect();
                    (leak(menu.id), blocks)
                })
                .collect(),
            modes,
            methods,
        })
    }
}

/// Puts the tools' methods in a ribbon (docs/adr/0287): after the tool's last
/// entry in a split that has it, the entries it has already left out; a
/// tool's plain button becomes a split of its methods, the default first (the
/// method without an option, else one named by the tool's title).
pub(super) fn place_methods(tabs: &mut [Tab], methods: &[Entry]) {
    for panel in tabs.iter_mut().flat_map(|t| t.panels.iter_mut()) {
        for item in &mut panel.items {
            match item {
                Item::Split { entries, .. } => {
                    for m in methods.iter().filter(|m| m.option.is_some()) {
                        let Some(last) = entries.iter().rposition(|e| e.id == m.id) else {
                            continue;
                        };
                        if entries.iter().any(|e| e.id == m.id && e.option == m.option) {
                            continue;
                        }
                        entries.insert(last + 1, m.clone());
                    }
                }
                Item::Command { id, size } => {
                    let own: Vec<&Entry> = methods.iter().filter(|m| m.id == *id).collect();
                    if !own.iter().any(|m| m.option.is_some()) {
                        continue;
                    }
                    let mut entries: Vec<Entry> = Vec::new();
                    if !own.iter().any(|m| m.option.is_none()) {
                        let title = own[0].title;
                        entries.push(Entry {
                            id: own[0].id,
                            title,
                            label: title,
                            option: None,
                            description: None,
                            aliases: &[],
                            icon: None,
                        });
                    }
                    entries.extend(own.iter().map(|m| (*m).clone()));
                    let key = id.strip_prefix("tool.").unwrap_or(id);
                    *item = Item::Split {
                        key,
                        entries,
                        size: *size,
                    };
                }
                _ => {}
            }
        }
    }
}

/// A command of the additions (docs/adr/0228 §4): it runs on the desktop unless it waits.
pub(super) fn added_command(c: &kentos_commands::AddedCommand) -> Command {
    let standing = match c.status {
        kentos_commands::AddedStatus::Implemented => Standing::Ported,
        kentos_commands::AddedStatus::Pending => Standing::Pending,
    };
    let title = leak(c.title.clone());
    let description = leak(c.description.clone());
    let pending_note = c.pending_note.clone().map(leak);
    Command {
        id: leak(c.id.clone()),
        title,
        short: c.short.clone().map_or(title, leak),
        description,
        line_note: line_note(standing, pending_note, description),
        aliases: leak_list(c.aliases.clone()),
        shortcuts: leak_list(c.shortcuts.clone()),
        shortcuts_in_input: &[],
        icon: icons::from_web(c.icon.as_deref()),
        standing,
        pending_note,
        category: leak(c.category.clone()),
    }
}

/// Whether something that names `works` shows for a CAD work (docs/adr/0801): it names it or no work; everything
/// shows without a work (a CAD project not asked its work, CBS).
pub(super) fn shows(works: &[String], work: Option<&str>) -> bool {
    work.is_none_or(|w| works.is_empty() || works.iter().any(|x| x == w))
}

/// Whether something that names project types shows in this one (a contextual tab's panel, button, menu block or ▾
/// command, docs/adr/0802): it names it or none.
pub(super) fn in_mode(modes: &[String], mode: &str) -> bool {
    modes.is_empty() || modes.iter().any(|m| m == mode)
}

/// A panel as a CAD work of a project type (`cad`, `gis`) shows it (docs/adr/0802): its name in the work, its buttons,
/// a split's entries, menu blocks and ▾ commands that name the work (or none) and the type (or none); a menu with no
/// block left goes, and a split with no entry. A split named by its key is the inventory's (`splits`) from here on.
pub(super) fn cut(
    panel: &RawPanel,
    work: Option<&str>,
    mode: &str,
    splits: &HashMap<String, RawItem>,
) -> RawPanel {
    let mut panel = panel.clone();
    if let Some(label) = work.and_then(|w| panel.work_labels.get(w)) {
        panel.label.clone_from(label);
    }
    let keeps = |works: &[String], modes: &[String]| shows(works, work) && in_mode(modes, mode);
    panel.items = std::mem::take(&mut panel.items)
        .into_iter()
        .map(|item| resolve(item, splits))
        .collect();
    panel.items.retain_mut(|item| match item {
        RawItem::Command { works, modes, .. } | RawItem::SplitRef { works, modes, .. } => {
            keeps(works, modes)
        }
        RawItem::Split {
            works,
            modes,
            split,
            ..
        } => {
            split.retain(|e| shows(&e.works, work));
            keeps(works, modes) && !split.is_empty()
        }
        RawItem::Menu {
            works,
            modes,
            blocks,
            ..
        } => {
            blocks.retain(|b| keeps(&b.works, &b.modes));
            keeps(works, modes) && !blocks.is_empty()
        }
        RawItem::Builtin { .. } => true,
    });
    panel.overflow.retain(|o| keeps(o.works(), o.modes()));
    panel
}

/// A split button named by its key made the inventory's, with this size and its entries' works (docs/adr/0622 §3,
/// 0802); any other item as it is.
pub(super) fn resolve(raw: RawItem, splits: &HashMap<String, RawItem>) -> RawItem {
    match raw {
        RawItem::SplitRef {
            split,
            size,
            works,
            modes,
            entry_works,
        } => match splits.get(&split) {
            Some(RawItem::Split { split: entries, key, .. }) => RawItem::Split {
                split: entries
                    .iter()
                    .map(|e| {
                        let mut e = e.clone();
                        if let Some(only) = entry_works.get(&e.label) {
                            e.works.clone_from(only);
                        }
                        e
                    })
                    .collect(),
                key: key.clone(),
                size,
                works,
                modes,
            },
            // A key the inventory lacks: a test keeps every key known; a broken build still opens.
            _ => RawItem::Builtin {
                builtin: String::new(),
            },
        },
        other => other,
    }
}

pub(super) fn item(raw: RawItem) -> Item {
    let size = |size: &str| match size {
        "large" => Size::Large,
        "lead" => Size::Lead,
        _ => Size::Small,
    };
    match raw {
        RawItem::Command {
            command, size: s, ..
        } => Item::Command {
            id: leak(command),
            size: size(&s),
        },
        RawItem::Split {
            split,
            key,
            size: s,
            ..
        } => Item::Split {
            entries: split
                .into_iter()
                .map(|s| {
                    let label = leak(s.label);
                    Entry {
                        id: leak(s.command),
                        title: s.title.map_or(label, leak),
                        label,
                        option: s.option.map(leak),
                        description: s.description.map(leak),
                        aliases: leak_list(s.aliases),
                        icon: s.icon.map(leak),
                    }
                })
                .collect(),
            key: leak(key),
            size: size(&s),
        },
        RawItem::Menu {
            menu,
            size: s,
            blocks,
            checklist,
            ..
        } => {
            let blocks: Vec<(&'static str, Vec<&'static str>)> = blocks
                .into_iter()
                .map(|block| (leak(block.label), entries(block.items)))
                .collect();
            Item::Menu {
                label: leak(menu),
                ids: blocks
                    .iter()
                    .flat_map(|(_, ids)| ids.iter().copied())
                    .collect(),
                blocks,
                checklist,
                size: size(&s),
            }
        }
        RawItem::Builtin { builtin } => Item::Builtin(leak(builtin)),
        // Resolved before (`resolve`).
        RawItem::SplitRef { .. } => Item::Builtin(""),
    }
}

/// Every command id of a menu's blocks and submenus, in order.
pub(super) fn flatten(blocks: Vec<RawBlock>) -> Vec<&'static str> {
    blocks
        .into_iter()
        .flat_map(|block| entries(block.items))
        .collect()
}

/// Every command id of a block's entries and their submenus, in order.
pub(super) fn entries(items: Vec<RawEntry>) -> Vec<&'static str> {
    items
        .into_iter()
        .flat_map(|entry| match entry {
            RawEntry::Command(id) => vec![leak(id)],
            RawEntry::Submenu { items, .. } => flatten(items),
        })
        .collect()
}

/// The command line list's note of a command (`Command::line_note`).
pub(super) fn line_note(
    standing: Standing,
    pending_note: Option<&'static str>,
    description: &'static str,
) -> &'static str {
    let standing = match standing {
        Standing::Ported => return description,
        Standing::OnTheWeb => "Web'de var; masaüstüne henüz taşınmadı.".to_owned(),
        Standing::Pending => format!("{}.", pending_note.unwrap_or("Geliştirme aşamasında")),
    };
    leak(if description.is_empty() {
        standing
    } else {
        format!("{standing} {description}")
    })
}

pub(super) fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

pub(super) fn leak_list(list: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        list.into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}
