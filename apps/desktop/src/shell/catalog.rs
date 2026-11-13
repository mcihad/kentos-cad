//! Desktop command names, aliases, icons and shortcuts from the frozen web inventory plus additions.
//! Placement belongs to the desktop's `catalog/v1/ribbon.json` (ADR 0622): project-specific tabs and the selected
//! object's contextual tab. `PORTED` records working desktop handlers; a pending command keeps its reason.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use kentos_ui::icon::Icon;
use serde::Deserialize;

use crate::icons;

/// The web inventory, embedded when the desktop is built.
const INVENTORY: &str = include_str!("../../../../docs/inventory/web.json");

/// The commands added while the web is paused (docs/adr/0228 §4): their
/// titles, icons and ribbon places, beside the frozen inventory's.
const ADDITIONS: &str = include_str!("../../../../catalog/v1/additions.json");

/// The desktop's ribbon (docs/adr/0622 §3–§4): each project type's tabs and
/// the tabs a selection's kinds open. Its split buttons by key take the
/// inventory's (a tool's methods, a family's tools); the web's frozen ribbon
/// and the additions' places are not read for the desktop any more.
const RIBBON: &str = include_str!("../../../../catalog/v1/ribbon.json");

/// The product commands each desktop command and tool goes through (`via`, docs/adr/0244; hand-kept beside their
/// levels): what an agent on a module program's link may run (docs/adr/8056 §1).
const OPERATIONS: &str = include_str!("../../../../catalog/v1/operations.json");

/// Where the desktop does otherwise than the web, its own description: the
/// web's would say otherwise (docs/adr/0053: the desktop reads a zipped
/// Shapefile; docs/adr/0058: no search box in the ribbon yet, Esc cancels first).
/// The desktop's own titles of commands the frozen web inventory names otherwise: the tab row's search looks
/// through the menus (Menülerde ara), as Office's search box does, not through a list of commands.
const DESKTOP_TITLES: &[(&str, &str)] = &[("view.commandSearch", "Menülerde ara")];

/// The desktop's own icons of commands of the frozen web inventory: Nokta draws a solid dot, as a layer without a
/// point symbol now draws its points (docs/adr/1540).
const DESKTOP_ICONS: &[(&str, &str)] = &[("tool.point", "pointDot")];

const DESKTOP_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "file.open",
        "KCAD, DWG, DXF veya NCZ çizimini doğrudan açar; önce yeni proje oluşturmak gerekmez. Dış CAD dosyası ayrı bir CAD çizimidir; Kaydet yeni bir .kcad dosyası ister, kaynak dosyaya dokunmaz. Mevcut projeye eklemek için İçe aktar kullanın.",
    ),
    (
        "file.import.shp",
        "Shapefile katmanını içe aktarır: .shp, .shx, .dbf, .prj ve .cpg dosyaları birlikte ya da katmanın .zip arşivi seçilir (arşivdeki her .shp bir katmandır, biri seçilir). Noktalar, çizgiler ve delikli alanlar, köşe kotlarıyla (Z); .dbf alanları metin öznitelik olur. .prj'deki sistem gösterilir; projeninkinden başkaysa alınmaz.",
    ),
    (
        "view.commandSearch",
        "Bir komutu adıyla ya da takma adıyla bulup çalıştırır: komut satırı bütün komutların listesini açar, yazdıkça süzer.",
    ),
    (
        "view.fullscreen",
        "Uygulamayı ekranın tamamına yayar; aynı düğme ya da, iptal edilecek komut ve seçim yokken, Esc çıkar.",
    ),
];

/// The desktop's own command: the web has no Python console (python/,
/// docs/adr/0132). Kept out of [`PORTED`], which names web commands only.
pub const PYTHON_CONSOLE: &str = "python.console";

/// İşlem araç kutusu: a processing tool or model no ribbon holds is offered where it is (docs/adr/0802).
pub const TOOLBOX: &str = "processing.toolbox";
/// A processing tool's command: `processing.run.<tool>`.
pub const PROCESSING_RUN: &str = "processing.run.";
/// A model's command: `processing.model.<model>` (the user's models too).
pub const PROCESSING_MODEL: &str = "processing.model.";

/// The desktop's own commands, beside the web's.
fn desktop_commands() -> Vec<Command> {
    let description = "Alt paneli Python sekmesinde açar: çizimde kentos.cad ile kod ve betik çalıştırılır, ajan bağlantısı açılır; açıksa kapatır.";
    vec![Command {
        id: PYTHON_CONSOLE,
        title: "Python konsolu",
        short: "Python",
        description,
        line_note: description,
        aliases: &["PYTHON", "KONSOL"],
        shortcuts: &[],
        shortcuts_in_input: &[],
        icon: icons::from_web(Some("terminal")),
        standing: Standing::Ported,
        pending_note: None,
        category: "Araçlar",
    }]
}

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
    /// What the command line's list says of it: where it stands first, when
    /// it does not run here, then what it does (the list has two lines).
    pub line_note: &'static str,
    /// Command line spellings: the first is the shown name.
    pub aliases: &'static [&'static str],
    /// Key chords as the web writes them (`Ctrl+S`, `Shift+H`, `F2`).
    pub shortcuts: &'static [&'static str],
    /// Those of them that work while a text field has the keyboard too: the
    /// web binds them with `allowInInput` (app/keybindings.ts; the
    /// inventory's `shortcutsInInput`).
    pub shortcuts_in_input: &'static [&'static str],
    pub icon: Icon,
    pub standing: Standing,
    /// Why the web itself does not run it yet (e.g. “Yakında”).
    pub pending_note: Option<&'static str>,
    /// The web's category (“Dosya”, “Çizim”): Komut ara says it of a command
    /// the ribbon does not have.
    pub category: &'static str,
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

/// A work mode (the web's `WORKSPACES`, app/workspaces.ts): what it is
/// for, and whether it can be chosen yet.
#[derive(Debug, Clone)]
pub struct Mode {
    pub id: kentos_commands::Workspace,
    pub label: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub highlights: &'static [&'static str],
    /// Announced but not built: shown last, dimmed, “Yakında”.
    pub ready: bool,
}

/// The type a project not asked its type yet, or of an announced type,
/// shows as (the web's `FALLBACK_WORKSPACE`, docs/adr/0165 §1).
pub const FALLBACK_MODE: kentos_commands::Workspace = kentos_commands::Workspace::Gis;

/// The type the interface shows for a project's setting: its own when it
/// can be chosen, else CBS (the web's `effectiveWorkspace`).
pub fn effective_mode(workspace: Option<kentos_commands::Workspace>) -> &'static Mode {
    use kentos_commands::Workspace;
    let modes = catalog().modes();
    let wanted = workspace
        .filter(|w| *w != Workspace::LegacyHybrid)
        .unwrap_or(FALLBACK_MODE);
    modes
        .iter()
        .find(|m| m.id == wanted && m.ready)
        .or_else(|| modes.iter().find(|m| m.id == FALLBACK_MODE))
        .unwrap_or(&modes[0])
}

/// The name of the mode a project works in (see [`effective_mode`]).
pub fn mode_of(workspace: Option<kentos_commands::Workspace>) -> &'static str {
    effective_mode(workspace).label
}

/// A mode's command id (`workspace.cad`).
pub fn mode_command(mode: kentos_commands::Workspace) -> &'static str {
    use kentos_commands::Workspace;
    match mode {
        Workspace::Cad => "workspace.cad",
        // A project whose type is not asked shows as CBS (docs/adr/0165 §1).
        Workspace::Gis | Workspace::LegacyHybrid => "workspace.gis",
        Workspace::Plan3d => "workspace.plan3d",
        Workspace::Disaster => "workspace.disaster",
    }
}

/// A ribbon tab and its panels, in the web's order.
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: &'static str,
    pub label: &'static str,
    /// Shown only in a context (the web's Seçim tab, while something is
    /// selected; `contextual_in`), left out of `tabs` and `tabs_in`.
    pub contextual: bool,
    /// A contextual tab's object kinds (`point`, `polygon` …): it opens when
    /// every selected object is of one of them; `*` for any selection no
    /// other tab takes (docs/adr/0622 §4).
    pub kinds: &'static [&'static str],
    pub panels: Vec<Panel>,
}

impl Tab {
    /// Every command the tab offers: its panels' (`Panel::commands`).
    pub fn commands(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.panels.iter().flat_map(Panel::commands)
    }
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub label: &'static str,
    /// The icon of the button the panel folds into when the window is narrow.
    pub icon: Icon,
    pub items: Vec<Item>,
    /// Seldom used commands: under the ▾ beside the panel's title (the web's `overflow`).
    pub overflow: Vec<&'static str>,
    /// Keeps its large buttons until the tab's other panels show icons only.
    pub keep: bool,
    /// The corner button of the title (↘): another tab, or a command.
    pub launcher: Option<Launcher>,
    /// The roles the selection's objects must all have for a contextual panel to show (docs/adr/2480): `surface`
    /// (a terrain surface's points or breaklines), `network` (a utility network's pipes), `zoning` (a zoning plan's
    /// areas); none: every selection.
    pub roles: &'static [&'static str],
}

impl Panel {
    /// Every command the panel offers: its buttons, a family's tools, a menu's items, its seldom used ones and its
    /// launcher's command (the web's `panelCommands`).
    pub fn commands(&self) -> Vec<&'static str> {
        let mut ids = Vec::new();
        for item in &self.items {
            match item {
                Item::Command { id, .. } => ids.push(*id),
                Item::Split { entries, .. } => ids.extend(entries.iter().map(|e| e.id)),
                Item::Menu { ids: menu, .. } => ids.extend(menu.iter().copied()),
                Item::Builtin(_) => {}
            }
        }
        ids.extend(self.overflow.iter().copied());
        if let Some(Launcher {
            target: LauncherTarget::Command(id),
            ..
        }) = &self.launcher
        {
            ids.push(id);
        }
        ids
    }
}

/// A panel's launcher (the web's `RibbonLauncher`).
#[derive(Debug, Clone)]
pub struct Launcher {
    pub title: &'static str,
    pub target: LauncherTarget,
}

#[derive(Debug, Clone)]
pub enum LauncherTarget {
    Tab(&'static str),
    /// A command (the web may pass it a settings section; the desktop's
    /// settings window opens whole).
    Command(&'static str),
}

/// How large a ribbon button is drawn (the web's `RibbonSize`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Large,
    /// A panel's lead (Ölçülendirme): large while the panel shows labels,
    /// small only when it is down to icons (DESIGN.md §7.3.1).
    Lead,
    Small,
}

#[derive(Debug, Clone)]
pub enum Item {
    /// A command button.
    Command { id: &'static str, size: Size },
    /// A family of tools behind one button (Dikdörtgen ▾), or one tool's
    /// methods (Daire ▾: 2 nokta, 3 nokta …): the last chosen is shown
    /// (the layout's `ribbonSplits`, by `key`: the family or the tool).
    Split {
        key: &'static str,
        entries: Vec<Entry>,
        size: Size,
    },
    /// A drop-down button with a submenu of commands.
    Menu {
        label: &'static str,
        /// Every command of its blocks and submenus, in order.
        ids: Vec<&'static str>,
        /// Its blocks, by title ('' for none), each block's commands in order.
        blocks: Vec<(&'static str, Vec<&'static str>)>,
        /// A list ticked one row after another (the web's
        /// `SubmenuSpec.checklist`, docs/adr/0187 §5): short names with
        /// icons, titled blocks under headers, open as rows are ticked.
        checklist: bool,
        size: Size,
    },
    /// A panel the web draws itself, by its name: `layers` (the active
    /// layer), `properties` (the current properties, the plot scale),
    /// `selection` (the selection's summary); ribbon_panels.rs draws them.
    Builtin(&'static str),
}

/// One entry of a split button's menu (the web's `SplitEntry`, docs/adr/0032).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: &'static str,
    /// The button's text: the tool's name (a method does not rename it).
    pub title: &'static str,
    /// Its name in the menu: a family's tool by its title (`Düzgün çokgen`),
    /// a method by its own (`2 nokta`).
    pub label: &'static str,
    /// What a method gives the tool once it runs, as if typed (`2N`); none
    /// for a tool's default method and for a family's tools.
    pub option: Option<&'static str>,
    /// What it does (the method's hint).
    pub description: Option<&'static str>,
    /// Typed names that start the tool with this method (`DOR`: Ölçülendirme
    /// as Koordinat; the web's `ToolMethod.aliases`, docs/adr/0147 §7).
    pub aliases: &'static [&'static str],
    /// The method's own icon (Ölçülendirme's kinds; the web's `ToolMethod.icon`);
    /// none: its command's.
    pub icon: Option<&'static str>,
}

/// The catalog: commands by id and the ribbon.
#[derive(Debug, Default)]
pub struct Catalog {
    commands: Vec<Command>,
    by_id: HashMap<&'static str, usize>,
    /// The ribbon as a project not asked its type shows it: CBS's
    /// (the inventory's `layout.ribbon`, docs/adr/0165).
    tabs: Vec<Tab>,
    /// The ribbon of every ready type, as the web builds it with the type's
    /// filter (`app/workspaces.ts`): what it hides left out, its own tab
    /// names (CAD's Harita is Ölçme).
    mode_tabs: Vec<(kentos_commands::Workspace, Vec<Tab>)>,
    /// The CAD ribbon cut to each work (docs/adr/0801): the tabs and panels whose `works` name it, or none.
    work_tabs: Vec<(kentos_commands::CadWork, Vec<Tab>)>,
    /// The commands each KentOS module's program offers (docs/adr/0802): its ribbon's, contextual tabs too, and the
    /// ones the ribbon file gives it beyond them.
    module_commands: Vec<(kentos_launch::Module, HashSet<&'static str>)>,
    /// Every command some ribbon holds: one no ribbon holds is the shell's (Geri al, Komut ara …), every module's.
    ribbon_commands: HashSet<&'static str>,
    quick: Vec<&'static str>,
    modes: Vec<Mode>,
    /// The menu bar's menus by id: their blocks of command ids, in order.
    menus: Vec<(&'static str, Vec<Vec<&'static str>>)>,
    /// The tools' methods the additions give (docs/adr/0287): in the ribbon's
    /// splits, and typed by their names wherever the tool's button is.
    methods: Vec<Entry>,
}

impl Catalog {
    /// A menu bar menu's blocks of command ids (`help` is the ribbon's ?).
    pub fn menu(&self, id: &str) -> &[Vec<&'static str>] {
        self.menus
            .iter()
            .find(|(menu, _)| *menu == id)
            .map_or(&[], |(_, blocks)| blocks.as_slice())
    }

    pub fn get(&self, id: &str) -> Option<&Command> {
        self.by_id.get(id).map(|&i| &self.commands[i])
    }

    /// The tool's method a typed name starts (an entry of a ribbon split
    /// with that alias and an option), folded as command names are; in every
    /// project type, whichever ribbon shows the method.
    pub fn method_by_alias(&self, text: &str) -> Option<&Entry> {
        let folded = crate::app::fold(text);
        self.every_tab()
            .flat_map(|tab| tab.panels.iter())
            .flat_map(|panel| panel.items.iter())
            .filter_map(|item| match item {
                Item::Split { entries, .. } => Some(entries.iter()),
                _ => None,
            })
            .flatten()
            .chain(self.methods.iter())
            .find(|e| e.option.is_some() && e.aliases.iter().any(|a| crate::app::fold(a) == folded))
    }

    /// The tools' methods the additions give (docs/adr/0287).
    #[cfg(test)]
    pub fn added_methods(&self) -> &[Entry] {
        &self.methods
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Ribbon tabs, the contextual ones left out.
    pub fn tabs(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter().filter(|tab| !tab.contextual)
    }

    /// The tabs of every project type's ribbon (a tab two types share comes
    /// once per type), the contextual ones left out: what holds whatever the
    /// type (a method's typed name, a tab a launcher opens).
    pub fn every_tab(&self) -> impl Iterator<Item = &Tab> {
        self.tabs
            .iter()
            .chain(self.mode_tabs.iter().flat_map(|(_, tabs)| tabs.iter()))
            .filter(|tab| !tab.contextual)
    }

    /// The whole ribbon of a project type (CBS's when the type has none of
    /// its own), the contextual tabs left out: every CAD work's at once. The
    /// window shows `tabs_for`; the tests hold the whole one.
    #[cfg(test)]
    pub fn tabs_in(&self, mode: kentos_commands::Workspace) -> impl Iterator<Item = &Tab> {
        self.tabs_for(mode, None)
    }

    /// The ribbon tabs of a project type cut to a CAD work (docs/adr/0801): a CAD
    /// project's work shows the tabs and panels that name it or no work; no
    /// work, and any other type, shows the type's whole ribbon.
    pub fn tabs_for(
        &self,
        mode: kentos_commands::Workspace,
        work: Option<kentos_commands::CadWork>,
    ) -> impl Iterator<Item = &Tab> {
        let cut = match (mode, work) {
            (kentos_commands::Workspace::Cad, Some(work)) => self
                .work_tabs
                .iter()
                .find(|(w, _)| *w == work)
                .map(|(_, tabs)| tabs),
            _ => None,
        };
        cut.or_else(|| {
            self.mode_tabs
                .iter()
                .find(|(m, _)| *m == mode)
                .map(|(_, tabs)| tabs)
        })
        .unwrap_or(&self.tabs)
        .iter()
        .filter(|tab| !tab.contextual)
    }

    /// Whether a module's program offers a command (docs/adr/0802): its ribbon holds it (for one of its works,
    /// contextual tabs too) or the ribbon file gives it to the module. A processing tool or model no ribbon holds
    /// (the user's models among them) is offered where İşlem araç kutusu is; any other command no ribbon holds and
    /// the file names for no module is the shell's (Geri al, Komut ara …), every module's.
    pub fn module_has(&self, module: kentos_launch::Module, id: &str) -> bool {
        if self.ribbon_commands.contains(id) {
            return self
                .module_commands
                .iter()
                .any(|(m, ids)| *m == module && ids.contains(id));
        }
        if id.starts_with(PROCESSING_RUN) || id.starts_with(PROCESSING_MODEL) {
            return self.module_has(module, TOOLBOX);
        }
        true
    }

    /// The contextual tabs of a project type cut to a CAD work (docs/adr/0802), as `tabs_for` cuts the others.
    pub fn contextual_for_work(
        &self,
        mode: kentos_commands::Workspace,
        work: Option<kentos_commands::CadWork>,
    ) -> impl Iterator<Item = &Tab> {
        let cut = match (mode, work) {
            (kentos_commands::Workspace::Cad, Some(work)) => self
                .work_tabs
                .iter()
                .find(|(w, _)| *w == work)
                .map(|(_, tabs)| tabs),
            _ => None,
        };
        cut.or_else(|| {
            self.mode_tabs
                .iter()
                .find(|(m, _)| *m == mode)
                .map(|(_, tabs)| tabs)
        })
        .unwrap_or(&self.tabs)
        .iter()
        .filter(|tab| tab.contextual)
    }

    /// The contextual ribbon tabs of a project type, every CAD work's at once (the window shows
    /// `contextual_for_work`; the tests hold the whole one).
    #[cfg(test)]
    pub fn contextual_in(&self, mode: kentos_commands::Workspace) -> impl Iterator<Item = &Tab> {
        self.mode_tabs
            .iter()
            .find(|(m, _)| *m == mode)
            .map_or(&self.tabs, |(_, tabs)| tabs)
            .iter()
            .filter(|tab| tab.contextual)
    }

    /// The contextual tab a selection of these kinds opens (docs/adr/0622 §4):
    /// the first whose kinds hold every one of them, else the one for any
    /// selection (`*`); none without kinds.
    pub fn contextual_for(
        &self,
        mode: kentos_commands::Workspace,
        work: Option<kentos_commands::CadWork>,
        kinds: &[&str],
    ) -> Option<&Tab> {
        if kinds.is_empty() {
            return None;
        }
        let fits = |tab: &&Tab| kinds.iter().all(|k| tab.kinds.contains(k));
        self.contextual_for_work(mode, work)
            .find(fits)
            .or_else(|| {
                self.contextual_for_work(mode, work)
                    .find(|tab| tab.kinds.contains(&"*"))
            })
    }

    /// Quick access bar (Kaydet, Geri al, Yinele).
    pub fn quick(&self) -> &[&'static str] {
        &self.quick
    }

    /// The work modes, in the web's order: the ones that can be chosen first.
    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

}

/// The catalog, built on first use.
/// A product command → the desktop commands that go through it, a tool as its button (`tool.<id>`).
fn product_routes() -> &'static HashMap<String, Vec<String>> {
    #[derive(Default, Deserialize)]
    struct Operations {
        #[serde(default)]
        commands: HashMap<String, Operation>,
        #[serde(default)]
        tools: HashMap<String, Operation>,
    }
    #[derive(Deserialize)]
    struct Operation {
        #[serde(default)]
        via: Vec<String>,
    }
    static ROUTES: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    ROUTES.get_or_init(|| {
        let operations: Operations = serde_json::from_str(OPERATIONS).unwrap_or_else(|error| {
            // The embedded file is checked by a test; a broken build still opens, every product command offered.
            eprintln!("{error}");
            Operations::default()
        });
        let mut routes: HashMap<String, Vec<String>> = HashMap::new();
        for (id, operation) in operations.commands {
            for product in operation.via {
                routes.entry(product).or_default().push(id.clone());
            }
        }
        for (id, operation) in operations.tools {
            for product in operation.via {
                routes
                    .entry(product)
                    .or_default()
                    .push(format!("tool.{id}"));
            }
        }
        routes
    })
}

impl Catalog {
    /// Whether a module's interface goes through a product command (docs/adr/8056 §1): a desktop command or tool
    /// that names it in `via` is offered there ([`Catalog::module_has`]). Another discipline's command a module's
    /// ribbon uses (Harita's Katman koordinat sistemi, CBS's `cad.layers.crs`) is that module's too.
    pub fn routes_offer(&self, module: kentos_launch::Module, product: &str) -> bool {
        product_routes()
            .get(product)
            .is_some_and(|routes| routes.iter().any(|id| self.module_has(module, id)))
    }
}

pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        Catalog::load(INVENTORY, ADDITIONS, RIBBON).unwrap_or_else(|error| {
            // The embedded file is checked by a test; a broken build still opens.
            eprintln!("{error}");
            Catalog::default()
        })
    })
}

// ── The inventory's shape (only the parts read here) ─────────────────────

#[derive(Deserialize)]
struct RawInventory {
    commands: Vec<RawCommand>,
    layout: RawLayout,
    #[serde(default)]
    workspaces: Vec<RawMode>,
    /// The web's icon set, name → SVG markup (docs/adr/0054).
    #[serde(default)]
    icons: HashMap<String, String>,
}

#[derive(Deserialize)]
struct RawMode {
    id: String,
    label: String,
    title: String,
    description: Option<String>,
    #[serde(default)]
    highlights: Vec<String>,
    status: String,
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
    #[serde(default)]
    shortcuts_in_input: Vec<String>,
    status: String,
    pending_note: Option<String>,
    category: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLayout {
    ribbon: Vec<RawTab>,
    /// The other ready modes' ribbons, by mode id.
    #[serde(default)]
    ribbon_by_mode: std::collections::BTreeMap<String, Vec<RawTab>>,
    quick_access: Vec<String>,
    /// The menu bar's menus (the web's `MAIN_MENU`): the ribbon's ? opens Yardım.
    #[serde(default)]
    menus: Vec<RawMenu>,
}

#[derive(Deserialize)]
struct RawMenu {
    id: String,
    blocks: Vec<RawBlock>,
}

#[derive(Deserialize)]
struct RawTab {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    label: String,
    panels: Vec<RawPanel>,
}

/// The desktop's ribbon file (`catalog/v1/ribbon.json`, docs/adr/0622 §3).
#[derive(Deserialize)]
struct RawRibbon {
    format: String,
    version: u32,
    /// Each project type's tabs, by its id (`cad`, `gis`).
    tabs: std::collections::BTreeMap<String, Vec<RawRibbonTab>>,
    /// The tabs a selection opens, by the kinds of its objects (docs/adr/0622 §4).
    #[serde(default)]
    contextual: Vec<RawContextual>,
    /// The commands a module's program offers beyond its ribbon (docs/adr/0802).
    #[serde(default)]
    modules: RawModules,
}

/// Commands only CBS's ribbon (or only a work's tabs) holds that a CAD module's program offers too (docs/adr/0802):
/// every module's (`everywhere`: a new layer, the project's coordinate system …) and Harita's (`map`: a layer's
/// coordinate system, labels and filter, bound sources, CBS's file formats); and the commands no ribbon holds that
/// only some modules' programs offer (`only`, by the modules' ids; none: kentos-cad's alone).
#[derive(Deserialize, Default)]
struct RawModules {
    #[serde(default)]
    everywhere: Vec<String>,
    #[serde(default)]
    map: Vec<String>,
    #[serde(default)]
    only: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct RawRibbonTab {
    id: String,
    label: String,
    panels: Vec<RawPanel>,
    /// The CAD works it shows for (docs/adr/0801); none: every work.
    #[serde(default)]
    works: Vec<String>,
}

#[derive(Deserialize)]
struct RawContextual {
    id: String,
    label: String,
    kinds: Vec<String>,
    /// The project types that have it; none: every type.
    #[serde(default)]
    modes: Vec<String>,
    panels: Vec<RawPanel>,
}

#[derive(Deserialize, Clone)]
struct RawPanel {
    label: String,
    /// The CAD works it shows for (docs/adr/0801); none: every work.
    #[serde(default)]
    works: Vec<String>,
    /// The project types that show it (a contextual tab's panel, docs/adr/0802); none: every type.
    #[serde(default)]
    modes: Vec<String>,
    #[serde(default)]
    icon: Option<String>,
    items: Vec<RawItem>,
    #[serde(default)]
    overflow: Vec<RawOverflow>,
    #[serde(default)]
    keep: bool,
    #[serde(default)]
    launcher: Option<RawLauncher>,
    /// Its name in a CAD work that holds other things than `label` says (docs/adr/0802), by the work.
    #[serde(default, rename = "workLabels")]
    work_labels: std::collections::BTreeMap<String, String>,
    /// The roles the selection must have (a contextual panel, docs/adr/2480); none: every selection.
    #[serde(default)]
    roles: Vec<String>,
}

/// A seldom used command under a panel's ▾: its id, or its id and the CAD works (and project types) it shows for
/// (docs/adr/0802).
#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum RawOverflow {
    Id(String),
    Entry {
        command: String,
        #[serde(default)]
        works: Vec<String>,
        #[serde(default)]
        modes: Vec<String>,
    },
}

impl RawOverflow {
    fn works(&self) -> &[String] {
        match self {
            RawOverflow::Id(_) => &[],
            RawOverflow::Entry { works, .. } => works,
        }
    }

    fn modes(&self) -> &[String] {
        match self {
            RawOverflow::Id(_) => &[],
            RawOverflow::Entry { modes, .. } => modes,
        }
    }

    fn command(self) -> String {
        match self {
            RawOverflow::Id(id) | RawOverflow::Entry { command: id, .. } => id,
        }
    }
}

#[derive(Deserialize, Clone)]
struct RawLauncher {
    title: String,
    tab: Option<String>,
    command: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum RawItem {
    Command {
        command: String,
        size: String,
        /// The CAD works it shows for (docs/adr/0802); none: every work.
        #[serde(default)]
        works: Vec<String>,
        /// The project types that show it (in a contextual tab, docs/adr/0802); none: every type.
        #[serde(default)]
        modes: Vec<String>,
    },
    /// The inventory's split button with this key (docs/adr/0622 §3).
    SplitRef {
        split: String,
        size: String,
        #[serde(default)]
        works: Vec<String>,
        #[serde(default)]
        modes: Vec<String>,
        /// The CAD works some of its entries show for, by their labels (docs/adr/0802); the others: every work.
        #[serde(default, rename = "entryWorks")]
        entry_works: std::collections::BTreeMap<String, Vec<String>>,
    },
    Split {
        split: Vec<RawSplit>,
        /// The button's key: the tools' family, or the tool with methods.
        #[serde(default)]
        key: String,
        size: String,
        #[serde(default)]
        works: Vec<String>,
        #[serde(default)]
        modes: Vec<String>,
    },
    Menu {
        menu: String,
        size: String,
        blocks: Vec<RawBlock>,
        #[serde(default)]
        checklist: bool,
        #[serde(default)]
        works: Vec<String>,
        #[serde(default)]
        modes: Vec<String>,
    },
    Builtin {
        builtin: String,
    },
}

#[derive(Deserialize, Clone)]
struct RawSplit {
    command: String,
    label: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    option: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    icon: Option<String>,
    /// The CAD works it shows for (docs/adr/0802: a split by key takes them from its `entryWorks`); none: every work.
    #[serde(default)]
    works: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct RawBlock {
    /// Its title; '' for an untitled block.
    #[serde(default)]
    label: String,
    items: Vec<RawEntry>,
    /// The CAD works it shows for (docs/adr/0802); none: every work.
    #[serde(default)]
    works: Vec<String>,
    /// The project types that show it (docs/adr/0802); none: every type.
    #[serde(default)]
    modes: Vec<String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum RawEntry {
    Command(String),
    Submenu { items: Vec<RawBlock> },
}

#[cfg(test)]
#[path = "catalog/tests.rs"]
mod tests;

#[path = "catalog/ported.rs"]
mod ported;
pub use ported::*;

#[path = "catalog/reading.rs"]
mod reading;
