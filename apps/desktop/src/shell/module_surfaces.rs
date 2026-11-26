//! What the window shows for each module (docs/adr/0802 §5, 9195 §2): the bottom panel's tabs, the dock's
//! panels, the status bar's cells, Proje ayarları' and Uygulama ayarları' sections and rows, the layer menu's
//! entries, the application menu's file formats, Kaynaklar's sections, the files it takes when they are
//! dropped on the window, the properties' rows, the hover card's parcel rows, the project type the sheet mode
//! and the cloud start with, the tools' options, Katman stili's renderers and the style library's branches.
//! The active module decides (CLAUDE.md §25): the open project's, else the program's (`App::active_module`), in a
//! module's program and in kentos-cad alike; kentos-cad with nothing open shows the common surfaces alone
//! (`NoProject`), and switches as its tabs' projects do. What each module shows is its profile's, in its own folder
//! (module_profile.rs: arch/profile.rs, map/profile.rs, road/profile.rs, gis/profile.rs); this file asks the profile
//! and never a module by name.
//!
//! Eklenti denetimi shows only while an extension offers the module an audit rule, or a run is on it. The layer menu's
//! entries are placed by the profile by module and in layer_entries.rs by layer kind (docs/adr/9195 §5).
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
use crate::module_profile::{ModuleProfile, NoProject, profile};
// The layer menu's entries, placed by module here and by layer kind there (docs/adr/9195 §5).
pub(crate) use crate::layer_entries::LayerEntry;
use crate::project::settings::Section as ProjectSection;
use crate::settings_sections::Section as SettingsSection;

/// A status bar cell only some programs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
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
    /// Çizgi kalınlığı's lamp (`view.lineWeights`): a drawing's habit; a CBS project's stays in Görünüm
    /// (docs/adr/9185).
    LineWeights,
}

/// An entry of Uygulama ayarları only some programs show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSetting {
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
    /// The Kısayollar section (settings_keys.rs, docs/adr/9308): the user's keys and names, the person's in every
    /// program.
    Shortcuts,
    /// Görünüm › Nokta etiketleri and Nokta kodları (`graphics.pointLabels`, `graphics.pointCodes`): labels are a
    /// CBS's (docs/adr/9316).
    PointLabels,
}

/// The application menu's İçe aktar and Dışa aktar formats a program lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formats {
    /// Drawings alone: DWG, DXF, NCZ and PDF.
    Drawings,
    /// Every one: GIS files and coordinate lists too.
    All,
}

/// A section of Kaynaklar only some programs show (Klasörler, the recent and the favourite sources and KentOS
/// are every one's that shows Kaynaklar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSection {
    /// Veritabanları and its header's Bağlantı ekle (docs/adr/0623).
    Databases,
    /// Servisler and its header's Servis ekle (docs/adr/0599 §4).
    Services,
}

/// Rows of Öznitelikler only some programs show (Etkin aracın ayarları' Kat row is `Cell::Level`'s; a polyline's
/// Genişlik is `ToolOption::Widths`'s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyRows {
    /// Çizim's Koordinat sistemi and SRID with nothing selected.
    Crs,
    /// Servis katmanı and Veri kaynağı of the layer chosen (docs/adr/0208 §14).
    Services,
    /// An area's Maske (docs/adr/0310 §5): a drawing's, not a GIS's.
    Mask,
    /// A polyline's Mahal ayırıcı (docs/adr/0679): a building's, KentOS Mimari's (one that is a separator shows
    /// it everywhere, to be turned off).
    Separators,
    /// Kaynağı: where the selected object comes from in the map's work (docs/adr/1200 §4): a map's.
    MapWork,
    /// GNSS ölçümü: an accepted point's quality, solutions and state, read only (docs/adr/1340 §1): a map's and a
    /// GIS's.
    Survey,
    /// Topoloji: the selected object's findings of the Topoloji tab's last check, read only (docs/adr/3800): a map's
    /// and a GIS's, the programs with that tab.
    Topology,
}

/// Whether `program` (none: kentos-cad, every module's) may show `panel`: the dock is laid out by it before the app
/// is; what the active module does not show is closed after (`App::keep_own_surfaces`).
pub(crate) fn shows_panel_in(program: Option<Module>, panel: Panel) -> bool {
    program.is_none_or(|m| profile(m).panel(panel))
}

/// The tools' options `module` (none: kentos-cad with nothing open, every one) offers: what the drafting aids give
/// the tools (app.rs fills them with the settings).
pub(crate) fn tool_offers_in(module: Option<Module>) -> Offers {
    module.map_or(Offers::ALL, |m| Offers::without(profile(m).left_out()))
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

/// The panel in front of Katmanlar' slot.
fn front_of_layers(docks: &kentos_ui::widget::docking::Docks<Panel>) -> Option<Panel> {
    use kentos_ui::widget::docking::Slot;
    let Some(Slot::Docked(side, i)) = docks.slot(Panel::Layers) else {
        return None;
    };
    docks.stacks(side).get(i).and_then(|s| s.active())
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
        "view.lineWeights" => Some(Cell::LineWeights),
        _ => None,
    }
}

/// The file's extension, lower case (`plan.DWG` → `dwg`).
fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// “KentOS Harita'da, KentOS GIS'te ve KentOS Yol'da”: where the programs are, in a sentence.
fn joined(modules: &[Module]) -> String {
    let places: Vec<&str> = modules
        .iter()
        .map(|m| crate::program::in_program(*m))
        .collect();
    crate::program::and_list(&places)
}

impl App {
    /// The active module's profile (docs/adr/9195 §2): the open project's module's, else the program's; the common
    /// surfaces in kentos-cad with nothing open; none for an announced type (plan3d …), which shows every one.
    fn surfaces(&self) -> Option<&'static dyn ModuleProfile> {
        match self.active_module() {
            Some(m) => Some(profile(m)),
            None if self.document.is_none() => Some(&NoProject),
            None => None,
        }
    }

    /// Whether the bottom panel has `tab` here: Eklenti denetimi only while an extension offers this module an audit
    /// rule or a rule's run is on it, an agent's too (docs/adr/9195 §5).
    pub fn shows_bottom_tab(&self, tab: BottomTab) -> bool {
        if tab == BottomTab::ExtAudit && self.extensions.audit.is_none() && !self.offers_ext_audits() {
            return false;
        }
        self.surfaces().is_none_or(|s| s.bottom(tab))
    }

    /// The bottom panel's tabs here, in their order.
    pub fn bottom_tabs(&self) -> Vec<BottomTab> {
        BottomTab::ALL
            .into_iter()
            .filter(|t| self.shows_bottom_tab(*t))
            .collect()
    }

    /// Whether the dock has `panel` here.
    pub fn shows_panel(&self, panel: Panel) -> bool {
        self.surfaces().is_none_or(|s| s.panel(panel))
    }

    /// Whether the status bar shows `cell` here (and what goes with it, see [`Cell`]).
    pub fn shows_cell(&self, cell: Cell) -> bool {
        self.surfaces().is_none_or(|s| s.cell(cell))
    }

    /// Whether the status bar shows the drafting aid `id`'s lamp here: the profile's cells say (a CBS project's bar
    /// has no Kalınlık lamp, docs/adr/9185).
    pub fn shows_aid(&self, id: &str) -> bool {
        aid_cell(id).is_none_or(|cell| self.shows_cell(cell))
    }

    /// Whether Proje ayarları has `section` here.
    pub(crate) fn shows_project_section(&self, section: ProjectSection) -> bool {
        self.surfaces().is_none_or(|s| s.project_section(section))
    }

    /// Proje ayarları' sections here, in their order.
    pub fn project_sections(&self) -> Vec<ProjectSection> {
        ProjectSection::ALL
            .into_iter()
            .filter(|s| self.shows_project_section(*s))
            .collect()
    }

    /// Whether Uygulama ayarları shows `setting` here.
    pub fn shows_setting(&self, setting: AppSetting) -> bool {
        self.surfaces().is_none_or(|s| s.setting(setting))
    }

    /// Whether Uygulama ayarları has `section` here: Yeni projeler and Kısayollar are placed by the profile.
    pub fn shows_settings_section(&self, section: SettingsSection) -> bool {
        match section {
            SettingsSection::NewProjects => self.shows_setting(AppSetting::NewProjects),
            SettingsSection::Shortcuts => self.shows_setting(AppSetting::Shortcuts),
            _ => true,
        }
    }

    /// Whether the active module's layer menu has `entry`, whatever the layer (its kind decides too, layer_entries.rs).
    pub fn shows_layer_entry(&self, entry: LayerEntry) -> bool {
        self.surfaces().is_none_or(|s| s.layer_entry(entry))
    }

    /// Whether the application menu lists the format whose command is `id`.
    pub fn shows_format(&self, id: &str) -> bool {
        match self.surfaces().map(|s| s.formats()) {
            Some(Formats::Drawings) => DRAWING_FORMATS.contains(&id),
            Some(Formats::All) | None => true,
        }
    }

    /// Whether Kaynaklar has `section` here.
    pub fn shows_source_section(&self, section: SourceSection) -> bool {
        self.surfaces().is_none_or(|s| s.source_section(section))
    }

    /// Whether Öznitelikler shows `rows` here.
    pub fn shows_property_rows(&self, rows: PropertyRows) -> bool {
        self.surfaces().is_none_or(|s| s.rows(rows))
    }

    /// Whether the hover card shows an object's parcel rows.
    pub fn shows_parcel_rows(&self) -> bool {
        self.surfaces().is_none_or(|s| s.parcels())
    }

    /// Whether the tools here offer `option` (Draft::offers), and what goes with it outside them: Şablon
    /// düzenleyici's Blok ekle with Blocks, Öznitelikler' Genişlik of a polyline with Widths.
    pub fn offers_tool_option(&self, option: ToolOption) -> bool {
        tool_offers_in(self.active_module()).has(option)
    }

    /// Whether Katman stili lists the thematic renderers (docs/adr/0213).
    pub fn shows_thematic_renderers(&self) -> bool {
        self.surfaces().is_none_or(|s| s.thematic())
    }

    /// The system style library's branches Stil yöneticisi and the symbol pickers leave out here.
    pub fn library_hidden(&self) -> &'static [&'static str] {
        self.surfaces().map_or(&[], |s| s.library_hidden())
    }

    /// Whether the program names the axes as a CAD does, X east and Y north (KentOS Mimari, KentOS Harita,
    /// kentos-cad with a CAD drawing), or as a CBS, Y east and X north (docs/adr/0165 §4).
    pub(crate) fn cad_axes(&self) -> bool {
        self.program_type().unwrap_or_else(|| self.work_mode()) == Workspace::Cad
    }

    /// The project type the sheet mode ranks its templates by; none in kentos-cad (the drawing's type).
    pub fn sheet_project_type(&self) -> Option<ProjectType> {
        self.surfaces().map(|s| s.sheet())
    }

    /// The type Bulut projeleri lists at first; none: every type.
    pub fn cloud_kind(&self) -> Option<ProjectType> {
        self.surfaces().and_then(|s| s.cloud())
    }

    /// The type Buluta yükle offers first.
    pub fn upload_kind(&self) -> ProjectType {
        self.surfaces().map_or(ProjectType::Cad, |s| s.upload())
    }

    /// Nokta hesapla's constructions this program has not (its menus leave them out).
    pub fn hidden_point_calcs(&self) -> &'static [CalcKind] {
        self.surfaces().map_or(&[], |s| s.hidden_calc())
    }

    /// Whether Nokta hesapla has `kind` here, said when it has not: the menus and its typed alias ask first.
    pub(crate) fn offers_point_calc(&mut self, kind: CalcKind) -> bool {
        if !self.hidden_point_calcs().contains(&kind) {
            return true;
        }
        let others = self.others_showing(|s| !s.hidden_calc().contains(&kind));
        let here = self.here_words();
        let title = CalcDef::of(kind).label;
        self.warn(format!("“{title}” {here} yok; {} var.", joined(&others)));
        false
    }

    /// The modules other than the active one that show what `shows` says, in the launcher's order.
    fn others_showing(&self, shows: impl Fn(&dyn ModuleProfile) -> bool) -> Vec<Module> {
        let here = self.active_module();
        Module::ALL
            .into_iter()
            .filter(|m| Some(*m) != here && shows(profile(*m)))
            .collect()
    }

    /// Opens the bottom panel on `tab` when this program has it, else on Komut geçmişi with a line on the
    /// programs that have it: whether `tab` opened. An opener asks first, before it does its tab's work.
    pub(crate) fn open_bottom_tab(&mut self, tab: BottomTab) -> bool {
        if self.shows_bottom_tab(tab) {
            return true;
        }
        let others = self.others_showing(|s| s.bottom(tab));
        let here = self.here_words();
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
    pub fn open_panel(&mut self, panel: Panel) -> bool {
        if self.shows_panel(panel) {
            return true;
        }
        let others = self.others_showing(|s| s.panel(panel));
        let here = self.here_words();
        let title = panel.title();
        self.output(if others.is_empty() {
            format!("“{title}” {here} yok.")
        } else {
            format!("“{title}” {here} yok; {} var.", joined(&others))
        });
        false
    }

    /// After every message (layout.rs): what the active module does not show is not on screen, whichever way it came
    /// (a remembered tab of another program, an opener, a test setting the program late, another module's project in
    /// kentos-cad's tab). A panel closed for that comes back where it was when a project whose module shows it is on
    /// screen again; one the user closed stays closed.
    pub(crate) fn keep_own_surfaces(&mut self) {
        // The tools' options too: a program set after the settings were applied (a test) is followed.
        self.draft.offers = tool_offers_in(self.active_module());
        if !self.shows_bottom_tab(self.bottom_tab) {
            self.bottom_tab = BottomTab::History;
        }
        let back: Vec<Panel> = self
            .surface_closed
            .iter()
            .copied()
            .filter(|p| self.shows_panel(*p))
            .collect();
        if !back.is_empty() {
            self.surface_closed.retain(|p| !back.contains(p));
            let docks = self.hidden_docks.as_mut().unwrap_or(&mut self.docks);
            // The tabs in front stay in front: the panels come back behind them.
            let fronts: Vec<Panel> = docks
                .stacks(kentos_ui::widget::docking::Side::Right)
                .iter()
                .filter_map(|s| s.active())
                .collect();
            for panel in back {
                docks.show(panel, kentos_ui::widget::docking::Side::Right);
            }
            // Each in its place among Katmanlar' slot, as a module's program docks them.
            if let Some(slot) = docks.slot(Panel::Layers) {
                let in_slot: Vec<Panel> =
                    Panel::TOP_SLOT.into_iter().filter(|p| docks.slot(*p) == Some(slot)).collect();
                for (i, panel) in in_slot.into_iter().enumerate() {
                    docks.update(kentos_ui::widget::docking::Event::Moved(
                        panel,
                        kentos_ui::widget::docking::Target::Tab(slot, i),
                    ));
                }
            }
            for front in fronts {
                docks.update(kentos_ui::widget::docking::Event::Selected(front));
            }
        }
        let mut closed_any = false;
        for panel in Panel::ALL {
            if self.shows_panel(panel) {
                continue;
            }
            let mut closed = self.docks.close(panel);
            if let Some(docks) = &mut self.hidden_docks {
                closed |= docks.close(panel);
            }
            closed_any |= closed;
            if closed && !self.surface_closed.contains(&panel) {
                self.surface_closed.push(panel);
            }
        }
        // A panel closed here leaves Katmanlar in front, as at start, so the kept one in front stays kept for the
        // modules that show it (layout.rs keeps no fallback over another module's).
        if closed_any && !self.shows_panel(self.layout.kept_dock_tab()) {
            let docks = self.hidden_docks.as_mut().unwrap_or(&mut self.docks);
            if docks.contains(Panel::Layers) {
                docks.update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
            }
        }
        // Another module now (a project opened in kentos-cad, its tab chosen): the kept bottom tab and panel in front
        // come back when this module shows them, as each module's program would show its own, where the screen fell
        // back (Komut geçmişi, Katmanlar); a tab chosen meanwhile stays.
        let module = self.active_module();
        if self.surfaces_of != Some(module) {
            let first = self.surfaces_of.is_none();
            self.surfaces_of = Some(module);
            if !first {
                let tab = self.layout.kept_bottom_tab();
                if self.bottom_tab == BottomTab::History && self.shows_bottom_tab(tab) {
                    self.bottom_tab = tab;
                }
                let front = self.layout.kept_dock_tab();
                let docks = self.hidden_docks.as_mut().unwrap_or(&mut self.docks);
                if front_of_layers(docks) == Some(Panel::Layers) && docks.contains(front) {
                    docks.update(kentos_ui::widget::docking::Event::Selected(front));
                }
            }
        }
    }

    /// A file dropped on a program without Kaynaklar (KentOS Mimari): a drawing (KCAD, DWG, DXF, NCZ) opens in a
    /// tab of its own, the drawing on screen kept; a PDF opens PDF içe aktar with it; any other file is
    /// refused, the programs that take it named.
    pub(crate) fn drop_drawing(&mut self, path: PathBuf) -> Task<Message> {
        let name = crate::sources::folder_name(&path);
        let ext = extension(&path);
        // kentos-cad opens a GeoPackage as a CBS project in a tab of its own, beside the project on screen
        // (docs/adr/9195 §2: its tabs hold every module's projects).
        let gpkg =
            self.program.is_none() && kentos_native_application::project_store::is_geopackage(&path);
        if DROPPED_DRAWINGS.contains(&ext.as_str()) || gpkg {
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
        let takes = match self.program {
            Some(m) => format!("{} çizimleri (KCAD, DWG, DXF, NCZ) açar, PDF'i içe aktarır.", m.title()),
            None => "Bu proje çizimleri (KCAD, DWG, DXF, NCZ) açar, PDF'i içe aktarır.".to_owned(),
        };
        let here = self.here_words();
        let others = self.others_showing(|s| s.panel(Panel::Sources));
        let said = match kentos_interaction::sources::source_kind(&name) {
            Some(kind) if !others.is_empty() => format!(
                "“{name}” {here} açılmaz: {takes} {} {} açılır.",
                kind.label(),
                joined(&others)
            ),
            _ => format!("“{name}” {here} açılmaz: {takes}"),
        };
        self.warn(said);
        Task::none()
    }
}
