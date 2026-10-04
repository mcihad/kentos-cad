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

use kentos_contracts::{ResolveReason, SettingConstraint};
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
    Properties,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Layers => "Katmanlar",
            Panel::Processing => "İşlemler",
            Panel::Blocks => "Bloklar",
            Panel::Properties => "Öznitelikler",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Panel::Layers => Icon::Layers,
            Panel::Processing => crate::icons::from_web(Some("processing")),
            Panel::Blocks => crate::icons::from_web(Some("blocks")),
            Panel::Properties => Icon::Properties,
        }
    }

    fn layout() -> Docks<Panel> {
        let mut docks = Docks::new();
        docks.dock(Panel::Layers, Side::Right);
        // İşlemler and Bloklar share the top slot with Katmanlar, as the web's tabs; Katmanlar in front.
        docks.dock(Panel::Processing, Side::Right);
        docks.dock(Panel::Blocks, Side::Right);
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
    /// Bul ve değiştir (find_replace.rs, docs/adr/0145 §6); the window is `App::find_replace`.
    FindReplace,
    /// Nokta editörü's batch operations (points/batch_view.rs, docs/adr/0153 §5); the window is
    /// `App::points.batch`.
    PointBatch,
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
    Upload(kentos_contracts::ProjectStorage),
    /// The drawing's own cloud project again, from the server.
    Reopen,
    /// The new project the Yeni proje window built (project/new.rs).
    NewProject,
    /// A recent file (start.rs); its path waits in `App::opening_recent`.
    OpenRecent,
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
    /// Köşe tablosu in the Koordinat listesi tab (vertices/, docs/adr/0172).
    Vertices(crate::vertices::Event),
    /// The bottom panel's top edge dragged: the open history's new height.
    BottomResized(f32),
    /// The bottom panel's top edge double-clicked: its first height again.
    BottomReset,
    /// Geçmişi temizle (the bottom panel's tab row).
    HistoryCleared,
    /// A frame of the status bar's message fading, or the moment it goes (message_log.rs).
    LogFrame,
    /// The bottom panel's log list scrolled: whether new lines follow to its end.
    LogScrolled(iced::widget::scrollable::Viewport),
    /// The kept layout is due to be written (layout.rs).
    LayoutSave,
    /// The NTv2 grids the project names, looked for in the device's library (grids.rs).
    GridsLooked(Vec<crate::grids::Looked>),
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
    /// Öznitelikler: an edit or a section toggled (properties/).
    Properties(crate::properties::Event),
    /// Blok oluştur's window (blocks.rs).
    Blocks(crate::blocks::Event),
    /// Bloklar panel (blocks_panel.rs).
    BlocksPanel(crate::blocks_panel::Event),
    /// Blok öznitelikleri's window (block_attributes.rs).
    BlockAttributes(crate::block_attributes::Event),
    /// Blok ekle's Öznitelik değerleri (attribute_values.rs).
    AttributeValues(crate::attribute_values::Event),
    FindReplace(crate::find_replace::Event),
    /// Metin dosyası yerleştir's file: its name and bytes, or none (text_file.rs).
    TextFile(Option<(String, Vec<u8>)>),
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
    PromptChoice(&'static str, &'static str),
    /// Nokta hesapla: a construction over the running command (docs/adr/0083).
    PointCalc(kentos_interaction::point_calc::CalcKind),
    /// A key press no text box captured (keys.rs); routed by ADR 0018.
    Key(KeyPress),
    /// Shift, Ctrl, Alt or the logo key changed (Shift turns ortho over for a click).
    Modifiers(keyboard::Modifiers),
    Dock(docking::Event<Panel>),
    LayerSelected(String),
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
    Appearance(crate::appearance::Event),
    /// The server's answer to `server.check` (view_commands.rs).
    ServerChecked(Result<kentos_contracts::Health, String>),
    /// A layer ticked or unticked on the Çakışma cell's menu (docs/adr/0162 §1).
    OverlapLayer(String),
    /// The Kenet cell's menu: a Karelaj spacing, the settings (snap_menu.rs, docs/adr/0163 §6).
    Snap(crate::snap_menu::Event),
    /// The status bar's scale selector (screen_scale.rs, docs/adr/0165 §5).
    ScreenScale(crate::screen_scale::Event),
    /// İkinci sistem: the project's second coordinate system, how geographic
    /// values are written (second_crs.rs, docs/adr/0167 §1).
    SecondCrs(crate::second_crs::Event),
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
    /// The block windows (blocks.rs, docs/adr/0144).
    pub(crate) blocks: crate::blocks::Blocks,
    /// The Bloklar panel (blocks_panel.rs).
    pub(crate) blocks_panel: crate::blocks_panel::PanelState,
    /// Blok öznitelikleri's window, while it is open or away for a point (block_attributes.rs).
    pub(crate) block_attributes: Option<crate::block_attributes::Window>,
    /// Blok ekle's Öznitelik değerleri, while it asks (attribute_values.rs).
    pub(crate) attribute_values: Option<crate::attribute_values::Window>,
    /// Bul ve değiştir's window, while it is open (find_replace.rs).
    pub(crate) find_replace: Option<crate::find_replace::Window>,
    /// Metin dosyası yerleştir asked for its file: the dialog opens after the update (text_file.rs).
    pub(crate) text_file_wanted: bool,
    pub(crate) text_field_select: bool,
    pub(crate) text_field_release: bool,
    /// Öznitelikler's closed sections, by id, while the app runs (the web's `collapsed`).
    pub(crate) props_closed: std::collections::HashSet<&'static str>,
    /// Öznitelikler for a selection of several objects, as last worked out (properties/).
    pub(crate) properties_cache: crate::properties::PanelCache,
    /// The bottom panel's Noktalar tab: its query and rows (points/, docs/adr/0153).
    pub(crate) points: crate::points::PointsPanel,
    /// Köşe tablosu's rows selected and cell edited (vertices/, docs/adr/0172).
    pub(crate) vertices: crate::vertices::VertexPanel,
    /// The labels the view shows, as last asked of the geometry store (labels.rs).
    pub(crate) label_spots: crate::labels::Spots,
    /// The last left press's object with no command running, and when (a double click edits a text).
    pub(crate) last_click: Option<(kentos_domain::Slot, Instant)>,
    /// The log (message_log.rs): every line said, 500 at most, with its time
    /// and level; the bottom panel lists it, the status bar shows its newest.
    pub(crate) log: crate::log_plan::Log,
    /// What was typed in the command line: its ↑ brings them back.
    pub(crate) typed: Vec<Entry>,
    /// What the log's views follow: the status bar's message, the list's end.
    pub(crate) follow: crate::message_log::Follow,
    /// The layout kept between runs (layout.rs) and the window's size it is shown in.
    pub(crate) layout: crate::layout::Keeper,
    /// The device's NTv2 grid library and the grids looked for (grids.rs, docs/adr/0168 §4).
    pub(crate) grids: crate::grids::Grids,
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
    /// The point the tools got for the cursor, beside the world point under
    /// it then: the snap's or the tracking lock's when there is one. The
    /// status bar shows it while the cursor is still there (the web's
    /// `cursorWorld`, which is the pointer's `world`).
    pub(crate) cursor_point: Option<(kentos_interaction::Vec2, kentos_interaction::Vec2)>,
    /// The drawing (its session) and generation the store and the selection last followed.
    followed: Option<(u64, u64)>,
    /// The value field beside the cursor, while it is open (ADR 0018).
    pub field: Option<Field>,
    /// Drafting aids for new points: ortho, polar tracking, the snap aperture
    /// (the typed settings' `drafting.*`, applied by `apply_settings`), and
    /// the colour new objects take (the ribbon's Renk, ribbon_panels.rs).
    pub draft: Draft,
    /// The line type new objects take, `None` by layer (the ribbon's Tip):
    /// the session's, which no tool reads yet (web). The weight is the
    /// draft's (`Draft::line_weight`), which the tools give new objects.
    pub new_line_type: Option<kentos_contracts::LineType>,
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
    /// The digitizing locks (docs/adr/0166): what holds the next point; the
    /// session's, the tools see them in their context.
    pub locks: kentos_interaction::LockState,
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
    /// The ribbon's key tips while they show (ribbon_keys.rs, docs/adr/0118).
    pub(crate) key_tips: Option<crate::ribbon_keys::KeyTips>,
    /// Alt is down alone: letting it go shows the key tips.
    pub(crate) alt_armed: bool,
    /// The folded ribbon's tab open over the drawing.
    pub(crate) ribbon_peek: bool,
    /// The layer or group Katmanlar → Sil asks about (`Dialog::RemoveLayer`).
    pub removing_layer: Option<String>,
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
    pub(crate) server_health: Option<Result<kentos_contracts::Health, String>>,
    // Sheet layouts (sheets.rs, docs/sheet/design.md §11).
    pub sheets: kentos_sheet_ui::Designer,
    pub(crate) sheet_maps: crate::sheets::SheetMaps,
    /// The contextual Pafta tab is open (a sheet is in front).
    pub(crate) sheet_tab: bool,
    /// Where the books are kept; none in tests and snapshots.
    pub sheet_store: Option<kentos_sheet_ui::Store>,
    pub(crate) sheet_project: Option<kentos_sheet_ui::ProjectKey>,
    pub(crate) sheet_session: Option<u64>,
    pub(crate) sheet_generation: Option<(u64, u64)>,
    pub(crate) sheet_attributes: std::cell::Cell<bool>,
    /// The sheet templates' cloud library (sheet_library.rs, docs/sheet/design.md §13).
    pub(crate) sheet_library: crate::sheet_library::SheetLibrary,
    /// What the sheet mode's tables, coordinate lists and legends were last given for (sheet_inputs.rs).
    pub(crate) sheet_data_key: Option<u64>,
}

impl App {
    /// The shell with settings in memory, opening `path` at once when given:
    /// what tests, snapshots and the trace player start from, never touching
    /// the user's files. `main` opens the real settings ([`App::start`]).
    pub fn boot(path: Option<PathBuf>) -> (Self, Task<Message>) {
        Self::start(path, Settings::memory())
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
            ribbon_search: String::new(),
            ribbon_flash: None,
            ribbon_flash_model: None,
            layers_keyboard: false,
            layer_reveal: None,
            drawing_menu: None,
            snap_once: None,
            text_field: None,
            text_field_focus: false,
            blocks: crate::blocks::Blocks::default(),
            blocks_panel: crate::blocks_panel::PanelState::default(),
            block_attributes: None,
            attribute_values: None,
            find_replace: None,
            text_file_wanted: false,
            text_field_select: false,
            text_field_release: false,
            props_closed: std::collections::HashSet::new(),
            properties_cache: Default::default(),
            points: Default::default(),
            vertices: Default::default(),
            label_spots: Default::default(),
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
            typed: Vec::new(),
            follow: crate::message_log::Follow::default(),
            layout: crate::layout::Keeper::memory(),
            grids: crate::grids::Grids::default(),
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
            sheet_project: None,
            sheet_session: None,
            sheet_generation: None,
            sheet_attributes: std::cell::Cell::new(false),
            sheet_library: crate::sheet_library::SheetLibrary::default(),
            sheet_data_key: None,
            session: Session::new(),
            spatial: Spatial::new(),
            styles: crate::style::Styles::new(),
            memory: Memory::default(),
            selection: Selection::new(),
            clipboard: Clipboard::new(),
            snap: None,
            cursor_point: None,
            followed: None,
            field: None,
            draft: Draft::default(),
            new_line_type: None,
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
            locks: kentos_interaction::LockState::default(),
            overlap_last: kentos_interaction::Overlap::Layer,
            tracking_tool: "",
            tracking_waited: 0,
            dwell_on_time: true,
            processing: crate::processing::Processing::default(),
            builder: None,
            builder_flow: false,
            key_tips: None,
            alt_armed: false,
            ribbon_peek: false,
            removing_layer: None,
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
            // A file dropped on the window goes to the SVG editor while it is open (style/svgedit/).
            if self.styles.svg_editor.is_some() {
                event::listen_with(crate::style::svgedit::dropped)
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
            // The status bar's message: when it goes, and its fades (message_log.rs).
            self.log_subscription(Instant::now()),
            // The kept layout, written after its last change; the window's size.
            self.layout_subscription(),
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
        let task = self.handle(message);
        if locks.is_some_and(|l| l != (self.locks.length, self.locks.toward)) {
            self.repoint();
        }
        self.follow_document();
        // The sheets follow the project and the drawing (sheets.rs).
        self.follow_sheets();
        self.follow_selection_layers();
        // The contextual Seçim tab goes with the selection (the web's `updateContextual`).
        if self.selection.is_empty() {
            self.ribbon_context = false;
        }
        // The key tips follow what the ribbon shows now (ribbon_keys.rs).
        self.refresh_key_tips();
        self.dialog_back();
        let task = Task::batch([
            task,
            self.text_field_tasks(),
            self.blocks_tasks(),
            self.attribute_values_tasks(),
            self.find_replace_tasks(),
            self.text_file_tasks(),
            self.follow_hover(),
            self.follow_tracking(),
            // The grids the project's datum choices name, read into the core (grids.rs).
            self.follow_grids(),
            self.follow_log(Instant::now()),
            // The sheet templates' library follows the sign-in and the connection (sheet_library.rs).
            self.follow_sheet_library(Instant::now()),
        ]);
        self.follow_layout(Instant::now());
        self.cloud_after(Instant::now());
        task
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
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        // What the drawing area's device can draw with, known after its first frame (AA-01).
        self.sync_device();
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
            Message::Run(_) | Message::RunMethod { .. } | Message::SplitChosen { .. }
        ) {
            // A command closes the folded ribbon open over the drawing (the web's).
            if !matches!(message, Message::Run("view.keyTips")) {
                self.ribbon_peek = false;
            }
            self.close_text_field(true);
            self.layers_keyboard = false;
            self.blocks_panel.keyboard = false;
            self.vertices.keyboard = false;
        }
        match message {
            Message::Run(id) => return self.run(id),
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
            Message::Vertices(event) => return self.vertices_event(event),
            Message::BottomResized(height) => self.bottom_dragged(Some(height), Instant::now()),
            Message::BottomReset => self.bottom_dragged(None, Instant::now()),
            Message::HistoryCleared => self.clear_history(),
            Message::LogFrame => self.log_frame(Instant::now()),
            Message::LogScrolled(viewport) => self.log_scrolled(viewport),
            Message::LayoutSave => self.layout.write(Instant::now(), false),
            Message::GridsLooked(looked) => self.grids_looked(looked),
            Message::WindowResized(size) => self.window_resized(size),
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
                    self.vertices.keyboard = false;
                }
            }
            Message::PromptOption(key) => return self.prompt_option(key),
            Message::PromptChoice(key, typed) => return self.prompt_choice(key, typed),
            Message::PointCalc(kind) => return self.start_point_calc(kind),
            Message::Key(press) => return self.key(press),
            Message::Modifiers(modifiers) => {
                self.modifiers = modifiers;
                // Alt let go after a tap alone: the ribbon's key tips (ribbon_keys.rs).
                return self.alt_released(modifiers);
            }
            Message::Dock(event) => {
                self.docks.update(event.clone());
                self.dock_dragged(&event, Instant::now());
            }
            Message::LayerSelected(id) => self.layer_pressed(id),
            Message::LayerSearch(text) => self.layer_search(text),
            Message::LayerSearchDown => return self.layer_search_down(),
            Message::RibbonSearch(text) => self.search_typed(text),
            Message::RibbonSearchRun(index) => return self.search_run(index),
            Message::RibbonSearchReveal(index) => return self.search_reveal(index),
            Message::RibbonFlashEnd(id) => self.search_flash_end(id),
            Message::RibbonModelFlashEnd(id) => self.search_model_flash_end(&id),
            Message::Layer(event) => return self.layer_event(event),
            Message::DrawingMenu(event) => self.drawing_menu_event(event),
            Message::DropLock(length) => self.drop_lock(length),
            Message::TextField(event) => self.text_field_event(event),
            Message::Properties(event) => self.properties_event(event),
            Message::Blocks(event) => self.blocks_event(event),
            Message::BlocksPanel(event) => return self.blocks_panel_event(event),
            Message::BlockAttributes(event) => return self.block_attributes_event(event),
            Message::AttributeValues(event) => self.attribute_values_event(event),
            Message::FindReplace(event) => return self.find_replace_event(event),
            Message::TextFile(file) => self.text_file_given(file),
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
            Message::LayerVisible(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_visible(&id);
                }
            }
            Message::LayerLocked(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_locked(&id);
                }
            }
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document
                    && let Some(expanded) = doc.find(&id).map(|n| !n.expanded)
                {
                    // Folding is kept in the file but is not an edit (web).
                    doc.model.set_layer_expanded(&id, expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(doc))) => {
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
                // Another drawing was opened meanwhile: the file is written, but it is
                // not the open drawing's file, which keeps its path and its state.
                _ => self.output(format!("Kaydedildi: {}.", written.path.display())),
            },
            Message::CloseRequested(window) => {
                // The layout is written now, not 250 ms later.
                self.layout.write(Instant::now(), true);
                // A save stopped by the window closing would leave no file (the previous one
                // stays): it finishes first.
                if let Some(s) = &self.saving {
                    let name = s.name();
                    self.warn(format!(
                        "{name} kaydediliyor; kayıt bitince pencereyi yeniden kapatın."
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
                _ => {}
            },
            Message::DialogClosed => self.close_dialog(),
            Message::Cloud(event) => return self.cloud_event(*event),
            Message::Exchange(event) => return self.exchange_event(*event),
            Message::Project(event) => return self.project_event(*event),
            Message::AppMenu(event) => return self.app_menu_event(event),
            Message::Start(event) => return self.start_event(event),
            Message::Appearance(event) => return self.appearance_event(event),
            Message::Viewport(event) => return self.pointer(event),
            Message::Settings(edit) => return self.settings_edit(edit),
            Message::Opening(event) => return self.opening_event(event),
            Message::Saving(event) => return self.saving_event(event),
            Message::Recovery(event) => return self.recovery_event(event),
            Message::ServerChecked(answer) => self.server_checked(answer),
            Message::OverlapLayer(id) => self.toggle_overlap_layer(id),
            Message::Snap(event) => self.snap_event(event),
            Message::ScreenScale(event) => return self.screen_scale_event(event),
            Message::SecondCrs(event) => self.second_crs_event(event),
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
            geographic: kentos_interaction::second::Notation::parse(&s.text("display.geographic")),
        };
        self.cursor_input = s.bool("drafting.cursorInput");
        self.command_bar = s.bool("drafting.commandBar");
        self.hover_info = s.bool("drafting.hoverInfo");
        self.crosshair = crate::marks::CrosshairSize::parse(&s.text("appearance.crosshair"));
        self.viewport.grid_shown = s.bool("drafting.grid");
        self.apply_appearance();
    }

    /// What the drawing area draws with: the effective sample count and pixel ratio.
    pub fn graphics(&self) -> Graphics {
        Graphics {
            samples: self.settings.number("graphics.msaa").max(1.0) as u32,
            hi_dpi: self.settings.bool("graphics.hiDpi"),
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
    }

    /// The theme chosen from a command: a preference, kept.
    fn choose_theme(&mut self, mode: Mode) {
        let _ = self.appearance_event(crate::appearance::Event::Theme(mode));
    }

    /// Puts a drawing on screen: the one there before is left (its cloud
    /// project's copy written whole and let go), and so are the tool's draft,
    /// the selection and the geometry store, which belong to the drawing they
    /// were made on.
    pub(crate) fn show_document(&mut self, doc: Document) {
        self.close_cloud_project();
        self.cancel();
        self.selected_layer = None;
        self.spatial.reload(&doc.model);
        self.viewport.opened(&doc, self.spatial.extent());
        // The views left belong to the drawing they were left on (docs/adr/0141).
        self.view_history.clear();
        self.styles.follow_project(Some(doc.model.styles()));
        self.selection = Selection::new();
        self.followed = Some((doc.session, doc.model.generation()));
        self.document = Some(doc);
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

        if let Some(tool) = id.strip_prefix("tool.")
            && Session::tools().contains(&tool)
        {
            return self.start_tool(tool);
        }
        // Modele dön (MODEL), as the web's: the sheet mode's own message (docs/adr/0164).
        if id == "sheet.model" {
            if !self.sheets.is_active() {
                self.output("Model zaten önde.");
                return Task::none();
            }
            return self.update(Message::Sheet(kentos_sheet_ui::Message::Model));
        }
        if id.starts_with("cloud.") {
            return self.cloud_command(id);
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
            "style.layerStyle" => self.open_layer_style(None),
            // The style library (style/manager/, docs/adr/0092).
            "style.manager" => return self.open_style_manager(None, None),
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
            crate::catalog::PYTHON_CONSOLE => self.toggle_python(),
            // The navigation commands keep the view they leave (navigation.rs, docs/adr/0141).
            "view.zoomIn" => self.navigating(Self::zoom_in),
            "view.zoomOut" => self.navigating(Self::zoom_out),
            "view.zoomExtents" => self.navigating(Self::zoom_extents),
            "view.zoomSelection" => self.navigating(Self::zoom_selection),
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => return self.open_shortcuts(),
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &mut self.document {
                Some(doc) => doc.model.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            // Edits, not undo steps (layering.rs).
            "layer.new" => self.new_layer(),
            "layer.newGroup" => self.new_group(),
            // The blocks (blocks_panel.rs, docs/adr/0144).
            "block.panel" => self.show_blocks_panel(),
            "block.purge" => self.purge_blocks(),
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
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let step = if undo {
            doc.model.undo()
        } else {
            doc.model.redo()
        };
        self.output(match (step, undo) {
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
            "draft.lock.keep" => self.locks.keep,
            "draft.lock.construction" => self.session.construction(),
            "draft.overlap.allow" => self.draft.overlap == kentos_interaction::Overlap::Allow,
            "draft.overlap.layer" => self.draft.overlap == kentos_interaction::Overlap::Layer,
            "draft.overlap.layers" => self.draft.overlap == kentos_interaction::Overlap::Layers,
            "draft.grid" => self.settings.bool("drafting.grid"),
            "view.lineWeights" => self.settings.bool("graphics.lineWeights"),
            "view.symbols.plot" => self.settings.text("graphics.symbolSize") != "screen",
            "view.symbols.screen" => self.settings.text("graphics.symbolSize") == "screen",
            "view.theme.dark" => self.mode == Mode::Dark,
            "view.theme.light" => self.mode == Mode::Light,
            "view.bottomPanel" => self.command_expanded,
            crate::catalog::PYTHON_CONSOLE => {
                self.command_expanded && self.bottom_tab == crate::bottom::BottomTab::Python
            }
            "view.rightPanel" => self.right_panel_shown(),
            "view.fullscreen" => self.fullscreen,
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
            | "draft.lock.parallel"
            | "draft.lock.perpendicular"
            | "draft.lock.keep" => self.session.lock_reference().is_some(),
            "draft.lock.reference" | "draft.lock.construction" => self.session.takes_points(),
            "draft.lock.deflection" => {
                self.session.lock_reference().is_some() && self.session.travel().is_some()
            }
            "draft.lock.clear" => self.session.lock_reference().is_some() && self.locks.any(),
            "edit.deselect" | "view.zoomSelection" => !self.selection.is_empty(),
            // Only where there is a view to go to (docs/adr/0141).
            "view.previous" => self.view_history.can_back(),
            "view.next" => self.view_history.can_forward(),
            "view.extentCheck" => doc.is_some(),
            id if crate::clipboard::COMMANDS.contains(&id) => self.clipboard_available(id),
            "server.check" => !self.server_checking,
            id if id.starts_with("cloud.") => self.cloud_available(id),
            "layer.new" | "layer.newGroup" => self.tree_locked().is_none(),
            "block.purge" => doc.is_some_and(|d| !d.blocks().is_empty()),
            "block.attributes" => doc.is_some_and(|d| {
                kentos_interaction::blocks::selected_block(d, &self.selection).is_some()
            }),
            // Katman stili opens for the active layer: not for a group.
            "style.layerStyle" => doc.is_some_and(|d| {
                d.layers()
                    .get(d.layers().active())
                    .is_some_and(|n| n.kind == kentos_contracts::LayerNodeType::Layer)
            }),
            // Symbols for the selected objects; taking them away when one has its own.
            "style.assign" => doc.is_some() && !self.selection.is_empty(),
            "style.legend" => doc.is_some(),
            "style.clearSymbol" => doc.is_some_and(|d| {
                self.selection
                    .ids()
                    .iter()
                    .any(|&s| d.get(s).is_some_and(|e| e.base().symbol.is_some()))
            }),
            _ => true,
        }
    }

    /// Why a command is off, when it says (the web's `whyDisabled`): its tip
    /// shows it.
    pub fn why_disabled(&self, id: &str) -> Option<&'static str> {
        match id {
            "layer.new" | "layer.newGroup" => self.tree_locked(),
            _ => None,
        }
    }

    /// A name typed in the command line: an alias, the command's name or its
    /// title. A tool started from there gets the keyboard for the drawing.
    pub(crate) fn run_typed(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return Task::none();
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
        let found = catalog().commands().iter().find(|c| {
            c.aliases.iter().any(|a| fold(a) == folded) || fold(c.title) == folded || c.id == text
        });
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
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return self.start_opening(path, Purpose::File);
        }
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Çizim aç")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| Message::Opening(opening::Event::Picked(path)),
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
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
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
