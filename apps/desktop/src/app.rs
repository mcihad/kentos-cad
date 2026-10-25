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
    Properties,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Layers => "Katmanlar",
            Panel::Processing => "İşlemler",
            Panel::Blocks => "Bloklar",
            Panel::Templates => "Şablonlar",
            Panel::Sources => "Kaynaklar",
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
            Panel::Properties => Icon::Properties,
        }
    }

    fn layout() -> Docks<Panel> {
        let mut docks = Docks::new();
        docks.dock(Panel::Layers, Side::Right);
        // İşlemler, Bloklar, Şablonlar and Kaynaklar share the top slot with Katmanlar, as the web's tabs; Katmanlar in front.
        docks.dock(Panel::Processing, Side::Right);
        docks.dock(Panel::Blocks, Side::Right);
        docks.dock(Panel::Templates, Side::Right);
        docks.dock(Panel::Sources, Side::Right);
        docks.update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
        docks.split(Panel::Properties, Side::Right);
        docks.set_size(Side::Right, DOCK_WIDTH);
        docks
    }
}

/// What waits for an answer in the middle of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
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
    /// A Hesap window (calc/).
    Calc,
    /// A processing tool's or model's window (processing/).
    Processing,
    /// Katman stili (style/layer_style/); the window is `App::styles.layer_style`.
    LayerStyle,
    /// Stil yöneticisi (style/manager/); the window is `App::styles.manager`.
    StyleManager,
    /// Lejant (style/legend/); the window is `App::styles.legend`.
    Legend,
    /// Sembol tasarımcısı (style/designer/); the window is `App::styles.designer`.
    SymbolDesigner,
    /// Model tasarımcısı (processing/designer/); the window is `App::processing.designer`.
    ModelDesigner,
    /// SVG çizim düzenleyicisi (style/svgedit/); the window is `App::styles.svg_editor`.
    SvgEditor,
    /// Katmanlar → Sil on a layer or group with objects (layering.rs); the
    /// node is `App::removing_layer`.
    RemoveLayer,
    /// Şablonlar → Sil (templates_panel.rs); the template is the panel's `deleting`.
    RemoveTemplate,
    /// Kaynaktan oku over the layer's own edits (bound.rs); the layer is `App::bound_pending`.
    SourceDiscard,
    /// Kaynakla uzlaştır (source_reconcile.rs, docs/adr/0538); the window is `App::reconcile`.
    SourceReconcile,
    /// Şablon düzenleyici (template_editor.rs); its form is `App::template_editor`.
    TemplateEditor,
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
    /// Blok ekle's Öznitelik değerleri (attribute_values.rs); the window is `App::attribute_values`.
    AttributeValues,
    /// Blok kartı (block_card.rs, docs/adr/0317 §4); the window is `App::block_card`.
    BlockCard,
    /// Blok çizelgesi (block_schedule.rs, docs/adr/0317 §3); the window is `App::block_schedule`.
    BlockSchedule,
    /// Bul ve değiştir (find_replace.rs, docs/adr/0145 §6); the window is `App::find_replace`.
    FindReplace,
    /// Katmanları birleştir (layer_merge.rs, docs/adr/0177 §3); the window is `App::layer_merge`.
    LayerMerge,
    /// Katmanları çevir (layer_translate.rs, docs/adr/0311 §5); the window is `App::layer_translate`.
    LayerTranslate,
    /// Çizim standardı (drawing_standard.rs, docs/adr/0312 §7); the window is `App::drawing_standard`.
    DrawingStandard,
    /// Veri profili (data_profile.rs, docs/adr/0541); the window is `App::data_profile`.
    DataProfile,
    /// Alt ağlar (subnetworks_window.rs, docs/adr/0557); the window is `App::subnetworks_window`.
    Subnetworks,
    /// Kaynak durumu (source_status.rs, docs/adr/0543); the window is `App::source_status`.
    SourceStatus,
    /// Çizgi tipleri (line_types_window.rs, docs/adr/0313 §7); the window is `App::line_types_window`.
    LineTypes,
    /// Parametreler (parameters_window.rs, docs/adr/0338 §1); the window is `App::parameters_window`.
    Parameters,
    /// Tarama desenleri (hatch_patterns_window.rs, docs/adr/0331 §1); the window is `App::hatch_patterns_window`.
    HatchPatterns,
    /// Değişikliğin etkisi (style_impact.rs, docs/adr/0314 §4); the window is `App::style_impact`.
    StyleImpact,
    /// Alanlar (layer_fields.rs, docs/adr/0199 §3); the window is `App::layer_fields`.
    LayerFields,
    /// Katman süzgeci (layer_filters.rs, docs/adr/0211 §4); the window is `App::layer_filter`.
    LayerFilter,
    /// Alan hesapla and Yapıştır (features/write.rs, docs/adr/0507); the window is `App::features.write`.
    FieldsWrite,
    /// Etiketler (labelling/, docs/adr/0212 §4); the window is `App::labelling`.
    Labelling,
    /// Bağlantılar (services/connections.rs).
    Connections,
    /// Harita servisi (services/window.rs).
    ServiceAdd,
    /// Servisten veri al (services/feed_window.rs).
    Feed,
    /// Topoloji kuralları (topology/rules.rs, docs/adr/0202 §6); the window is `App::topology_rules`.
    TopologyRules,
    /// Ağlar (networks/window.rs, docs/adr/0209 §10); the window is `App::networks.window`.
    Networks,
    /// Adres verisi (address/data.rs, docs/adr/0229 §13); the window is `App::address.data`.
    AddressData,
    /// Adres sağlayıcıları (address/providers.rs, docs/adr/0229 §13); the window is `App::address.providers`.
    AddressProviders,
    /// Zaman ayarları (temporal/layer.rs, docs/adr/0210 §10); the window is `App::time.layer`.
    TimeLayer,
    /// Senaryo oluştur and Senaryoyu uygula's question (temporal/scenario.rs); the window is `App::time.scenario`.
    Scenario,
    /// Katman durumları (layer_states.rs, docs/adr/0177 §4); the window is `App::layer_states_window`.
    LayerStates,
    /// Görünümler (views.rs, docs/adr/0285 §1); the window is `App::views_window`.
    Views,
    /// Formlar (forms/designer.rs, docs/adr/0401); the window is `App::forms_window`.
    Forms,
    /// Formla düzenle (forms/entry.rs, docs/adr/0401); the window is `App::form_entry`.
    FormEntry,
    /// İlişkiler (forms/relations.rs, docs/adr/0405); the window is `App::relations_window`.
    Relations,
    /// Birleşik görünümler (joins_window.rs, docs/adr/0527); the window is `App::joins_window`.
    Joins,
    /// Olay bağları (event_links_window.rs, docs/adr/0569); the window is `App::event_links_window`.
    EventLinks,
    /// Kayıtları denetle (forms/audit.rs, docs/adr/0406); the window is `App::form_audit`.
    FormAudit,
    /// Koşullu biçim (forms/table_styles.rs, docs/adr/0413); the window is `App::table_styles`.
    TableStyles,
    /// Alt türler (forms/subtypes.rs, docs/adr/0422); the window is `App::subtypes`.
    Subtypes,
    /// Değer bileşimleri (forms/contingents.rs, docs/adr/0423); the window is `App::contingents`.
    Contingents,
    /// Kayıt kuralları (forms/layer_rules.rs, docs/adr/0438); the window is `App::layer_rules`.
    LayerRules,
    /// Değer listeleri (forms/domains.rs, docs/adr/0430); the window is `App::domains`.
    Domains,
    /// Yazı stilleri and Ölçü stilleri (annotation_styles.rs, docs/adr/0183 §5); the window is
    /// `App::annotation_styles`.
    AnnotationStyles,
    /// Duvar stilleri (wall_styles.rs, docs/adr/0339 §6); the window is `App::wall_styles`.
    WallStyles,
    /// Birleşimleri denetle (wall_joins.rs, docs/adr/0340 §6); the window is `App::wall_joins`.
    WallJoins,
    /// Etiketleri denetle (tags_check.rs, docs/adr/0344 §2); the window is `App::tags_check`.
    TagsCheck,
    /// Başvurular (viewmark_refs.rs, docs/adr/0345 §3); the window is `App::view_refs`.
    ViewRefs,
    /// Pafta seti (sheet_set.rs, docs/adr/0347 §1); the window is `App::sheet_set`.
    SheetSet,
    /// Dış başvurular (xrefs_window.rs, docs/adr/0348 §5); its state is `App::xrefs`.
    Xrefs,
    /// Dış başvuru ekle (xrefs_window.rs); its choices are `App::xrefs.attach`.
    XrefAttach,
    /// Metraj (quantities.rs, docs/adr/0342 §5); the window is `App::quantities`.
    Quantities,
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
    /// Veri karşılaştır (data_compare.rs, docs/adr/0179); the window is `App::data_compare`.
    DataCompare,
    /// Raster ekle (rasters/add.rs, docs/adr/0204 §8); the window is `App::rasters.add`.
    RasterAdd,
    /// Sanal mozaik (rasters/mosaic.rs, docs/adr/0517 §5); the window is `App::rasters.mosaic`.
    RasterMosaic,
    /// STAC'ta ara (rasters/stac.rs, docs/adr/0528 §7); the window is `App::rasters.stac`.
    RasterStac,
    /// Raster stili (rasters/look.rs, docs/adr/0204 §8); the window is `App::rasters.look`.
    RasterStyle,
    /// Mesh ekle and Raster ekle's NetCDF (rasters/multidim.rs, docs/adr/0243 §11); the window
    /// is `App::rasters.multidim`.
    Multidim,
    /// Nokta bulutu ekle (pointclouds/add.rs, docs/adr/0207 §9); the window is `App::clouds.add`.
    PointCloudAdd,
    /// Nokta bulutu stili (pointclouds/look.rs, docs/adr/0207 §9); the window is `App::clouds.look`.
    PointCloudStyle,
    /// Kayıtlı ölçüleri denetle (cogo.rs, docs/adr/0180); the window is `App::cogo`.
    Cogo,
    /// Tablo ekle (tables/insert.rs, docs/adr/0184); the window is `App::table_insert`.
    TableInsert,
    /// Tabloyu düzenle (tables/editor.rs, docs/adr/0184 §5); the window is `App::table_editor`.
    TableEditor,
    /// Nokta editörü's batch operations (points/batch_view.rs, docs/adr/0153 §5); the window is
    /// `App::points.batch`.
    PointBatch,
    /// Adres ekle and Eki düzenle (attachments.rs, docs/adr/0225 §2); the window is `App::attachment_form`.
    Attachment,
    /// The photo viewer (attachments.rs, docs/adr/0225 §2); the window is `App::photo_viewer`.
    PhotoViewer,
    /// Bilgi kartı ve eylemler (layer_info.rs, docs/adr/0225 §5); the window is `App::layer_info`.
    LayerInfo,
    /// Veri kümesi bilgisi (dataset_window.rs, docs/adr/0633); the window is `App::dataset_info`.
    DatasetInfo,
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
    /// The raster windows (Raster ekle, Raster stili) and the rasters' panel (rasters/).
    Rasters(crate::rasters::Event),
    /// The point cloud service made nodes, opened a file or an index moved
    /// on: the drawing draws again with them (docs/adr/0207 §6).
    CloudsReady,
    /// The map services brought tiles or read what they need, or one failed:
    /// the drawing draws again, the failure goes to the log (docs/adr/0208 §6).
    ServicesReady,
    /// The point cloud windows, XYZ sor's answers and the indexes' panel (pointclouds/).
    PointClouds(crate::pointclouds::Event),
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
    /// The Noktalar tab (points/, docs/adr/0153).
    Points(crate::points::Event),
    /// The Tablo tab, Öznitelik tablosu (features/, docs/adr/0199 §4).
    Features(crate::features::Event),
    /// The İstatistik tab (statistics/, docs/adr/0227 §3).
    Statistics(crate::statistics::Event),
    /// The Grafik tab (charts/, docs/adr/0227 §4).
    Charts(crate::charts::Event),
    /// A table written as a file (docs/adr/0223 §2): what was written, or why not.
    TableWritten(crate::table_write::Outcome),
    /// Alanlar, a layer's fields (layer_fields.rs, docs/adr/0199 §3).
    LayerFields(crate::layer_fields::Event),
    /// The Arama tab (search/, docs/adr/0178).
    Search(crate::search::Event),
    /// A call's buttons in Komut geçmişi (history/, docs/adr/0284).
    History(crate::history::Event),
    /// The Topoloji tab (topology/, docs/adr/0202 §5).
    Topology(crate::topology::Event),
    /// Topoloji kuralları (topology/rules.rs, docs/adr/0202 §6).
    TopologyRules(crate::topology::rules::Event),
    /// Ağ analizi: the networks' answers and Ağlar (networks/, docs/adr/0209).
    Networks(crate::networks::Event),
    /// Adres arama: the Adres tab, Konumdan adres's answers and the address windows (address/, docs/adr/0229).
    Address(crate::address::Event),
    /// Zaman sürgüsü, Zaman ayarları and the scenarios (temporal/, docs/adr/0210).
    Time(crate::temporal::Event),
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
    /// A frame of the status bar's message fading, or the moment it goes (message_log.rs).
    LogFrame,
    /// The bottom panel's log list scrolled: whether new lines follow to its end.
    LogScrolled(iced::widget::scrollable::Viewport),
    /// The kept layout is due to be written (layout.rs).
    LayoutSave,
    /// The NTv2 grids the project names, looked for in the device's library (grids.rs).
    GridsLooked(Vec<crate::grids::Looked>),
    /// The rows round a bound layer's part, read on a thread of their own (band_ghosts.rs, docs/adr/0578 §1).
    BandRead(crate::band_ghosts::Read),
    /// The window's new size: the kept sizes are shown within it.
    WindowResized(iced::Size),
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
    /// Şablonlar panel (templates_panel.rs).
    TemplatesPanel(crate::templates_panel::Event),
    /// Kaynaklar panel (sources/, docs/adr/0199 §7).
    Sources(crate::sources::Event),
    /// Veritabanı bağlantıları: the connection window, the password question, QGIS's file (db_connections.rs).
    Db(crate::db_connections::Event),
    /// Şablon düzenleyici (template_editor.rs).
    TemplateEditor(crate::template_editor::Event),
    /// Blok öznitelikleri's window (block_attributes.rs).
    BlockAttributes(crate::block_attributes::Event),
    /// Blok ekle's Öznitelik değerleri (attribute_values.rs).
    AttributeValues(crate::attribute_values::Event),
    BlockCard(crate::block_card::Event),
    BlockSchedule(crate::block_schedule::Event),
    FindReplace(crate::find_replace::Event),
    LayerMerge(crate::layer_merge::Event),
    LayerTranslate(crate::layer_translate::Event),
    /// Çizim standardı's window (drawing_standard.rs, docs/adr/0312 §7).
    DrawingStandard(crate::drawing_standard::Event),
    /// Veri profili's window (data_profile.rs, docs/adr/0541).
    DataProfile(crate::data_profile::Event),
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    Subnetworks(crate::subnetworks_window::Event),
    /// Kaynak durumu's window (source_status.rs, docs/adr/0543).
    SourceStatus(crate::source_status::Event),
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
    /// Katman süzgeci's window (layer_filters.rs, docs/adr/0211 §4).
    LayerFilter(crate::layer_filters::Event),
    /// An object's attachments: their windows, the viewer and the photos decoded (attachments.rs).
    Attachment(crate::attachments::Event),
    /// Bilgi's card over the drawing (info_card.rs, docs/adr/0225 §5).
    InfoCard(crate::info_card::Event),
    /// Bilgi kartı ve eylemler (layer_info.rs).
    LayerInfo(crate::layer_info::Event),
    /// Veri kümesi bilgisi (dataset_window.rs).
    DatasetInfo(crate::dataset_window::Event),
    /// Etiketler's window (labelling/, docs/adr/0212 §4).
    Labelling(crate::labelling::Event),
    /// Katman durumları's window and Katman durumları ▾ (layer_states.rs).
    LayerStates(crate::layer_states::Event),
    /// Görünümler's window (views.rs, docs/adr/0285 §1).
    Views(crate::views::Event),
    /// Formlar's window (forms/designer.rs, docs/adr/0401).
    Forms(crate::forms::designer::Event),
    /// An event of İlişkiler (forms/relations.rs, docs/adr/0405).
    Relations(crate::forms::relations::Event),
    /// An event of Birleşik görünümler (joins_window.rs, docs/adr/0527).
    Joins(crate::joins_window::Event),
    /// An event of Olay bağları (event_links_window.rs, docs/adr/0569).
    EventLinks(crate::event_links_window::Event),
    TableStyles(crate::forms::table_styles::Event),
    Subtypes(crate::forms::subtypes::Event),
    Contingents(crate::forms::contingents::Event),
    LayerRules(crate::forms::layer_rules::Event),
    Domains(crate::forms::domains::Event),
    /// An event of Kayıtları denetle (forms/audit.rs, docs/adr/0406).
    FormAudit(crate::forms::audit::Event),
    /// Formla düzenle (forms/entry.rs, docs/adr/0401).
    FormEntry(crate::forms::entry::Event),
    /// An event of Formlar's preview (forms/entry.rs, docs/adr/0403 §3).
    FormPreview(crate::forms::entry::Event),
    AnnotationStyles(crate::annotation_styles::Event),
    WallStyles(crate::wall_styles::Event),
    WallJoins(crate::wall_joins::Event),
    TagsCheck(crate::tags_check::Event),
    ViewRefs(crate::viewmark_refs::Event),
    SheetSet(crate::sheet_set::Event),
    Xrefs(crate::xrefs_window::Event),
    XrefWatch(crate::xrefs_watch::Event),
    Quantities(crate::quantities::Event),
    LayerPurge(crate::layer_purge::Event),
    LayerList(crate::layer_list::Event),
    FontReport(crate::font_report::Event),
    TextTools(crate::text_tools::Event),
    LeaderTools(crate::leader_tools::Event),
    /// Ölçek yaz…'s window (annotation_scale.rs, docs/adr/0205 §4).
    PlotScale(crate::annotation_scale::Event),
    /// Seçilenleri dosyaya kaydet, Başka çizimden al and Dosyadan blok ekle (drawing_exchange.rs).
    DrawingExchange(crate::drawing_exchange::Event),
    /// Veri karşılaştır's window (data_compare.rs).
    DataCompare(crate::data_compare::Event),
    /// Kayıtlı ölçüleri denetle's window (cogo.rs).
    Cogo(crate::cogo::Event),
    /// Tablo ekle's window (tables/insert.rs).
    TableInsert(crate::tables::insert::Event),
    /// Tabloyu düzenle's window (tables/editor.rs).
    TableEditor(crate::tables::editor::Event),
    /// Tabloyu güncelle's files (tables/update.rs).
    TableUpdate(crate::tables::update::Event),
    /// Genel bakış and Büyüteç over the drawing (navigation_cards.rs).
    Navigation(crate::navigation_cards::Event),
    /// The map services' credits and windows (services/app.rs, docs/adr/0208).
    Services(crate::services::app::Event),
    /// Bağlı katmanlar (bound.rs, docs/adr/0503).
    Bound(crate::bound::Event),
    /// Kaynakla uzlaştır's window (source_reconcile.rs, docs/adr/0538).
    Reconcile(crate::source_reconcile::Event),
    /// Metin dosyası yerleştir's file: its name and bytes, or none (text_file.rs).
    TextFile(Option<(String, Vec<u8>)>),
    /// Resim ekle's file: its name, path and bytes, or none (cancelled).
    ImageFile(Option<(String, Option<String>, Vec<u8>)>),
    /// The rollover card's wait is over, for this hover (hover_card.rs).
    HoverCard(u64),
    /// The pointer rested on a snap past the tracking dwell (tracking.rs).
    TrackDwell(u64),
    /// The Hesap windows (calc/).
    Calc(crate::calc::Event),
    /// A processing tool's window (processing/).
    Processing(crate::processing::Event),
    /// Model tasarımcısı (processing/designer/).
    ModelDesigner(crate::processing::designer::Event),
    /// Katman stili (style/layer_style/).
    LayerStyle(crate::style::layer_style::Event),
    /// İfade oluşturucu over an expression field's window (expression/).
    Builder(crate::expression::Event),
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
    /// Esc in the empty command line: the running command ends.
    CommandCancelled,
    /// The command line's text box took or let go of the keyboard.
    CommandFocus(bool),
    /// An option button of the running command's prompt (its key: `G`, `Enter`).
    PromptOption(&'static str),
    /// A value an option offers, chosen from its menu: the option's key and
    /// what typing the value gives (Yazı's Hiza, docs/adr/0145 §6).
    PromptChoice(&'static str, String),
    /// Nokta hesapla: a construction over the running command (docs/adr/0083).
    PointCalc(kentos_interaction::point_calc::CalcKind),
    /// A key press no text box captured (keys.rs); routed by ADR 0018.
    Key(KeyPress),
    /// Shift, Ctrl, Alt or the logo key changed (Shift turns ortho over for a click).
    Modifiers(keyboard::Modifiers),
    Dock(docking::Event<Panel>),
    LayerSelected(String),
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
    /// İkinci sistem: the project's second coordinate system, how geographic
    /// values are written (second_crs.rs, docs/adr/0167 §1).
    SecondCrs(crate::second_crs::Event),
    /// Harita sistemi chosen (docs/adr/0583 §3).
    MapCrs(crate::map_crs::Choose),
    /// A layer's source system (docs/adr/0583 §1–§2).
    LayerCrs(crate::layer_crs::Event),
    /// A notice closed, by its ×, its time or Tümünü kapat (toasts.rs).
    ToastClosed(kentos_ui::widget::toast::Id),
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
    /// The Şablonlar panel (templates_panel.rs), and the templates drawn with
    /// in this session, the newest first (templates.rs).
    pub(crate) templates_panel: crate::templates_panel::PanelState,
    /// Kaynaklar (sources/, docs/adr/0199 §7): its folders, what is open and read.
    pub(crate) sources: crate::sources::SourcesPanel,
    /// The connection window while it is open (db_connections.rs, docs/adr/0623).
    pub(crate) db_editor: Option<crate::db_connections::Editor>,
    /// The password question while a server waits for one.
    pub(crate) db_password: Option<crate::db_connections::PasswordAsk>,
    /// Sil's question for a connection, by its name.
    pub(crate) db_removing: Option<String>,
    /// The database connections this window lists: the device's (`kentos_postgis::connections::store`), whose saved
    /// passwords the bound layers' commands find; a test's app keeps its own in memory.
    pub(crate) db_store: &'static kentos_postgis::connections::ConnectionStore,
    /// The connection PostGIS'e bağlan listed last, chosen when it opens again.
    pub(crate) postgis_last: Option<String>,
    pub(crate) recent_templates: Vec<String>,
    /// Şablon düzenleyici while it is open (template_editor.rs).
    pub(crate) template_editor: Option<crate::template_editor::Editor>,
    /// Blok öznitelikleri's window, while it is open or away for a point (block_attributes.rs).
    pub(crate) block_attributes: Option<crate::block_attributes::Window>,
    /// Blok ekle's Öznitelik değerleri, while it asks (attribute_values.rs).
    pub(crate) attribute_values: Option<crate::attribute_values::Window>,
    pub(crate) block_card: Option<crate::block_card::Window>,
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
    /// Raster ekle and Raster stili (rasters/, docs/adr/0204 §8).
    pub(crate) rasters: crate::rasters::Windows,
    /// XYZ sor's places and reaches whose clouds are to be read (docs/adr/0207 §7).
    pub(crate) cloud_query_wanted: Vec<(kentos_interaction::Vec2, f64)>,
    /// The network tools' questions, asked after the update (networks/, docs/adr/0209 §12).
    pub(crate) network_wanted: Vec<kentos_interaction::network::NetworkAsk>,
    /// The project's networks as built on the network thread, and Ağlar's window.
    pub(crate) networks: crate::networks::State,
    /// Adres arama: the Adres tab, its locators on the address thread and the address windows (address/).
    pub(crate) address: crate::address::State,
    /// Zaman sürgüsü and the windows of Zaman and Senaryo (temporal/, docs/adr/0210).
    pub(crate) time: crate::temporal::State,
    /// Commands a tool asked the host to run (Ağlar from the network tools).
    pub(crate) command_wanted: Vec<&'static str>,
    /// Nokta bulutu ekle and Nokta bulutu stili (pointclouds/, docs/adr/0207 §9).
    pub(crate) clouds: crate::pointclouds::Windows,
    /// Resim ekle asked for a picture file (docs/adr/0192 §5).
    pub(crate) image_file_wanted: bool,
    pub(crate) text_field_select: bool,
    pub(crate) text_field_release: bool,
    /// Öznitelikler's closed sections, by id, while the app runs (the web's `collapsed`).
    pub(crate) props_closed: std::collections::HashSet<&'static str>,
    /// Öznitelikler for a selection of several objects, as last worked out (properties/).
    pub(crate) properties_cache: crate::properties::PanelCache,
    /// The bottom panel's Noktalar tab: its query and rows (points/, docs/adr/0153).
    pub(crate) points: crate::points::PointsPanel,
    /// The bottom panel's Tablo tab: its choices and rows (features/, docs/adr/0199 §4).
    pub(crate) features: crate::features::FeaturesPanel,
    /// The İstatistik and Grafik tabs: their choices and what was worked out (charts/, docs/adr/0227).
    pub(crate) charts: crate::charts::ChartsPanel,
    /// Alanlar, while it is open (layer_fields.rs, docs/adr/0199 §3).
    pub(crate) layer_fields: Option<crate::layer_fields::Window>,
    /// The bottom panel's Arama tab: its choices and rows, and the place it marked (search/, docs/adr/0178).
    pub(crate) search: crate::search::SearchPanel,
    /// The bottom panel's Topoloji tab: the last check, the chosen rows and the filter (topology/, docs/adr/0202 §5).
    pub(crate) topology: crate::topology::TopologyPanel,
    /// Topoloji kuralları, while it is open (topology/rules.rs, docs/adr/0202 §6).
    pub(crate) topology_rules: Option<crate::topology::rules::Window>,
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
    /// The device's NTv2 grid library and the grids looked for (grids.rs, docs/adr/0168 §4).
    pub(crate) grids: crate::grids::Grids,
    /// The rows round the bound layers' parts, asked for and arrived (band_ghosts.rs, docs/adr/0578 §1).
    pub(crate) bands: crate::band_ghosts::Bands,
    pub(crate) window_size: iced::Size,
    pub command_input: String,
    pub command_expanded: bool,
    /// The bottom panel's tab (bottom.rs).
    pub bottom_tab: crate::bottom::BottomTab,
    /// The Python console, the bottom panel's Python tab (python/, docs/adr/0132).
    pub(crate) python: crate::python::Console,
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
    /// The Hesap windows' fields, kept while the app runs (calc/).
    pub calc: crate::calc::Calc,
    /// Nesne izleme: the points acquired by resting on a snap and the cursor's lock (tracking.rs, docs/adr/0085).
    pub tracking: kentos_interaction::object_tracking::ObjectTracking,
    /// Seçili katmanlarda önle's layers, by id (docs/adr/0162 §1): the
    /// session's, not a setting; the tools see them in their context.
    pub(crate) overlap_layers: Vec<String>,
    /// Seçim süzgeci's kinds, as `kentos_interaction::selectable::bit`s: the
    /// session's, every kind at first (docs/adr/0187 §5).
    pub(crate) select_kinds: u32,
    /// The digitizing locks (docs/adr/0166): what holds the next point; the
    /// session's, the tools see them in their context.
    pub locks: kentos_interaction::LockState,
    /// The UCSs Önceki gives back (ucs.rs, docs/adr/0261 §1).
    pub(crate) ucs_history: crate::ucs::UcsHistory,
    /// The object template being drawn with (templates.rs, docs/adr/0176 §3):
    /// its stamp is in the tools' context while its tool runs.
    pub(crate) template: Option<crate::templates::TemplateRun>,
    /// The last template started, for Son komutu yinele; none once another
    /// command is.
    pub(crate) last_template: Option<String>,
    /// Each point template's next name, or a text template's next text, by
    /// its id: its series goes on from run to run in the drawing (docs/adr/0176
    /// §3b), and a group's members take theirs from it (§5).
    pub(crate) template_names: std::collections::HashMap<String, String>,
    /// Katmanları birleştir's window (layer_merge.rs, docs/adr/0177 §3).
    pub(crate) layer_merge: Option<crate::layer_merge::Window>,
    /// Katmanları çevir's window (layer_translate.rs, docs/adr/0311 §5).
    pub(crate) layer_translate: Option<crate::layer_translate::Window>,
    /// Çizim standardı's window (drawing_standard.rs, docs/adr/0312 §7).
    pub(crate) drawing_standard: Option<crate::drawing_standard::Window>,
    /// Veri profili's window (data_profile.rs, docs/adr/0541).
    pub(crate) data_profile: Option<crate::data_profile::Window>,
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    pub(crate) subnetworks_window: Option<crate::subnetworks_window::Window>,
    /// Kaynak durumu's window (source_status.rs, docs/adr/0543).
    pub(crate) source_status: Option<crate::source_status::Window>,
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
    /// Katman süzgeci's window (layer_filters.rs, docs/adr/0211 §4).
    pub(crate) layer_filter: Option<crate::layer_filters::Window>,
    /// Bilgi's card: the object it shows and the drawing's point it was clicked at (info_card.rs).
    pub(crate) info_card: Option<(kentos_domain::Slot, kentos_interaction::Vec2)>,
    /// The attachments' photos decoded for the shell, each once (photo_view.rs).
    pub(crate) photos: crate::photo_view::Photos,
    /// The photo viewer (attachments.rs).
    pub(crate) photo_viewer: Option<crate::attachments::Viewer>,
    /// Adres ekle and Eki düzenle (attachments.rs).
    pub(crate) attachment_form: Option<crate::attachments::Form>,
    /// Bilgi kartı ve eylemler (layer_info.rs).
    pub(crate) layer_info: Option<crate::layer_info::Window>,
    /// Veri kümesi bilgisi (dataset_window.rs).
    pub(crate) dataset_info: Option<crate::dataset_window::Window>,
    /// Etiketler's window (labelling/, docs/adr/0212 §4).
    pub(crate) labelling: Option<crate::labelling::Window>,
    /// Katman durumları's window (layer_states.rs, docs/adr/0177 §4).
    pub(crate) layer_states_window: Option<crate::layer_states::Window>,
    /// Görünümler's window (views.rs, docs/adr/0285 §1).
    pub(crate) views_window: Option<crate::views::Window>,
    /// Formlar's window (forms/designer.rs, docs/adr/0401).
    pub(crate) forms_window: Option<crate::forms::designer::Window>,
    /// The designer's copied item with its layer's fields (docs/adr/0437): pasted into this or another form.
    /// The designer's section library and kept drafts on this device (forms/library.rs, docs/adr/0441).
    pub(crate) form_files: crate::forms::library::FormFiles,
    pub(crate) form_clipboard:
        Option<(kentos_commands::FormItem, Vec<kentos_commands::LayerField>)>,
    /// Formla düzenle (forms/entry.rs, docs/adr/0401).
    pub(crate) form_entry: Option<crate::forms::entry::Entry>,
    /// The records whose forms wait under a child's (docs/adr/0440 §7): their drafts kept, the
    /// nearest last.
    pub(crate) form_stack: Vec<crate::forms::entry::Entry>,
    /// İlişkiler (forms/relations.rs, docs/adr/0405).
    pub(crate) relations_window: Option<crate::forms::relations::Window>,
    /// Birleşik görünümler (joins_window.rs, docs/adr/0527).
    pub(crate) joins_window: Option<crate::joins_window::Window>,
    /// Olay bağları (event_links_window.rs, docs/adr/0569).
    pub(crate) event_links_window: Option<crate::event_links_window::Window>,
    pub(crate) table_styles: Option<crate::forms::table_styles::Window>,
    pub(crate) subtypes: Option<crate::forms::subtypes::Window>,
    pub(crate) contingents: Option<crate::forms::contingents::Window>,
    /// Kayıt kuralları (docs/adr/0438).
    pub(crate) layer_rules: Option<crate::forms::layer_rules::Window>,
    pub(crate) domains: Option<crate::forms::domains::Window>,
    /// Kayıtları denetle (forms/audit.rs, docs/adr/0406).
    pub(crate) form_audit: Option<crate::forms::audit::Window>,
    /// What the session remembers of the forms: the "reuse last" values (forms/entry.rs, docs/adr/0402 §7).
    pub(crate) form_memory: crate::forms::entry::Memory,
    /// Formlar's preview of its copy (forms/entry.rs, docs/adr/0403 §3).
    pub(crate) form_preview: Option<crate::forms::entry::Entry>,
    /// The object a drawing tool has just written: its form opens (forms/mod.rs, docs/adr/0403 §4).
    pub(crate) form_on_create: Option<kentos_domain::Slot>,
    /// Yazı stilleri or Ölçü stilleri (annotation_styles.rs, docs/adr/0183 §5).
    pub(crate) annotation_styles: Option<crate::annotation_styles::Window>,
    /// Duvar stilleri (wall_styles.rs, docs/adr/0339 §6).
    pub(crate) wall_styles: Option<crate::wall_styles::Window>,
    /// Birleşimleri denetle's window (wall_joins.rs, docs/adr/0340 §6).
    pub(crate) wall_joins: Option<crate::wall_joins::Window>,
    /// Etiketleri denetle's window (tags_check.rs, docs/adr/0344 §2).
    pub(crate) tags_check: Option<crate::tags_check::Window>,
    /// Başvurular' window (viewmark_refs.rs, docs/adr/0345 §3).
    pub(crate) view_refs: Option<crate::viewmark_refs::Window>,
    pub(crate) sheet_set: Option<crate::sheet_set::Window>,
    /// Dış başvurular and Dış başvuru ekle (xrefs_window.rs, docs/adr/0348 §5).
    pub(crate) xrefs: crate::xrefs_window::State,
    /// The loaded references' files followed, and the card of a changed one (xrefs_watch.rs, docs/adr/0349 §3).
    pub(crate) xref_watch: crate::xrefs_watch::Watch,
    /// Metraj's window (quantities.rs, docs/adr/0342 §5).
    pub(crate) quantities: Option<crate::quantities::Window>,
    /// Kullanılmayanları temizle's window (layer_purge.rs, docs/adr/0177 §5).
    pub(crate) layer_purge: Option<crate::layer_purge::Window>,
    /// Başka çizimden al's window (drawing_exchange.rs, docs/adr/0193 §2).
    pub(crate) take_from: Option<crate::drawing_exchange::Window>,
    /// Katman listesi's window (layer_list.rs, docs/adr/0177 §6).
    pub(crate) layer_list: Option<crate::layer_list::Window>,
    /// Ölçek yaz…'s typed scale while its window is open (annotation_scale.rs, docs/adr/0205 §4).
    pub(crate) plot_scale_field: Option<String>,
    /// Veri karşılaştır's window (data_compare.rs, docs/adr/0179).
    pub(crate) data_compare: Option<crate::data_compare::Window>,
    /// Kayıtlı ölçüleri denetle's window (cogo.rs, docs/adr/0180).
    pub(crate) cogo: Option<crate::cogo::Window>,
    /// Tablo ekle's choices, kept while the app runs (tables/insert.rs, docs/adr/0184).
    pub(crate) table_insert: crate::tables::insert::Insert,
    /// Tabloyu düzenle's window (tables/editor.rs).
    pub(crate) table_editor: Option<crate::tables::editor::Editor>,
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
    /// İşlemler: the processing tools and models, their window and history (processing/).
    pub processing: crate::processing::Processing,
    /// İfade oluşturucu, while it is open over the window that asked for it (expression/).
    pub(crate) builder: Option<crate::expression::Builder>,
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
    /// Kaynakla uzlaştır's window (source_reconcile.rs, docs/adr/0538).
    pub(crate) reconcile: Option<crate::source_reconcile::Window>,
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
    /// The sheet templates' cloud library (sheet_library.rs, docs/sheet/design.md §13).
    pub(crate) sheet_library: crate::sheet_library::SheetLibrary,
    /// What the sheet mode's tables, coordinate lists and legends were last given for (sheet_inputs.rs).
    pub(crate) sheet_data_key: Option<u64>,
    /// The window's notices in its corner (toasts.rs): a save that closed its window, a copy.
    pub(crate) toasts: kentos_ui::widget::Toasts<Message>,
}

impl App {
    /// The shell with settings in memory, opening `path` at once when given:
    /// what tests, snapshots and the trace player start from, never touching
    /// the user's files. `main` opens the real settings ([`App::start`]).
    pub fn boot(path: Option<PathBuf>) -> (Self, Task<Message>) {
        // The database connections too stay in memory (docs/adr/0623), each app its own.
        kentos_postgis::connections::keep_in_memory();
        let (mut app, task) = Self::start(path, Settings::memory());
        app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
        (app, task)
    }

    /// The shell with these settings, opening `path` at once when given (`kentos-cad cizim.kcad`).
    pub fn start(path: Option<PathBuf>, settings: Settings) -> (Self, Task<Message>) {
        Self::start_with(path, settings, Recovery::off())
    }

    /// `start`, keeping recovery copies in `recovery`: work a crash left is offered first.
    pub fn start_with(
        path: Option<PathBuf>,
        settings: Settings,
        recovery: Recovery,
    ) -> (Self, Task<Message>) {
        // The drawing's typefaces before the first frame (docs/adr/0055).
        crate::drawing_fonts::load();
        let mut app = Self {
            document: None,
            tab: catalog()
                .tabs()
                .nth(1)
                .or(catalog().tabs().next())
                .map_or("home", |tab| tab.id),
            ribbon_context: false,
            selection_tab_cache: std::cell::Cell::new(None),
            ribbon_follows: None,
            ribbon_collapsed: false,
            mode: Mode::Dark,
            accent: Accent::default(),
            docks: Panel::layout(),
            selected_layer: None,
            selection_layers: Vec::new(),
            layers_follow: false,
            followed_selection: 0,
            renaming: None,
            last_layer_press: None,
            layer_query: String::new(),
            layer_props: Default::default(),
            ribbon_search: String::new(),
            ribbon_flash: None,
            ribbon_flash_model: None,
            layers_keyboard: false,
            layer_reveal: None,
            drawing_menu: None,
            snap_once: None,
            text_field: None,
            text_field_focus: false,
            paragraph: None,
            paragraph_focus: false,
            blocks: crate::blocks::Blocks::default(),
            blocks_panel: crate::blocks_panel::PanelState::default(),
            templates_panel: crate::templates_panel::PanelState::default(),
            sources: crate::sources::SourcesPanel::default(),
            db_editor: None,
            db_password: None,
            db_removing: None,
            db_store: kentos_postgis::connections::store(),
            postgis_last: None,
            recent_templates: Vec::new(),
            template_editor: None,
            block_attributes: None,
            attribute_values: None,
            block_card: None,
            block_schedule: None,
            find_replace: None,
            text_tools: Default::default(),
            leader_tools: Default::default(),
            text_file_wanted: false,
            raster_values_wanted: Vec::new(),
            rasters: crate::rasters::Windows::default(),
            cloud_query_wanted: Vec::new(),
            network_wanted: Vec::new(),
            networks: crate::networks::State::default(),
            address: crate::address::State::default(),
            time: crate::temporal::State::default(),
            command_wanted: Vec::new(),
            clouds: crate::pointclouds::Windows::default(),
            image_file_wanted: false,
            text_field_select: false,
            text_field_release: false,
            props_closed: std::collections::HashSet::new(),
            properties_cache: Default::default(),
            points: Default::default(),
            features: Default::default(),
            charts: Default::default(),
            layer_fields: None,
            search: Default::default(),
            topology: Default::default(),
            topology_rules: None,
            vertices: Default::default(),
            label_spots: Default::default(),
            split: None,
            service_marks: Default::default(),
            service_credits: false,
            connections: None,
            service_window: None,
            feed_window: None,
            service_info: Default::default(),
            last_click: None,
            log: {
                let mut log = crate::log_plan::Log::default();
                log.push(
                    Level::Info,
                    "KentOS CAD masaüstü hazır. Web'deki bütün komutlar şeritte; masaüstüne taşınmayanlar bunu söyler.",
                    crate::cloud::now_ms(),
                );
                log
            },
            history: Default::default(),
            typed: Vec::new(),
            follow: crate::message_log::Follow::default(),
            layout: crate::layout::Keeper::memory(),
            grids: crate::grids::Grids::default(),
            bands: crate::band_ghosts::Bands::default(),
            window_size: iced::Size::new(1440.0, 900.0),
            command_input: String::new(),
            command_expanded: false,
            bottom_tab: crate::bottom::BottomTab::default(),
            python: crate::python::Console::default(),
            bottom_log: None,
            dialog: None,
            dialog_under: None,
            shortcuts_query: String::new(),
            viewport: Viewport::new(),
            sheets: kentos_sheet_ui::Designer::default(),
            sheet_maps: crate::sheets::SheetMaps::default(),
            sheet_tab: false,
            sheet_store: None,
            sheet_book: crate::sheet_book::BookLink::default(),
            sheet_session: None,
            sheet_generation: None,
            sheet_attributes: std::cell::Cell::new(false),
            sheet_extent: std::cell::Cell::new(None),
            sheet_library: crate::sheet_library::SheetLibrary::default(),
            sheet_data_key: None,
            toasts: kentos_ui::widget::Toasts::new(),
            session: Session::new(),
            spatial: Spatial::new(),
            styles: crate::style::Styles::new(),
            memory: Memory::default(),
            selection: Selection::new(),
            clipboard: Clipboard::new(),
            snap: None,
            snap_cycle: None,
            cursor_point: None,
            followed: None,
            field: None,
            draft: Draft::default(),
            cursor_input: true,
            command_bar: false,
            hover_info: true,
            crosshair: crate::marks::CrosshairSize::default(),
            panning: false,
            view_history: kentos_interaction::ViewHistory::new(),
            pan_recorded: false,
            scale_field: None,
            view_clock: None,
            hover_card: None,
            hover_seen: 0,
            calc: crate::calc::Calc::default(),
            tracking: kentos_interaction::object_tracking::ObjectTracking::new(),
            overlap_layers: Vec::new(),
            select_kinds: kentos_interaction::selectable::ALL,
            locks: kentos_interaction::LockState::default(),
            ucs_history: crate::ucs::UcsHistory::default(),
            template: None,
            last_template: None,
            template_names: std::collections::HashMap::new(),
            layer_merge: None,
            layer_translate: None,
            drawing_standard: None,
            data_profile: None,
            subnetworks_window: None,
            source_status: None,
            line_types_window: None,
            parameters_window: None,
            constraint_view: Default::default(),
            hatch_patterns_window: None,
            style_impact: None,
            block_editor: None,
            doc_tabs: Default::default(),
            clip_frames: Default::default(),
            layer_filter: None,
            info_card: None,
            photos: crate::photo_view::Photos::default(),
            photo_viewer: None,
            attachment_form: None,
            layer_info: None,
            dataset_info: None,
            labelling: None,
            layer_states_window: None,
            views_window: None,
            forms_window: None,
            form_clipboard: None,
            form_files: crate::forms::library::FormFiles::default(),
            form_entry: None,
            form_stack: Vec::new(),
            relations_window: None,
            joins_window: None,
            event_links_window: None,
            table_styles: None,
            subtypes: None,
            contingents: None,
            layer_rules: None,
            domains: None,
            form_audit: None,
            form_memory: Default::default(),
            form_preview: None,
            form_on_create: None,
            annotation_styles: None,
            wall_styles: None,
            wall_joins: None,
            tags_check: None,
            view_refs: None,
            sheet_set: None,
            xrefs: Default::default(),
            xref_watch: Default::default(),
            quantities: None,
            reconcile: None,
            layer_purge: None,
            take_from: None,
            layer_list: None,
            plot_scale_field: None,
            data_compare: None,
            cogo: None,
            table_insert: Default::default(),
            table_editor: None,
            table_update: None,
            navigation: Default::default(),
            lens_spots: Default::default(),
            field_send_format: 0,
            overlap_last: kentos_interaction::Overlap::Layer,
            tracking_tool: "",
            tracking_waited: 0,
            dwell_on_time: true,
            processing: crate::processing::Processing::default(),
            builder: None,
            builder_flow: false,
            builder_recent: Vec::new(),
            key_tips: None,
            alt_armed: false,
            ribbon_peek: false,
            removing_layer: None,
            bound_pending: None,
            settings,
            settings_draft: None,
            reported_failure: None,
            line_focused: false,
            modifiers: keyboard::Modifiers::default(),
            last_level: None,
            picker: Picker::Dialog,
            opening: None,
            importing: None,
            saving: None,
            save_faults: saving::Faults::NONE,
            recovery,
            cloud: CloudState::default(),
            exchange: None,
            project: None,
            app_menu: None,
            recent: crate::recent::RecentFiles::memory(),
            opening_recent: None,
            backdrop: crate::appearance::Backdrop::default(),
            typography: kentos_ui::theme::typography::current(),
            shape: kentos_ui::theme::shape::current(),
            fullscreen: false,
            hidden_docks: None,
            server_checking: false,
            server_quiet: false,
            server_health: None,
        };
        if !app.recovery.offers.is_empty() {
            app.dialog = Some(Dialog::Recovery);
        }
        if let Some(why) = app.recovery.why_unavailable().map(str::to_owned) {
            app.warn(format!(
                "Kaydedilmemiş çalışmanın kurtarma kopyaları tutulamıyor ({why}); KentOS çökerse kaydedilmemiş değişiklikler kaybolur. Veri klasörünün yazılabilir olduğunu denetleyin, o zamana dek sık kaydedin."
            ));
        }
        // The organisation's policy: no server sends one yet; a local file may stand in (docs/adr/0023).
        match Settings::policy_from_env() {
            Some(Ok(policy)) => {
                app.settings.set_policy(policy);
                app.output("Kurum politikası KENTOS_SETTINGS_POLICY dosyasından okundu (yerel deneme).");
            }
            Some(Err(error)) => app.warn(format!(
                "Kurum politikası okunamadı: {error}. Dosyayı denetleyin ya da KENTOS_SETTINGS_POLICY'yi kaldırın."
            )),
            None => {}
        }
        app.apply_settings();
        app.report_settings_open();
        // The layout's defaults until the kept one is read (main.rs, layout.rs).
        app.apply_layout();
        let task = match path {
            Some(path) => app.start_opening(path, Purpose::File),
            None => Task::none(),
        };
        (app, task)
    }

    pub fn title(&self) -> String {
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

    pub fn subscription(&self) -> Subscription<Message> {
        // While the drawing has unsaved changes, the app looks every few seconds whether a
        // recovery copy is due; a database project's unsent work goes to its device draft instead.
        let unsaved = self.recovery.on()
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.dirty() && !d.is_database());
        Subscription::batch([
            event::listen_with(keys::key_event),
            window::close_requests().map(Message::CloseRequested),
            if unsaved {
                Subscription::run(recovery::ticks)
            } else {
                Subscription::none()
            },
            if self.sheet_publish_running(){Subscription::run(crate::exchange::sheet_publish::ticks)}else{Subscription::none()},
            // Toplu çalıştır: a running batch job is looked at every tick (processing/batch.rs).
            if self.processing.batch_running() {
                Subscription::run(crate::processing::batch::ticks)
            } else {
                Subscription::none()
            },
            // A file dropped on the window goes to the SVG editor while it is open (style/svgedit/), else to
            // Kaynaklar, which adds it by its kind (sources/, docs/adr/0599 §8).
            if self.styles.svg_editor.is_some() {
                event::listen_with(crate::style::svgedit::dropped)
            } else {
                event::listen_with(crate::sources::dropped)
            },
            // A Kaynaklar row being dragged: the pointer and the button let go, wherever they are.
            if self.sources.drag.is_some() {
                event::listen_with(crate::sources::dragging)
            } else {
                Subscription::none()
            },
            // The cloud's timers: autosave, the draft, following, the catalog's search.
            if self.cloud.wants_ticks() {
                Subscription::run(cloud::ticks)
            } else {
                Subscription::none()
            },
            // The sheet templates' library: its long poll's next wait, a retry, a run after events.
            if self.sheet_library.wants_ticks() {
                Subscription::run(crate::sheet_library::ticks)
            } else {
                Subscription::none()
            },
            // The rasters' tiles as they are made (rasters/tiles.rs, docs/adr/0204 §5).
            if crate::rasters::tiles::in_use() {
                Subscription::run(crate::rasters::tiles::ready)
            } else {
                Subscription::none()
            },
            // The point clouds' nodes as they are made (pointclouds/service.rs, docs/adr/0207 §6).
            if crate::pointclouds::service::in_use() {
                Subscription::run(crate::pointclouds::ready)
            } else {
                Subscription::none()
            },
            // The map services' tiles as they come (services/, docs/adr/0208 §6).
            if crate::services::in_use() {
                Subscription::run(crate::services::ready)
            } else {
                Subscription::none()
            },
            // The loaded external references' files, looked at every few seconds (xrefs_watch.rs, docs/adr/0349 §3).
            if self.xrefs_watching() {
                Subscription::run(crate::xrefs_watch::ticks)
            } else {
                Subscription::none()
            },
            // Zaman sürgüsü's playback (temporal/, docs/adr/0210 §5): a tick is handled after the
            // drawing showed the last step; the step itself waits for the speed.
            if self.time.slider.playing {
                Subscription::run(crate::temporal::ticks)
            } else {
                Subscription::none()
            },
            // The status bar's message: when it goes, and its fades (message_log.rs).
            self.log_subscription(Instant::now()),
            // The kept layout, written after its last change; the window's size.
            self.layout_subscription(),
            // Genel bakış's picture, drawn again a moment after a change (navigation_cards.rs).
            self.navigation_subscription(),
            window::resize_events().map(|(_, size)| Message::WindowResized(size)),
            // A large import writes a slice of its objects each frame (exchange/drawing_import.rs).
            if self.importing.is_some() {
                window::frames().map(|_| {
                    Message::Exchange(Box::new(crate::exchange::Event::DrawingImport(
                        crate::exchange::drawing_import::Event::Frame,
                    )))
                })
            } else {
                Subscription::none()
            },
            // İstatistik's summaries, a few fields a frame (statistics/, docs/adr/0227 §11).
            if self.stats_counting() {
                window::frames().map(|_| Message::Statistics(crate::statistics::Event::Count))
            } else {
                Subscription::none()
            },
            // While the key tips show (or Alt is down alone): a click or the
            // window losing the focus sends them away (ribbon_keys.rs).
            if self.key_tips.is_some() || self.alt_armed {
                event::listen_with(crate::ribbon_keys::away_events)
            } else {
                Subscription::none()
            },
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // A lock that changes with the pointer still shows at once (locks.rs);
        // the drawing area's own events move the pointer themselves.
        let locks = (!matches!(message, Message::Viewport(_)))
            .then_some((self.locks.length, self.locks.toward));
        let held = self.window_over();
        let view = self.viewpoint();
        let task = self.handle(message);
        // A template's run ends with its tool (templates.rs).
        self.follow_template();
        // The running command's preview follows a view change and a tool run over it (navigation.rs, docs/adr/0283).
        self.follow_view(view);
        // Another drawing goes back to one view (split_view.rs).
        self.follow_split();
        if locks.is_some_and(|l| l != (self.locks.length, self.locks.toward)) {
            self.repoint();
        }
        self.follow_document();
        // Önceki's history follows the drawing's UCS (ucs.rs, docs/adr/0261 §1).
        self.follow_ucs();
        // The sheets follow the project and the drawing (sheets.rs).
        self.follow_sheets();
        self.follow_selection_layers();
        // Follow selection before rebuilding keyboard targets for the visible tab.
        self.follow_selection_tab();
        // The key tips follow what the ribbon shows now (ribbon_keys.rs).
        self.refresh_key_tips();
        self.dialog_back();
        let task = Task::batch([
            task,
            self.text_field_tasks(),
            self.paragraph_tasks(),
            self.blocks_tasks(),
            self.template_editor_tasks(),
            self.attribute_values_tasks(),
            self.find_replace_tasks(),
            self.text_file_tasks(),
            self.image_file_tasks(),
            // A drawn object's form (forms/mod.rs, docs/adr/0403 §4).
            self.open_form_on_create(),
            // The attachments' photos the shell shows, decoded off the thread (attachments.rs).
            self.photo_tasks(),
            self.raster_values_tasks(),
            self.service_info_tasks(),
            self.cloud_query_tasks(),
            self.network_tasks(),
            // Konumdan adres's places, asked on the address thread or of a provider (address/).
            self.address_tasks(),
            self.command_tasks(),
            self.follow_hover(),
            self.follow_tracking(),
            // The grids the project's datum choices name, read into the core (grids.rs).
            self.follow_grids(),
            // The rows round a bound layer's part, the obstacles of its labels at the part's edge (band_ghosts.rs).
            self.follow_bands(),
            // The drawing's new calls join Komut geçmişi before the log is followed (history/).
            {
                self.follow_history();
                self.follow_log(Instant::now())
            },
            // The operations and selections as code in the Python console when asked (python/echo.rs).
            self.follow_python_echo(),
            // The sheet templates' library follows the sign-in and the connection (sheet_library.rs).
            self.follow_sheet_library(Instant::now()),
        ]);
        self.follow_layout(Instant::now());
        self.follow_navigation(Instant::now());
        // İstatistik and Grafik keep the layer they read (charts/).
        self.follow_charts();
        // Bilgi's card goes with its tool and its object (info_card.rs).
        self.follow_info_card();
        // Google's geocoding answers only beside a shown Google basemap (address/online.rs).
        self.follow_address();
        self.cloud_after(Instant::now());
        // A window that opens over the drawing has the keyboard: a text box under
        // it (the command line that opened it) lets go first, before the window's
        // own field takes it. The view keeps the window's widgets as they were
        // when something comes over them (view.rs), so nothing else would.
        if !held && self.window_over() {
            self.line_focused = false;
            return release_keyboard().chain(task);
        }
        task
    }

    /// Whether a window is open over the drawing: a dialog, the application
    /// menu or the sheet templates' gallery.
    fn window_over(&self) -> bool {
        self.dialog.is_some() || self.app_menu.is_some() || self.sheets.gallery_open()
    }

    /// After every message: the geometry store takes the drawing's changes
    /// and the selection lets go of objects that are gone (an undo, a
    /// delete, another editor's deletion taken in from the cloud; the web's
    /// `selection.retain` on `changed`), only when the drawing changed
    /// (docs/adr/0029). It keys on the generation, which changes from outside
    /// move too; the revision is the saves' (docs/adr/0040). The snap marker
    /// belongs to a running tool.
    fn follow_document(&mut self) {
        if !self.session.is_running() {
            self.snap = None;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let now = (doc.session, doc.model.generation());
        if self.followed == Some(now) {
            return;
        }
        self.followed = Some(now);
        self.spatial.sync(&doc.model);
        self.styles.follow_project(Some(doc.model.styles()));
        let model = &doc.model;
        self.selection.retain(|slot| model.get(slot).is_some());
        // The slider's range follows the drawing (docs/adr/0210 §5).
        self.time_refresh();
        // A new object its layer's filter leaves out is said (docs/adr/0211 §3).
        self.say_hidden_new();
        self.follow_map();
    }

    /// The drawing area draws in the project's map system (docs/adr/0583 §3): vectors moved in the style engine's
    /// sink, pictures, rasters and point clouds meshed onto it as they are drawn (docs/adr/0630).
    fn follow_map(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let map = kentos_project::systems::map_projection(doc.model.settings());
        if let Some(split) = self.split.as_mut() {
            for o in &mut split.others {
                o.viewport.set_map(map);
            }
        }
        self.viewport.set_map(map);
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        // What the drawing area's device can draw with, known after its first frame (AA-01).
        self.sync_device();
        // Dots Nokta yoğunluğu left out are said once a layer (docs/adr/0213 §2.4).
        for (name, n) in self.viewport.take_dropped() {
            self.warn(format!(
                "“{name}” katmanında Nokta yoğunluğunun {} noktası çizilmedi: bir katmanda en çok 1 000 000 nokta çizilir. Nokta değerini büyütün.",
                crate::view::thousands(n as f64)
            ));
        }
        // While a drawing is being opened the app takes no command (opening.rs).
        if let Some(task) = self.while_opening(&message) {
            return task;
        }
        // While Python code runs the drawing takes no other edit (python/).
        if let Some(task) = self.while_scripting(&message) {
            return task;
        }
        // Nor while a large import goes in, a frame at a time (exchange/drawing_import.rs).
        if let Some(task) = self.while_importing(&message) {
            return task;
        }
        // A command from the ribbon or a menu keeps the text field's text first (the web's blur),
        // and takes the keyboard from the layer tree (the web's button takes the focus).
        if matches!(
            message,
            Message::Run(_)
                | Message::RunMethod { .. }
                | Message::SplitChosen { .. }
                | Message::DrawTemplate(_)
                | Message::ApplyTemplate(_)
        ) {
            // A command closes the folded ribbon open over the drawing (the web's).
            if !matches!(message, Message::Run("view.keyTips")) {
                self.ribbon_peek = false;
            }
            self.close_text_field(true);
            self.close_paragraph(true);
            self.layers_keyboard = false;
            self.blocks_panel.keyboard = false;
            self.templates_panel.keyboard = false;
            self.vertices.keyboard = false;
        }
        match message {
            Message::Run(id) | Message::RunKept(id) => return self.run(id),
            Message::DrawTemplate(id) => return self.draw_template(&id),
            Message::ApplyTemplate(id) => self.apply_template(&id),
            Message::RunMethod { id, option, label } => {
                return self.run_method(id, option, label);
            }
            Message::SplitChosen {
                key,
                id,
                option,
                label,
            } => return self.split_chosen(key, id, option, label),
            Message::QuickAccess(id, on) => self.quick_access_changed(&id, on),
            Message::ShortcutsSearch(text) => self.shortcuts_query = text,
            Message::Swallowed => {}
            Message::KeyTipsAway => {
                self.key_tips = None;
                self.alt_armed = false;
            }
            Message::RibbonPeekAway => self.ribbon_peek = false,
            // The contextual Pafta tab is the sheet mode's; any other closes it (sheets.rs).
            Message::RibbonTab(id) => {
                self.sheet_tab = id == crate::sheets::SHEET_TAB;
                if !self.sheet_tab {
                    self.tab_clicked(id);
                }
            }
            Message::Sheet(m) => return self.sheet_message(m),
            Message::SheetLibrary(m) => return self.sheet_library_message(m),
            Message::SheetExportTo(kind, path) => self.sheet_export_to(kind, path),
            Message::RibbonPanel(event) => self.ribbon_panel_event(event),
            Message::CommandInput(text) => self.command_input = text,
            Message::CommandSubmitted => {
                let text = std::mem::take(&mut self.command_input);
                return self.submit_line(text.trim());
            }
            Message::CommandRun(name) => {
                self.command_input.clear();
                return self.run_typed(&name);
            }
            Message::CommandHistoryToggled => self.toggle_bottom(),
            Message::BottomTab(tab) => self.show_bottom(tab),
            Message::Points(event) => return self.points_event(event),
            Message::Features(event) => return self.features_event(event),
            Message::Statistics(event) => return self.statistics_event(event),
            Message::Charts(event) => return self.charts_event(event),
            Message::TableWritten(outcome) => match outcome {
                crate::table_write::Outcome::Written { file, rows, notes } => {
                    self.say(Level::Success, format!("“{file}” yazıldı: {rows} satır."));
                    for n in notes {
                        self.say(Level::Info, format!("“{file}”: {n}"));
                    }
                }
                crate::table_write::Outcome::Failed(why) => self.warn(why),
            },
            Message::LayerFields(event) => return self.layer_fields_event(event),
            Message::Search(event) => return self.data_event(event),
            Message::History(event) => return self.calls_event(event),
            Message::Topology(event) => return self.topology_event(event),
            Message::TopologyRules(event) => return self.topology_rules_event(event),
            Message::Networks(event) => return self.networks_event(event),
            Message::Address(event) => return self.address_event(event),
            Message::Time(event) => {
                let moved = self.time_event(event);
                return Task::batch([moved, self.time_sources()]);
            }
            Message::Vertices(event) => return self.vertices_event(event),
            Message::BottomResized(height) => self.bottom_dragged(Some(height), Instant::now()),
            Message::BottomReset => self.bottom_dragged(None, Instant::now()),
            Message::HistoryCleared => self.clear_history(),
            Message::AskProjectType => self.ask_project_type(),
            Message::LogFrame => self.log_frame(Instant::now()),
            Message::LogScrolled(viewport) => self.log_scrolled(viewport),
            Message::LayoutSave => self.layout.write(Instant::now(), false),
            Message::GridsLooked(looked) => self.grids_looked(looked),
            Message::BandRead(read) => self.band_read(read),
            Message::WindowResized(size) => self.window_resized(size),
            // The next frame draws them; the bottom panel shows the pyramids' progress.
            Message::RastersReady => {}
            Message::Rasters(event) => return self.rasters_event(event),
            // The next frame draws them; the panel shows the indexes' progress.
            Message::CloudsReady => {}
            // The next frame draws them; a service that failed is said once.
            Message::ServicesReady => {
                for text in crate::services::hub().take_notices() {
                    self.warn(text);
                }
            }
            Message::PointClouds(event) => return self.pointclouds_event(event),
            Message::Services(event) => return self.services_event(event),
            Message::Bound(event) => return self.bound_event(event),
            Message::Reconcile(event) => return self.reconcile_event(event),
            Message::CommandCancelled => {
                self.line_focused = false;
                return self.run("tool.cancel");
            }
            Message::CommandFocus(focused) => {
                self.line_focused = focused;
                // The value field and the layer tree lose the keyboard to the command line (web: their blur).
                if focused {
                    self.field = None;
                    self.layers_keyboard = false;
                    self.blocks_panel.keyboard = false;
                    self.templates_panel.keyboard = false;
                    self.vertices.keyboard = false;
                }
            }
            Message::PromptOption(key) => return self.prompt_option(key),
            Message::PromptChoice(key, typed) => return self.prompt_choice(key, &typed),
            Message::PointCalc(kind) => return self.start_point_calc(kind),
            Message::Key(press) => return self.key(press),
            Message::Modifiers(modifiers) => {
                let shift = self.modifiers.shift() != modifiers.shift();
                self.modifiers = modifiers;
                // Shift turns ortho over while held (docs/adr/0268 §2): the preview follows at once, the pointer still.
                if shift && self.session.takes_points() {
                    self.repoint();
                }
                // Alt let go after a tap alone: the ribbon's key tips (ribbon_keys.rs).
                return self.alt_released(modifiers);
            }
            Message::Dock(event) => {
                self.docks.update(event.clone());
                self.dock_dragged(&event, Instant::now());
            }
            Message::LayerSelected(id) => self.layer_pressed(id),
            Message::LayerSearch(text) => self.layer_search(text),
            Message::LayerProps(prop) => self.layer_props(prop),
            Message::LayerSearchDown => return self.layer_search_down(),
            Message::RibbonSearch(text) => self.search_typed(text),
            Message::RibbonSearchRun(index) => return self.search_run(index),
            Message::RibbonSearchReveal(index) => return self.search_reveal(index),
            Message::RibbonFlashEnd(id) => self.search_flash_end(id),
            Message::RibbonModelFlashEnd(id) => self.search_model_flash_end(&id),
            Message::Layer(event) => return self.layer_event(event),
            Message::DrawingMenu(event) => return self.drawing_menu_event(event),
            Message::DropLock(length) => self.drop_lock(length),
            Message::TextField(event) => self.text_field_event(event),
            Message::Paragraph(event) => self.paragraph_event(event),
            Message::Properties(event) => self.properties_event(event),
            Message::Blocks(event) => self.blocks_event(event),
            Message::BlocksPanel(event) => return self.blocks_panel_event(event),
            Message::TemplatesPanel(event) => return self.templates_panel_event(event),
            Message::Sources(event) => return self.sources_event(event),
            Message::Db(event) => return self.db_event(event),
            Message::TemplateEditor(event) => return self.template_editor_event(event),
            Message::BlockAttributes(event) => return self.block_attributes_event(event),
            Message::AttributeValues(event) => self.attribute_values_event(event),
            Message::BlockCard(event) => return self.block_card_event(event),
            Message::BlockSchedule(event) => return self.block_schedule_event(event),
            Message::FindReplace(event) => return self.find_replace_event(event),
            Message::LayerMerge(event) => return self.layer_merge_event(event),
            Message::LayerTranslate(event) => return self.layer_translate_event(event),
            Message::DrawingStandard(event) => return self.drawing_standard_event(event),
            Message::DataProfile(event) => return self.data_profile_event(event),
            Message::Subnetworks(event) => return self.subnetworks_event(event),
            Message::SourceStatus(event) => return self.source_status_event(event),
            Message::LineTypes(event) => return self.line_types_event(event),
            Message::Parameters(event) => return self.parameters_event(event),
            Message::HatchPatterns(event) => return self.hatch_patterns_event(event),
            Message::StyleImpact(event) => return self.style_impact_event(event),
            Message::BlockEditor(event) => return self.block_editor_event(event),
            Message::DocTabs(event) => return self.doc_tabs_event(event),
            Message::LayerFilter(event) => return self.layer_filter_event(event),
            Message::Attachment(event) => return self.attachment_event(event),
            Message::InfoCard(event) => return self.info_card_event(event),
            Message::LayerInfo(event) => return self.layer_info_event(event),
            Message::DatasetInfo(event) => return self.dataset_info_event(event),
            Message::Labelling(event) => return self.labelling_event(event),
            Message::LayerStates(event) => return self.layer_states_event(event),
            Message::Views(event) => return self.views_event(event),
            Message::Forms(event) => return self.forms_event(event),
            Message::Relations(event) => return self.relations_event(event),
            Message::Joins(event) => return self.joins_event(event),
            Message::EventLinks(event) => return self.event_links_event(event),
            Message::TableStyles(event) => return self.table_styles_event(event),
            Message::Subtypes(event) => return self.subtypes_event(event),
            Message::Contingents(event) => return self.contingents_event(event),
            Message::LayerRules(event) => return self.layer_rules_event(event),
            Message::Domains(event) => return self.domains_event(event),
            Message::FormAudit(event) => return self.form_audit_event(event),
            Message::FormEntry(event) => return self.form_entry_event(event),
            Message::FormPreview(event) => return self.form_preview_event(event),
            Message::AnnotationStyles(event) => return self.annotation_styles_event(event),
            Message::WallStyles(event) => return self.wall_styles_event(event),
            Message::WallJoins(event) => return self.wall_joins_event(event),
            Message::TagsCheck(event) => return self.tags_check_event(event),
            Message::ViewRefs(event) => return self.view_refs_event(event),
            Message::SheetSet(event) => return self.sheet_set_event(event),
            Message::Xrefs(event) => return self.xrefs_event(event),
            Message::XrefWatch(event) => return self.xref_watch_event(event),
            Message::Quantities(event) => return self.quantities_event(event),
            Message::LayerPurge(event) => return self.layer_purge_event(event),
            Message::DrawingExchange(event) => return self.drawing_exchange_event(event),
            Message::LayerList(event) => return self.layer_list_event(event),
            Message::FontReport(event) => return self.font_report_event(event),
            Message::TextTools(event) => return self.text_tools_event(event),
            Message::LeaderTools(event) => return self.leader_tools_event(event),
            Message::PlotScale(event) => return self.plot_scale_event(event),
            Message::DataCompare(event) => return self.data_compare_event(event),
            Message::Cogo(event) => return self.cogo_event(event),
            Message::TableInsert(event) => return self.table_insert_event(event),
            Message::TableEditor(event) => return self.table_editor_event(event),
            Message::TableUpdate(event) => return self.table_update_event(event),
            Message::Navigation(event) => self.navigation_event(event),
            Message::TextFile(file) => self.text_file_given(file),
            Message::ImageFile(file) => self.image_file_given(file),
            Message::HoverCard(version) => self.hover_card_due(version),
            Message::TrackDwell(number) => {
                let spatial = &self.spatial;
                self.tracking
                    .dwell_due(number, |id, at| spatial.extensions_at(id, at));
            }
            Message::Calc(event) => return self.calc_event(event),
            Message::Processing(event) => return self.processing_event(event),
            Message::ModelDesigner(event) => return self.model_designer_event(event),
            Message::Builder(event) => return self.builder_event(event),
            Message::LayerStyle(event) => return self.layer_style_event(event),
            Message::StyleManager(event) => return self.style_manager_event(*event),
            Message::Legend(event) => return self.legend_event(event),
            Message::Designer(event) => return self.designer_event(*event),
            Message::SvgEdit(event) => return self.svgedit_event(*event),
            Message::Python(event) => return self.python_event(event),
            // The layer tree's changes go through the document, as on the web: visibility
            // and lock are edits (unsaved) but not undo steps.
            // Through `cad.layers.set`, as Python and MCP do (docs/adr/0248 §6).
            Message::LayerVisible(id) => self.toggle_layer_visible(&id),
            Message::LayerLocked(id) => self.toggle_layer_locked(&id),
            Message::LayerPlot(id) => self.toggle_layer_plot(&id),
            Message::LayerFrozen(id) => self.toggle_layer_frozen(&id),
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document
                    && let Some(expanded) = doc.find(&id).map(|n| !n.expanded)
                {
                    // Folding is kept in the file but is not an edit (web).
                    doc.model.set_layer_expanded(&id, expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(mut doc))) => {
                // Bound GeoPackages moved with the project are found again (docs/adr/0506).
                let located = match doc.path.clone() {
                    Some(path) if doc.cloud_source().is_none() => {
                        kentos_native_application::source_places::relocate(&mut doc.model, &path)
                    }
                    _ => Default::default(),
                };
                match doc.cloud_source() {
                    Some(source) => self.say(
                        Level::Success,
                        format!(
                            "“{}” bulut projesi açıldı ({}, {}): {} nesne.",
                            doc.name(),
                            source.workspace,
                            cloud::words::storage_title(source.storage()),
                            doc.entity_count()
                        ),
                    ),
                    None => self.output(format!(
                        "{} açıldı: {} nesne, {} katman.",
                        doc.name(),
                        doc.entity_count(),
                        doc.layer_count()
                    )),
                }
                if doc.legacy {
                    self.output(
                        "Dosya eski biçimde (KCAD v1). Kaydet, yeni biçimde (v2) yazmak için yer sorar; eski dosyanın üzerine kendiliğinden yazmaz.",
                    );
                }
                self.show_document(*doc);
                self.say_located(&located);
                // A drawing opened from a file goes first in the recent files (a recovered one has none).
                self.remember_file();
            }
            Message::Opened(Some(Err(error))) | Message::Saved(Some(Err(error))) => {
                self.error(error)
            }
            Message::Saved(Some(Ok(written))) => match &mut self.document {
                Some(doc) if doc.session == written.session => {
                    // “Yerel dosyaya kaydet” (cloud/file_follow.rs): the work is in the
                    // file now, so a save of it kept for the project goes, and the
                    // drawing leaves the project, which stays as it is.
                    let detached = self
                        .cloud
                        .detaching
                        .take()
                        .filter(|(session, _)| {
                            *session == doc.session && doc.cloud_source().is_some()
                        })
                        .map(|(_, name)| name);
                    if detached.is_some()
                        && let Some(held) = self
                            .cloud
                            .held
                            .as_mut()
                            .filter(|h| h.session == doc.session && h.kept_save)
                        && held.replica.clear_save().is_ok()
                    {
                        held.kept_save = false;
                    }
                    doc.saved(written.path.clone(), written.revision);
                    let later = if doc.dirty() {
                        " Kayıt sürerken yapılan değişiklikler kaydedilmedi."
                    } else {
                        ""
                    };
                    self.output(format!("Kaydedildi: {}.{later}", written.path.display()));
                    if let Some(name) = detached {
                        self.cloud.file_conflict = None;
                        self.output(crate::cloud::revisions::texts::detached(&name));
                    }
                    self.recovery_saved();
                    self.remember_file();
                }
                // Its drawing set aside in its tab meanwhile (doc_tabs.rs, docs/adr/0351 §3): the file is
                // its file all the same.
                _ if let Some(doc) = self.doc_tabs.find_mut(written.session) => {
                    doc.saved(written.path.clone(), written.revision);
                    self.output(format!("Kaydedildi: {}.", written.path.display()));
                }
                // Another drawing was opened meanwhile: the file is written, but it is
                // not the open drawing's file, which keeps its path and its state.
                _ => self.output(format!("Kaydedildi: {}.", written.path.display())),
            },
            Message::CloseRequested(_) if self.block_editor.is_some() => {
                self.warn(crate::block_editor::OPEN);
            }
            Message::CloseRequested(window) => {
                // The layout is written now, not 250 ms later.
                self.layout.write(Instant::now(), true);
                // A save stopped by the window closing would leave no file (the previous one
                // stays): it finishes first.
                let unsaved = self.doc_tabs.unsaved();
                if let Some(s) = &self.saving {
                    let name = s.name();
                    self.warn(format!(
                        "{name} kaydediliyor; kayıt bitince pencereyi yeniden kapatın."
                    ));
                } else if !unsaved.is_empty() {
                    // A drawing set aside in its tab is not asked about from here: it is brought in front first.
                    self.warn(format!(
                        "Sekmelerdeki {} çizimde kaydedilmemiş değişiklik var ({}): o sekmeye geçip kaydedin ya da Çizimi kapat ile kapatın, sonra pencereyi yeniden kapatın.",
                        unsaved.len(),
                        unsaved.join(", ")
                    ));
                } else {
                    // Unsent cloud work to its draft first, else the question (cloud/leaving.rs).
                    return self.leave(Then::Close(window));
                }
            }
            Message::DialogConfirmed => match self.dialog.take() {
                Some(Dialog::Unsaved(then)) => {
                    // The unsaved changes are dropped on purpose: their recovery copy goes too.
                    self.recovery.discard();
                    self.cloud.leave_failure = None;
                    return self.proceed(then);
                }
                Some(Dialog::RemoveLayer) => {
                    if let Some(id) = self.removing_layer.take() {
                        self.remove_layer(&id);
                    }
                }
                Some(Dialog::RemoveTemplate) => self.remove_template(),
                Some(Dialog::SourceDiscard) => self.bound_discard(),
                _ => {}
            },
            Message::DialogClosed => self.close_dialog(),
            Message::Cloud(event) => return self.cloud_event(*event),
            Message::Exchange(event) => return self.exchange_event(*event),
            Message::Project(event) => return self.project_event(*event),
            Message::AppMenu(event) => return self.app_menu_event(event),
            Message::Start(event) => return self.start_event(event),
            Message::Viewport(event) => return self.pointer(event),
            Message::OtherViewport(i, event) => return self.other_viewport(i, event),
            Message::Split(event) => return self.split_event(event),
            Message::Settings(edit) => return self.settings_edit(edit),
            Message::Opening(event) => return self.opening_event(event),
            Message::Saving(event) => return self.saving_event(event),
            Message::Recovery(event) => return self.recovery_event(event),
            Message::ServerChecked(answer) => self.server_checked(answer),
            Message::OverlapLayer(id) => self.toggle_overlap_layer(id),
            Message::CycleTo(index) => {
                self.selection.cycle_to(index);
                self.say_sub_object();
            }
            Message::CycleHover(slot) => self.selection.set_hover(slot),
            Message::Snap(event) => self.snap_event(event),
            Message::ScreenScale(event) => return self.screen_scale_event(event),
            Message::SecondCrs(event) => self.second_crs_event(event),
            Message::MapCrs(choose) => self.choose_map(choose),
            Message::LayerCrs(event) => self.layer_crs_event(event),
            Message::ToastClosed(id) => {
                self.toasts.dismiss(id);
            }
        }
        Task::none()
    }

    /// Puts the settings in use (docs/adr/0023): the tool session's drafting
    /// aids and value field, the theme; the drawing area reads `graphics()`
    /// each frame. Called after every change.
    pub(crate) fn apply_settings(&mut self) {
        let s = &self.settings;
        self.draft = Draft {
            ortho: s.bool("drafting.ortho"),
            right_angle: s.bool("drafting.rightAngle"),
            polar: s
                .bool("drafting.polar")
                .then(|| s.number("drafting.polarIncrement")),
            snap_aperture: s.number("drafting.snapAperture"),
            snap: s.bool("drafting.snap"),
            snap_kinds: snap_kinds(|key| s.bool(key)),
            snap_grid: [s.number("snap.gridEast"), s.number("snap.gridNorth")],
            snap_self: s.bool("snap.self"),
            snap_scale: [s.number("snap.scaleMin"), s.number("snap.scaleMax")],
            pick_aperture: s.number("drafting.pickAperture"),
            tracking: s.bool("drafting.tracking"),
            topology: s.bool("drafting.topology"),
            topology_points: s.bool("drafting.topologyPoints"),
            overlap: kentos_interaction::Overlap::parse(&s.text("drafting.overlap")),
            // The session's, not settings: kept through a settings change.
            color: self.draft.color,
            line_weight: self.draft.line_weight,
            line_type: self.draft.line_type,
            ucs: self.draft.ucs,
            geographic: kentos_interaction::second::Notation::parse(&s.text("display.geographic")),
            // Seçim süzgeci's kinds are the session's (docs/adr/0187 §5).
            select_kinds: s.bool("drafting.selectFilter").then_some(self.select_kinds),
            unplaced_labels: s.bool("graphics.unplacedLabels"),
        };
        self.cursor_input = s.bool("drafting.cursorInput");
        self.command_bar = s.bool("drafting.commandBar");
        self.hover_info = s.bool("drafting.hoverInfo");
        self.crosshair = crate::marks::CrosshairSize::parse(&s.text("appearance.crosshair"));
        self.viewport.grid_shown = s.bool("drafting.grid");
        self.apply_appearance();
    }

    /// What the drawing area draws with: the effective sample count and pixel ratio.
    /// Görünüm kipleri's colour mode (`graphics.colorMode`, docs/adr/0195 §1).
    pub fn color_mode(&self) -> kentos_native_style::color::ColorMode {
        kentos_native_style::color::ColorMode::from_key(&self.settings.text("graphics.colorMode"))
    }

    /// What the label engine is asked besides the labels (docs/adr/0212 §4):
    /// Yerleşmeyen etiketleri göster, Etiketi gizle's Göster, Sabit etiketleri
    /// vurgula; the drawing area's labels are kept for the tools' hit test.
    pub fn label_engine(&self) -> crate::labels::Engine {
        crate::labels::Engine {
            options: kentos_geometry_core::store::placing::PlaceOptions {
                unplaced: self.settings.bool("graphics.unplacedLabels"),
                hidden: self.session.shows_hidden_labels(),
            },
            pinned: self.settings.bool("graphics.pinnedLabels"),
            keep: true,
            moving: self.viewport.moving(),
        }
    }

    /// How the drawing's text is sized on the screen (docs/adr/0205 §5, 0626 §6): the
    /// preference, at the open project's plot scale; Türe göre a CAD project's true sizes.
    pub fn label_size(&self) -> kentos_interaction::spatial::LabelSize {
        let plot = self
            .document
            .as_ref()
            .map_or(1000.0, |d| d.settings().plot_scale);
        kentos_interaction::spatial::LabelSize::for_project(
            &self.settings.text("graphics.annotationSize"),
            plot,
            self.work_mode() == kentos_commands::Workspace::Cad,
        )
    }

    pub fn graphics(&self) -> Graphics {
        Graphics {
            samples: self.settings.number("graphics.msaa").max(1.0) as u32,
            hi_dpi: self.settings.bool("graphics.hiDpi"),
            highlight_width: self
                .settings
                .number("graphics.highlightWidth")
                .clamp(1.0, 5.0) as u32,
        }
    }

    /// Feeds the drawing area's device into the settings (TODOS.md AA-01,
    /// SET-03): the sample counts it takes, or the count it could not make
    /// targets for (AA-02); the effective value follows, the requested one stays.
    pub(crate) fn sync_device(&mut self) {
        let status = self.viewport.status();
        if status.supported.is_empty() {
            return;
        }
        let values = |counts: &mut dyn Iterator<Item = u32>| counts.map(Value::from).collect();
        let listed: Vec<String> = status.supported.iter().map(|&n| samples_label(n)).collect();
        let constraint = match &status.failure {
            Some(f) => SettingConstraint {
                allowed: values(
                    &mut status
                        .supported
                        .iter()
                        .copied()
                        .filter(|&c| c < f.requested),
                ),
                reason: ResolveReason::DeviceFailed,
                detail: format!("{}× hedefi kurulamadı ({}).", f.requested, f.error),
            },
            None => SettingConstraint {
                allowed: values(&mut status.supported.iter().copied()),
                reason: ResolveReason::DeviceUnsupported,
                detail: format!("Desteklenenler: {}.", listed.join(", ")),
            },
        };
        self.settings
            .set_constraint("graphics.msaa", Some(constraint));
        if let Some(f) = &status.failure
            && self.reported_failure != Some((f.requested, f.working))
        {
            self.reported_failure = Some((f.requested, f.working));
            self.warn(format!(
                "Kenar yumuşatma {}× bu aygıtta kurulamadı; son çalışan ayara ({}) dönüldü. Neden: {}",
                f.requested,
                samples_label(f.working),
                f.error
            ));
        }
    }

    /// What opening the settings did, in the command line: the one migration, a recovery.
    fn report_settings_open(&mut self) {
        let report = self.settings.report.clone();
        if let Some(m) = &report.migrated {
            self.output(format!(
                "Ayarlar {} dosyasından alındı ({} değer); o dosya olduğu gibi duruyor.",
                m.from, m.moved
            ));
        }
        if let Some(r) = &report.recovered {
            self.warn(format!(
                "Ayar dosyası okunamadı ya da geçersiz değer içeriyordu; eski metni {} olarak saklandı.",
                r.backup.display()
            ));
        }
    }

    /// A drafting aid of this session turned over (F8, F10): said as AutoCAD says it.
    /// Çakışma's cell: between Serbest and the last mode that avoided overlap (docs/adr/0162 §1).
    fn toggle_overlap(&mut self) {
        use kentos_interaction::Overlap;
        let next = if self.draft.overlap == Overlap::Allow {
            self.overlap_last
        } else {
            Overlap::Allow
        };
        self.choose_overlap(next);
        self.output(match next {
            Overlap::Allow => "Çakışma serbest.".to_owned(),
            Overlap::Layer => "Çakışma önleniyor: kendi katmanında.".to_owned(),
            Overlap::Layers => "Çakışma önleniyor: seçili katmanlarda.".to_owned(),
        });
    }

    /// One of the overlap control's modes: the session's setting; a mode that avoids is the cell's next.
    pub(crate) fn choose_overlap(&mut self, mode: kentos_interaction::Overlap) {
        let _ = self
            .settings
            .choose(&[("drafting.overlap", Value::String(mode.key().to_owned()))]);
        if mode != kentos_interaction::Overlap::Allow {
            self.overlap_last = mode;
        }
        self.apply_settings();
    }

    /// A layer ticked on the Çakışma cell's menu joins Seçili katmanlarda önle's
    /// layers (unticked, it leaves them), and that mode is put on (docs/adr/0162 §1).
    fn toggle_overlap_layer(&mut self, id: String) {
        match self.overlap_layers.iter().position(|l| *l == id) {
            Some(i) => {
                self.overlap_layers.remove(i);
            }
            None => self.overlap_layers.push(id),
        }
        self.choose_overlap(kentos_interaction::Overlap::Layers);
    }

    pub(crate) fn toggle_session(&mut self, key: &'static str, name: &str) {
        let on = !self.settings.bool(key);
        let _ = self.settings.choose(&[(key, Value::Bool(on))]);
        self.apply_settings();
        self.output(format!("{name} {}", if on { "açık" } else { "kapalı" }));
        // The running command keeps its points, options and locks; its preview takes the aid at once
        // (docs/adr/0268 §3).
        if self.session.takes_points() {
            self.repoint();
        }
    }

    /// The theme chosen from a command: a preference, kept.
    fn choose_theme(&mut self, mode: Mode) {
        self.set_theme(mode);
    }

    /// Puts a drawing on screen: the one there before is left (its cloud
    /// project's copy written whole and let go), and so are the tool's draft,
    /// the selection and the geometry store, which belong to the drawing they
    /// were made on.
    pub(crate) fn show_document(&mut self, doc: Document) {
        self.close_cloud_project();
        self.cancel();
        // A point template's names go on in the drawing they were drawn in:
        // another drawing starts them again (docs/adr/0176 §3b).
        self.release_template();
        self.template_names.clear();
        // The annotation tools' heights typed in the last drawing: the new one's annotations take
        // its own project's (docs/adr/0205 §2).
        self.memory.forget_heights();
        self.selected_layer = None;
        // A linked photo's key is its path from the drawing's folder (attachments.rs); Bilgi's card was
        // on an object of the drawing left.
        self.photos.clear();
        self.info_card = None;
        self.networks_reset();
        self.address_reset();
        self.spatial.reload(&doc.model);
        self.viewport.opened(&doc, self.spatial.extent());
        // The views left belong to the drawing they were left on (docs/adr/0141).
        self.view_history.clear();
        self.styles.follow_project(Some(doc.model.styles()));
        self.selection = Selection::new();
        // İstatistik and Grafik start again on another drawing (docs/adr/0227 §8).
        self.charts = Default::default();
        self.followed = Some((doc.session, doc.model.generation()));
        self.xrefs = Default::default();
        self.document = Some(doc);
        // A linked file that changed or went is said, never loaded silently (docs/adr/0348 §5, 0349 §2).
        self.xrefs_check_opened();
    }

    /// Runs a web command id: the desktop's handler, or a note that it is not here yet.
    pub fn run(&mut self, id: &'static str) -> Task<Message> {
        // Whatever runs a command closes the application menu (the web's `executed`).
        self.app_menu = None;
        let Some(command) = catalog().get(id) else {
            self.error(format!("Komut bulunamadı: {id}"));
            return Task::none();
        };
        match command.standing {
            Standing::Ported => {}
            Standing::OnTheWeb => {
                self.output(format!(
                    "“{}” web'de var; masaüstüne henüz taşınmadı.",
                    command.title
                ));
                return Task::none();
            }
            Standing::Pending => {
                let why = command.pending_note.unwrap_or("Geliştirme aşamasında");
                self.output(format!("“{}”: {why}.", command.title));
                return Task::none();
            }
        }

        // While Blok düzenleyici is open the drawing set aside waits: its file, cloud and sheet commands are refused
        // (block_editor.rs, docs/adr/0316 §1).
        if self.block_editor.is_some()
            && ["file.", "cloud.", "sheet.", "project."]
                .iter()
                .any(|p| id.starts_with(p))
        {
            self.warn(crate::block_editor::OPEN);
            return Task::none();
        }
        // Yeni sekmede aç, Çizimi kapat, Sonraki çizim (doc_tabs.rs, docs/adr/0351 §3).
        if crate::doc_tabs::COMMANDS.contains(&id) {
            return self.doc_tabs_command(id);
        }
        if let Some(tool) = id.strip_prefix("tool.")
            && Session::tools().contains(&tool)
        {
            return self.start_tool(tool);
        }
        // Modele dön (MODEL), as the web's: the sheet mode's own message (docs/adr/0164).
        if id == "sheet.drawingSet" {
            if self.sheets.book().sheets.is_empty() { self.warn("Önce bir pafta oluşturun."); return Task::none(); }
            let show = if self.sheets.is_active() { Task::none() } else { self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(1))) };
            let open = self.update(Message::Sheet(kentos_sheet_ui::Message::DrawingSet(kentos_sheet_ui::drawing_set::SetMessage::Open)));
            return Task::batch([show,open]);
        }
        if id == "sheet.model" {
            if !self.sheets.is_active() {
                self.output("Model zaten önde.");
                return Task::none();
            }
            return self.update(Message::Sheet(kentos_sheet_ui::Message::Model));
        }
        if matches!(id, "sheet.fitDrawing" | "sheet.fitSelection") {
            if !self.sheets.is_active() || self.sheets.selection().is_empty() {
                self.warn("Paftada bir sabit görünüm penceresi seçin.");
                return Task::none();
            }
            return self.update(Message::Sheet(if id == "sheet.fitDrawing" {
                kentos_sheet_ui::Message::MapFitDrawing
            } else { kentos_sheet_ui::Message::MapFitSelection }));
        }
        if id.starts_with("cloud.") {
            return self.cloud_command(id);
        }
        // A ready basemap (services/basemaps.rs, docs/adr/0208 §14).
        if id.starts_with("basemap.") && id != "basemap.remove" {
            return self.basemap_command(id);
        }
        if crate::exchange::COMMANDS.contains(&id) {
            return self.exchange_command(id);
        }
        if crate::project::COMMANDS.contains(&id) {
            return self.project_command(id);
        }
        if crate::calc::COMMANDS.contains(&id) {
            return self.calc_command(id);
        }
        if crate::processing::answers(id) {
            return self.processing_command(id);
        }
        if crate::clipboard::COMMANDS.contains(&id) {
            return self.clipboard_command(id);
        }
        if crate::view_commands::COMMANDS.contains(&id) {
            return self.view_command(id);
        }
        // The snap kinds one by one and Çizilmekte olan nesneye (snap_menu.rs, docs/adr/0163 §6).
        if self.toggle_snap_setting(id) {
            return Task::none();
        }
        // Önceki and Sonraki görünüm, Kapsam denetimi (navigation.rs, docs/adr/0141).
        if crate::navigation::COMMANDS.contains(&id) {
            return self.navigation_command(id);
        }
        // Görünümler and the view's twist (views.rs, docs/adr/0285).
        if crate::views::COMMANDS.contains(&id) {
            return self.views_command(id);
        }
        // Formlar and Formla düzenle (forms/, docs/adr/0401).
        if crate::forms::COMMANDS.contains(&id) {
            return self.forms_command(id);
        }
        // Çizimler arası alışveriş (drawing_exchange.rs, docs/adr/0193).
        if crate::drawing_exchange::COMMANDS.contains(&id) {
            return self.drawing_exchange_command(id);
        }
        match id {
            // The drawing on screen is left first: its unsent cloud work to its draft, or the question.
            "file.open" => return self.leave(Then::Open),
            "file.start" => self.open_start(),
            // A cloud project saves to the server (cloud/file.rs); Farklı kaydet writes a local file.
            "file.save"
                if self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.cloud_source().is_some()) =>
            {
                return self.save_cloud();
            }
            "file.save" => return self.save(false),
            "file.saveAs" => return self.save(true),
            "view.theme.dark" => self.choose_theme(Mode::Dark),
            "view.theme.light" => self.choose_theme(Mode::Light),
            "view.theme.toggle" => self.choose_theme(self.mode.toggled()),
            id if id.starts_with("workspace.") => self.choose_mode(id),
            "tools.options" => self.open_settings(),
            // Yeni pafta and Şablondan pafta (docs/adr/0622 §3): the sheet mode's own, from the ribbon.
            "sheet.new" => return self.sheet_message(kentos_sheet_ui::Message::NewSheet),
            "sheet.fromTemplate" => {
                return self.sheet_message(kentos_sheet_ui::Message::Gallery(
                    kentos_sheet_ui::gallery::GalleryMessage::Open,
                ));
            }
            // Koordinat oku: a tool that reads the clicked point into the log (docs/adr/0140).
            "crs.query" => return self.start_tool(kentos_interaction::coordinate::ID),
            "draft.ortho" => self.toggle_session("drafting.ortho", "Orto"),
            "draft.rightAngle" => self.toggle_session("drafting.rightAngle", "Dik açı"),
            "draft.polar" => self.toggle_session("drafting.polar", "Kutupsal izleme"),
            "draft.tracking" => self.toggle_session("drafting.tracking", "Nesne izleme"),
            "draft.topology" => self.toggle_session("drafting.topology", "Topolojik düzenleme"),
            "draft.topologyPoints" => self.toggle_session(
                "drafting.topologyPoints",
                "Topolojik düzenlemede noktalar da",
            ),
            "draft.overlap" => self.toggle_overlap(),
            // The digitizing locks (docs/adr/0166 §6), on the running command.
            "draft.lock.length" => self.ask_lock(kentos_interaction::LockAsk::Length),
            "draft.lock.angle" => self.ask_lock(kentos_interaction::LockAsk::Angle),
            "draft.lock.deflection" => self.ask_lock(kentos_interaction::LockAsk::Deflection),
            "draft.lock.east" => self.ask_lock(kentos_interaction::LockAsk::East),
            "draft.lock.north" => self.ask_lock(kentos_interaction::LockAsk::North),
            "draft.lock.parallel" => {
                self.with_tool(|s, cx| {
                    s.pick_lock_edge(kentos_interaction::LockPick::Parallel, cx)
                });
            }
            "draft.lock.perpendicular" => {
                self.with_tool(|s, cx| {
                    s.pick_lock_edge(kentos_interaction::LockPick::Perpendicular, cx)
                });
            }
            "draft.lock.reference" => {
                self.with_tool(|s, cx| s.ask_reference(cx));
            }
            // Geçici izleme noktası (docs/adr/0270 §3).
            "draft.trackPoint" => {
                self.with_tool(|s, cx| s.ask_track_point(cx));
            }
            "draft.lock.construction" => {
                let on = !self.session.construction();
                self.with_tool(|s, cx| s.set_construction(on, cx));
            }
            "draft.lock.keep" => {
                let keep = !self.locks.keep;
                self.with_tool(|s, cx| s.keep_locks(keep, cx));
            }
            "draft.lock.clear" => {
                self.with_tool(|s, cx| s.clear_locks(cx));
            }
            "draft.overlap.allow" => self.choose_overlap(kentos_interaction::Overlap::Allow),
            "draft.overlap.layer" => self.choose_overlap(kentos_interaction::Overlap::Layer),
            "draft.overlap.layers" => self.choose_overlap(kentos_interaction::Overlap::Layers),
            "draft.snap" => self.toggle_session("drafting.snap", "Kenetleme"),
            "draft.grid" => self.toggle_session("drafting.grid", "Izgara"),
            // The styled drawing's view choices (style/, docs/adr/0090).
            "view.lineWeights" => self.toggle_session("graphics.lineWeights", "Çizgi kalınlığı"),
            // Görünüm kipleri (docs/adr/0195).
            "view.colorMode.color" | "view.colorMode.mono" | "view.colorMode.gray" => {
                self.choose_color_mode(id);
            }
            // Yazıların boyu (docs/adr/0205 §5).
            "view.annotationSize.legible"
            | "view.annotationSize.true"
            | "view.annotationSize.screen" => self.choose_annotation_size(id),
            "view.fills" => self.toggle_session("graphics.fills", "Dolgular ve taramalar"),
            "view.areaEdges" => self.toggle_session("graphics.areaEdges", "Alan sınırları"),
            "view.transparency" => self.toggle_session("graphics.transparency", "Saydamlık"),
            // Sabit ve yerleşmeyen etiketler (docs/adr/0212 §4): the user's view, the drawing unchanged.
            "view.pinnedLabels" => {
                self.toggle_session("graphics.pinnedLabels", "Sabit etiketleri vurgula");
            }
            "view.unplacedLabels" => {
                self.toggle_session("graphics.unplacedLabels", "Yerleşmeyen etiketleri göster");
            }
            "style.layerStyle" => self.open_layer_style(None),
            // The style library (style/manager/, docs/adr/0092).
            "style.manager" => return self.open_style_manager(None, None),
            // Tablo (tables/, docs/adr/0184).
            "table.insert" => self.open_table_insert(false),
            "table.edit" => {
                if let [slot] = self.selection.ids()
                    && matches!(
                        self.document.as_ref().and_then(|d| d.model.get(*slot)),
                        Some(kentos_commands::Entity::Table(_))
                    )
                {
                    let slot = *slot;
                    return self.open_table_editor(slot);
                } else {
                    self.output("Bir tablo seçin (ya da tabloya çift tıklayın).");
                }
            }
            "table.update" => return self.update_tables(),
            // Tablonun dosyası ve blok çizelgesi (docs/adr/0327 §4–§5).
            "table.link" => return self.link_table(),
            "table.blockSchedule" => self.place_block_schedule(),
            // Yazı ve ölçü stilleri (docs/adr/0183 §5).
            "style.textStyles" => self.open_annotation_styles(crate::annotation_styles::Kind::Text),
            // Duvar stilleri (docs/adr/0339 §6).
            "arch.wallStyles" => self.open_wall_styles(),
            "arch.wallJoinCheck" => self.open_wall_joins(),
            "arch.tagsCheck" => self.open_tags_check(),
            "arch.viewRefs" => self.open_view_refs(),
            "arch.sheetSet" => self.open_sheet_set(),
            "xref.manager" => self.open_xrefs(),
            "xref.attach" => return self.start_xref_attach(),
            "arch.template" => self.apply_arch_template(),
            id if id.starts_with("arch.phase.") => self.set_phase_view(&id["arch.phase.".len()..]),
            // Metraj and Mahalleri birleştir (docs/adr/0342 §5).
            "arch.quantities" => self.open_quantities(),
            "arch.spaceMerge" => self.space_merge(),
            "style.dimensionStyles" => {
                self.open_annotation_styles(crate::annotation_styles::Kind::Dimension)
            }
            // Şablonla çiz without a template: the Şablonlar panel, to choose one (templates_panel.rs).
            "template.draw" | "template.panel" | "template.apply" => {
                return self.show_templates_panel();
            }
            // Şablon düzenleyici, new or from the selection (template_editor.rs, docs/adr/0176 §4).
            "template.new" => return self.open_template_editor(None, None),
            "template.fromSelection" => return self.template_from_selection(),
            "style.legend" => self.open_legend(),
            "style.svgEditor" => self.open_svg_editor(crate::style::svgedit::Opening {
                id: None,
                path: None,
                after: crate::style::svgedit::After::Nothing,
            }),
            "style.assign" => return self.pick_for_selection(),
            "style.clearSymbol" => {
                let said = self.assign_symbol(None);
                self.output(said);
            }
            "view.symbols.plot" | "view.symbols.screen" => self.choose_symbol_size(id),
            // Selecting (docs/adr/0029): the pointer selects while no command runs.
            "tool.select" => self.leave_tool(),
            "edit.deselect" => self.selection.clear(),
            "edit.selectAll" => self.select_all(),
            "edit.invertSelection" => self.invert_selection(),
            // Seçim ekleri (docs/adr/0187, selection_commands.rs).
            "edit.previousSelection" => self.previous_selection(),
            "edit.cycleSelection" => self.cycle_selection(),
            "edit.selectFilter" => self.toggle_select_filter(),
            // Otomatik seçim (layer_types.rs, docs/adr/0622 §5).
            "edit.autoSelect" => self.toggle_auto_select(),
            "edit.selectFilterAll" => self.select_all_kinds(true),
            "edit.selectFilterNone" => self.select_all_kinds(false),
            id if id.starts_with("edit.selectFilter.") => {
                self.toggle_select_kind(&id["edit.selectFilter.".len()..]);
            }
            "view.ribbonCollapse" => {
                self.ribbon_collapsed = !self.ribbon_collapsed;
                self.ribbon_peek = false;
            }
            // Şerit harf ipuçları (F6; Alt tapped alone too, ribbon_keys.rs).
            "view.keyTips" => return self.toggle_key_tips(),
            // The bottom panel (bottom.rs): F2, and the coordinate list.
            "view.bottomPanel" => self.toggle_bottom(),
            "view.coords" => self.show_bottom(crate::bottom::BottomTab::Coords),
            // Nokta editörü: the bottom panel's Noktalar tab (points/, docs/adr/0153).
            "point.editor" => self.show_bottom(crate::bottom::BottomTab::Points),
            // Öznitelik tablosu: the bottom panel's Tablo tab (features/, docs/adr/0199 §4).
            "data.featureTable" => self.open_feature_table(),
            // Alan istatistikleri and Grafik: the bottom panel's İstatistik and Grafik tabs (docs/adr/0227 §8).
            "data.statistics" => self.open_statistics(None),
            "data.chart" => self.open_chart(None, None, None),
            id if id.starts_with("chart.") => {
                if let Some(k) = crate::charts::KINDS.iter().find(|k| k.command == id) {
                    self.open_chart(None, Some(k.kind), None);
                }
            }
            // Veri kaynakları: the dock's Kaynaklar tab (sources/, docs/adr/0199 §7).
            "data.sources" => self.show_sources(),
            // Alanlar: the active layer's fields (layer_fields.rs; the layer tree's menu gives its layer).
            "layer.fields" => {
                let layer = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                if let Some(layer) = layer {
                    return self.open_layer_fields(&layer);
                }
            }
            // Veride ara: the bottom panel's Arama tab, its box taking the keyboard (search/, docs/adr/0178).
            "data.search" => return self.open_data_search(),
            "data.unmark" => {
                self.address_unmark();
                self.sources.extent_mark = None;
                return self.data_event(crate::search::Event::Unmark);
            }
            // Topoloji kuralları (topology/, docs/adr/0202 §5, §6).
            "topology.check" => return self.open_topology_check(),
            "topology.rules" => return self.open_topology_rules(),
            // Ağlar (networks/, docs/adr/0209 §10).
            "network.manage" => return self.open_networks(),
            // Adres arama (address/, docs/adr/0229 §13).
            "address.search" => return self.open_address_search(),
            "address.reverse" => return self.start_tool(kentos_interaction::address_reverse::ID),
            "address.data" => return self.open_address_data(),
            // docs/adr/0583: a layer's source system and the map's.
            "layer.crs" => self.layer_crs_run(false),
            "layer.reproject" => self.layer_crs_run(true),
            "view.mapCrs" => self.map_crs_run(),
            "address.providers" => return self.open_address_providers(),
            // Gevşeterek düzenle: the next write relaxes the constraints (docs/adr/0337 §1); again, it does not.
            "constraint.relaxNext" => self.relax_next(),
            // Parametreler and the constraints' marks (docs/adr/0338 §1).
            "constraint.parameters" => self.open_parameters(),
            "constraint.show" => self.constraint_view.toggle(),
            // The user coordinate system (ucs.rs, docs/adr/0261).
            "ucs.world" => self.ucs_world(),
            "ucs.previous" => self.ucs_previous(),
            // Ölçekli açıklamalar (annotative.rs, docs/adr/0323 §4).
            "annotation.make" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Make)
            }
            "annotation.unmake" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Unmake)
            }
            "annotation.reset" => {
                self.annotative_selection(kentos_commands::AnnotativeOperation::Reset)
            }
            // Sırala (draw_order.rs, docs/adr/0260 §5).
            "edit.bringToFront" => self.order_selection(kentos_commands::OrderTo::Front),
            "edit.sendToBack" => self.order_selection(kentos_commands::OrderTo::Back),
            "edit.textsToFront" => self.order_kinds(
                &crate::draw_order::WORDS,
                kentos_commands::OrderTo::Front,
                "Yazıları öne getir",
            ),
            "edit.hatchesToBack" => self.order_kinds(
                &[kentos_commands::SelectKind::Hatch],
                kentos_commands::OrderTo::Back,
                "Taramaları arkaya gönder",
            ),
            // Zaman and Senaryo (temporal/, docs/adr/0210 §10).
            "time.slider" => self.time_toggle(),
            "time.layer" => self.open_time_layer(None),
            "time.compare" => self.open_time_compare(),
            "scenario.create" => self.open_scenario_create(),
            "scenario.show" => {
                if let Some(id) = self.scenario_at_hand() {
                    self.show_scenario(&id);
                }
            }
            "scenario.base" => self.show_base(),
            "scenario.compare" => self.open_scenario_compare(None),
            "scenario.apply" => self.ask_scenario_apply(None),
            crate::catalog::PYTHON_CONSOLE => self.toggle_python(),
            // The navigation commands keep the view they leave (navigation.rs, docs/adr/0141).
            "view.zoomIn" => self.navigating(Self::zoom_in),
            "view.zoomOut" => self.navigating(Self::zoom_out),
            "view.zoomExtents" => self.navigating(Self::zoom_extents),
            // Görünümü böl (split_view.rs, docs/adr/0285 §3).
            "view.split" => self.toggle_split(),
            // Görünümleri bağla, Kaydırmalı karşılaştır (split_view.rs, docs/adr/0523).
            "view.splitLink" => self.toggle_split_link(),
            "view.splitAdd" => self.add_view(),
            "view.swipe" => self.toggle_swipe(),
            "view.zoomSelection" => self.navigating(Self::zoom_selection),
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => return self.open_shortcuts(),
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &self.document {
                Some(_) => self.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            // Kopyasını oluştur and Katmanları birleştir (docs/adr/0177 §3): the active layer's;
            // Katmanlar's menu names its layer (layering.rs).
            "layer.duplicate" => self.duplicate_layer(None),
            "layer.merge" => self.open_layer_merge(None),
            // Katman durumları, Kullanılmayanları temizle and Katman listesi (docs/adr/0177 §4–§6).
            "layer.states" => self.open_layer_states(),
            "layer.stateSave" => {
                self.quick_save_layer_state();
            }
            "layer.purge" => self.open_layer_purge(),
            "layer.list" => self.open_layer_list(),
            "text.fonts" => self.open_font_report(),
            // Alanlar and the text commands (text_tools.rs, docs/adr/0325 §4).
            id if self.text_tools_run(id) => {}
            // Çok kollu kılavuz (leader_tools.rs, docs/adr/0326 §5).
            id if self.leader_tools_run(id) => {}
            // Katman süzgeci (layer_filters.rs, docs/adr/0211 §4): the active layer's; Katmanlar's menu names its
            // layer (layering.rs).
            "layer.filter" => return self.open_layer_filter(None),
            "layer.filterFromSelection" => self.filter_from_selection(None),
            "layer.filterClear" => self.clear_layer_filter(None),
            // Nesne ekleri (attachments.rs, layer_info.rs; docs/adr/0225 §2, §5).
            "layer.info" => return self.open_layer_info(None),
            // Veri kümesi bilgisi (dataset_window.rs, docs/adr/0633).
            "layer.dataset" => return self.open_dataset_info(None),
            "attachment.add" | "attachment.addLink" => return self.attachment_command(id),
            // Etiketler (labelling/, docs/adr/0212 §4): the active layer's; Katmanlar's menu names its layer.
            "layer.labels" => self.open_labelling(None),
            // Veri karşılaştır (docs/adr/0179).
            "data.compare" => self.open_data_compare(),
            // Harita servisleri (services/, docs/adr/0208 §14).
            "basemap.remove" => self.remove_basemap(),
            "service.connections" => self.open_connections(None),
            // Bağlı katmanlar (bound.rs, docs/adr/0503): the layer selected in the tree, else the active one.
            "source.write" => return self.bound_event(crate::bound::Event::Write(None)),
            "source.writeTogether" => {
                return self.bound_event(crate::bound::Event::WriteTogether(None));
            }
            "source.read" => return self.bound_event(crate::bound::Event::Read(None)),
            "source.reconcile" => return self.bound_event(crate::bound::Event::Reconcile(None)),
            "source.status" => return self.bound_event(crate::bound::Event::Status),
            "source.unlink" => return self.bound_event(crate::bound::Event::Unlink(None)),
            "source.relink" => return self.bound_event(crate::bound::Event::Relink(None)),
            "service.add" => self.open_service_window(None),
            "service.feed" => self.open_feed_window(),
            "service.info" => self.start_service_info(),
            // Raster ekle and Raster stili (rasters/, docs/adr/0204 §8); Raster oturt is a Hesap window.
            "raster.add" => return self.raster_add_command(),
            // Sanal mozaik (rasters/mosaic.rs, docs/adr/0517 §5).
            "raster.mosaic" => return self.raster_mosaic_command(),
            // STAC'ta ara (rasters/stac.rs, docs/adr/0528 §7).
            "raster.stac" => return self.raster_stac_command(),
            "raster.style" => return self.raster_look_command(),
            // Mesh ekle (rasters/multidim.rs, docs/adr/0243 §11).
            "mesh.add" => return self.mesh_add_command(),
            // Nokta bulutu (pointclouds/, docs/adr/0207 §9).
            "pointcloud.add" => return self.cloud_add_command(),
            "pointcloud.style" => return self.cloud_look_command(),
            "pointcloud.query" => return self.start_tool(kentos_interaction::cloud_query::ID),
            "pointcloud.vpcSave" => return self.cloud_vpc_command(),
            // Kayıtlı ölçüler (docs/adr/0180): the check's window, and the selection's recorded values from the drawing.
            "cogo.check" => self.open_cogo_check(),
            "cogo.update" => {
                self.cogo_update();
            }
            // Genel bakış and Büyüteç over the drawing (docs/adr/0181).
            "view.overview" => self.toggle_overview(),
            "view.magnifier" => self.toggle_magnifier(),
            // Yalıtımı kaldır (docs/adr/0177 §1): what Katmanı yalıt hid, shown again.
            "layer.unisolate" => match &mut self.document {
                Some(doc) => {
                    let (level, text) = kentos_interaction::layer_tools::unisolate(&mut doc.model);
                    self.say(level, text);
                }
                None => self.output("Açık çizim yok."),
            },
            // Bütün katmanları çöz and Katmanları çevir (docs/adr/0311).
            "layer.thawAll" => self.thaw_all_layers(),
            "layer.translate" => self.open_layer_translate(),
            // Standardı denetle and Standardı uygula: one window (docs/adr/0312 §7).
            "standards.check" | "standards.apply" => self.open_drawing_standard(),
            "profile.window" => self.open_data_profile(),
            "network.subnetworks" => self.open_subnetworks(),
            // Olay bağları (event_links_window.rs, docs/adr/0569 §5).
            "linearRef.eventLinks" => self.open_event_links(),
            // Çizgi tipleri (docs/adr/0313 §7).
            "lineTypes.manage" => self.open_line_types(),
            "hatchPatterns.manage" => self.open_hatch_patterns(),
            // Edits, not undo steps (layering.rs).
            "layer.new" => self.new_layer(),
            // A typed layer (layer_types.rs, docs/adr/0622 §5).
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                if let Some((_, g)) = crate::layer_types::NEW_TYPED.iter().find(|(c, _)| *c == id) {
                    self.new_typed_layer(*g);
                }
            }
            "layer.newGroup" => self.new_group(),
            // The blocks (blocks_panel.rs, docs/adr/0144).
            "block.panel" => self.show_blocks_panel(),
            "block.purge" => self.purge_blocks(),
            // Blok düzenleyici (block_editor.rs, docs/adr/0316 §1).
            "block.edit" => self.block_editor_from_selection(),
            // Bloğu dosyaya yaz and Başka çizimden yeniden tanımla (blocks_files.rs, docs/adr/0316 §3).
            "block.writeFile" => {
                if let Some(id) = self.block_at_hand() {
                    return self.blocks_write_file(id);
                }
            }
            "block.redefineFromFile" => {
                if let Some(id) = self.block_at_hand() {
                    return self.blocks_redefine_from(id);
                }
            }
            // Blok çizelgesi, Öznitelikleri eşitle and Blok kartı (docs/adr/0317).
            "block.schedule" => self.open_block_schedule(None),
            // KentOS's own blocks (blocks_files.rs, docs/adr/0318 §4).
            "block.library" => self.blocks_builtin(),
            // Sabit bloğa çevir (docs/adr/0319 §4).
            "block.freeze" => self.blocks_freeze(),
            "dimension.space" => self.dimensions_edit(|uids| {
                let mut uids = uids.into_iter();
                let base = uids.next().unwrap_or_default();
                kentos_commands::DimensionEdit::Space {
                    base,
                    uids: uids.collect(),
                    spacing: None,
                }
            }),
            "dimension.break" => {
                self.dimensions_edit(|uids| kentos_commands::DimensionEdit::Break {
                    uids,
                    objects: None,
                })
            }
            "dimension.unbreak" => {
                self.dimensions_edit(|uids| kentos_commands::DimensionEdit::Unbreak { uids })
            }
            "block.sync" => {
                if let Some(id) = self.block_at_hand() {
                    self.blocks_sync(id, false);
                }
            }
            "block.card" => {
                if let Some(id) = self.block_at_hand() {
                    self.open_block_card(id);
                }
            }
            // Blokları dışa aktar and Blokları al (blocks_files.rs, docs/adr/0314 §3).
            "block.export" => return self.blocks_export(None),
            // Değişikliğin etkisi for the active layer (style_impact.rs, docs/adr/0314 §4).
            "layer.impact" => {
                if let Some(active) = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned())
                {
                    return self.style_impact_event(crate::style_impact::Event::Open(
                        kentos_commands::ImpactTarget::Layer,
                        active,
                    ));
                }
            }
            "block.import" => return self.blocks_import(),
            "block.attributes" => self.open_selected_block_attributes(),
            // Bul ve değiştir (find_replace.rs, docs/adr/0145 §6).
            "text.findReplace" => self.open_find_replace(),
            "edit.undo" => self.undo(),
            "edit.redo" => self.step_history(false),
            "tool.confirm" => return self.confirm(),
            // Esc with nothing to cancel leaves full screen (view_commands.rs).
            "tool.cancel" if self.fullscreen && !self.cancellable() => {
                return self.toggle_fullscreen();
            }
            "tool.cancel" => self.cancel(),
            "tool.repeat" => return self.repeat_last(),
            _ => self.error(format!(
                "{id}: masaüstü işleyicisi eksik (catalog::PORTED ile karşılaştırın)"
            )),
        }
        Task::none()
    }

    /// Undoes or redoes the drawing's last step, saying which (the web's
    /// “Geri alındı: Ekle”); with nothing to undo or redo, says that.
    pub(crate) fn step_history(&mut self, undo: bool) {
        if let Some(problem) = self
            .document
            .as_ref()
            .and_then(|d| d.model.sheet_history_problem(!undo))
        {
            self.warn(problem);
            return;
        }
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        // A step already written to a bound layer's source: the undo is the drawing's until the next write
        // (docs/adr/0598 §4).
        let written = if undo && !doc.model.settings().sources.is_empty() {
            doc.model
                .undo_report()
                .map(|r| kentos_native_application::sources::undoing_written(&doc.model, &r))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let step = if undo {
            doc.model.undo()
        } else {
            doc.model.redo()
        };
        if step.is_some() {
            self.echo_history(undo);
        }
        self.output(match (step, undo) {
            (Some(label), true) if !written.is_empty() => format!(
                "Geri alındı: {label}. Bu adım {} kaynağına yazılmıştı; kaynakta hâlâ duruyor. Kaynağa yaz geri almayı \
                 kaynağa da yazar; o arada kaynakta değişen satır varsa yazma reddedilir.",
                written.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ")
            ),
            (Some(label), true) => format!("Geri alındı: {label}"),
            (Some(label), false) => format!("Yinelendi: {label}"),
            (None, true) => "Geri alınacak değişiklik yok.".to_owned(),
            (None, false) => "Yinelenecek değişiklik yok.".to_owned(),
        });
    }

    /// Whether a ported command can run now: undo with a step to take, in
    /// the drawing or in the running command's draft, redo with one to take
    /// (web: `isEnabled`, `watch: [doc.canUndo, tools.prompt]`). Buttons of
    /// commands that cannot run are drawn dimmed.
    /// A command's on or off state where it has one (the ribbon and its menus show it).
    pub fn checked(&self, id: &str) -> Option<bool> {
        Some(match id {
            "draft.ortho" => self.draft.ortho,
            "view.split" => self.split.is_some(),
            "view.splitLink" => self
                .split
                .as_ref()
                .is_some_and(|s| s.link != kentos_geometry_core::tools::view_link::Link::Off),
            "view.swipe" => self.split.as_ref().is_some_and(|s| s.swipe.is_some()),
            "draft.rightAngle" => self.draft.right_angle,
            "draft.polar" => self.draft.polar.is_some(),
            "draft.snap" => self.draft.snap,
            id if crate::snap_menu::setting(id).is_some() => {
                crate::snap_menu::setting(id).is_some_and(|key| self.settings.bool(key))
            }
            "draft.tracking" => self.draft.tracking,
            "draft.topology" => self.draft.topology,
            "draft.topologyPoints" => self.draft.topology_points,
            "draft.overlap" => self.draft.overlap != kentos_interaction::Overlap::Allow,
            "edit.selectFilter" => self.draft.select_kinds.is_some(),
            "edit.autoSelect" => self.auto_select_on(),
            id if id.starts_with("edit.selectFilter.") => {
                self.select_kind_checked(&id["edit.selectFilter.".len()..])
            }
            id if id.starts_with("arch.phase.") => {
                self.phase_view().name() == &id["arch.phase.".len()..]
            }
            "draft.lock.keep" => self.locks.keep,
            "draft.lock.construction" => self.session.construction(),
            "draft.overlap.allow" => self.draft.overlap == kentos_interaction::Overlap::Allow,
            "draft.overlap.layer" => self.draft.overlap == kentos_interaction::Overlap::Layer,
            "draft.overlap.layers" => self.draft.overlap == kentos_interaction::Overlap::Layers,
            "draft.grid" => self.settings.bool("drafting.grid"),
            "view.lineWeights" => self.settings.bool("graphics.lineWeights"),
            "view.colorMode.color" => self.settings.text("graphics.colorMode") == "color",
            "view.colorMode.mono" => self.settings.text("graphics.colorMode") == "mono",
            "view.colorMode.gray" => self.settings.text("graphics.colorMode") == "gray",
            // The rule the drawing is drawn by: Türe göre's too (docs/adr/0626 §6).
            "view.annotationSize.legible" => {
                matches!(self.label_size(), kentos_interaction::spatial::LabelSize::Legible)
            }
            "view.annotationSize.true" => {
                matches!(self.label_size(), kentos_interaction::spatial::LabelSize::Print)
            }
            "view.annotationSize.screen" => matches!(
                self.label_size(),
                kentos_interaction::spatial::LabelSize::Screen { .. }
            ),
            "view.fills" => self.settings.bool("graphics.fills"),
            "view.areaEdges" => self.settings.bool("graphics.areaEdges"),
            "view.pinnedLabels" => self.settings.bool("graphics.pinnedLabels"),
            "view.unplacedLabels" => self.settings.bool("graphics.unplacedLabels"),
            "view.transparency" => self.settings.bool("graphics.transparency"),
            "view.symbols.plot" => self.settings.text("graphics.symbolSize") != "screen",
            "view.symbols.screen" => self.settings.text("graphics.symbolSize") == "screen",
            "view.theme.dark" => self.mode == Mode::Dark,
            "view.theme.light" => self.mode == Mode::Light,
            "view.bottomPanel" => self.command_expanded,
            crate::catalog::PYTHON_CONSOLE => {
                self.command_expanded && self.bottom_tab == crate::bottom::BottomTab::Python
            }
            "view.rightPanel" => self.right_panel_shown(),
            "view.overview" => self.overview_shown(),
            "view.magnifier" => self.magnifier_shown(),
            "view.fullscreen" => self.fullscreen,
            // Zaman sürgüsü and Mevcut durum (docs/adr/0210 §10).
            "time.slider" => self.time.slider.open,
            "scenario.base" => {
                self.has_scenarios()
                    && self.document.as_ref().is_some_and(|d| {
                        crate::temporal::shown_scenario(d.model.layers())
                            == crate::temporal::Shown::Base
                    })
            }
            id if id.starts_with("workspace.") => {
                crate::catalog::mode_command(self.work_mode()) == id
            }
            _ => return None,
        })
    }

    pub fn available(&self, id: &str) -> bool {
        let doc = self.document.as_ref().map(|doc| &doc.model);
        match id {
            "edit.undo" => {
                doc.is_some_and(kentos_domain::Document::can_undo)
                    || (self.session.is_running() && self.session.point_count() > 0)
            }
            "edit.redo" => doc.is_some_and(kentos_domain::Document::can_redo),
            // The locks work while a point is expected after another (docs/adr/0166 §6).
            "draft.lock.length"
            | "draft.lock.angle"
            | "draft.lock.east"
            | "draft.lock.north"
            | "draft.lock.parallel"
            | "draft.lock.perpendicular"
            | "draft.lock.keep" => self.session.lock_reference().is_some(),
            "draft.lock.reference" | "draft.lock.construction" | "draft.trackPoint" => {
                self.session.takes_points()
            }
            "draft.lock.deflection" => {
                self.session.lock_reference().is_some() && self.session.travel().is_some()
            }
            "draft.lock.clear" => self.session.lock_reference().is_some() && self.locks.any(),
            "edit.deselect" | "view.zoomSelection" => !self.selection.is_empty(),
            // An attachment goes on one object (attachments.rs, docs/adr/0225 §2), as the web's.
            "attachment.add" | "attachment.addLink" => doc.is_some() && self.selection.len() == 1,
            "edit.previousSelection" => !self.previous_selection_ids().is_empty(),
            "edit.cycleSelection" => self.selection.cycle().is_some(),
            // Only while a place is marked (search/, docs/adr/0178 §6).
            "data.unmark" => self.data_mark().is_some() || self.extent_mark().is_some(),
            // Only where there is a view to go to (docs/adr/0141).
            "view.previous" => self.view_history.can_back(),
            "view.next" => self.view_history.can_forward(),
            "view.extentCheck" => doc.is_some(),
            // Only while a ready basemap is at the bottom (docs/adr/0208 §14).
            "basemap.remove" => doc.is_some_and(|d| {
                crate::services::basemaps::bottom_basemap(d.layers().nodes()).is_some()
            }),
            "service.connections" | "service.add" | "service.feed" | "service.info" => {
                doc.is_some()
            }
            // The scenarios' commands while the project has one (docs/adr/0210 §10).
            "scenario.show" | "scenario.base" | "scenario.compare" | "scenario.apply" => {
                self.has_scenarios()
            }
            id if crate::clipboard::COMMANDS.contains(&id) => self.clipboard_available(id),
            "server.check" => !self.server_checking,
            id if id.starts_with("cloud.") => self.cloud_available(id),
            "layer.new" | "layer.newGroup" => self.tree_locked().is_none(),
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                self.tree_locked().is_none()
            }
            "edit.autoSelect" => doc.is_some(),
            // A drawing tool that writes another kind than the typed active layer holds (docs/adr/0622 §5).
            id if self.layer_misfit(id).is_some() => false,
            "block.purge" | "block.export" => doc.is_some_and(|d| !d.blocks().is_empty()),
            "block.import" | "layer.impact" => doc.is_some(),
            "block.attributes" => doc.is_some_and(|d| {
                kentos_interaction::blocks::selected_block(d, &self.selection).is_some()
            }),
            // Katman stili opens for the active layer: not for a group.
            "style.layerStyle" => doc.is_some_and(|d| {
                d.layers()
                    .get(d.layers().active())
                    .is_some_and(|n| n.kind == kentos_commands::LayerNodeType::Layer)
            }),
            // Symbols for the selected objects; taking them away when one has its own.
            "style.assign" => doc.is_some() && !self.selection.is_empty(),
            "template.fromSelection" => doc.is_some() && !self.selection.is_empty(),
            "style.legend" => doc.is_some(),
            "style.clearSymbol" => doc.is_some_and(|d| {
                self.selection
                    .ids()
                    .iter()
                    .any(|&s| d.get(s).is_some_and(|e| e.base().symbol.is_some()))
            }),
            // A project keeps its type (docs/adr/0622 §1): only one not asked its type yet takes one.
            id if id.starts_with("workspace.") => {
                doc.is_some_and(|d| d.settings().project_type().is_none())
            }
            _ => true,
        }
    }

    /// Why a command is off, when it says (the web's `whyDisabled`): its tip
    /// shows it.
    pub fn why_disabled(&self, id: &str) -> Option<String> {
        match id {
            "layer.new" | "layer.newGroup" => self.tree_locked().map(str::to_owned),
            id if crate::layer_types::NEW_TYPED.iter().any(|(c, _)| *c == id) => {
                self.tree_locked().map(str::to_owned)
            }
            id if id.starts_with("workspace.") && !self.available(id) => Some(
                "Proje türü projeyle başlar ve değişmez; CAD ile CBS arasında geçiş ileride dönüştürmeyle olacak."
                    .to_owned(),
            ),
            "scenario.show" | "scenario.base" | "scenario.compare" | "scenario.apply"
                if !self.has_scenarios() =>
            {
                Some("Projede senaryo yok; önce Senaryo oluştur ile bir senaryo yapın.".to_owned())
            }
            // The typed active layer does not hold what the tool draws (docs/adr/0622 §5).
            id => self.layer_misfit(id),
        }
    }

    /// A name typed in the command line: an alias, the command's name or its
    /// title. A tool started from there gets the keyboard for the drawing.
    pub(crate) fn run_typed(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return Task::none();
        }
        // AutoCAD takma adları, when the preference is on (alias_profile.rs, docs/adr/0282):
        // the profile holds no name of KentOS's, which keep their meaning.
        if let Some(task) = self.run_profile_name(text) {
            return task;
        }
        // A tool's method by its own name (DOR: Ölçülendirme as Koordinat; docs/adr/0147 §7),
        // as the ribbon's menu starts it: the tool, then its option.
        if let Some(method) = catalog().method_by_alias(text)
            && let Some(option) = method.option
        {
            self.remember(text);
            let task = self.run_method(method.id, option, method.label);
            if self.session.is_running() {
                self.line_focused = false;
                return Task::batch([task, release_keyboard()]);
            }
            return task;
        }
        let folded = fold(text);
        // A full name first: an alias, a title (docs/adr/0281).
        let named = crate::typed_names::named(text, self.sheet_tab);
        let found = match &named {
            crate::typed_names::Named::One(command, false) => Some(*command),
            _ => None,
        };
        // A model of the user's, by its name (“Parsel ölçüleri”, “Parsel ölçüleri…”).
        let model = found.is_none().then(|| {
            self.user_models()
                .find(|m| {
                    fold(&m.label) == folded
                        || fold(&format!("{}…", m.label)) == folded
                        || text == format!("processing.model.{}", m.id)
                })
                .map(|m| m.id.clone())
        });
        if let Some(Some(id)) = model {
            self.remember(text);
            return self.processing_command(&format!("processing.model.{id}"));
        }
        // The start of one command's name runs it, said; a name or start several share is said with them.
        let found = match named {
            crate::typed_names::Named::One(command, true) => {
                self.output(format!("“{text}”: {}.", command.title));
                Some(command)
            }
            crate::typed_names::Named::Several(commands) => {
                self.error(format!(
                    "“{text}” birden çok komutu anlatıyor: {}. Komutun adını tam yazın.",
                    crate::typed_names::candidates(&commands)
                ));
                return Task::none();
            }
            _ => found,
        };
        match found {
            Some(command) => {
                // As on the web (`CommandLine.run`): the line's ↑ brings it back; the
                // typed name is not said, a tool says its own as it starts.
                self.remember(text);
                let task = self.run(command.id);
                if command.id.starts_with("tool.") && self.session.is_running() {
                    self.line_focused = false;
                    return Task::batch([task, release_keyboard()]);
                }
                task
            }
            None => {
                self.error(format!(
                    "“{text}” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın."
                ));
                Task::none()
            }
        }
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
                    .add_filter("AutoCAD (.dwg, .dxf)", &["dwg", "dxf"])
                    .add_filter("Netcad (.ncz)", &["ncz"])
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
mod tests {
    use super::*;
    use crate::catalog::PORTED;

    #[test]
    fn every_ported_command_has_a_handler() {
        for id in PORTED {
            let (mut app, _) = App::boot(None);
            let _ = app.run(id);
            assert!(
                !app.log.said(Level::Error, "işleyicisi eksik"),
                "{id} is listed as ported but has no handler"
            );
        }
    }

    #[test]
    fn a_command_not_ported_says_so_and_changes_nothing() {
        // One the web runs and the desktop not yet, whichever is left as they are ported.
        let Some(id) = catalog()
            .commands()
            .iter()
            .find(|c| c.standing == Standing::OnTheWeb)
            .map(|c| c.id)
        else {
            return;
        };
        let (mut app, _) = App::boot(None);
        let before = app.log.len();
        let _ = app.run(id);
        assert_eq!(app.log.len(), before + 1);
        assert!(app.log.last().is_some_and(
            |l| l.level == Level::Info && l.text.contains("masaüstüne henüz taşınmadı")
        ));
    }

    fn with_demo() -> App {
        let (mut app, _) = App::boot(None);
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/document/v1/sample.json"
        ))
        .expect("the web's demo file reads");
        app.document = Some(Document::new(snapshot, None).expect("opens"));
        app
    }

    /// A tool's method from its ribbon menu starts the tool and gives it the
    /// method's option, as the web's `runEntry` does (docs/adr/0032).
    #[test]
    fn a_method_from_the_ribbon_starts_its_tool_with_its_option() {
        let mut app = with_demo();
        let _ = app.update(Message::RunMethod {
            id: "tool.circle",
            option: "TTT",
            label: "Teğet, teğet, teğet",
        });
        assert_eq!(app.session.tool_id(), "circle");
        assert_eq!(
            app.session.prompt().text(),
            "Daire: birinci teğet çizgi, yay ya da daireyi seçin"
        );
        let _ = app.update(Message::RunMethod {
            id: "tool.arc",
            option: "M",
            label: "Merkez, başlangıç, bitiş",
        });
        assert_eq!(app.session.tool_id(), "arc");
        assert_eq!(app.session.prompt().text(), "Yay: yayın merkezini belirtin");
        assert!(
            !app.log.said(Level::Warn, "başlatılamadı"),
            "both methods started"
        );

        // With no drawing open the tool does not start; that says why, once.
        let (mut closed, _) = App::boot(None);
        let before = closed.log.len();
        let _ = closed.update(Message::RunMethod {
            id: "tool.circle",
            option: "2N",
            label: "2 nokta",
        });
        assert!(!closed.session.is_running());
        assert_eq!(closed.log.len(), before + 1);
        assert_eq!(
            last_output(&closed),
            "Açık çizim yok. Önce bir çizim açın (Ctrl+O)."
        );
    }

    fn last_output(app: &App) -> &str {
        match app.log.last() {
            Some(line) if line.level == Level::Info || line.level == Level::Success => &line.text,
            other => panic!("expected an output line, found {other:?}"),
        }
    }

    #[test]
    fn undo_and_redo_run_through_the_document_and_say_what_they_did() {
        let mut app = with_demo();
        assert!(!app.available("edit.undo") && !app.available("edit.redo"));
        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alınacak değişiklik yok.");

        // An edit made on the document directly.
        let doc = app.document.as_mut().expect("open");
        let first = doc.model.entities().next().expect("an object").clone();
        let count = doc.entity_count();
        doc.model.add(first).expect("a slot");
        assert!(app.available("edit.undo"));

        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alındı: Ekle");
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.entity_count(), count);
        assert!(doc.dirty(), "an undo is a change to save");
        assert!(app.available("edit.redo") && !app.available("edit.undo"));

        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelendi: Ekle");
        assert_eq!(
            app.document.as_ref().map(Document::entity_count),
            Some(count + 1)
        );
        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelenecek değişiklik yok.");
    }

    #[test]
    fn a_save_that_finishes_after_another_drawing_was_opened_leaves_that_one_alone() {
        let mut app = with_demo();
        let first = app.document.as_ref().map(|doc| doc.session).expect("open");
        // The first drawing's save is still running when another is opened.
        let second = with_demo().document.expect("open");
        assert_ne!(second.session, first);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(second)))));
        app.document.as_mut().expect("open").model.mark_unsaved();
        let _ = app.update(Message::Saved(Some(Ok(Written {
            session: first,
            path: PathBuf::from("ilk.kcad"),
            revision: 0,
        }))));
        let doc = app.document.as_ref().expect("open");
        assert_eq!(
            doc.path, None,
            "Ctrl+S must not write this drawing over the first one's file"
        );
        assert!(doc.dirty());
        assert_eq!(last_output(&app), "Kaydedildi: ilk.kcad.");
    }

    /// Another editor's deletion comes in from outside (docs/adr/0040): the
    /// revision stays, so nothing is to save, but the store, the selection
    /// and the screen follow the generation and let the object go.
    #[test]
    fn changes_from_outside_reach_the_store_and_the_selection() {
        let (mut app, _) = App::boot(None);
        let demo = with_demo().document.expect("open");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(demo)))));
        let doc = app.document.as_ref().expect("open");
        let slot = kentos_domain::Slot(doc.model.entities().next().expect("an object").base().id);
        let uid = doc.model.uid(slot).expect("a persistent id");
        let (revision, objects) = (doc.model.revision(), app.spatial.len());
        assert!(objects > 0, "the store follows the drawing");
        app.selection.set([slot]);
        let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
        assert_eq!(app.selection.len(), 1);

        let doc = app.document.as_mut().expect("open");
        doc.model
            .apply_external(kentos_domain::External {
                remove: vec![uid],
                ..Default::default()
            })
            .expect("taken in");
        let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
        assert!(app.selection.is_empty(), "the selection lets it go");
        assert_eq!(app.spatial.len(), objects - 1, "the store follows");
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.model.revision(), revision);
        assert!(!doc.dirty(), "a change from outside is nothing to save");
    }

    #[test]
    fn layer_changes_are_edits_but_not_undo_steps_and_folding_is_neither() {
        let mut app = with_demo();
        let group = app
            .document
            .as_ref()
            .and_then(|doc| doc.layers().iter().find(|n| !n.children.is_empty()))
            .map(|n| n.id.clone())
            .expect("the demo has a group");
        let _ = app.update(Message::LayerExpanded(group.clone()));
        let doc = app.document.as_ref().expect("open");
        assert!(doc.find(&group).is_some_and(|n| !n.expanded));
        assert!(!doc.dirty(), "folding is not an edit (web)");

        let _ = app.update(Message::LayerVisible(group.clone()));
        let _ = app.update(Message::LayerLocked(group));
        let _ = app.run("layer.showAll");
        let doc = app.document.as_ref().expect("open");
        assert!(doc.dirty());
        assert!(
            !app.available("edit.undo"),
            "visibility and lock are not undo steps (web)"
        );
    }

    #[test]
    fn the_web_keys_undo_and_redo() {
        use iced::Event;
        use keyboard::key::{Code, Physical};
        use keyboard::{Key, Location, Modifiers};
        let chord = |modifiers: Modifiers, letter: &str| {
            let message = keys::key_event(
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: Key::Character(letter.into()),
                    modified_key: Key::Character(letter.into()),
                    physical_key: Physical::Code(Code::KeyZ),
                    location: Location::Standard,
                    modifiers,
                    text: None,
                    repeat: false,
                }),
                event::Status::Ignored,
                window::Id::unique(),
            );
            let Some(Message::Key(press)) = message else {
                panic!("a key press for the router, not {message:?}");
            };
            keys::chord(&press).and_then(|chord| App::boot(None).0.shortcut(&chord))
        };
        assert_eq!(chord(Modifiers::CTRL, "z"), Some("edit.undo"));
        assert_eq!(chord(Modifiers::CTRL, "y"), Some("edit.redo"));
        assert_eq!(
            chord(Modifiers::CTRL | Modifiers::SHIFT, "Z"),
            Some("edit.redo")
        );
    }

    #[test]
    fn typed_names_find_commands_whatever_the_case_and_turkish_letters() {
        let (mut app, _) = App::boot(None);
        app.mode = Mode::Light;
        let _ = app.update(Message::CommandRun("temayı değiştir".into()));
        assert_eq!(app.mode, Mode::Dark, "the title works as a name");
        let _ = app.update(Message::CommandRun("OLMAYAN".into()));
        assert_eq!(
            app.log.last().map(|l| (l.level, l.text.as_str())),
            Some((
                Level::Error,
                "“OLMAYAN” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın."
            ))
        );
        assert_eq!(
            app.typed,
            [Entry::Input("temayı değiştir".into())],
            "an unknown name is not brought back by ↑"
        );
    }

    #[test]
    fn a_typed_tool_is_said_once_by_its_own_name() {
        let mut app = with_demo();
        let before = app.log.len();
        let _ = app.update(Message::CommandRun("çizgi".into()));
        assert_eq!(app.session.tool_id(), "line");
        // The tool's name, once (the web logs the tool, not the typed text).
        assert_eq!(app.log.len(), before + 1);
        assert_eq!(
            app.log.last().map(|l| (l.level, l.text.as_str())),
            Some((Level::Command, "Çizgi"))
        );
        // A command that is not a tool says nothing of what was typed (the web's
        // CommandLine); the line's ↑ brings both back, newest first.
        let _ = app.run("tool.cancel");
        let before = app.log.len();
        let _ = app.update(Message::CommandRun("ZE".into()));
        assert_eq!(app.log.len(), before);
        assert_eq!(
            app.typed,
            [Entry::Input("çizgi".into()), Entry::Input("ZE".into())]
        );
    }

    /// A value or an option given to the running command is echoed as typed
    /// (the web's `› X`), never taken for a command's short name: X is also
    /// Patlat's (docs/adr/0061).
    #[test]
    fn a_value_or_an_option_is_echoed_as_typed() {
        let mut app = with_demo();
        let _ = app.run("tool.dimension");
        assert_eq!(app.session.tool_id(), "dimension");
        let _ = app.prompt_option("D");
        assert_eq!(
            app.log.last().map(|l| (l.level, l.text.as_str())),
            Some((Level::Command, "› D"))
        );
        // The typed point, then the tool's echo of it; the line's ↑ brings back
        // what was typed there, not the option's letter.
        let before = app.log.last_id();
        let _ = app.submit_line("0,0");
        assert_eq!(
            app.log
                .lines()
                .find(|l| l.id > before)
                .map(|l| (l.level, l.text.as_str())),
            Some((Level::Command, "› 0,0"))
        );
        assert_eq!(app.typed, [Entry::Input("0,0".into())]);
        assert_eq!(
            app.session.prompt().text(),
            "Ölçü: ikinci ölçü noktasını belirtin"
        );
    }

    #[test]
    fn a_tool_needs_an_open_drawing_and_says_so() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tool.polygon");
        assert!(!app.session.is_running());
        assert_eq!(
            last_output(&app),
            "Açık çizim yok. Önce bir çizim açın (Ctrl+O)."
        );
        let mut app = with_demo();
        let _ = app.run("tool.polygon");
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(
            app.log.last().map(|l| (l.level, l.text.as_str())),
            Some((Level::Command, "Kapalı alan"))
        );
        // Opening another drawing drops the command and its draft.
        let other = with_demo().document.expect("open");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
        assert!(!app.session.is_running());
    }

    /// The line and polyline tools (docs/adr/0027) start from their web
    /// shortcuts (L, P) and from their names in the command line.
    #[test]
    fn the_line_and_polyline_tools_start_from_their_keys_and_names() {
        use keyboard::key::{Code, Physical};
        use keyboard::{Key, Modifiers};
        let letter = |c: &str, code: Code| {
            Message::Key(KeyPress {
                key: Key::Character(c.into()),
                physical: Physical::Code(code),
                modifiers: Modifiers::empty(),
                text: Some(c.to_owned()),
                repeat: false,
            })
        };
        let mut app = with_demo();
        let _ = app.update(letter("l", Code::KeyL));
        assert_eq!(app.session.tool_id(), "line");
        let _ = app.run("tool.cancel");
        let _ = app.update(letter("p", Code::KeyP));
        assert_eq!(app.session.tool_id(), "polyline");
        for (name, tool) in [
            ("çizgi", "line"),
            ("PL", "polyline"),
            ("coklucizgi", "polyline"),
        ] {
            let _ = app.run("tool.cancel");
            let _ = app.update(Message::CommandRun(name.into()));
            assert_eq!(app.session.tool_id(), tool, "{name}");
        }
    }

    /// While a command runs the command line suggests its options, never
    /// another command: a name typed there goes to the tool, as on the web
    /// (docs/adr/0027), and the draft stays.
    #[test]
    fn a_running_command_owns_what_is_typed() {
        use kentos_ui::widget::command_line::{Suggested, suggested};
        let mut app = with_demo();
        assert!(!app.line_commands().is_empty());
        let _ = app.run("tool.polygon");
        assert!(app.line_commands().is_empty());
        let _ = app.update(Message::Viewport(viewport::Event::Pressed(
            iced::Point::new(10.0, 10.0),
        )));
        assert_eq!(app.session.point_count(), 1);
        // After the first corner the prompt has options; they are still suggested.
        let offered = suggested(&app.line_commands(), app.line_prompt().as_ref(), "Ya");
        assert!(
            matches!(offered.as_slice(), [Suggested::Option { label, .. }] if label == "Yay"),
            "{offered:?}"
        );
        let _ = app.update(Message::CommandInput("KA".into()));
        let _ = app.update(Message::CommandSubmitted);
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(app.session.point_count(), 1, "the draft stays");
        assert!(
            app.log
                .last()
                .is_some_and(|l| l.level == Level::Warn && l.text.contains("“KA” anlaşılamadı"))
        );
    }
}
