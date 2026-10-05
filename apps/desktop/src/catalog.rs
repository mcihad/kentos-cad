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
    "view.keyTips",
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
    "draft.rightAngle",
    "draft.polar",
    // The line and polyline tools (docs/adr/0027), each writing through its product command.
    "tool.line",
    "tool.polyline",
    // Selecting, deleting and snapping (docs/adr/0029): the select tool, the erase tool
    // through cad.entities.delete (Delete), F3 and the selection commands.
    "tool.select",
    "tool.erase",
    "draft.snap",
    "draft.grid",
    "edit.deselect",
    "edit.selectAll",
    "edit.invertSelection",
    // Drawing tools, round 2 (docs/adr/0032): point, circle and arc through their own
    // product commands; the rectangles and the regular polygon through cad.polygon.create.
    "tool.point",
    "tool.circle",
    "tool.arc",
    "tool.rectangle",
    "tool.rectangle3",
    "tool.regularPolygon",
    // Modify tools, round 1 (docs/adr/0037): move, copy, rotate, scale and mirror on the
    // selection, through cad.entities.transform.
    "tool.move",
    "tool.copy",
    "tool.rotate",
    "tool.scale",
    "tool.mirror",
    // Modify tools, round 2 (docs/adr/0047): the edge, corner and object tools, through
    // cad.entities.edit.
    "tool.offset",
    "tool.trim",
    "tool.extend",
    "tool.fillet",
    "tool.chamfer",
    "tool.break",
    "tool.join",
    "tool.explode",
    "tool.lengthen",
    "tool.vertex",
    // The cloud interface (docs/adr/0041): signing in and out, the catalog of cloud
    // projects, a drawing uploaded as a new project, and the save conflicts.
    "cloud.signIn",
    "cloud.signOut",
    "cloud.open",
    "cloud.upload",
    "cloud.conflicts",
    // The open project's own actions, Buluta dosya olarak kaydet and Son
    // revizyonu aç (cloud/actions.rs, docs/adr/0073).
    "cloud.uploadFile",
    "cloud.openNewest",
    // Proje geçmişi…: the catalog's Geçmiş tab on the open project (docs/adr/0087).
    "cloud.history",
    "cloud.rename",
    "cloud.delete",
    // Projeyi paylaş: the people, their roles and the invitations (cloud/share.rs, docs/adr/0111).
    "cloud.share",
    // New layers and groups (layering.rs): edits that are not undone, as on the web.
    "layer.new",
    "layer.newGroup",
    // File exchange (exchange/): DXF and coordinate lists in and out, through the shared
    // readers and writers; the source coordinate system is asked, never reprojected.
    "file.import.dxf",
    "file.import.ncn",
    "crs.points",
    "file.export.dxf",
    "file.export.ncn",
    // A new project and the project's settings (project/): the web's windows; the
    // standard layer tree checked against the web's drawings (fixtures/project/v1).
    "file.new",
    "file.settings",
    // The start screen with the recent files (start.rs).
    "file.start",
    // The project types that can be chosen (modes.rs, docs/adr/0165); the announced
    // ones stay pending.
    "workspace.cad",
    "workspace.gis",
    // GeoJSON in and out, a Shapefile in from its files or its zip archive
    // (exchange/gis_import.rs, geojson_export.rs; docs/adr/0046, 0053): what the file
    // says of its coordinate system is shown and checked, never reprojected.
    "file.import.geojson",
    "file.import.shp",
    "file.export.geojson",
    // GNSS içe aktar (exchange/gnss_import.rs, docs/adr/0169 §6): a receiver's GPX or
    // NMEA positions moved from WGS 84 into the project's system, with their accuracy.
    "file.import.gnss",
    // Cihaza gönder (exchange/field_send.rs, docs/adr/0169 §4): the selected points as an
    // instrument's coordinate file, through the shared writer.
    "field.send",
    // Modify tools, round 3 (docs/adr/0047, part 2): stretch through cad.entities.edit,
    // the arrays through cad.entities.array, align through cad.entities.transform.
    "tool.stretch",
    "tool.array",
    "tool.arrayPolar",
    "tool.align",
    // The bottom panel's tabs (bottom.rs): F2, the history, the coordinate list, warnings.
    "view.bottomPanel",
    "view.coords",
    // Nokta editörü: the bottom panel's Noktalar tab (points/, docs/adr/0153).
    "point.editor",
    // The clipboard and the view tools (docs/adr/0056): cut, copy and the pastes over the
    // session's clipboard, written through the document as on the web; pan, zoom window,
    // zoom to the selection and repeating the last command.
    "edit.cut",
    "edit.copy",
    "edit.paste",
    "edit.pasteOriginal",
    "tool.pan",
    "tool.zoomWindow",
    "view.zoomSelection",
    "tool.repeat",
    // The window's arrangement and the server check (view_commands.rs): the side panels
    // (F4), full screen, the command search (Alt+Q); Koordinat sistemi… is Proje ayarları
    // on its page (project/).
    "view.rightPanel",
    "view.fullscreen",
    "view.commandSearch",
    "crs.set",
    "server.check",
    // Drawing tools, round 3 (docs/adr/0057): the ellipse, spline, construction lines,
    // parallel line, perpendiculars, donut and divide through cad.entities.create; the
    // revision cloud through cad.polygon.create, the spot elevation through cad.point.create.
    "tool.ellipse",
    "tool.spline",
    "tool.xline",
    "tool.ray",
    "tool.parallel",
    "tool.perpIn",
    "tool.perpOut",
    "tool.donut",
    "tool.revcloud",
    "tool.spot",
    "tool.divide",
    // Yazı and the text field over the drawing, which also edits a text's or a
    // dimension's value on a double click (docs/adr/0060).
    "tool.text",
    // Ölçülendirme: aligned, linear, angular, radius and diameter dimensions
    // through cad.entities.create (docs/adr/0061); ordinate and arc length, its
    // methods by their own names too (docs/adr/0147 §7).
    "tool.dimension",
    // Tarama: the region by a closed object or by the line work, islands left
    // out, through cad.entities.create as “Tarama” (docs/adr/0062).
    "tool.hatch",
    // Blocks (docs/adr/0144).
    "tool.blockDefine",
    "tool.blockInsert",
    "block.panel",
    "block.purge",
    // Blok öznitelikleri: a block's attribute definitions (docs/adr/0144 §7).
    "block.attributes",
    // Alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine
    // tıklayarak alan; into the document as the web's (docs/adr/0065).
    // Mesafe ölç, Alan hesapla and Parsel oluştur: the path tool's other
    // shapes; a parcel through cad.entities.create on the parcel layer (docs/adr/0067).
    "tool.measure",
    "tool.area",
    "tool.parcel",
    // Hesap: Önden and Geriden kestirme, Aplikasyon; Çizimden picks a point
    // with the pick tool (calc/, docs/adr/0070). Poligon hesabı and Kutupsal
    // alım with the measurements table (docs/adr/0071).
    "calc.forward",
    "calc.resection",
    "calc.stakeout",
    "calc.traverse",
    "calc.polar",
    "calc.fieldbook",
    // Vektör oturtma: control points, Helmert, affine or projective by least squares,
    // written through cad.entities.transform (calc/fit/, docs/adr/0156 §7).
    "transform.fit",
    // Kenar eşleme: the line ends of two sheets put together across their edge,
    // written through cad.entities.edit (calc/edgematch/, docs/adr/0159 §9).
    "transform.edgematch",
    // Koordinat dönüştür: a point or a list between the coordinate systems,
    // nothing written (calc/convert.rs, docs/adr/0167 §4).
    "crs.transform",
    "tool.areaUnion",
    "tool.areaIntersect",
    "tool.areaSubtract",
    "tool.areaSplit",
    "tool.toArea",
    "tool.toPolyline",
    "tool.boundary",
    // Toplu alan: every region the line work closes, its label the attribute (polygonize.rs, docs/adr/0151).
    "tool.polygonize",
    // Bitişik alan: the region a drawn path closes with the neighbouring areas (adjoin.rs, docs/adr/0162 §3).
    "tool.adjoin",
    // Köşelere nokta: a named point at every vertex of the selection (vertex_points.rs, docs/adr/0152 §5).
    "tool.vertexPoints",
    // Etiketleri yazıya çevir: the layers' labels as texts at a scale (labels_to_text.rs, docs/adr/0175 §3).
    "tool.labelsToText",
    // Drawing and editing tools, phase 1 (docs/adr/0140): every corner at once, Parçala
    // with its three methods, direction, thinning, cleaning and property copying.
    "tool.filletAll",
    "tool.chamferAll",
    "tool.split",
    "tool.reverse",
    "tool.simplify",
    // Okunur yap: the texts that read upside down turned half round (docs/adr/0145 §6).
    "tool.readable",
    // Bul ve değiştir's window (find_replace.rs, docs/adr/0145 §6).
    "text.findReplace",
    // Metin dosyası yerleştir: a text file's lines as texts (text_file.rs, docs/adr/0145 §6).
    "tool.placeTextFile",
    // Kılavuz: the arrow, its line, landing and note (leader.rs, docs/adr/0146 §7).
    "tool.leader",
    "tool.cleanup",
    // Topolojik temizlik: ends, vertices, extend and trim within a tolerance (topology.rs, docs/adr/0148).
    "tool.topology",
    "tool.matchProperties",
    // Phase 2 and 3 of docs/adr/0140: the slice, points between two points, the point found from
    // distances, bearings or lines, the angle, the chained and stacked dimensions, the coordinate
    // read (Harita › Koordinatlar) and the array along a path.
    "tool.sector",
    "tool.pointsBetween",
    "tool.intersectPoint",
    "tool.measureAngle",
    "tool.dimContinue",
    "tool.dimBaseline",
    // docs/adr/0147 §7: Hızlı ölçü.
    "tool.quickDimension",
    "crs.query",
    "tool.arrayPath",
    // İşlemler (docs/adr/0084): each tool's and model's window, and Harita's
    // Kenar ölçülerini yaz, which opens Kenar uzunluklarını yaz.
    "processing.run.points.numberVertices",
    "processing.run.annotation.edgeLengths",
    "processing.run.attributes.calculate",
    "processing.run.selection.byExpression",
    "processing.model.builtin.parcelSheet",
    "processing.newModel",
    "map.edgeLengths",
    // The dock's İşlemler tab: the toolbox and this session's runs (docs/adr/0084, part 2).
    "processing.toolbox",
    "processing.history",
    // Nesne izleme, Shift+F3 (docs/adr/0085).
    "draft.tracking",
    // Topolojik düzenleme and its Noktalar da (docs/adr/0160).
    "draft.topology",
    "draft.topologyPoints",
    // The digitizing locks (docs/adr/0166).
    "draft.lock.length",
    "draft.lock.angle",
    "draft.lock.deflection",
    "draft.lock.parallel",
    "draft.lock.perpendicular",
    "draft.lock.reference",
    "draft.lock.construction",
    "draft.lock.keep",
    "draft.lock.clear",
    // Çakışma denetimi and its modes (docs/adr/0162).
    "draft.overlap",
    "draft.overlap.allow",
    "draft.overlap.layer",
    "draft.overlap.layers",
    // The snap kinds one by one and Çizilmekte olan nesneye (docs/adr/0163 §6).
    "draft.snap.endpoint",
    "draft.snap.midpoint",
    "draft.snap.intersection",
    "draft.snap.center",
    "draft.snap.perpendicular",
    "draft.snap.tangent",
    "draft.snap.node",
    "draft.snap.nearest",
    "draft.snap.centroid",
    "draft.snap.extension",
    "draft.snap.parallel",
    "draft.snap.grid",
    "draft.snap.self",
    // The styled drawing's view choices (docs/adr/0090): layer line weights on or off
    // (Kalınlık), symbols at the plot scale or at a fixed size on the screen.
    "view.lineWeights",
    "view.symbols.plot",
    "view.symbols.screen",
    // Katman stili: the active layer's renderer (docs/adr/0091).
    "style.layerStyle",
    // Stil yöneticisi, and symbols given to the selected objects or taken away (docs/adr/0092).
    "style.manager",
    "style.assign",
    "style.clearSymbol",
    // Lejant, saved as a PNG on white paper (docs/adr/0093).
    "style.legend",
    // SVG çizim düzenleyicisi, its import, export, document properties and tracing (docs/adr/0095).
    "style.svgEditor",
    // Netcad NCZ in, through the same window as DXF (exchange/drawing_import.rs, docs/adr/0138).
    "file.import.ncz",
    // Navigation of docs/adr/0141 (navigation.rs): the views left, and the objects far from the drawing.
    "view.previous",
    "view.next",
    "view.extentCheck",
    // Dik ayak ölç (docs/adr/0141): a point read against a line, to the log.
    "tool.stationOffset",
    // The selecting tools of docs/adr/0141, Giriş › Seçim's Seç ▾: what a fence crosses, a
    // circle holds or touches, and the areas around a point.
    "tool.selectFence",
    "tool.selectCircle",
    "tool.selectContaining",
    // Kot ver (docs/adr/0142): the vertices of the selection given elevations, through
    // cad.entities.edit as Kot ver; Sabit, Artır and Sıfırla are its methods.
    "tool.setElevation",
    // Çok parçalı alan (docs/adr/0143): the parts joined into one area, an area split into its parts.
    "tool.partsJoin",
    "tool.partsSplit",
    // Delikler (docs/adr/0173 §5): a ring cut from an area, a hole removed, a hole filled with a new area.
    "tool.holeAdd",
    "tool.holeRemove",
    "tool.holeFill",
    // Sürdür (docs/adr/0173 §4): a line or a polyline continued from an end.
    "tool.continue",
    // Biçim değiştir (docs/adr/0173 §2–§3): an area or a path reshaped by a line drawn over it.
    "tool.reshape",
    // Modele dön (MODEL): the sheet mode's own, from a sheet back to the drawing (docs/adr/0164).
    "sheet.model",
];

/// Where the desktop does otherwise than the web, its own description: the
/// web's would say otherwise (docs/adr/0053: the desktop reads a zipped
/// Shapefile; docs/adr/0058: no search box in the ribbon yet, Esc cancels first).
const DESKTOP_DESCRIPTIONS: &[(&str, &str)] = &[
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

/// Puts the desktop's own commands in the ribbon: Python konsolu after
/// Komut satırına git, in the Komut panel (a CAD project's Yönet, a CBS
/// project's Analiz; docs/adr/0165 §6).
fn place_desktop_commands(tabs: &mut [Tab]) {
    let komut = tabs
        .iter_mut()
        .filter(|tab| !tab.contextual)
        .flat_map(|tab| tab.panels.iter_mut())
        .filter(|panel| {
            panel.items.iter().any(|item| {
                matches!(
                    item,
                    Item::Command {
                        id: "commandline.focus",
                        ..
                    }
                )
            })
        });
    for panel in komut {
        panel.items.push(Item::Command {
            id: PYTHON_CONSOLE,
            size: Size::Large,
        });
    }
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
    pub id: kentos_contracts::Workspace,
    pub label: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub highlights: &'static [&'static str],
    /// Announced but not built: shown last, dimmed, “Yakında”.
    pub ready: bool,
}

/// The type a project not asked its type yet, or of an announced type,
/// shows as (the web's `FALLBACK_WORKSPACE`, docs/adr/0165 §1).
pub const FALLBACK_MODE: kentos_contracts::Workspace = kentos_contracts::Workspace::Gis;

/// The type the interface shows for a project's setting: its own when it
/// can be chosen, else CBS (the web's `effectiveWorkspace`).
pub fn effective_mode(workspace: Option<kentos_contracts::Workspace>) -> &'static Mode {
    use kentos_contracts::Workspace;
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
pub fn mode_of(workspace: Option<kentos_contracts::Workspace>) -> &'static str {
    effective_mode(workspace).label
}

/// A mode's command id (`workspace.cad`).
pub fn mode_command(mode: kentos_contracts::Workspace) -> &'static str {
    use kentos_contracts::Workspace;
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
    pub panels: Vec<Panel>,
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
        ids: Vec<&'static str>,
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
    mode_tabs: Vec<(kentos_contracts::Workspace, Vec<Tab>)>,
    quick: Vec<&'static str>,
    modes: Vec<Mode>,
    /// The menu bar's menus by id: their blocks of command ids, in order.
    menus: Vec<(&'static str, Vec<Vec<&'static str>>)>,
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
            .find(|e| e.option.is_some() && e.aliases.iter().any(|a| crate::app::fold(a) == folded))
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

    /// The ribbon tabs of a project type (CBS's when the type has none of
    /// its own), the contextual ones left out.
    pub fn tabs_in(&self, mode: kentos_contracts::Workspace) -> impl Iterator<Item = &Tab> {
        self.mode_tabs
            .iter()
            .find(|(m, _)| *m == mode)
            .map_or(&self.tabs, |(_, tabs)| tabs)
            .iter()
            .filter(|tab| !tab.contextual)
    }

    /// The contextual ribbon tabs of a project type (the web's Seçim, shown
    /// while something is selected).
    pub fn contextual_in(&self, mode: kentos_contracts::Workspace) -> impl Iterator<Item = &Tab> {
        self.mode_tabs
            .iter()
            .find(|(m, _)| *m == mode)
            .map_or(&self.tabs, |(_, tabs)| tabs)
            .iter()
            .filter(|tab| tab.contextual)
    }

    /// Quick access bar (Kaydet, Geri al, Yinele).
    pub fn quick(&self) -> &[&'static str] {
        &self.quick
    }

    /// The work modes, in the web's order: the ones that can be chosen first.
    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

    /// Reads the embedded inventory. The strings live as long as the program
    /// (the catalog is built once), which the command line's borrowed
    /// suggestions need.
    fn load(text: &str) -> Result<Self, String> {
        let mut raw: RawInventory =
            serde_json::from_str(text).map_err(|e| format!("web envanteri okunamadı: {e}"))?;
        // Before the commands and panels take their icons.
        icons::keep_web_set(
            std::mem::take(&mut raw.icons)
                .into_iter()
                .map(|(name, markup)| (leak(name), leak(markup)))
                .collect(),
        );

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
                    icon: icons::from_web(c.icon.as_deref()),
                    standing,
                    pending_note,
                    category: leak(c.category.unwrap_or_default()),
                }
            })
            .chain(desktop_commands())
            .collect();
        let by_id = commands
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        let mut tabs: Vec<Tab> = raw.layout.ribbon.into_iter().map(tab).collect();
        place_desktop_commands(&mut tabs);
        let mode_tabs = raw
            .layout
            .ribbon_by_mode
            .into_iter()
            .filter_map(|(mode, tabs)| {
                let mode = serde_json::from_value(serde_json::Value::String(mode)).ok()?;
                let mut tabs: Vec<Tab> = tabs.into_iter().map(tab).collect();
                place_desktop_commands(&mut tabs);
                Some((mode, tabs))
            })
            .collect();
        fn tab(tab: RawTab) -> Tab {
            Tab {
                id: leak(tab.id),
                label: leak(tab.label),
                contextual: tab.contextual.is_some(),
                panels: tab
                    .panels
                    .into_iter()
                    .map(|panel| Panel {
                        label: leak(panel.label),
                        icon: icons::from_web(panel.icon.as_deref()),
                        items: panel.items.into_iter().map(item).collect(),
                        overflow: panel.overflow.into_iter().map(leak).collect(),
                        keep: panel.keep,
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

        Ok(Self {
            commands,
            by_id,
            tabs,
            mode_tabs,
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
    let size = |size: &str| match size {
        "large" => Size::Large,
        "lead" => Size::Lead,
        _ => Size::Small,
    };
    match raw {
        RawItem::Command { command, size: s } => Item::Command {
            id: leak(command),
            size: size(&s),
        },
        RawItem::Split { split, key, size: s } => Item::Split {
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
        } => Item::Menu {
            label: leak(menu),
            ids: flatten(blocks),
            size: size(&s),
        },
        RawItem::Builtin { builtin } => Item::Builtin(leak(builtin)),
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

/// The command line list's note of a command (`Command::line_note`).
fn line_note(
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
    id: String,
    label: String,
    contextual: Option<String>,
    panels: Vec<RawPanel>,
}

#[derive(Deserialize)]
struct RawPanel {
    label: String,
    #[serde(default)]
    icon: Option<String>,
    items: Vec<RawItem>,
    #[serde(default)]
    overflow: Vec<String>,
    #[serde(default)]
    keep: bool,
    #[serde(default)]
    launcher: Option<RawLauncher>,
}

#[derive(Deserialize)]
struct RawLauncher {
    title: String,
    tab: Option<String>,
    command: Option<String>,
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
        /// The button's key: the tools' family, or the tool with methods.
        #[serde(default)]
        key: String,
        size: String,
    },
    Menu {
        menu: String,
        size: String,
        blocks: Vec<RawBlock>,
    },
    Builtin {
        builtin: String,
    },
}

#[derive(Deserialize)]
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
        let catalog = Catalog::load(INVENTORY).expect("the web inventory parses");
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
            ]
        );
        let arc = "tool.arc";
        assert_eq!(
            of(arc),
            [
                (arc, "3 nokta", None),
                (arc, "Merkez, başlangıç, bitiş", Some("M")),
                (arc, "Devam", Some("D")),
            ]
        );
        assert_eq!(
            of("tool.rectangle"),
            [
                ("tool.rectangle", "Dikdörtgen", None),
                ("tool.rectangle3", "Döndürülmüş dikdörtgen", None),
                ("tool.regularPolygon", "Düzgün çokgen", None),
            ]
        );
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

    /// The chords that pass a text field are the web's `allowInInput`
    /// bindings, read from the inventory: Ctrl+S and the F-keys, not Ctrl+Z
    /// (the text field's own), each also a shortcut of its command.
    #[test]
    fn chords_that_work_in_text_fields_come_from_the_web() {
        let catalog = Catalog::load(INVENTORY).expect("the web inventory parses");
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
}
