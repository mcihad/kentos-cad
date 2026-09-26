//! The desktop shell's interface commands, taken from the web app's feature
//! inventory (docs/inventory/web.json; TODOS.md BASE-04, UI-11). Every web
//! command appears here with its title, aliases, shortcuts and its place in
//! the ribbon and menus, in the web's order, so the desktop shows the same
//! program and ports it command by command (docs/adr/0017).
//!
//! [`PORTED`] lists the commands the desktop runs; the others say, when
//! used, that they exist on the web and are on their way here. The inventory
//! (`pnpm inventory`) reads `apps/desktop/ported.json`, which a test keeps
//! equal to [`PORTED`], to fill its desktop column.

use std::collections::HashMap;
use std::sync::OnceLock;

use kentos_ui::icon::Icon;
use serde::Deserialize;

use crate::icons;

/// The web inventory, embedded when the desktop is built.
const INVENTORY: &str = include_str!("../../../docs/inventory/web.json");

/// Web command ids the desktop runs. Keep in the order they were ported.
pub const PORTED: &[&str] = &[
    "file.open",
    "file.save",
    "file.saveAs",
    "view.theme.dark",
    "view.theme.light",
    "view.theme.toggle",
    "view.ribbonCollapse",
    "commandline.focus",
    "help.about",
    "help.shortcuts",
    "layer.showAll",
    "edit.undo",
    "edit.redo",
    // The tool session (docs/adr/0021): the closed-area tool, its confirm and
    // cancel, and the view commands the interaction traces use.
    "tool.polygon",
    "tool.confirm",
    "tool.cancel",
    "view.zoomIn",
    "view.zoomOut",
    "view.zoomExtents",
    // Typed settings (docs/adr/0023): the settings window, and the drafting
    // aids of the session the tool session reads.
    "tools.options",
    "draft.ortho",
    "draft.polar",
    // The line and polyline tools (docs/adr/0027), each writing through its product command.
    "tool.line",
    "tool.polyline",
];

/// Where a command stands, from the desktop's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// The desktop runs it.
    Ported,
    /// The web runs it; the desktop does not yet.
    OnTheWeb,
    /// Not built anywhere yet (pending on the web too).
    Pending,
}

/// One interface command.
#[derive(Debug, Clone)]
pub struct Command {
    pub id: &'static str,
    pub title: &'static str,
    /// Short name for tight places (ribbon buttons); the title otherwise.
    pub short: &'static str,
    pub description: &'static str,
    /// Command line spellings: the first is the shown name.
    pub aliases: &'static [&'static str],
    /// Key chords as the web writes them (`Ctrl+S`, `Shift+H`, `F2`).
    pub shortcuts: &'static [&'static str],
    pub icon: Icon,
    pub standing: Standing,
    /// Why the web itself does not run it yet (e.g. “Yakında”).
    pub pending_note: Option<&'static str>,
}

impl Command {
    /// The name the command line shows and accepts first.
    pub fn name(&self) -> &'static str {
        self.aliases.first().copied().unwrap_or(self.id)
    }

    /// Tooltip body: what it does, and where it stands on the desktop.
    pub fn note(&self) -> String {
        let standing = match self.standing {
            Standing::Ported => String::new(),
            Standing::OnTheWeb => "Web'de var; masaüstüne henüz taşınmadı.".to_owned(),
            Standing::Pending => self
                .pending_note
                .map_or("Geliştirme aşamasında.".to_owned(), |note| {
                    format!("{note}.")
                }),
        };
        match (self.description.is_empty(), standing.is_empty()) {
            (true, _) => standing,
            (false, true) => self.description.to_owned(),
            (false, false) => format!("{}\n\n{standing}", self.description),
        }
    }
}

/// A ribbon tab and its panels, in the web's order.
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: &'static str,
    pub label: &'static str,
    /// Shown only in a context (the web's selection tab); left out here.
    pub contextual: bool,
    pub panels: Vec<Panel>,
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub label: &'static str,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// A command button, large or small.
    Command { id: &'static str, large: bool },
    /// A family of tools behind one button (Dikdörtgen ▾): the first is shown.
    Split { ids: Vec<&'static str>, large: bool },
    /// A drop-down button with a submenu of commands.
    Menu {
        label: &'static str,
        ids: Vec<&'static str>,
        large: bool,
    },
    /// A panel the web draws itself (layer picker, properties, selection);
    /// the desktop has these as docked panels.
    Builtin,
}

/// The catalog: commands by id and the ribbon.
#[derive(Debug, Default)]
pub struct Catalog {
    commands: Vec<Command>,
    by_id: HashMap<&'static str, usize>,
    tabs: Vec<Tab>,
    quick: Vec<&'static str>,
}

impl Catalog {
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.by_id.get(id).map(|&i| &self.commands[i])
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Ribbon tabs, the contextual ones left out.
    pub fn tabs(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter().filter(|tab| !tab.contextual)
    }

    /// Quick access bar (Kaydet, Geri al, Yinele).
    pub fn quick(&self) -> &[&'static str] {
        &self.quick
    }

    /// Reads the embedded inventory. The strings live as long as the program
    /// (the catalog is built once), which the command line's borrowed
    /// suggestions need.
    fn load(text: &str) -> Result<Self, String> {
        let raw: RawInventory =
            serde_json::from_str(text).map_err(|e| format!("web envanteri okunamadı: {e}"))?;

        let commands: Vec<Command> = raw
            .commands
            .into_iter()
            .map(|c| {
                let id = leak(c.id);
                let title = leak(c.title);
                let standing = if PORTED.contains(&id) {
                    Standing::Ported
                } else if c.status == "pending" {
                    Standing::Pending
                } else {
                    Standing::OnTheWeb
                };
                Command {
                    id,
                    title,
                    short: c.short.map_or(title, leak),
                    description: leak(c.description.unwrap_or_default()),
                    aliases: leak_list(c.aliases),
                    shortcuts: leak_list(c.shortcuts),
                    icon: icons::from_web(c.icon.as_deref()),
                    standing,
                    pending_note: c.pending_note.map(leak),
                }
            })
            .collect();
        let by_id = commands
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        let tabs = raw
            .layout
            .ribbon
            .into_iter()
            .map(|tab| Tab {
                id: leak(tab.id),
                label: leak(tab.label),
                contextual: tab.contextual.is_some(),
                panels: tab
                    .panels
                    .into_iter()
                    .map(|panel| Panel {
                        label: leak(panel.label),
                        items: panel.items.into_iter().map(item).collect(),
                    })
                    .collect(),
            })
            .collect();

        Ok(Self {
            commands,
            by_id,
            tabs,
            quick: raw.layout.quick_access.into_iter().map(leak).collect(),
        })
    }
}

/// The catalog, built on first use.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        Catalog::load(INVENTORY).unwrap_or_else(|error| {
            // The embedded file is checked by a test; a broken build still opens.
            eprintln!("{error}");
            Catalog::default()
        })
    })
}

fn item(raw: RawItem) -> Item {
    let large = |size: &str| size == "large";
    match raw {
        RawItem::Command { command, size } => Item::Command {
            id: leak(command),
            large: large(&size),
        },
        RawItem::Split { split, size } => Item::Split {
            ids: split.into_iter().map(|s| leak(s.command)).collect(),
            large: large(&size),
        },
        RawItem::Menu { menu, size, blocks } => Item::Menu {
            label: leak(menu),
            ids: flatten(blocks),
            large: large(&size),
        },
        RawItem::Builtin {} => Item::Builtin,
    }
}

/// Every command id of a menu's blocks and submenus, in order.
fn flatten(blocks: Vec<RawBlock>) -> Vec<&'static str> {
    blocks
        .into_iter()
        .flat_map(|block| block.items)
        .flat_map(|entry| match entry {
            RawEntry::Command(id) => vec![leak(id)],
            RawEntry::Submenu { items, .. } => flatten(items),
        })
        .collect()
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn leak_list(list: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        list.into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

// ── The inventory's shape (only the parts read here) ─────────────────────

#[derive(Deserialize)]
struct RawInventory {
    commands: Vec<RawCommand>,
    layout: RawLayout,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCommand {
    id: String,
    title: String,
    short: Option<String>,
    icon: Option<String>,
    description: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    shortcuts: Vec<String>,
    status: String,
    pending_note: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLayout {
    ribbon: Vec<RawTab>,
    quick_access: Vec<String>,
}

#[derive(Deserialize)]
struct RawTab {
    id: String,
    label: String,
    contextual: Option<String>,
    panels: Vec<RawPanel>,
}

#[derive(Deserialize)]
struct RawPanel {
    label: String,
    items: Vec<RawItem>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawItem {
    Command {
        command: String,
        size: String,
    },
    Split {
        split: Vec<RawSplit>,
        size: String,
    },
    Menu {
        menu: String,
        size: String,
        blocks: Vec<RawBlock>,
    },
    Builtin {},
}

#[derive(Deserialize)]
struct RawSplit {
    command: String,
}

#[derive(Deserialize)]
struct RawBlock {
    items: Vec<RawEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Command(String),
    Submenu { items: Vec<RawBlock> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_embedded_inventory_reads_and_every_ribbon_command_is_known() {
        let catalog = Catalog::load(INVENTORY).expect("the web inventory parses");
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
                        Item::Split { ids, .. } | Item::Menu { ids, .. } => ids.clone(),
                        Item::Builtin => vec![],
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

    #[test]
    fn every_ported_command_is_a_web_command() {
        let catalog = Catalog::load(INVENTORY).expect("the web inventory parses");
        for id in PORTED {
            assert!(
                catalog.get(id).is_some(),
                "{id} is not in the web inventory"
            );
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
}
