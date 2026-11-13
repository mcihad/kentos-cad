//! The desktop shell's state and update (docs/adr/0017): the open drawing,
//! the ribbon, the docked panels, the command line, the dialogs, and the
//! tool session drawing on the drawing (docs/adr/0021).
//!
//! Every button, shortcut and typed name runs a web command id through
//! [`App::run`]; the ones the desktop does not run yet say so instead of
//! doing nothing (CLAUDE.md §4.5). Keys, clicks and typed values go through
//! `input.rs` by ADR 0018's rules.

use std::path::PathBuf;
use std::time::Instant;

use iced::widget::operation;
use iced::{Subscription, Task, Theme, event, keyboard, window};
use serde_json::Value;

use kentos_commands::{ResolveReason, SettingConstraint};
use kentos_interaction::{
    Clipboard, Draft, Level, Memory, Selection, Session, SnapHit, Spatial, snap_kinds,
};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{self, Accent, Mode};
use kentos_ui::widget::command_line::Entry;
use kentos_ui::widget::docking::{self, Docks, Side};

use crate::catalog::{Standing, catalog};
use crate::cloud::{self, CloudState};
use crate::document::Document;
use crate::input::{Field, release_keyboard};
use crate::keys::{self, KeyPress};
use crate::opening::{self, Purpose};
use crate::recovery::{self, Recovery};
use crate::saving;
use crate::settings::Settings;
use crate::settings_view::{Edit, SettingsDraft, samples_label};
use crate::viewport::{self, Graphics, Viewport};

pub const COMMAND_INPUT: &str = "komut-satiri";

/// Width of the right dock at start (12 px body text; the showcase's).
const DOCK_WIDTH: f32 = 320.0;

/// The docked panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Layers,
    /// İşlemler: the processing toolbox and this session's runs, a tab beside Katmanlar (processing/panel.rs).
    Processing,
    /// Bloklar: the drawing's blocks, a tab beside Katmanlar and İşlemler (blocks_panel.rs, docs/adr/0144).
    Blocks,
    /// Şablonlar: the library's object templates, a tab beside Bloklar (templates_panel.rs, docs/adr/0176 §4).
    Templates,
    /// Kaynaklar: folders and KentOS projects to add layers from, a tab beside Şablonlar (sources/, docs/adr/0199 §7).
    Sources,
    /// Gezgin: levels, views, sheets and schedules, and where the selected object shows (navigator.rs, docs/adr/0778).
    Navigator,
    /// Yol projesi: the road project's sources and drawings with their states (road_project_panel.rs, docs/adr/0911 §4).
    RoadProject,
    /// Harita işi: a map project's items and derivations with their states (map_work_panel.rs, docs/adr/1200 §4).
    MapWork,
    /// GNSS arşivi: a map project's arazi ölçümleri, solutions and accepted points (map_survey_panel.rs,
    /// docs/adr/1340 §6).
    MapSurvey,
    /// Asistan: the user's own AI agent, chatting and drawing (assistant/, docs/adr/8056 §4).
    Assistant,
    Properties,
}

impl Panel {
    /// Every panel, in the dock's order at start (a module's program docks its own, module_surfaces.rs).
    pub(crate) const ALL: [Panel; 11] = [
        Panel::Layers,
        Panel::Processing,
        Panel::Blocks,
        Panel::Templates,
        Panel::Sources,
        Panel::Navigator,
        Panel::RoadProject,
        Panel::MapWork,
        Panel::MapSurvey,
        Panel::Assistant,
        Panel::Properties,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Panel::Layers => "Katmanlar",
            Panel::Processing => "İşlemler",
            Panel::Blocks => "Bloklar",
            Panel::Templates => "Şablonlar",
            Panel::Sources => "Kaynaklar",
            Panel::Navigator => "Gezgin",
            Panel::RoadProject => "Yol projesi",
            Panel::MapWork => "Harita işi",
            Panel::MapSurvey => "GNSS arşivi",
            Panel::Assistant => "Asistan",
            Panel::Properties => "Öznitelikler",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Panel::Layers => Icon::Layers,
            Panel::Processing => crate::icons::from_web(Some("processing")),
            Panel::Blocks => crate::icons::from_web(Some("blocks")),
            Panel::Templates => crate::icons::from_web(Some("templates")),
            Panel::Sources => crate::icons::from_web(Some("dataSources")),
            Panel::Navigator => crate::icons::from_web(Some("navigator")),
            Panel::RoadProject => crate::icons::from_web(Some("corridorEditor")),
            Panel::MapWork => crate::icons::from_web(Some("sourceLinked")),
            Panel::MapSurvey => crate::icons::from_web(Some("gnssArchive")),
            Panel::Assistant => crate::icons::from_web(Some("assistant")),
            Panel::Properties => Icon::Properties,
        }
    }

    /// The dock at start: `program`'s panels (none: kentos-cad's, every one; docs/adr/0802 §5).
    fn layout(program: Option<kentos_launch::Module>) -> Docks<Panel> {
        let mut docks = Docks::new();
        docks.dock(Panel::Layers, Side::Right);
        // İşlemler, Bloklar, Şablonlar and Kaynaklar share the top slot with Katmanlar, as the web's tabs; Katmanlar in front.
        for panel in [
            Panel::Processing,
            Panel::Blocks,
            Panel::Templates,
            Panel::Sources,
            Panel::Navigator,
            Panel::RoadProject,
            Panel::MapWork,
            Panel::MapSurvey,
            Panel::Assistant,
        ] {
            if crate::module_surfaces::shows_panel_in(program, panel) {
                docks.dock(panel, Side::Right);
            }
        }
        docks.update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
        docks.split(Panel::Properties, Side::Right);
        docks.set_size(Side::Right, DOCK_WIDTH);
        docks
    }
}

/// What waits for an answer in the middle of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Read-only candidate identity review for one split revision.
    RevisionMatches(usize),
    /// Farkları al for one split revision (docs/adr/1000 §6).
    RevisionTake(usize),
    /// İnceleme notları (docs/adr/1000 §6).
    ReviewNotes,
    /// Standart kaynaklarını kur (docs/adr/1002 §5).
    StandardInstall,
    About,
    Shortcuts,
    /// The drawing has unsaved changes; asked before `then` throws them away.
    Unsaved(Then),
    /// Uygulama ayarları (`tools.options`); its draft is `App::settings_draft`.
    Settings,
    /// Unsaved work a crash left: the first of `App::recovery.offers` (recovery.rs).
    Recovery,
    /// “Buluta giriş” (cloud/account.rs); its fields are `App::cloud.sign_in`.
    SignIn,
    /// “Bulut projeleri” (cloud/catalog.rs).
    Catalog,
    /// “Buluta yükle” (cloud/upload.rs).
    Upload,
    /// A database project's save conflicts (cloud/follow.rs).
    Conflicts,
    /// A question about the open file project's revisions (cloud/file_follow.rs):
    /// a refused or known-to-be-refused Kaydet, the newest revision offered
    /// over unsaved work or a clean drawing; the question is `App::cloud.question`.
    Revision,
    /// A copy to remove from this device whose draft holds unsent work (cloud/catalog.rs).
    RemoveCopy,
    /// The open cloud project ended for this account (deleted, archived, access taken away).
    Ended,
    /// A file exchange window (exchange/): DXF or a coordinate list, in or out.
    Exchange,
    /// Yeni proje or Proje ayarları (project/).
    Project,
    /// Başlangıç (start.rs).
    Start,
    /// Katman stili (style/layer_style/); the window is `App::styles.layer_style`.
    LayerStyle,
    /// Stil yöneticisi (style/manager/); the window is `App::styles.manager`.
    StyleManager,
    /// Lejant (style/legend/); the window is `App::styles.legend`.
    Legend,
    /// Sembol tasarımcısı (style/designer/); the window is `App::styles.designer`.
    SymbolDesigner,
    /// SVG çizim düzenleyicisi (style/svgedit/); the window is `App::styles.svg_editor`.
    SvgEditor,
    /// Katmanlar → Sil on a layer or group with objects (layering.rs); the
    /// node is `App::removing_layer`.
    RemoveLayer,
    /// Kaynaktan oku over the layer's own edits (bound.rs); the layer is `App::bound_pending`.
    SourceDiscard,
    /// The open cloud project's actions (cloud/actions.rs): Yeniden adlandır,
    /// Çöp kutusuna taşı.
    CloudRename,
    CloudTrash,
    /// Projeyi paylaş for the open project (cloud/share.rs); over the
    /// catalog the window is the catalog's.
    Share,
    /// Blok oluştur's window (blocks.rs, docs/adr/0144); the window is `App::blocks`.
    BlockDefine,
    /// Blok öznitelikleri (block_attributes.rs, docs/adr/0144 §7); the window is `App::block_attributes`.
    BlockAttributes,
    /// Blok kartı (block_card.rs, docs/adr/0317 §4); the window is `App::block_card`.
    BlockCard,
    BlockParameterGroups,
    BlockTypeValues,
    /// Blok çizelgesi (block_schedule.rs, docs/adr/0317 §3); the window is `App::block_schedule`.
    BlockSchedule,
    /// Bul ve değiştir (find_replace.rs, docs/adr/0145 §6); the window is `App::find_replace`.
    FindReplace,
    /// Çizim standardı (drawing_standard.rs, docs/adr/0312 §7); the window is `App::drawing_standard`.
    DrawingStandard,
    /// Çizgi tipleri (line_types_window.rs, docs/adr/0313 §7); the window is `App::line_types_window`.
    LineTypes,
    /// Parametreler (parameters_window.rs, docs/adr/0338 §1); the window is `App::parameters_window`.
    Parameters,
    /// Tarama desenleri (hatch_patterns_window.rs, docs/adr/0331 §1); the window is `App::hatch_patterns_window`.
    HatchPatterns,
    /// Değişikliğin etkisi (style_impact.rs, docs/adr/0314 §4); the window is `App::style_impact`.
    StyleImpact,
    /// Read-only impact of the current block editor draft.
    BlockReview,
    BlockReplace,
    BlockFind,
    ArrayFind,
    /// Bağlantılar (services/connections.rs).
    Connections,
    /// Harita servisi (services/window.rs).
    ServiceAdd,
    /// Servisten veri al (services/feed_window.rs).
    Feed,
    /// Katman durumları (layer_states.rs, docs/adr/0177 §4); the window is `App::layer_states_window`.
    LayerStates,
    /// Görünümler (views.rs, docs/adr/0285 §1); the window is `App::views_window`.
    Views,
    /// Kırmızı kot önerisi (propose_window.rs, plan/road.md Y25); the window is `App::propose_window`.
    Propose,
    /// 3B görünüm (scene3d_window.rs, plan/road.md Y12.3d); the window is `App::scene3d_window`.
    Scene3d,
    /// Yazı stilleri and Ölçü stilleri (annotation_styles.rs, docs/adr/0183 §5); the window is
    /// `App::annotation_styles`.
    AnnotationStyles,
    /// Tablo stilleri (table_style_window.rs, docs/adr/1127 §4); the window is `App::table_style_window`.
    TableStyleDefs,
    /// Pafta seti (sheet_set.rs, docs/adr/0347 §1); the window is `App::sheet_set`.
    SheetSet,
    /// Dış başvurular (xrefs_window.rs, docs/adr/0348 §5); its state is `App::xrefs`.
    Xrefs,
    /// 2B düzleme indir (flatten_window.rs, docs/adr/1061 §4); its state is `App::flatten`.
    Flatten,
    /// Taşınma raporu (clean_report_window.rs, docs/adr/1262); its state is `App::clean_report`.
    CleanReport,
    /// Dışa aktarma kaybı denetimi (exchange_check_window.rs, docs/adr/1660); its state is `App::exchange_check`.
    ExchangeCheck,
    /// Katalog geçiş raporu (catalog_report_window.rs, docs/adr/4940); its state is `App::catalog_report`.
    CatalogReport,
    /// Dış başvuru ekle (xrefs_window.rs); its choices are `App::xrefs.attach`.
    XrefAttach,
    /// Kullanılmayanları temizle (layer_purge.rs, docs/adr/0177 §5); the window is `App::layer_purge`.
    LayerPurge,
    /// Başka çizimden al (drawing_exchange.rs, docs/adr/0193 §2); the window is `App::take_from`.
    TakeFrom,
    /// Katman listesi (layer_list.rs, docs/adr/0177 §6); the window is `App::layer_list`.
    LayerList,
    /// Yazı tipleri (font_report.rs, docs/adr/0324 §5).
    FontReport,
    /// Alan ekle, Numarala and Yazı denetimi (text_tools.rs, docs/adr/0325 §4); the windows are `App::text_tools`.
    FieldInsert,
    TextNumber,
    TextCheck,
    /// Hizala and Kılavuz stilleri (leader_tools.rs, docs/adr/0326 §5); the windows are `App::leader_tools`.
    LeaderAlign,
    LeaderStyles,
    /// Ölçek yaz… (annotation_scale.rs, docs/adr/0205 §4); the field is `App::plot_scale_field`.
    PlotScale,
    /// Tablo ekle (tables/insert.rs, docs/adr/0184); the window is `App::table_insert`.
    TableInsert,
    /// Tabloyu düzenle (tables/editor.rs, docs/adr/0184 §5); the window is `App::table_editor`.
    TableEditor,
    /// Veri çıkarımı (tables/extract.rs, docs/adr/1030 §8); the window is `App::extract`.
    Extract,
    /// Detay paftaları (docs/adr/1112).
    SheetDetails,
    /// Baskı stili tablosu (plot_styles_window.rs, docs/adr/1130); the window is `App::plot_styles`.
    PlotStyles,
    /// Adres ekle and Eki düzenle (attachments.rs, docs/adr/0225 §2); the window is `App::attachment_form`.
    Attachment,
    /// The photo viewer (attachments.rs, docs/adr/0225 §2); the window is `App::photo_viewer`.
    PhotoViewer,
    /// A discipline's own windows (docs/adr/8054 §6), drawn by its `state.rs`.
    Road(crate::road::RoadDialogKind),
    Map(crate::map::MapDialogKind),
    Gis(crate::gis::GisDialogKind),
    Arch(crate::arch::ArchDialogKind),
}

/// Where the app goes once the drawing on screen is left (cloud/leaving.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// A local file (Aç).
    Open,
    Close(window::Id),
    /// Bulut oturumunu kapat.
    SignOut,
    /// A cloud project from the catalog.
    OpenCloud {
        tenant: kentos_domain::Uuid,
        project: kentos_domain::Uuid,
    },
    /// Buluta yükle, on the storage it starts with.
    Upload(kentos_commands::ProjectStorage),
    /// The drawing's own cloud project again, from the server.
    Reopen,
    /// The new project the Yeni proje window built (project/new.rs).
    NewProject,
    /// A recent file (start.rs); its path waits in `App::opening_recent`.
    OpenRecent,
    /// Çizimi kapat: the drawing's tab goes, the next one comes (doc_tabs.rs, docs/adr/0351 §3).
    CloseTab,
}

/// Where the open and save dialogs are answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Picker {
    /// The system's file dialog (`rfd`).
    #[default]
    Dialog,
    /// This file, without asking: the trace player's (the web runner's picker).
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Run a web command id (ribbon, shortcut, command line, dialog).
    Run(&'static str),
    /// Etkin kat chosen on the status bar (docs/adr/0713): a level by its id; none, Katsız.
    ActiveLevel(Option<String>),
    /// Katman anahtarı on or off (arch_start.rs, docs/adr/8008).
    LayerKeys(bool),
    /// Kat görünümü chosen (docs/adr/0715).
    LevelView(kentos_interaction::level_view::LevelView),
    /// Run a command ticked on a menu that stays open (a checklist's row:
    /// the selection filter's and the snap kinds, docs/adr/0187 §5): as
    /// `Run`, but the ribbon opened over the drawing stays as it is.
    RunKept(&'static str),
    /// Draw with the style library's object template of this id
    /// (`template.draw`, docs/adr/0176 §3; templates.rs).
    DrawTemplate(String),
    /// Şablonu uygula with the library's template of this id (docs/adr/0176 §6).
    ApplyTemplate(String),
    /// One of a tool's methods from its ribbon menu (Daire: 2 nokta): the
    /// tool starts, then takes the method's option as if typed (docs/adr/0032).
    RunMethod {
        id: &'static str,
        option: &'static str,
        label: &'static str,
    },
    /// An entry chosen from a split button's list: kept on top by the
    /// button's key (`ribbonSplits`), then run (docs/adr/0117).
    SplitChosen {
        key: &'static str,
        id: &'static str,
        option: Option<&'static str>,
        label: &'static str,
    },
    /// A command put on the quick access bar (true) or taken off (docs/adr/0117).
    QuickAccess(String, bool),
    /// What Fare ve klavye kısayolları searches for (shortcuts.rs).
    ShortcutsSearch(String),
    /// A pointer event taken so nothing under it reacts (a right click on
    /// Komut ara keeps the ribbon's menu away).
    Swallowed,
    /// The raster service made tiles, or a pyramid moved on: the drawing
    /// draws again with them (docs/adr/0204 §5).
    RastersReady,
    /// The point cloud service made nodes, opened a file or an index moved
    /// on: the drawing draws again with them (docs/adr/0207 §6).
    CloudsReady,
    /// The map services brought tiles or read what they need, or one failed:
    /// the drawing draws again, the failure goes to the log (docs/adr/0208 §6).
    ServicesReady,
    /// A mouse press anywhere or the window losing the focus while the key
    /// tips show: they go (ribbon_keys.rs, docs/adr/0118).
    KeyTipsAway,
    /// A press outside the folded ribbon open over the drawing: it closes.
    RibbonPeekAway,
    RibbonTab(&'static str),
    /// The sheet mode's own (sheets.rs).
    Sheet(kentos_sheet_ui::Message),
    /// The sheet templates' cloud library (sheet_library.rs).
    SheetLibrary(crate::sheet_library::LibraryMsg),
    /// Where the user chose to write a sheet's export; none: they did not.
    SheetExportTo(kentos_sheet_ui::ExportKind, Option<PathBuf>),
    /// A choice in the ribbon's own panels: the current properties for new
    /// objects, the plot scale (ribbon_panels.rs).
    RibbonPanel(crate::ribbon_panels::Event),
    CommandInput(String),
    CommandSubmitted,
    CommandRun(String),
    CommandHistoryToggled,
    /// A tab of the bottom panel chosen: the panel opens on it.
    BottomTab(crate::bottom::BottomTab),
    /// A table written as a file (docs/adr/0223 §2): what was written, or why not.
    TableWritten(crate::table_write::Outcome),
    /// The Arama tab (search/, docs/adr/0178).
    Search(crate::search::Event),
    /// A call's buttons in Komut geçmişi (history/, docs/adr/0284).
    History(crate::history::Event),
    CadInspection(crate::cad_inspection::Event),
    /// Kurtar's file, picked (docs/adr/1013).
    Recover(Option<std::path::PathBuf>),
    /// Köşe tablosu in the Koordinat listesi tab (vertices/, docs/adr/0172).
    Vertices(crate::vertices::Event),
    /// The bottom panel's top edge dragged: the open history's new height.
    BottomResized(f32),
    /// The bottom panel's top edge double-clicked: its first height again.
    BottomReset,
    /// Geçmişi temizle (the bottom panel's tab row).
    HistoryCleared,
    /// The status bar's type of a project not asked its type yet, pressed:
    /// the question (docs/adr/0622 §1).
    AskProjectType,
    /// The status bar's type of a CAD project not asked its work yet, pressed: the question (docs/adr/0801).
    AskCadWork,
    /// A frame of the status bar's message fading, or the moment it goes (message_log.rs).
    LogFrame,
    /// The bottom panel's log list scrolled: whether new lines follow to its end.
    LogScrolled(iced::widget::scrollable::Viewport),
    /// The kept layout is due to be written (layout.rs).
    LayoutSave,
    /// The rows round a bound layer's part, read on a thread of their own (band_ghosts.rs, docs/adr/0578 §1).
    BandRead(crate::band_ghosts::Read),
    /// The window's new size: the kept sizes are shown within it.
    WindowResized(iced::Size),
    /// The window's device pixels per logical pixel, at opening and on another screen: the brand's sizes follow it.
    DeviceScale(f32),
    /// The layer tree's rows and their menu (layering.rs).
    Layer(crate::layering::Event),
    /// The right button's menus over the drawing and the one-shot snap (drawing_menus.rs).
    DrawingMenu(crate::drawing_menus::Event),
    /// A lock chip's × in the value field: the length's (true) or the direction's (locks.rs).
    DropLock(bool),
    /// The text field over the drawing (text_field.rs).
    TextField(crate::text_field::Event),
    /// The paragraph editor over the drawing (paragraph_editor.rs, docs/adr/0182 §4).
    Paragraph(crate::paragraph_editor::Event),
    /// Öznitelikler: an edit or a section toggled (properties/).
    Properties(crate::properties::Event),
    /// Blok oluştur's window (blocks.rs).
    Blocks(crate::blocks::Event),
    /// Bloklar panel (blocks_panel.rs).
    BlocksPanel(crate::blocks_panel::Event),
    Navigator(crate::navigator::Event),
    /// Blok öznitelikleri's window (block_attributes.rs).
    BlockAttributes(crate::block_attributes::Event),
    BlockCard(crate::block_card::Event),
    BlockParameterGroups(crate::block_parameter_groups::Event),
    BlockTypeValues(crate::block_type_values::Event),
    BlockSchedule(crate::block_schedule::Event),
    FindReplace(crate::find_replace::Event),
    /// Çizim standardı's window (drawing_standard.rs, docs/adr/0312 §7).
    DrawingStandard(crate::drawing_standard::Event),
    ReviewNotes(crate::review_window::Event),
    StandardInstall(crate::standard_install::Event),
    /// Çizgi tipleri's window (line_types_window.rs, docs/adr/0313 §7).
    LineTypes(crate::line_types_window::Event),
    /// Parametreler's window (parameters_window.rs, docs/adr/0338 §1).
    Parameters(crate::parameters_window::Event),
    /// Tarama desenleri's window (hatch_patterns_window.rs, docs/adr/0331 §1).
    HatchPatterns(crate::hatch_patterns_window::Event),
    /// Değişikliğin etkisi (style_impact.rs, docs/adr/0314 §4).
    StyleImpact(crate::style_impact::Event),
    /// Blok düzenleyici (block_editor.rs, docs/adr/0316 §1).
    BlockEditor(crate::block_editor::Event),
    /// The open drawings' tabs (doc_tabs.rs, docs/adr/0351 §3).
    DocTabs(crate::doc_tabs::Event),
    /// An object's attachments: their windows, the viewer and the photos decoded (attachments.rs).
    Attachment(crate::attachments::Event),
    /// Bilgi's card over the drawing (info_card.rs, docs/adr/0225 §5).
    InfoCard(crate::info_card::Event),
    /// Katman durumları's window and Katman durumları ▾ (layer_states.rs).
    LayerStates(crate::layer_states::Event),
    /// Görünümler's window (views.rs, docs/adr/0285 §1).
    Views(crate::views::Event),
    /// An event of Kırmızı kot önerisi (propose_window.rs, plan/road.md Y25).
    Propose(crate::propose_window::Event),
    /// Resim olarak kaydet from a window's picture (picture.rs, docs/adr/0937): where it goes is asked.
    SavePicture(kentos_ui::widget::picture::Saved),
    /// The picture written (its path and size), or why not; none when the file was not chosen.
    PictureWritten(Option<Result<(std::path::PathBuf, u32, u32), String>>),
    /// An event of 3B görünüm (scene3d_window.rs, plan/road.md Y12.3d).
    Scene3d(crate::scene3d_window::Event),
    /// LandXML ver's file, or none when the dialog was closed (road_exchange.rs, plan/road.md Y10.2).
    LandxmlExport(Option<std::path::PathBuf>),
    /// Makine kontrolüne ver's file, or none (road_exchange.rs, docs/adr/1031).
    LandxmlMachine(Option<std::path::PathBuf>),
    /// LandXML'den aks al's file, or none (road_exchange.rs, plan/road.md Y10.2).
    LandxmlAlignments(Option<std::path::PathBuf>),
    /// IFC ver's file, or none (road_exchange.rs, plan/road.md Y10.3).
    IfcExport(Option<std::path::PathBuf>),
    /// IFC dışa aktar (bina): the file chosen (arch_ifc.rs, docs/adr/8047).
    IfcBuildingExport(Option<std::path::PathBuf>),
    GltfExport(Option<std::path::PathBuf>),
    AnnotationStyles(crate::annotation_styles::Event),
    TableStyleDefs(crate::table_style_window::Event),
    SheetSet(crate::sheet_set::Event),
    Xrefs(crate::xrefs_window::Event),
    Flatten(crate::flatten_window::Event),
    CleanReport(crate::clean_report_window::Event),
    ExchangeCheck(crate::exchange_check_window::Event),
    CatalogReport(crate::catalog_report_window::Event),
    XrefWatch(crate::xrefs_watch::Event),
    /// The card of a write the bonds refused (bond_notice.rs, docs/adr/0790 §2).
    BondNotice(crate::bond_notice::Event),
    LayerPurge(crate::layer_purge::Event),
    LayerList(crate::layer_list::Event),
    FontReport(crate::font_report::Event),
    TextTools(crate::text_tools::Event),
    LeaderTools(crate::leader_tools::Event),
    /// Ölçek yaz…'s window (annotation_scale.rs, docs/adr/0205 §4).
    PlotScale(crate::annotation_scale::Event),
    /// Seçilenleri dosyaya kaydet, Başka çizimden al and Dosyadan blok ekle (drawing_exchange.rs).
    DrawingExchange(crate::drawing_exchange::Event),
    /// Tablo ekle's window (tables/insert.rs).
    TableInsert(crate::tables::insert::Event),
    /// Tabloyu düzenle's window (tables/editor.rs).
    TableEditor(crate::tables::editor::Event),
    /// Tabloyu güncelle's files (tables/update.rs).
    TableUpdate(crate::tables::update::Event),
    /// Selected CAD table to CSV, XLSX or ODS.
    TableExport(crate::tables::export::Event),
    /// Veri çıkarımı (tables/extract.rs, docs/adr/1030 §8).
    Extract(crate::tables::extract::Event),
    SheetDetails(crate::sheet_details_window::Event),
    /// Baskı stili tablosu (docs/adr/1130).
    PlotStyles(crate::plot_styles_window::Event),
    /// Genel bakış and Büyüteç over the drawing (navigation_cards.rs).
    Navigation(crate::navigation_cards::Event),
    /// The map services' credits and windows (services/app.rs, docs/adr/0208).
    Services(crate::services::app::Event),
    /// Bağlı katmanlar (bound.rs, docs/adr/0503).
    Bound(crate::bound::Event),
    /// Metin dosyası yerleştir's file: its name and bytes, or none (text_file.rs).
    TextFile(Option<(String, Vec<u8>)>),
    /// Resim ekle's file: its name, path and bytes, or none (cancelled).
    ImageFile(Option<(String, Option<String>, Vec<u8>)>),
    /// The rollover card's wait is over, for this hover (hover_card.rs).
    HoverCard(u64),
    /// The pointer rested on a snap past the tracking dwell (tracking.rs).
    TrackDwell(u64),
    /// Katman stili (style/layer_style/).
    LayerStyle(crate::style::layer_style::Event),
    /// Stil yöneticisi (style/manager/).
    StyleManager(Box<crate::style::manager::Event>),
    /// Lejant (style/legend/).
    Legend(crate::style::legend::Event),
    /// Sembol tasarımcısı (style/designer/).
    Designer(Box<crate::style::designer::Event>),
    /// SVG çizim düzenleyicisi (style/svgedit/).
    SvgEdit(Box<crate::style::svgedit::Event>),
    /// The Python console (python/, docs/adr/0132).
    Python(crate::python::Event),
    /// Asistan's events (assistant/, docs/adr/8056 §4).
    Assistant(crate::assistant::Event),
    /// Esc in the empty command line: the running command ends.
    CommandCancelled,
    /// The command line's text box took or let go of the keyboard.
    CommandFocus(bool),
    /// An option button of the running command's prompt (its key: `G`, `Enter`).
    PromptOption(&'static str),
    /// A value an option offers, chosen from its menu: the option's key and
    /// what typing the value gives (Yazı's Hiza, docs/adr/0145 §6).
    PromptChoice(&'static str, String),
    /// A key press no text box captured (keys.rs); routed by ADR 0018.
    Key(KeyPress),
    CursorValueInput {
        token: u64,
        text: String,
        pasted: bool,
    },
    CursorValueSubmit(u64),
    /// The OS window lost focus: abandon capture, never synthesize a release.
    InputFocusLost,
    /// Text inputs consume Esc to blur; block command dialogs still cancel once.
    CapturedEscape,
    /// A worker's reply belongs to the drawing and command that asked.
    ScopedReply {
        document: u64,
        run: Option<u64>,
        reply: Box<Message>,
    },
    /// Shift, Ctrl, Alt or the logo key changed (Shift turns ortho over for a click).
    Modifiers(keyboard::Modifiers),
    Dock(docking::Event<Panel>),
    LayerSelected(String),
    /// An object by its persistent id selected and shown (the stale section's badge, docs/adr/8042 §5).
    SelectUid(String),
    /// Süzgeç ▾ over the layer tree: a property turned over (docs/adr/0311 §4).
    LayerProps(crate::layer_tree::Prop),
    /// Katman ara typed in; ↓ in it.
    LayerSearch(String),
    LayerSearchDown,
    /// Komut ara typed in; a found command run or its ribbon place shown;
    /// the outline's moment over (ribbon_search.rs).
    RibbonSearch(String),
    RibbonSearchRun(usize),
    RibbonSearchReveal(usize),
    RibbonFlashEnd(&'static str),
    /// A model's outline ends (ribbon_search.rs).
    RibbonModelFlashEnd(String),
    LayerVisible(String),
    LayerLocked(String),
    /// Yazdır: a node printed or not (`cad.layers.set`'s `plot`, docs/adr/0260 §4).
    LayerPlot(String),
    /// Dondur / Çöz a node (docs/adr/0311 §1).
    LayerFrozen(String),
    LayerExpanded(String),
    /// A drawing read from disk, or `None` when the file dialog was cancelled.
    /// Boxed: a drawing is large and messages are moved often.
    Opened(Option<Result<Box<Document>, String>>),
    /// A save finished, or `None` when the dialog was cancelled.
    Saved(Option<Result<Written, String>>),
    CloseRequested(window::Id),
    DialogConfirmed,
    DialogClosed,
    /// The drawing area: its size, the pointer, pan, zoom and clicks.
    Viewport(viewport::Event),
    /// The other view of a split drawing area (split_view.rs, docs/adr/0285 §3).
    OtherViewport(usize, viewport::Event),
    /// A split area's own controls: a view's theme, the comparison's divider (split_view.rs, docs/adr/0523).
    Split(crate::split_view::Event),
    /// The settings window: a value in its draft, a preset, Kaydet, the file actions.
    Settings(Edit),
    /// A drawing being opened in stages (opening.rs, docs/adr/0030).
    Opening(opening::Event),
    /// A drawing being saved off the UI thread (saving.rs).
    Saving(saving::Event),
    /// Recovery copies of unsaved work (recovery.rs).
    Recovery(recovery::Event),
    /// The cloud: signing in, the catalog, cloud projects (cloud/, docs/adr/0041).
    Cloud(Box<cloud::Event>),
    /// File exchange: DXF and coordinate lists, in and out (exchange/).
    Exchange(Box<crate::exchange::Event>),
    /// Yeni proje and Proje ayarları (project/).
    Project(Box<crate::project::Event>),
    /// The application menu (app_menu.rs).
    AppMenu(crate::app_menu::Event),
    /// Başlangıç (start.rs).
    Start(crate::start::Event),
    /// The interface's look from the Görünüm tab (appearance.rs).
    /// The server's answer to `server.check` (view_commands.rs).
    ServerChecked(Result<kentos_commands::Health, String>),
    /// A layer ticked or unticked on the Çakışma cell's menu (docs/adr/0162 §1).
    OverlapLayer(String),
    /// Sıradakini seç's list: the chip's candidate to choose (docs/adr/0187 §1).
    CycleTo(usize),
    /// The candidate the pointer rests on in that list: highlighted in the drawing.
    CycleHover(Option<kentos_domain::Slot>),
    /// The Kenet cell's menu: a Karelaj spacing, the settings (snap_menu.rs, docs/adr/0163 §6).
    Snap(crate::snap_menu::Event),
    /// The status bar's scale selector (screen_scale.rs, docs/adr/0165 §5).
    ScreenScale(crate::screen_scale::Event),
    /// A notice closed, by its ×, its time or Tümünü kapat (toasts.rs).
    ToastClosed(kentos_ui::widget::toast::Id),
    /// A discipline's own messages (docs/adr/8054 §6), handled by its `state.rs`.
    Road(crate::road::Event),
    Map(crate::map::Event),
    Gis(crate::gis::Event),
    Arch(crate::arch::Event),
}

/// A finished save: which opened drawing, where, and the revision written.
#[derive(Debug, Clone)]
pub struct Written {
    pub session: u64,
    pub path: PathBuf,
    pub revision: u64,
}

pub struct App {
    pub document: Option<Document>,
    pub tab: &'static str,
    /// The contextual Seçim tab is the one shown, while something is
    /// selected; an empty selection gives the ribbon back to `tab` (the web's).
    pub ribbon_context: bool,
    /// The contextual tab the selection opens, kept by the selection's and
    /// the drawing's change counts (docs/adr/0622 §4): the view asks it often.
    pub(crate) selection_tab_cache: std::cell::Cell<Option<crate::modes::SelectionTab>>,
    /// The selection's roles in the project (docs/adr/2480): what the contextual tabs' role panels ask.
    pub(crate) selection_roles: crate::selection_roles::RolesCache,
    /// The selection's change count the ribbon last followed: a new
    /// selection brings its tab to the front, none takes it away.
    pub(crate) ribbon_follows: Option<(u64, u64)>,
    pub ribbon_collapsed: bool,
    pub mode: Mode,
    pub accent: Accent,
    pub docks: Docks<Panel>,
    pub selected_layer: Option<String>,
    /// The layers of the drawing's selection, in the order first met; the
    /// layer tree shows them selected while `layers_follow` (layering.rs).
    pub(crate) selection_layers: Vec<String>,
    /// The last thing that chose the tree's selected rows was the drawing's
    /// selection, not a click in the tree.
    pub(crate) layers_follow: bool,
    /// The selection's version the tree last followed.
    pub(crate) followed_selection: u64,
    /// The layer being renamed in the tree and the name typed so far.
    pub(crate) renaming: Option<(String, String)>,
    /// The tree's last row press and when: a second one soon is a double click.
    pub(crate) last_layer_press: Option<(String, Instant)>,
    /// Süzgeç ▾'s properties over the layer tree (layer_tree.rs, docs/adr/0311 §4).
    pub(crate) layer_props: crate::layer_tree::LayerProps,
    /// Katman ara's text (layer_tree.rs).
    pub(crate) layer_query: String,
    /// Komut ara's text, and the command whose ribbon place is outlined
    /// for a moment (ribbon_search.rs).
    pub(crate) ribbon_search: String,
    pub(crate) ribbon_flash: Option<&'static str>,
    /// One of the user's models shown by Komut ara (its id).
    pub(crate) ribbon_flash_model: Option<String>,
    /// The layer tree has the keyboard: a row was pressed and nothing else
    /// took the keyboard since (layer_tree.rs).
    pub(crate) layers_keyboard: bool,
    /// The row the tree's keys chose, kept in view.
    pub(crate) layer_reveal: Option<String>,
    /// The menu open over the drawing (drawing_menus.rs).
    pub(crate) drawing_menu: Option<crate::drawing_menus::Open>,
    /// The one-shot snap and the command it was chosen in.
    pub(crate) snap_once: Option<(kentos_interaction::SnapKind, &'static str)>,
    /// The text field over the drawing (text_field.rs) and what it asks of the
    /// next task: the keyboard, its text chosen, the keyboard back.
    pub(crate) text_field: Option<crate::text_field::Open>,
    pub(crate) text_field_focus: bool,
    /// The paragraph editor over the drawing (paragraph_editor.rs) and whether it wants the keyboard.
    pub(crate) paragraph: Option<crate::paragraph_editor::Open>,
    pub(crate) paragraph_focus: bool,
    /// The block windows (blocks.rs, docs/adr/0144).
    pub(crate) blocks: crate::blocks::Blocks,
    /// The Bloklar panel (blocks_panel.rs).
    pub(crate) blocks_panel: crate::blocks_panel::PanelState,
    pub(crate) navigator: crate::navigator::PanelState,
    /// The bound standards' tool palettes (docs/adr/1014).
    pub(crate) office_palettes: crate::office_palettes::OfficePalettes,
    /// Sil's question for a connection, by its name.
    pub(crate) db_removing: Option<String>,
    /// The database connections this window lists: the device's (`kentos_postgis::connections::store`), whose saved
    /// passwords the bound layers' commands find; a test's app keeps its own in memory.
    pub(crate) db_store: &'static kentos_postgis::connections::ConnectionStore,
    /// The connection PostGIS'e bağlan listed last, chosen when it opens again.
    pub(crate) postgis_last: Option<String>,
    /// Blok öznitelikleri's window, while it is open or away for a point (block_attributes.rs).
    pub(crate) block_attributes: Option<crate::block_attributes::Window>,
    pub(crate) block_card: Option<crate::block_card::Window>,
    pub(crate) block_parameter_groups: Option<crate::block_parameter_groups::Window>,
    pub(crate) block_type_values: Option<crate::block_type_values::Window>,
    pub(crate) block_schedule: Option<crate::block_schedule::Window>,
    /// Bul ve değiştir's window, while it is open (find_replace.rs).
    pub(crate) find_replace: Option<crate::find_replace::Window>,
    /// Alan ekle, Numarala and Yazı denetimi (text_tools.rs, docs/adr/0325 §4).
    pub(crate) text_tools: crate::text_tools::TextTools,
    /// Kol ekle, Kol sil, Hizala and Kılavuz stilleri (leader_tools.rs, docs/adr/0326 §5).
    pub(crate) leader_tools: crate::leader_tools::LeaderTools,
    /// Metin dosyası yerleştir asked for its file: the dialog opens after the update (text_file.rs).
    pub(crate) text_file_wanted: bool,
    /// Koordinat oku's points whose rasters' values are to be read (docs/adr/0204 §8).
    pub(crate) raster_values_wanted: Vec<kentos_interaction::Vec2>,
    /// XYZ sor's places and reaches whose clouds are to be read (docs/adr/0207 §7).
    pub(crate) cloud_query_wanted: Vec<(kentos_interaction::Vec2, f64)>,
    /// The network tools' questions, asked after the update (networks/, docs/adr/0209 §12).
    pub(crate) network_wanted: Vec<kentos_interaction::network::NetworkAsk>,
    /// Commands a tool asked the host to run (Ağlar from the network tools).
    pub(crate) command_wanted: Vec<&'static str>,
    /// Resim ekle asked for a picture file (docs/adr/0192 §5).
    pub(crate) image_file_wanted: bool,
    pub(crate) text_field_select: bool,
    pub(crate) text_field_release: bool,
    /// Öznitelikler's closed sections, by id, while the app runs (the web's `collapsed`).
    pub(crate) props_closed: std::collections::HashSet<&'static str>,
    pub(crate) props_groups_closed: std::collections::HashSet<String>,
    /// Öznitelikler for a selection of several objects, as last worked out (properties/).
    pub(crate) properties_cache: crate::properties::PanelCache,
    pub(crate) properties_scope: crate::properties::InspectScope,
    /// The bottom panel's Arama tab: its choices and rows, and the place it marked (search/, docs/adr/0178).
    pub(crate) search: crate::search::SearchPanel,
    pub(crate) cad_inspection: crate::cad_inspection::Panel,
    /// Köşe tablosu's rows selected and cell edited (vertices/, docs/adr/0172).
    pub(crate) vertices: crate::vertices::VertexPanel,
    /// The labels the view shows, as last asked of the geometry store (labels.rs).
    pub(crate) label_spots: crate::labels::Spots,
    /// Görünümü böl's other view (split_view.rs, docs/adr/0285 §3); none with one view.
    pub(crate) split: Option<crate::split_view::Split>,
    /// The map services' labels and credits for the view, as last worked out (services/overlay.rs).
    pub(crate) service_marks: crate::services::overlay::Kept,
    /// Whether the services' credits card is open (services/app.rs).
    pub(crate) service_credits: bool,
    /// Bağlantılar, while it is open (services/connections.rs, docs/adr/0208 §12).
    pub(crate) connections: Option<crate::services::connections::Window>,
    /// Harita servisi, while it is open (services/window.rs, docs/adr/0208 §14).
    pub(crate) service_window: Option<crate::services::window::Window>,
    /// Servisten veri al, while it is open (services/feed_window.rs, docs/adr/0208 §10).
    pub(crate) feed_window: Option<crate::services::feed_window::Window>,
    /// Servis bilgisi's point and answers (services/info.rs, docs/adr/0208 §11).
    pub(crate) service_info: crate::services::info::Info,
    /// The last left press's object with no command running, and when (a double click edits a text).
    pub(crate) last_click: Option<(kentos_domain::Slot, Instant)>,
    /// The log (message_log.rs): every line said, 500 at most, with its time
    /// and level; the bottom panel lists it, the status bar shows its newest.
    pub(crate) log: crate::log_plan::Log,
    /// The commands' calls Komut geçmişi lists beside the log (history/, docs/adr/0284).
    pub(crate) history: crate::history::History,
    /// What was typed in the command line: its ↑ brings them back.
    pub(crate) typed: Vec<Entry>,
    /// What the log's views follow: the status bar's message, the list's end.
    pub(crate) follow: crate::message_log::Follow,
    /// The layout kept between runs (layout.rs) and the window's size it is shown in.
    pub(crate) layout: crate::layout::Keeper,
    /// Where the windows' sizes are written (docs/adr/0746).
    pub(crate) dialog_sizes: crate::dialog_sizes::Keeper,
    /// The rows round the bound layers' parts, asked for and arrived (band_ghosts.rs, docs/adr/0578 §1).
    pub(crate) bands: crate::band_ghosts::Bands,
    pub(crate) window_size: iced::Size,
    pub command_input: String,
    pub command_expanded: bool,
    /// The bottom panel's tab (bottom.rs).
    pub bottom_tab: crate::bottom::BottomTab,
    /// The Python console, the bottom panel's Python tab (python/, docs/adr/0132).
    pub(crate) python: crate::python::Console,
    /// Asistan: the user's agents and the chat with one (assistant/, docs/adr/8056 §4).
    pub(crate) assistant: crate::assistant::Assistant,
    /// The open history's height when the panel's edge was dragged (in memory,
    /// as the docks' layout); `None`: the command line's own.
    pub bottom_log: Option<f32>,
    pub dialog: Option<Dialog>,
    /// A window waiting under the one on top, back when that one closes:
    /// Uygulama ayarları under Proje ayarları, or the other way (the web's
    /// stacked dialogs; settings_sections.rs, project/settings.rs).
    pub dialog_under: Option<Dialog>,
    /// What Fare ve klavye kısayolları searches for (shortcuts.rs).
    pub shortcuts_query: String,
    /// The drawing area's camera and scene cache.
    pub viewport: Viewport,
    /// The running tool and the last one started (kentos-interaction, docs/adr/0021).
    pub session: Session,
    /// The geometry store kept in step with the open drawing: what a click
    /// picks, a box selects and a point snaps to (docs/adr/0029).
    pub spatial: Spatial,
    /// The style library and the styled drawing's pictures (style/, docs/adr/0090).
    pub styles: crate::style::Styles,
    /// What the drawing tools remember between runs for as long as the app
    /// lives: the last circle radius, the rectangle's rotation and corners, the
    /// regular polygon's sides (the web's static tool fields, docs/adr/0032).
    pub memory: Memory,
    /// The selected objects and the hovered one (session state, not the drawing's).
    pub selection: Selection,
    /// What Kes and Panoya kopyala put aside for Yapıştır (session state: it
    /// outlives the drawing on screen; clipboard.rs, docs/adr/0056).
    pub clipboard: Clipboard,
    /// The object snap under the pointer while a tool snaps: its marker.
    pub snap: Option<SnapHit>,
    /// Tab's steps through the snap candidates where the pointer stands (docs/adr/0271).
    pub(crate) snap_cycle: Option<crate::snap_cycle::SnapCycle>,
    /// The point the tools got for the cursor, beside the world point under
    /// it then: the snap's or the tracking lock's when there is one. The
    /// status bar shows it while the cursor is still there (the web's
    /// `cursorWorld`, which is the pointer's `world`).
    pub(crate) cursor_point: Option<(kentos_interaction::Vec2, kentos_interaction::Vec2)>,
    /// The drawing (its session) and generation the store and the selection last followed.
    pub(crate) followed: Option<(u64, u64)>,
    /// The value field beside the cursor, while it is open (ADR 0018).
    pub field: Option<Field>,
    /// Drafting aids for new points: ortho, polar tracking, the snap aperture
    /// (the typed settings' `drafting.*`, applied by `apply_settings`), and
    /// the colour new objects take (the ribbon's Renk, ribbon_panels.rs).
    pub draft: Draft,
    /// The line type new objects take, `None` by layer (the ribbon's Tip):
    /// the session's, which no tool reads yet (web). The weight is the
    /// draft's (`Draft::line_weight`), which the tools give new objects.
    /// Typed values open beside the cursor (`drafting.cursorInput`).
    pub cursor_input: bool,
    /// The strip over the drawing while a command runs (`drafting.commandBar`, command_bar.rs).
    pub command_bar: bool,
    /// The rollover card shows (`drafting.hoverInfo`, hover_card.rs).
    pub hover_info: bool,
    /// The crosshair's arms (`appearance.crosshair`, marks.rs).
    pub crosshair: crate::marks::CrosshairSize,
    /// The middle button pans the drawing: no crosshair meanwhile (input.rs).
    pub panning: bool,
    /// The views left by navigating, for Önceki and Sonraki görünüm; session
    /// state that never enters the drawing (navigation.rs, docs/adr/0141).
    pub(crate) view_history: kentos_interaction::ViewHistory,
    /// The Kaydır tool's drag has recorded the view it left (navigation.rs).
    pub(crate) pan_recorded: bool,
    /// The scale typed in the status bar's scale selector, while it is open (screen_scale.rs).
    pub(crate) scale_field: Option<String>,
    /// The time the wheel's passes are told by: the clock's, unless a test sets it (navigation.rs).
    pub(crate) view_clock: Option<std::time::Duration>,
    /// The object whose rollover card shows (hover_card.rs).
    pub hover_card: Option<kentos_domain::Slot>,
    /// The hover the card's wait was started for (`Selection::hover_version`).
    pub hover_seen: u64,
    /// Nesne izleme: the points acquired by resting on a snap and the cursor's lock (tracking.rs, docs/adr/0085).
    pub tracking: kentos_interaction::object_tracking::ObjectTracking,
    /// Seçili katmanlarda önle's layers, by id (docs/adr/0162 §1): the
    /// session's, not a setting; the tools see them in their context.
    pub(crate) overlap_layers: Vec<String>,
    /// The last write the bonds refused (docs/adr/0790 §2): its card over the drawing offers the way through.
    pub(crate) bonds_refused: Option<kentos_commands::CommandError>,
    /// Seçim süzgeci's kinds, as `kentos_interaction::selectable::bit`s: the
    /// session's, every kind at first (docs/adr/0187 §5).
    pub(crate) select_kinds: u32,
    /// The digitizing locks (docs/adr/0166): what holds the next point; the
    /// session's, the tools see them in their context.
    pub locks: kentos_interaction::LockState,
    /// The UCSs Önceki gives back (ucs.rs, docs/adr/0261 §1).
    pub(crate) ucs_history: crate::ucs::UcsHistory,
    /// Kat görünümü (active_level.rs, docs/adr/0715): which levels the plan shows, the session's.
    pub(crate) level_view: kentos_interaction::level_view::LevelView,
    /// The last template started, for Son komutu yinele; none once another
    /// command is.
    pub(crate) last_template: Option<String>,
    /// Each point template's next name, or a text template's next text, by
    /// its id: its series goes on from run to run in the drawing (docs/adr/0176
    /// §3b), and a group's members take theirs from it (§5).
    pub(crate) template_names: std::collections::HashMap<String, String>,
    /// Çizim standardı's window (drawing_standard.rs, docs/adr/0312 §7).
    pub(crate) drawing_standard: Option<crate::drawing_standard::Window>,
    pub(crate) review_window: Option<crate::review_window::Window>,
    pub(crate) standard_install: Option<crate::standard_install::Window>,
    /// Çizgi tipleri's window (line_types_window.rs, docs/adr/0313 §7).
    pub(crate) line_types_window: Option<crate::line_types_window::Window>,
    /// Parametreler's window (parameters_window.rs, docs/adr/0338 §1).
    pub(crate) parameters_window: Option<crate::parameters_window::Window>,
    /// The constraints' marks over the drawing (constraint_view.rs, docs/adr/0338 §1).
    pub(crate) constraint_view: crate::constraint_view::ConstraintView,
    /// Tarama desenleri's window (hatch_patterns_window.rs, docs/adr/0331 §1).
    pub(crate) hatch_patterns_window: Option<crate::hatch_patterns_window::Window>,
    /// Değişikliğin etkisi's window (style_impact.rs, docs/adr/0314 §4).
    pub(crate) style_impact: Option<crate::style_impact::Window>,
    /// Blok düzenleyici, while it is open (block_editor.rs, docs/adr/0316 §1).
    pub(crate) block_editor: Option<crate::block_editor::BlockEditor>,
    /// The drawings set aside in their tabs (doc_tabs.rs, docs/adr/0351 §3).
    pub(crate) doc_tabs: crate::doc_tabs::DocTabs,
    /// Kırp's frames shown on the screen (clip_frames.rs, docs/adr/0351 §1).
    pub(crate) clip_frames: crate::clip_frames::ClipFrames,
    /// Bilgi's card: the object it shows and the drawing's point it was clicked at (info_card.rs).
    pub(crate) info_card: Option<(kentos_domain::Slot, kentos_interaction::Vec2)>,
    /// The attachments' photos decoded for the shell, each once (photo_view.rs).
    pub(crate) photos: crate::photo_view::Photos,
    /// The photo viewer (attachments.rs).
    pub(crate) photo_viewer: Option<crate::attachments::Viewer>,
    /// Adres ekle and Eki düzenle (attachments.rs).
    pub(crate) attachment_form: Option<crate::attachments::Form>,
    /// Katman durumları's window (layer_states.rs, docs/adr/0177 §4).
    pub(crate) layer_states_window: Option<crate::layer_states::Window>,
    /// Görünümler's window (views.rs, docs/adr/0285 §1).
    pub(crate) views_window: Option<crate::views::Window>,
    pub(crate) form_clipboard:
        Option<(kentos_commands::FormItem, Vec<kentos_commands::LayerField>)>,
    /// Kırmızı kot önerisi (propose_window.rs, plan/road.md Y25).
    pub(crate) propose_window: Option<Box<crate::propose_window::Window>>,
    /// 3B görünüm (scene3d_window.rs, plan/road.md Y12.3d).
    pub(crate) scene3d_window: Option<Box<crate::scene3d_window::Window>>,
    /// Mahalleri birleştir asked: its input and its decision's words (docs/adr/0773 §4).
    pub(crate) space_merging: Option<(kentos_commands::SpacesEdit, String)>,
    /// The object a drawing tool has just written: its form opens (forms/mod.rs, docs/adr/0403 §4).
    pub(crate) form_on_create: Option<kentos_domain::Slot>,
    /// Yazı stilleri or Ölçü stilleri (annotation_styles.rs, docs/adr/0183 §5).
    pub(crate) annotation_styles: Option<crate::annotation_styles::Window>,
    /// Tablo stilleri (table_style_window.rs, docs/adr/1127 §4).
    pub(crate) table_style_window: Option<crate::table_style_window::Window>,
    pub(crate) sheet_set: Option<crate::sheet_set::Window>,
    /// Dış başvurular and Dış başvuru ekle (xrefs_window.rs, docs/adr/0348 §5).
    pub(crate) xrefs: crate::xrefs_window::State,
    pub(crate) flatten: crate::flatten_window::State,
    pub(crate) clean_report: crate::clean_report_window::State,
    pub(crate) exchange_check: crate::exchange_check_window::State,
    pub(crate) catalog_report: crate::catalog_report_window::State,
    /// The loaded references' files followed, and the card of a changed one (xrefs_watch.rs, docs/adr/0349 §3).
    pub(crate) xref_watch: crate::xrefs_watch::Watch,
    /// Kullanılmayanları temizle's window (layer_purge.rs, docs/adr/0177 §5).
    pub(crate) layer_purge: Option<crate::layer_purge::Window>,
    /// Başka çizimden al's window (drawing_exchange.rs, docs/adr/0193 §2).
    pub(crate) take_from: Option<crate::drawing_exchange::Window>,
    /// Katman listesi's window (layer_list.rs, docs/adr/0177 §6).
    pub(crate) layer_list: Option<crate::layer_list::Window>,
    /// Ölçek yaz…'s typed scale while its window is open (annotation_scale.rs, docs/adr/0205 §4).
    pub(crate) plot_scale_field: Option<String>,
    /// Tablo ekle's choices, kept while the app runs (tables/insert.rs, docs/adr/0184).
    pub(crate) table_insert: crate::tables::insert::Insert,
    /// Tabloyu düzenle's window (tables/editor.rs).
    pub(crate) table_editor: Option<crate::tables::editor::Editor>,
    /// Çizimden seç: the window stepped aside and the selection before the pick (window_pick.rs, docs/adr/8006).
    pub(crate) window_pick: Option<(Dialog, Vec<kentos_domain::Slot>)>,
    /// Veri çıkarımı's recipe, kept while the program runs (docs/adr/1030 §8).
    pub(crate) extract: Option<crate::tables::extract::Extract>,
    /// Detay paftaları' window (docs/adr/1112).
    pub(crate) sheet_details: Option<crate::sheet_details_window::DetailSet>,
    /// Baskı stili tablosu's window (docs/adr/1130).
    pub(crate) plot_styles: Option<crate::plot_styles_window::PlotStyles>,
    /// Tabloyu güncelle waiting for its files (tables/update.rs).
    pub(crate) table_update: Option<crate::tables::update::Run>,
    /// Genel bakış and Büyüteç's session state (navigation_cards.rs, docs/adr/0181).
    pub(crate) navigation: crate::navigation_cards::Navigation,
    /// The label spots the magnifier's window drew last (view.rs `lens_view`).
    pub(crate) lens_spots: crate::labels::Spots,
    /// Cihaza gönder's last format, kept for the session (exchange/field_send.rs).
    pub(crate) field_send_format: usize,
    /// The mode the Çakışma cell's click turns on again: the last that avoided overlap.
    pub(crate) overlap_last: kentos_interaction::Overlap,
    /// The command the tracking points belong to, and the last rest whose wait began.
    pub(crate) tracking_tool: &'static str,
    pub(crate) tracking_waited: u64,
    /// Whether a rest's dwell passes on real time; the trace player passes it by hand (traces/).
    pub(crate) dwell_on_time: bool,
    /// Whether İfade oluşturucu was left in Akış (for as long as the program runs).
    pub(crate) builder_flow: bool,
    /// İfade oluşturucu's Son kullanılanlar (for as long as the program runs; docs/adr/0603).
    pub(crate) builder_recent: Vec<String>,
    /// The ribbon's key tips while they show (ribbon_keys.rs, docs/adr/0118).
    pub(crate) key_tips: Option<crate::ribbon_keys::KeyTips>,
    /// Alt is down alone: letting it go shows the key tips.
    pub(crate) alt_armed: bool,
    /// The folded ribbon's tab open over the drawing.
    pub(crate) ribbon_peek: bool,
    /// The layer or group Katmanlar → Sil asks about (`Dialog::RemoveLayer`).
    pub removing_layer: Option<String>,
    /// The bound layer Kaynaktan oku asks about (bound.rs, `Dialog::SourceDiscard`).
    pub bound_pending: Option<String>,
    /// The typed settings (docs/adr/0023): kept in `ayarlar.json` when opened by `main`.
    pub settings: Settings,
    /// The settings window's draft while it is open.
    pub settings_draft: Option<SettingsDraft>,
    /// The sample-count failure already reported, so it is said once.
    reported_failure: Option<(u32, u32)>,
    /// Whether the command line's text box has the keyboard.
    pub line_focused: bool,
    pub modifiers: keyboard::Modifiers,
    /// The level of the newest message; the traces read it (ADR 0018).
    pub last_level: Option<Level>,
    pub picker: Picker,
    /// The open under way, if any (opening.rs).
    pub opening: Option<opening::Opening>,
    /// A large import going into the drawing a frame at a time (exchange/drawing_import.rs).
    pub importing: Option<crate::exchange::drawing_import::Importing>,
    /// Tables binding on a thread of their own (exchange/binding.rs, docs/adr/0757).
    pub binding: Option<crate::exchange::binding::Binding>,
    /// The save under way, if any (saving.rs).
    pub saving: Option<saving::Saving>,
    /// Where tests make the disk fail during a save (none in the app).
    pub save_faults: saving::Faults,
    /// Recovery copies of unsaved work; kept only when `main` opens their folder.
    pub recovery: Recovery,
    /// The cloud: the account, its windows, the open project's autosave (cloud/).
    pub cloud: CloudState,
    /// The open file exchange window (exchange/).
    pub exchange: Option<crate::exchange::Window>,
    /// The open project window (project/).
    pub project: Option<crate::project::Window>,
    /// The application menu, while it is open (app_menu.rs).
    pub app_menu: Option<crate::app_menu::State>,
    /// Drawings opened or saved lately (recent.rs); kept in a file when `main` opens its folder.
    pub recent: crate::recent::RecentFiles,
    /// The recent file to open once the drawing on screen is left (start.rs).
    pub opening_recent: Option<PathBuf>,
    /// The drawing area's background (Görünüm → Çizim zemini, appearance.rs).
    pub backdrop: crate::appearance::Backdrop,
    /// The interface's typefaces and text size as last applied (appearance.rs).
    pub typography: kentos_ui::theme::typography::Typography,
    /// The interface's corners and shadows as last applied (appearance.rs, docs/adr/0127).
    pub shape: kentos_ui::theme::shape::Shape,
    /// The window fills the screen (`view.fullscreen`, view_commands.rs).
    pub fullscreen: bool,
    /// The side panels' layout while F4 hides them, put back as it was.
    pub(crate) hidden_docks: Option<Docks<Panel>>,
    /// A server check is on its way (`server.check` waits for it).
    /// The check at start: its answer goes to the server cell, not to the log.
    pub(crate) server_quiet: bool,
    pub server_checking: bool,
    /// The last server check's answer: its health, or why there was none
    /// (KentOS CAD hakkında names the server by it).
    pub(crate) server_health: Option<Result<kentos_commands::Health, String>>,
    // Sheet layouts (sheets.rs, docs/sheet/design.md §11).
    pub sheets: kentos_sheet_ui::Designer,
    pub(crate) sheet_maps: crate::sheets::SheetMaps,
    /// The contextual Pafta tab is open (a sheet is in front).
    pub(crate) sheet_tab: bool,
    /// Where the books are kept; none in tests and snapshots.
    pub sheet_store: Option<kentos_sheet_ui::Store>,
    /// The open project's book as the designer shows it (docs/adr/0251; sheets.rs `follow_sheets`).
    pub(crate) sheet_book: crate::sheet_book::BookLink,
    pub(crate) sheet_session: Option<u64>,
    pub(crate) sheet_generation: Option<(u64, u64)>,
    pub(crate) sheet_attributes: std::cell::Cell<bool>,
    pub(crate) sheet_extent: std::cell::Cell<Option<[f64;4]>>,
    /// The box of the objects on the project's levels, the ground of Kat planı paftaları (docs/adr/0718).
    pub(crate) sheet_level_extent: std::cell::Cell<Option<[f64; 4]>>,
    /// The drawing's variables and what it says of itself for a title block (docs/adr/8037), with the drawing's
    /// session and generation they were read at: read again only when the drawing changes.
    pub(crate) sheet_drawing_variables:
        std::cell::RefCell<(Option<(u64, u64)>, Vec<kentos_sheet::model::Variable>)>,
    /// The sheet templates' cloud library (sheet_library.rs, docs/sheet/design.md §13).
    pub(crate) sheet_library: crate::sheet_library::SheetLibrary,
    /// What the sheet mode's tables, coordinate lists and legends were last given for (sheet_inputs.rs).
    pub(crate) sheet_data_key: Option<u64>,
    /// The window's notices in its corner (toasts.rs): a save that closed its window, a copy.
    pub(crate) toasts: kentos_ui::widget::Toasts<Message>,
    /// The module this program works as (docs/adr/0801): KentOS Mimari, Harita or GIS; none in kentos-cad, every
    /// module at once.
    pub(crate) program: Option<kentos_launch::Module>,
    /// Starts the program a file belongs to (program.rs); a test puts one of its own here.
    pub(crate) hand_over: fn(kentos_launch::Module, &std::path::Path) -> Result<String, String>,
    /// The disciplines' own windows and panels (docs/adr/8054 §6): each module keeps its state in its
    /// `state.rs`, so a new window of one leaves this struct as it is.
    pub(crate) road: crate::road::RoadUiState,
    pub(crate) map: crate::map::MapUiState,
    pub(crate) gis: crate::gis::GisUiState,
    pub(crate) arch: crate::arch::ArchUiState,
}

impl App {

    pub fn title(&self) -> String {
        // A module's program says its own name first (docs/adr/0801): “KentOS Mimari — Konut.kcad”.
        if let Some(program) = self.program {
            let Some(doc) = &self.document else {
                return program.title().to_owned();
            };
            let dot = if doc.dirty() { " •" } else { "" };
            return match doc.cloud_source() {
                Some(source) => format!(
                    "{} — {}{dot} — {}",
                    program.title(),
                    doc.name(),
                    source.workspace
                ),
                None => format!("{} — {}{dot}", program.title(), doc.name()),
            };
        }
        match &self.document {
            // A cloud project says its workspace too (docs/adr/0041).
            Some(doc) => match doc.cloud_source() {
                Some(source) => format!(
                    "{}{} — {} — KentOS CAD",
                    doc.name(),
                    if doc.dirty() { " •" } else { "" },
                    source.workspace
                ),
                None => format!(
                    "{}{} — KentOS CAD",
                    doc.name(),
                    if doc.dirty() { " •" } else { "" }
                ),
            },
            None => "KentOS CAD".to_owned(),
        }
    }

    /// How the open drawing's numbers read (its units and its type's axes);
    /// a new project's without a drawing.
    pub(crate) fn format(&self) -> kentos_interaction::Format {
        self.document
            .as_ref()
            .map_or_else(kentos_interaction::Format::default, |d| {
                kentos_interaction::Format::of(d.settings())
            })
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode, self.accent)
    }

    /// Asks for a drawing and opens it in stages, off the UI thread (opening.rs).
    pub(crate) fn open(&mut self) -> Task<Message> {
        self.open_into(Purpose::File)
    }

    /// Aç, or Yeni sekmede aç with `Purpose::Tab` (doc_tabs.rs): the file asked for, then opened for `purpose`.
    pub(crate) fn open_into(&mut self, purpose: Purpose) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return self.start_opening(path, purpose);
        }
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Çizim aç")
                    .add_filter("CAD çizimleri (.kcad, .dwg, .dxf, .ncz)", &["kcad", "dwg", "dxf", "ncz"])
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .add_filter("DWG ve DXF çizimleri (.dwg, .dxf)", &["dwg", "dxf"])
                    .add_filter("NCZ çizimi (.ncz)", &["ncz"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| match &purpose {
                Purpose::Tab => Message::Opening(opening::Event::PickedTab(path)),
                _ => Message::Opening(opening::Event::Picked(path)),
            },
        )
    }

    /// Saves to the drawing's file as `.kcad` v2, or asks where (always, with
    /// `choose`, and for a drawing opened from a v1 file, which is never
    /// written over by itself; docs/adr/0025). The drawing of this moment is
    /// written, off the UI thread (saving.rs): a change made while the file is
    /// written stays unsaved.
    fn save(&mut self, choose: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Kaydedilecek çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        };
        let title = if doc.legacy && !choose {
            "Yeni biçimde kaydet (KCAD v2)"
        } else {
            "Farklı kaydet"
        };
        let known = doc
            .path
            .clone()
            .filter(|_| !choose && !doc.legacy)
            .or(match &self.picker {
                Picker::File(path) => Some(path.clone()),
                Picker::Dialog => None,
            });
        if let Some(path) = known {
            let path = if kentos_native_application::cad_open::Format::of(&path).is_some() {
                path.with_extension("kcad")
            } else { path };
            return self.start_saving(path);
        }
        let suggested = doc.name().trim_end_matches(".kcad").to_owned();
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .set_file_name(format!("{suggested}.kcad"))
                    .save_file()
                    .await?;
                let path = file.path().to_path_buf();
                Some(if path.extension().is_some_and(|e| e == "kcad") {
                    path
                } else {
                    path.with_extension("kcad")
                })
            },
            |path| Message::Saving(saving::Event::Picked(path)),
        )
    }
}

/// Turkish-aware case folding for command names: “çizgi”, “Cizgi” and “CIZGI” match.
pub(crate) fn fold(text: &str) -> String {
    text.trim()
        .chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' | 'i' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
#[path = "app/tests.rs"]
mod tests;

/// The window's device pixels per logical pixel when it changes (moved to another screen), for the brand's sizes
/// (kentos_ui's brand.rs); at opening the app asks for it once.
fn device_scale(event: iced::Event, _: iced::event::Status, _: iced::window::Id) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::Rescaled(scale)) => Some(Message::DeviceScale(scale)),
        _ => None,
    }
}

#[path = "app/start.rs"]
mod start;

#[path = "app/update.rs"]
mod update;

#[path = "app/settings.rs"]
mod settings;

#[path = "app/commands.rs"]
mod commands;
