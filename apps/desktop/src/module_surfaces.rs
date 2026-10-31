//! What each KentOS program shows of its window (docs/adr/0802 §5): the bottom panel's tabs, the dock's
//! panels, the status bar's cells, Proje ayarları' and Uygulama ayarları' sections and rows, the layer menu's
//! entries, the application menu's file formats, Kaynaklar's sections, the files it takes when they are
//! dropped on the window, the properties' rows, the hover card's parcel rows, the project type the sheet mode
//! and the cloud start with, the tools' options, Katman stili's renderers and the style library's branches.
//! kentos-cad, every module at once, shows every one; a module's program its own:
//!
//! - KentOS Mimari, a building on its own coordinates: Komut geçmişi, Koordinat listesi, Arama (Veride ara
//!   finds the texts of a building too), Denetim, Uyarılar and Python; Katmanlar, Öznitelikler, Bloklar and
//!   Şablonlar; the Kat cell; Mimari in Proje ayarları;
//!   drawings (KCAD, DWG, DXF, NCZ, PDF) in and out; no survey's tool options (Ölçülendirme's Semt, Nokta's Ad,
//!   Kod and its question, Çizgi's recorded measurements) and no zoning plan's legends (the library's MPYY).
//! - KentOS Harita, a map on real coordinates: every tab but Grafik and Denetim; every panel; the map system,
//!   the second system, Topoloji and Çakışma; Kotlar ve epok and Ölçme; GIS files, sources and services; its
//!   new projects in metres.
//! - KentOS GIS: every tab but Denetim; every panel but Bloklar; Senaryo too; time, scenarios and typed layers;
//!   Katman stili's thematic renderers; no blocks and no polyline widths in the tools (Böl's Blok, Çoklu
//!   çizgi's Genişlik, the grip menu's Kenar genişliği…), nor a polyline's Genişlik and an area's Maske in
//!   Öznitelikler; its new projects in metres.
//!
//! The commands a program offers are catalog.rs's (`App::offers`). Each program keeps its own layout
//! (layout.rs: kentos-cad's yerlesim.json, a module's yerlesim-<modül>.json, the first run starting from
//! kentos-cad's): a tab or a panel the kept layout names that the program does not show is not opened here,
//! and one an opener asks for falls back to Komut geçmişi, saying where it is.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_commands::{ProjectType, Workspace};
use kentos_interaction::point_calc::{CalcDef, CalcKind};
use kentos_interaction::{Offers, ToolOption};
use kentos_launch::Module;

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
use crate::project::settings::Section as ProjectSection;
use crate::settings_sections::Section as SettingsSection;

/// A status bar cell only some programs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
    /// Harita sistemi (map_crs.rs, docs/adr/0583 §3); the coordinate system's Harita sistemi ▸ too.
    MapCrs,
    /// İkinci sistem's values (second_crs.rs, docs/adr/0167 §2); the coordinate system's İkinci sistem ▸ and
    /// Proje ayarları' İkinci sistem group too.
    SecondCrs,
    /// Topolojik düzenleme's lamp (`draft.topology`, docs/adr/0160).
    Topology,
    /// Çakışma denetimi's lamp (`draft.overlap`, docs/adr/0162).
    Overlap,
    /// Senaryo (temporal/, docs/adr/0210 §10).
    Scenario,
    /// Kat (active_level.rs, docs/adr/0713), and Etkin aracın ayarları' Kat row.
    Level,
}

/// An entry of Uygulama ayarları only some programs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppSetting {
    /// Görünüm › Koordinatlar: Coğrafi koordinatlar (`display.geographic`).
    Geographic,
    /// Görünüm › Şerit ve fare: Çizilen nesnenin formu (`drafting.formOnCreate`).
    FormOnCreate,
    /// The Yeni projeler section.
    NewProjects,
    /// Yeni projeler › Yeni projelerin türü (`newProjects.workspace`): a module's program fixes the type.
    NewProjectType,
    /// Yeni projeler › Çizim birimi (`newProjects.drawingUnit`): a map's works are in metres (docs/adr/0802).
    NewProjectUnit,
}

/// An entry of a layer's right-click menu only some programs show. İstatistik and Grafik follow the bottom
/// panel's tabs; the rest is every program's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerEntry {
    /// Veri ▸ Zaman ayarları… (docs/adr/0210 §10).
    Time,
    /// Senaryo ▸ on a scenario group (docs/adr/0210 §10).
    Scenario,
    /// Koordinat sistemi ▸: Sistem ata, Yeniden izdüşür (docs/adr/0583).
    Crs,
    /// Görünüş ▸ Stil yükle…, Stili kaydet… (docs/adr/0635 §9).
    StyleFiles,
    /// Görünüş ▸ SLD (docs/adr/0516).
    Sld,
    /// Görünüş ▸ QML (docs/adr/0555).
    Qml,
    /// Görünüş ▸ MapLibre (docs/adr/0560).
    MapLibre,
    /// Veri ▸ Veri kümesi bilgisi… (docs/adr/0633).
    Dataset,
    /// Katman türü ▸ (docs/adr/0622 §5).
    Kind,
    /// A map service layer's own items and a fed layer's Yenile (docs/adr/0208).
    Service,
    /// Kaynak ▸ on a bound layer (docs/adr/0503).
    Source,
}

/// The application menu's İçe aktar and Dışa aktar formats a program lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Formats {
    /// Drawings alone: DWG, DXF, NCZ and PDF.
    Drawings,
    /// Every one: GIS files and coordinate lists too.
    All,
}

/// A section of Kaynaklar only some programs show (Klasörler, the recent and the favourite sources and KentOS
/// are every one's that shows Kaynaklar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceSection {
    /// Veritabanları and its header's Bağlantı ekle (docs/adr/0623).
    Databases,
    /// Servisler and its header's Servis ekle (docs/adr/0599 §4).
    Services,
}

/// Rows of Öznitelikler only some programs show (Etkin aracın ayarları' Kat row is `Cell::Level`'s; a polyline's
/// Genişlik is `ToolOption::Widths`'s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropertyRows {
    /// Çizim's Koordinat sistemi and SRID with nothing selected.
    Crs,
    /// Servis katmanı and Veri kaynağı of the layer chosen (docs/adr/0208 §14).
    Services,
    /// An area's Maske (docs/adr/0310 §5): a drawing's, not a GIS's.
    Mask,
    /// A polyline's Mahal ayırıcı (docs/adr/0679): a building's, KentOS Mimari's (one that is a separator shows
    /// it everywhere, to be turned off).
    Separators,
}

/// What one module's program shows.
#[derive(Debug)]
pub(crate) struct Surfaces {
    /// The bottom panel's tabs.
    pub bottom: &'static [BottomTab],
    /// The dock's panels.
    pub panels: &'static [Panel],
    pub cells: &'static [Cell],
    /// Proje ayarları' sections.
    pub project: &'static [ProjectSection],
    pub settings: &'static [AppSetting],
    pub layer_menu: &'static [LayerEntry],
    pub formats: Formats,
    pub sources: &'static [SourceSection],
    pub rows: &'static [PropertyRows],
    /// The hover card's parcel rows: Ada, Mahalle, Nitelik, the deed's area and Parsel's title.
    pub parcels: bool,
    /// The project type the sheet mode ranks its templates by (docs/sheet/design.md §4).
    pub sheet: ProjectType,
    /// The type Bulut projeleri lists at first; none: every type (a map's projects carry five).
    pub cloud: Option<ProjectType>,
    /// The type Buluta yükle offers first.
    pub upload: ProjectType,
    /// Nokta hesapla's constructions it has not: a survey's (a route's km, a named point) in a building's.
    pub hidden_calc: &'static [CalcKind],
    /// The tools' options it leaves out (Draft::offers): a survey's in a building's, blocks' and a polyline's
    /// widths in a GIS.
    pub left_out: &'static [ToolOption],
    /// Katman stili's thematic renderers (docs/adr/0213): a GIS's analysis; a layer drawn with one keeps it
    /// everywhere.
    pub thematic: bool,
    /// The system style library's branches Stil yöneticisi and the symbol pickers leave out: a zoning plan's
    /// legends (MPYY) in a building's.
    pub library_hidden: &'static [&'static str],
}

/// KentOS Mimari: a building on its own coordinates.
const ARCHITECTURE: Surfaces = Surfaces {
    bottom: &[
        BottomTab::History,
        BottomTab::Coords,
        BottomTab::Search,
        BottomTab::Audit,
        BottomTab::Messages,
        BottomTab::Python,
    ],
    panels: &[
        Panel::Layers,
        Panel::Blocks,
        Panel::Templates,
        Panel::Navigator,
        Panel::Properties,
    ],
    cells: &[Cell::Level],
    project: &[
        ProjectSection::General,
        ProjectSection::Units,
        ProjectSection::Crs,
        ProjectSection::Variables,
        ProjectSection::Arch,
    ],
    settings: &[],
    layer_menu: &[],
    formats: Formats::Drawings,
    sources: &[],
    rows: &[PropertyRows::Mask, PropertyRows::Separators],
    parcels: false,
    sheet: ProjectType::Architecture,
    cloud: Some(ProjectType::Architecture),
    upload: ProjectType::Architecture,
    hidden_calc: &[CalcKind::Km, CalcKind::Name],
    left_out: &[
        ToolOption::Bearing,
        ToolOption::PointNames,
        ToolOption::Recorded,
    ],
    thematic: false,
    library_hidden: &["MPYY"],
};

/// KentOS Harita: a map on real coordinates; GIS's sources and services, not its time, scenarios and charts.
const MAP: Surfaces = Surfaces {
    bottom: &[
        BottomTab::History,
        BottomTab::Coords,
        BottomTab::Points,
        BottomTab::Table,
        BottomTab::Statistics,
        BottomTab::Search,
        BottomTab::Address,
        BottomTab::Topology,
        BottomTab::ServiceInfo,
        BottomTab::Messages,
        BottomTab::Python,
    ],
    panels: &[
        Panel::Layers,
        Panel::Processing,
        Panel::Blocks,
        Panel::Templates,
        Panel::Sources,
        Panel::Properties,
    ],
    cells: &[Cell::MapCrs, Cell::SecondCrs, Cell::Topology, Cell::Overlap],
    project: &[
        ProjectSection::General,
        ProjectSection::Units,
        ProjectSection::Crs,
        ProjectSection::Heights,
        ProjectSection::Survey,
        ProjectSection::Variables,
    ],
    settings: &[
        AppSetting::Geographic,
        AppSetting::FormOnCreate,
        AppSetting::NewProjects,
    ],
    layer_menu: &[
        LayerEntry::Crs,
        LayerEntry::StyleFiles,
        LayerEntry::Sld,
        LayerEntry::Qml,
        LayerEntry::MapLibre,
        LayerEntry::Dataset,
        LayerEntry::Service,
        LayerEntry::Source,
    ],
    formats: Formats::All,
    sources: &[SourceSection::Databases, SourceSection::Services],
    rows: &[
        PropertyRows::Crs,
        PropertyRows::Services,
        PropertyRows::Mask,
    ],
    parcels: true,
    sheet: ProjectType::ZoningPlan,
    cloud: None,
    upload: ProjectType::Cad,
    hidden_calc: &[],
    left_out: &[],
    thematic: false,
    library_hidden: &[],
};

/// KentOS GIS: no blocks and no architectural audit.
const GIS: Surfaces = Surfaces {
    bottom: &[
        BottomTab::History,
        BottomTab::Coords,
        BottomTab::Points,
        BottomTab::Table,
        BottomTab::Statistics,
        BottomTab::Chart,
        BottomTab::Search,
        BottomTab::Address,
        BottomTab::Topology,
        BottomTab::ServiceInfo,
        BottomTab::Messages,
        BottomTab::Python,
    ],
    panels: &[
        Panel::Layers,
        Panel::Processing,
        Panel::Templates,
        Panel::Sources,
        Panel::Properties,
    ],
    cells: &[
        Cell::MapCrs,
        Cell::SecondCrs,
        Cell::Topology,
        Cell::Overlap,
        Cell::Scenario,
    ],
    project: &[
        ProjectSection::General,
        ProjectSection::Units,
        ProjectSection::Crs,
        ProjectSection::Heights,
        ProjectSection::Survey,
        ProjectSection::Variables,
    ],
    settings: &[
        AppSetting::Geographic,
        AppSetting::FormOnCreate,
        AppSetting::NewProjects,
    ],
    layer_menu: &[
        LayerEntry::Time,
        LayerEntry::Scenario,
        LayerEntry::Crs,
        LayerEntry::StyleFiles,
        LayerEntry::Sld,
        LayerEntry::Qml,
        LayerEntry::MapLibre,
        LayerEntry::Dataset,
        LayerEntry::Kind,
        LayerEntry::Service,
        LayerEntry::Source,
    ],
    formats: Formats::All,
    sources: &[SourceSection::Databases, SourceSection::Services],
    rows: &[PropertyRows::Crs, PropertyRows::Services],
    parcels: true,
    sheet: ProjectType::Gis,
    cloud: Some(ProjectType::Gis),
    upload: ProjectType::Gis,
    hidden_calc: &[],
    left_out: &[ToolOption::Blocks, ToolOption::Widths],
    thematic: true,
    library_hidden: &[],
};

/// What `module`'s program shows.
pub(crate) fn surfaces(module: Module) -> &'static Surfaces {
    match module {
        Module::Architecture => &ARCHITECTURE,
        Module::Map => &MAP,
        Module::Gis => &GIS,
    }
}

/// Whether `program` (none: kentos-cad) shows `panel`: the dock is laid out by it before the app is.
pub(crate) fn shows_panel_in(program: Option<Module>, panel: Panel) -> bool {
    program.is_none_or(|m| surfaces(m).panels.contains(&panel))
}

/// The tools' options `program` (none: kentos-cad, every one) offers: what the drafting aids give the tools
/// (app.rs fills them with the settings).
pub(crate) fn tool_offers_in(program: Option<Module>) -> Offers {
    program.map_or(Offers::ALL, |m| Offers::without(surfaces(m).left_out))
}

/// Whether a style library item from `source` at `path` shows with the system branches `hidden` left out
/// (Stil yöneticisi, the symbol pickers; `App::library_hidden`).
pub(crate) fn library_shows(
    hidden: &[&str],
    source: kentos_native_style::library::Source,
    path: &[&str],
) -> bool {
    source != kentos_native_style::library::Source::System
        || path.first().is_none_or(|branch| !hidden.contains(branch))
}

/// The application menu's drawing formats: every program lists them.
const DRAWING_FORMATS: [&str; 7] = [
    "file.import.dwg",
    "file.import.dxf",
    "file.import.ncz",
    "file.import.pdf",
    "file.export.dwg",
    "file.export.dxf",
    "file.export.pdf",
];

/// The drawings a program without Kaynaklar takes when they are dropped on its window: opened in a tab of
/// their own; a PDF is imported.
const DROPPED_DRAWINGS: [&str; 4] = ["kcad", "dwg", "dxf", "ncz"];

/// The status bar lamps only some programs show, by their commands.
fn aid_cell(id: &str) -> Option<Cell> {
    match id {
        "draft.topology" => Some(Cell::Topology),
        "draft.overlap" => Some(Cell::Overlap),
        _ => None,
    }
}

/// The file's extension, lower case (`plan.DWG` → `dwg`).
fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// “KentOS Harita'da ve KentOS GIS'te”: where the programs are, in a sentence.
fn joined(modules: &[Module]) -> String {
    modules
        .iter()
        .map(|m| crate::program::in_program(*m))
        .collect::<Vec<_>>()
        .join(" ve ")
}

impl App {
    /// The program's profile; none in kentos-cad, which shows everything.
    fn surfaces(&self) -> Option<&'static Surfaces> {
        self.program.map(surfaces)
    }

    /// Whether the bottom panel has `tab` here.
    pub(crate) fn shows_bottom_tab(&self, tab: BottomTab) -> bool {
        self.surfaces().is_none_or(|s| s.bottom.contains(&tab))
    }

    /// The bottom panel's tabs here, in their order.
    pub(crate) fn bottom_tabs(&self) -> Vec<BottomTab> {
        BottomTab::ALL
            .into_iter()
            .filter(|t| self.shows_bottom_tab(*t))
            .collect()
    }

    /// Whether the dock has `panel` here.
    pub(crate) fn shows_panel(&self, panel: Panel) -> bool {
        shows_panel_in(self.program, panel)
    }

    /// Whether the status bar shows `cell` here (and what goes with it, see [`Cell`]).
    pub(crate) fn shows_cell(&self, cell: Cell) -> bool {
        self.surfaces().is_none_or(|s| s.cells.contains(&cell))
    }

    /// Whether the status bar shows the drafting aid `id`'s lamp here.
    pub(crate) fn shows_aid(&self, id: &str) -> bool {
        aid_cell(id).is_none_or(|cell| self.shows_cell(cell))
    }

    /// Whether Proje ayarları has `section` here.
    pub(crate) fn shows_project_section(&self, section: ProjectSection) -> bool {
        self.surfaces().is_none_or(|s| s.project.contains(&section))
    }

    /// Proje ayarları' sections here, in their order.
    pub(crate) fn project_sections(&self) -> Vec<ProjectSection> {
        ProjectSection::ALL
            .into_iter()
            .filter(|s| self.shows_project_section(*s))
            .collect()
    }

    /// Whether Uygulama ayarları shows `setting` here.
    pub(crate) fn shows_setting(&self, setting: AppSetting) -> bool {
        self.surfaces()
            .is_none_or(|s| s.settings.contains(&setting))
    }

    /// Whether Uygulama ayarları has `section` here: Yeni projeler is the one only some programs have.
    pub(crate) fn shows_settings_section(&self, section: SettingsSection) -> bool {
        section != SettingsSection::NewProjects || self.shows_setting(AppSetting::NewProjects)
    }

    /// Whether a layer's right-click menu has `entry` here.
    pub(crate) fn shows_layer_entry(&self, entry: LayerEntry) -> bool {
        self.surfaces()
            .is_none_or(|s| s.layer_menu.contains(&entry))
    }

    /// Whether the application menu lists the format whose command is `id`.
    pub(crate) fn shows_format(&self, id: &str) -> bool {
        match self.surfaces().map(|s| s.formats) {
            Some(Formats::Drawings) => DRAWING_FORMATS.contains(&id),
            Some(Formats::All) | None => true,
        }
    }

    /// Whether Kaynaklar has `section` here.
    pub(crate) fn shows_source_section(&self, section: SourceSection) -> bool {
        self.surfaces().is_none_or(|s| s.sources.contains(&section))
    }

    /// Whether Öznitelikler shows `rows` here.
    pub(crate) fn shows_property_rows(&self, rows: PropertyRows) -> bool {
        self.surfaces().is_none_or(|s| s.rows.contains(&rows))
    }

    /// Whether the hover card shows an object's parcel rows.
    pub(crate) fn shows_parcel_rows(&self) -> bool {
        self.surfaces().is_none_or(|s| s.parcels)
    }

    /// Whether the tools here offer `option` (Draft::offers), and what goes with it outside them: Şablon
    /// düzenleyici's Blok ekle with Blocks, Öznitelikler' Genişlik of a polyline with Widths.
    pub(crate) fn offers_tool_option(&self, option: ToolOption) -> bool {
        tool_offers_in(self.program).has(option)
    }

    /// Whether Katman stili lists the thematic renderers (docs/adr/0213).
    pub(crate) fn shows_thematic_renderers(&self) -> bool {
        self.surfaces().is_none_or(|s| s.thematic)
    }

    /// The system style library's branches Stil yöneticisi and the symbol pickers leave out here.
    pub(crate) fn library_hidden(&self) -> &'static [&'static str] {
        self.surfaces().map_or(&[], |s| s.library_hidden)
    }

    /// Whether the program names the axes as a CAD does, X east and Y north (KentOS Mimari, KentOS Harita,
    /// kentos-cad with a CAD drawing), or as a CBS, Y east and X north (docs/adr/0165 §4).
    pub(crate) fn cad_axes(&self) -> bool {
        self.program_type().unwrap_or_else(|| self.work_mode()) == Workspace::Cad
    }

    /// The project type the sheet mode ranks its templates by; none in kentos-cad (the drawing's type).
    pub(crate) fn sheet_project_type(&self) -> Option<ProjectType> {
        self.surfaces().map(|s| s.sheet)
    }

    /// The type Bulut projeleri lists at first; none: every type.
    pub(crate) fn cloud_kind(&self) -> Option<ProjectType> {
        self.surfaces().and_then(|s| s.cloud)
    }

    /// The type Buluta yükle offers first.
    pub(crate) fn upload_kind(&self) -> ProjectType {
        self.surfaces().map_or(ProjectType::Cad, |s| s.upload)
    }

    /// Nokta hesapla's constructions this program has not (its menus leave them out).
    pub(crate) fn hidden_point_calcs(&self) -> &'static [CalcKind] {
        self.surfaces().map_or(&[], |s| s.hidden_calc)
    }

    /// Whether Nokta hesapla has `kind` here, said when it has not: the menus and its typed alias ask first.
    pub(crate) fn offers_point_calc(&mut self, kind: CalcKind) -> bool {
        if !self.hidden_point_calcs().contains(&kind) {
            return true;
        }
        let others = self.others_showing(|s| !s.hidden_calc.contains(&kind));
        let here = self.program.map_or("", crate::program::in_program);
        let title = CalcDef::of(kind).label;
        self.warn(format!("“{title}” {here} yok; {} var.", joined(&others)));
        false
    }

    /// The programs other than this one that show what `shows` says, in the launcher's order.
    fn others_showing(&self, shows: impl Fn(&Surfaces) -> bool) -> Vec<Module> {
        Module::ALL
            .into_iter()
            .filter(|m| Some(*m) != self.program && shows(surfaces(*m)))
            .collect()
    }

    /// Opens the bottom panel on `tab` when this program has it, else on Komut geçmişi with a line on the
    /// programs that have it: whether `tab` opened. An opener asks first, before it does its tab's work.
    pub(crate) fn open_bottom_tab(&mut self, tab: BottomTab) -> bool {
        if self.shows_bottom_tab(tab) {
            return true;
        }
        let others = self.others_showing(|s| s.bottom.contains(&tab));
        let here = self.program.map_or("", crate::program::in_program);
        let title = tab.title();
        self.output(if others.is_empty() {
            format!("“{title}” {here} yok.")
        } else {
            format!("“{title}” {here} yok; {} var.", joined(&others))
        });
        self.show_bottom(BottomTab::History);
        false
    }

    /// Whether this program has `panel`, said when it has not: an opener asks first.
    pub(crate) fn open_panel(&mut self, panel: Panel) -> bool {
        if self.shows_panel(panel) {
            return true;
        }
        let others = self.others_showing(|s| s.panels.contains(&panel));
        let here = self.program.map_or("", crate::program::in_program);
        let title = panel.title();
        self.output(if others.is_empty() {
            format!("“{title}” {here} yok.")
        } else {
            format!("“{title}” {here} yok; {} var.", joined(&others))
        });
        false
    }

    /// After every message (layout.rs): what this program does not show is not on screen, whichever way it
    /// came (a remembered tab of another program, an opener, a test setting the program late).
    pub(crate) fn keep_own_surfaces(&mut self) {
        // The tools' options too: a program set after the settings were applied (a test) is followed.
        self.draft.offers = tool_offers_in(self.program);
        if self.program.is_none() {
            return;
        }
        if !self.shows_bottom_tab(self.bottom_tab) {
            self.bottom_tab = BottomTab::History;
        }
        for panel in Panel::ALL {
            if self.shows_panel(panel) {
                continue;
            }
            self.docks.close(panel);
            if let Some(docks) = &mut self.hidden_docks {
                docks.close(panel);
            }
        }
    }

    /// A file dropped on a program without Kaynaklar (KentOS Mimari): a drawing (KCAD, DWG, DXF, NCZ) opens in a
    /// tab of its own, the drawing on screen kept; a PDF opens PDF içe aktar with it; any other file is
    /// refused, the programs that take it named.
    pub(crate) fn drop_drawing(&mut self, path: PathBuf) -> Task<Message> {
        let name = crate::sources::folder_name(&path);
        let ext = extension(&path);
        if DROPPED_DRAWINGS.contains(&ext.as_str()) {
            return self.update(Message::Opening(crate::opening::Event::PickedTab(Some(
                path,
            ))));
        }
        if ext == "pdf" {
            if self.document.is_none() {
                self.output(
                    "Açık çizim yok. PDF'i içe aktarmak için önce bir çizim açın (Ctrl+O).",
                );
                return Task::none();
            }
            return self.update(Message::Exchange(Box::new(
                crate::exchange::Event::PdfImport(crate::exchange::pdf_import::Event::Picked(
                    Some(path),
                )),
            )));
        }
        let here = self.program.map_or("KentOS", Module::title);
        let takes = format!("{here} çizimleri (KCAD, DWG, DXF, NCZ) açar, PDF'i içe aktarır.");
        let others = self.others_showing(|s| s.panels.contains(&Panel::Sources));
        let said = match kentos_interaction::sources::source_kind(&name) {
            Some(kind) if !others.is_empty() => format!(
                "“{name}” {} açılmaz: {takes} {} {} açılır.",
                self.program.map_or("", crate::program::in_program),
                kind.label(),
                joined(&others)
            ),
            _ => format!(
                "“{name}” {} açılmaz: {takes}",
                self.program.map_or("", crate::program::in_program)
            ),
        };
        self.warn(said);
        Task::none()
    }
}
