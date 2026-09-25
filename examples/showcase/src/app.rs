//! Uygulama durumu ve güncelleme mantığı.

use std::path::PathBuf;
use std::time::Duration;

use iced::keyboard::Modifiers;
use iced::{Event, Point, Size, Subscription, Task, Theme, event, keyboard, window};

use kentos_rc::attribute::query::Edit;
use kentos_rc::attribute::{DateTime, Field, FieldKind, ObjectId, Query, Value, text};
use kentos_rc::spatial::model_space::{self, Backdrop, Options};
use kentos_rc::spatial::{
    Bounds, Draft, Feature, FeatureRef, Geometry, Layer, LonLat, Measurement, Selection,
    SelectionMode, Tool, Viewport, feature, format, query,
};
use kentos_rc::theme::typography::{self, Typography};
use kentos_rc::theme::{self, Accent, Mode};
use kentos_rc::widget::command_line::{self, Entry};
use kentos_rc::widget::docking::{self, Docks};
use kentos_rc::widget::floating::{self, Windows};
use kentos_rc::widget::tree_view::{self, Place};
use kentos_rc::widget::{Toast, Toasts, inspector};

use crate::command::{self, Command};
use crate::gallery::{Demo, Gallery, Page};
use crate::import::{self, ImportWizard};
use crate::jobs::{JobKind, Jobs, Outcome};
use crate::layer_tree::{self, LayerTree, NodeId};
use crate::message::{
    AppCommand, Confirmation, CoordinateFormat, DockPanel, Keyword, Message, Pane, Pending,
    QueryPurpose, RECENT_DRAWINGS, RibbonTab, Setting, SizeStep,
};
use crate::properties::{self, LayerProperties};
use crate::sample;
use crate::settings::Settings;
use crate::sheets::Sheets;
use crate::table::{self, TableView};

/// Çizim araçlarının geometri eklediği katman; listenin en üstündedir.
pub const DRAWING_LAYER: usize = 0;

/// Yerel saat dilimi: Türkiye (UTC+3), dakika olarak.
pub const TIME_ZONE: i32 = 180;

/// Pencerenin açılış boyutu.
pub const WINDOW_SIZE: Size = Size::new(1440.0, 900.0);

/// Komut kutusunun giriş kimliği: odaklamak ve komut listesini açmak için.
pub const COMMAND_INPUT: &str = "komut-kutusu";

const INITIAL_CENTER: LonLat = LonLat::new(32.0, 39.0);
const INITIAL_ZOOM: f64 = 6.0;

/// Yakınlaştır/uzaklaştır düğmelerinin adımı.
const ZOOM_STEP: f64 = 0.8;

/// Odaklanırken kenarlarda bırakılan boşluk ve noktalardaki en az
/// yakınlaştırma.
const FOCUS_PADDING: f32 = 140.0;
const FOCUS_ZOOM: f64 = 12.0;

/// Komut geçmişinde tutulan en fazla satır.
const HISTORY_LIMIT: usize = 80;

/// ViewCube'un her karede döndüğü açı (radyan).
const CUBE_SPEED: f32 = 0.012;

/// Arka plandaki işlerin bir adımı.
const JOB_STEP: Duration = Duration::from_millis(100);

/// KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
pub struct Showcase {
    pub(crate) viewport: Viewport,
    pub(crate) layers: Vec<Layer>,
    /// Katmanların gruplar hâlindeki ağacı ve görünürlükleri.
    pub(crate) layer_tree: LayerTree,
    pub(crate) active_layer: usize,
    pub(crate) selection: Selection,
    pub(crate) hover: Option<FeatureRef>,
    /// İmleç model alanındayken altındaki koordinat.
    pub(crate) cursor: Option<LonLat>,
    /// İmlecin model alanındaki son konumu; imleç dışarıdayken durum
    /// çubuğunda soluk gösterilir.
    pub(crate) last_cursor: Option<LonLat>,
    pub(crate) coordinate_format: CoordinateFormat,
    /// Basılı değiştirici tuşlar; tabloda Shift ve Ctrl ile seçim için.
    modifiers: Modifiers,

    pub(crate) inspector: inspector::State,
    /// Haritadan varlık seçimi sürüyorsa ayrıntıları.
    pub(crate) picking: Option<Pick>,
    /// Katmanların tablo ayarları; katmanlarla aynı sırada.
    pub(crate) tables: Vec<TableView>,
    /// Açık "Öznitelikle seç" ya da "Tabloyu filtrele" penceresi.
    pub(crate) query: Option<QueryDialog>,

    pub(crate) tool: Tool,
    pub(crate) measurement: Measurement,
    pub(crate) draft: Draft,
    drawn_count: usize,

    pub(crate) options: Options,
    pub(crate) view_cube: bool,
    pub(crate) cube_rotation: f32,
    pub(crate) mode: Mode,
    /// Vurgu rengi ve harita zemini.
    pub(crate) accent: Accent,
    pub(crate) backdrop: Backdrop,
    /// Yazı ailesi ve boyutu; kütüphanenin genel yazı ayarıyla aynıdır.
    pub(crate) typography: Typography,
    /// Yuvadaki paneller: katmanlar, özellikler, öznitelik tablosu,
    /// görevler.
    pub(crate) docks: Docks<DockPanel>,
    /// Ayarların saklandığı dosya; yoksa ayarlar saklanmaz (testler, ekransız
    /// görüntü).
    settings_path: Option<PathBuf>,

    pub(crate) ribbon_tab: RibbonTab,
    /// Şerit daraltılmış: yalnızca sekmeler görünür.
    pub(crate) ribbon_collapsed: bool,
    /// Hızlı erişim düğmelerinin görünürlüğü (dışa aktar, geri al, tümünü
    /// gör, kısayollar).
    pub(crate) quick: [bool; 4],
    pub(crate) app_menu_open: bool,
    pub(crate) app_menu_hover: Option<AppCommand>,
    pub(crate) help_open: bool,

    pub(crate) command_input: String,
    pub(crate) history: Vec<Entry>,
    /// Komut geçmişi açık mı (F2).
    pub(crate) command_expanded: bool,
    /// Seçenek bekleyen komut (YAZITIPI, PUNTO).
    pub(crate) pending: Option<Pending>,

    pub(crate) gallery: Gallery,

    /// Harita üstündeki kayan araç pencereleri.
    pub(crate) windows: Windows<Pane>,
    /// Koordinata git penceresinde yazılanlar.
    pub(crate) go_to: GoTo,

    /// Haritanın köşesindeki bildirimler.
    pub(crate) toasts: Toasts<Message>,
    /// Son silinen çizimler; bildirimdeki "Geri al" onları geri koyar.
    deleted: Vec<Feature>,
    /// Arka plandaki işler: dışa aktarma, dizin, içe aktarma.
    pub(crate) jobs: Jobs,
    /// Onay bekleyen iş; onay kutusu açıktır.
    pub(crate) confirm: Option<Confirmation>,
    /// Örnek verinin salt okunur olduğunu anlatan şerit görünür mü.
    pub(crate) read_only_notice: bool,
    /// Açık veri içe aktarma sihirbazı.
    pub(crate) import: Option<ImportWizard>,
    /// Açık katman özellikleri penceresi.
    pub(crate) properties: Option<LayerProperties>,
    /// Katmanların kaynağı, katmanlarla aynı sırada.
    pub(crate) sources: Vec<String>,
    /// Model alanı ve düzen (pafta) sekmeleri.
    pub(crate) sheets: Sheets,
    /// Yerinde yeniden adlandırılan düğüm ve yazılan ad.
    pub(crate) renaming: Option<(NodeId, String)>,
    /// Son tıklanan yer bir ağaç (katman ağacı ya da galerideki örnek): F2
    /// seçili düğümü yeniden adlandırır.
    tree_focused: bool,
    /// Dairesel araç menüsü açık mı (Boşluk).
    pub(crate) radial_open: bool,
    /// Düzende cetveller gösteriliyor mu (Ctrl+R).
    pub(crate) rulers: bool,
}

/// Koordinata git penceresinin alanları.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GoTo {
    pub latitude: String,
    pub longitude: String,
}

impl GoTo {
    /// Enlem: boşsa `Ok(None)`, hatalıysa nedeni.
    pub fn latitude(&self) -> Result<Option<f64>, String> {
        degrees(&self.latitude, "Enlem", 90.0)
    }

    pub fn longitude(&self) -> Result<Option<f64>, String> {
        degrees(&self.longitude, "Boylam", 180.0)
    }

    /// İki alan da doğru yazıldıysa konum.
    pub fn location(&self) -> Option<LonLat> {
        match (self.latitude(), self.longitude()) {
            (Ok(Some(lat)), Ok(Some(lon))) => Some(LonLat::new(lon, lat)),
            _ => None,
        }
    }
}

/// Ondalık derece; virgül de ondalık ayırıcı sayılır.
fn degrees(text: &str, name: &str, limit: f64) -> Result<Option<f64>, String> {
    let text = text.trim();

    if text.is_empty() {
        return Ok(None);
    }

    let value: f64 = text
        .replace(',', ".")
        .parse()
        .map_err(|_| format!("{name} ondalık derece olmalı (ör. 39.92)."))?;

    if !value.is_finite() || value.abs() > limit {
        return Err(format!("{name} −{limit} ile {limit} arasında olmalı."));
    }

    Ok(Some(value))
}

/// Haritadan varlık seçimi: nesne inceleyicideki bir başvuru alanının
/// değeri, model alanında hedef katmandan bir öğeye tıklanarak seçilir.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    /// Değeri değişecek öğe.
    pub subject: FeatureRef,
    /// Başvuru alanı.
    pub field: usize,
    /// Seçilecek öğenin katmanı.
    pub target: usize,
    /// İmlecin yanında gösterilen istem.
    pub prompt: String,
}

/// "Öznitelikle seç" ya da "Tabloyu filtrele" penceresi.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryDialog {
    pub purpose: QueryPurpose,
    /// Sorgulanan katman.
    pub layer: usize,
    pub query: Query,
    /// Bulunan öğelerin seçimle birleşme yöntemi; yalnızca seçimde.
    pub mode: SelectionMode,
}

impl Showcase {
    pub fn new() -> Self {
        let mut layers = vec![sample::drawing_layer()];
        layers.extend(sample::layers());

        let feature_count: usize = layers.iter().map(|layer| layer.features.len()).sum();
        let sources = (0..layers.len())
            .map(|index| {
                if index == DRAWING_LAYER {
                    "Bu oturumda çizildi; bellekte".to_owned()
                } else {
                    "Türkiye örnek verisi; bellekte".to_owned()
                }
            })
            .collect();

        let history = vec![
            Entry::Output(format!(
                "KentOS CAD hazır: {} katman, {feature_count} öğe yüklendi.",
                layers.len()
            )),
            Entry::Output(
                "Komut için yazmaya başlayın. Komut kutusunda ↓ bütün komutları, ↑ öncekileri getirir."
                    .to_owned(),
            ),
        ];

        let mut layer_tree = sample::layer_tree();
        layer_tree.selected = Some(NodeId::Layer(DRAWING_LAYER + 1));

        let viewport = Viewport::new(INITIAL_CENTER, INITIAL_ZOOM, Size::new(900.0, 640.0));

        Self {
            viewport,
            layer_tree,
            active_layer: DRAWING_LAYER + 1,
            selection: Selection::new(),
            hover: None,
            cursor: None,
            last_cursor: None,
            coordinate_format: CoordinateFormat::default(),
            modifiers: Modifiers::default(),
            inspector: inspector::State::new(),
            picking: None,
            tables: vec![TableView::default(); layers.len()],
            query: None,
            layers,
            tool: Tool::Select,
            measurement: Measurement::new(),
            draft: Draft::new(),
            drawn_count: 0,
            options: Options::default(),
            view_cube: true,
            cube_rotation: 0.6,
            mode: Mode::Dark,
            accent: Accent::default(),
            backdrop: Backdrop::default(),
            typography: typography::current(),
            docks: DockPanel::layout(),
            settings_path: None,
            ribbon_tab: RibbonTab::Home,
            ribbon_collapsed: false,
            quick: [true; 4],
            app_menu_open: false,
            app_menu_hover: None,
            help_open: false,
            command_input: String::new(),
            history,
            command_expanded: false,
            pending: None,
            gallery: Gallery::default(),
            windows: Windows::new(),
            go_to: GoTo::default(),
            toasts: Toasts::new(),
            deleted: Vec::new(),
            jobs: Jobs::default(),
            confirm: None,
            read_only_notice: false,
            import: None,
            properties: None,
            sources,
            sheets: Sheets::new(viewport),
            renaming: None,
            tree_focused: false,
            radial_open: false,
            rulers: true,
        }
    }

    /// Saklanan ayarlarla açılış: tema ve yazı ayarı dosyadan gelir,
    /// değişiklikler aynı dosyaya yazılır. Yazı ayarı pencere açılmadan
    /// verilmiş olmalıdır.
    pub fn boot(settings: Settings, path: Option<PathBuf>) -> Self {
        Self {
            mode: settings.mode,
            accent: settings.accent,
            backdrop: settings.backdrop,
            typography: typography::current(),
            docks: settings.docks,
            settings_path: path,
            ..Self::new()
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode, self.accent)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![event::listen_with(keyboard_event)];

        if self.view_cube {
            subscriptions.push(iced::time::every(Duration::from_millis(33)).map(|_| Message::Tick));
        }

        // Galerideki zaman çizelgeleri yalnızca oynatılırken ilerletilir.
        if self.ribbon_tab == RibbonTab::Gallery && self.gallery.is_playing() {
            subscriptions.push(
                iced::time::every(Duration::from_millis(16))
                    .map(|now| Message::Gallery(Demo::Tick(now))),
            );
        }

        // Arka plandaki işler yalnızca sürerken ilerletilir.
        if self.jobs.is_busy() {
            subscriptions.push(iced::time::every(JOB_STEP).map(|_| Message::JobTick));
        }

        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModelSpace(event) => self.handle_model_space(event),
            Message::SheetSelected(tab) => self.sheets.select(tab),
            Message::SheetClosed(tab) => self.sheets.close(tab),
            Message::SheetMoved(from, to) => self.sheets.reorder(from, to),
            Message::SheetAdded => self.sheets.add(self.viewport),
            Message::SheetView(event) => self.handle_sheet_view(event),
            Message::SheetGuide(event) => {
                if let Some(sheet) = self.sheets.sheet_mut() {
                    sheet.guides.update(event);
                }
            }
            Message::GuidesCleared => {
                if let Some(sheet) = self.sheets.sheet_mut() {
                    let count = sheet.guides.len();

                    sheet.guides.clear();
                    self.log(format!("{count} kılavuz silindi."));
                }
            }
            Message::RulersToggled => self.rulers = !self.rulers,
            Message::ToolSelected(tool) => self.select_tool(tool),

            Message::ZoomIn => self.zoom(ZOOM_STEP),
            Message::ZoomOut => self.zoom(-ZOOM_STEP),
            Message::FitAll => {
                if let Some(bounds) = feature::visible_bounds(&self.layers) {
                    self.viewport.fit_bounds(bounds, 56.0);
                }
            }
            Message::ResetView => {
                self.viewport.center = INITIAL_CENTER;
                self.viewport.zoom = INITIAL_ZOOM;
            }
            Message::FocusSelection => {
                if let Some(bounds) = bounds_of(&self.layers, self.selection.iter()) {
                    self.viewport.focus(bounds, FOCUS_PADDING, FOCUS_ZOOM);
                }
            }
            Message::FocusFeature(reference) => {
                if let Some(bounds) = bounds_of(&self.layers, [reference]) {
                    self.viewport.focus(bounds, FOCUS_PADDING, FOCUS_ZOOM);
                }
            }

            Message::LayerOpacity(index, opacity) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    layer.opacity = opacity;
                }
            }
            Message::LayerColor(index, color) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    layer.color = color;
                }
            }
            Message::LayerStroke(index, width) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    layer.stroke_width = width;
                }
            }
            Message::LayerActivated(index) => self.activate_layer(index),
            Message::ZoomToLayer(index) => {
                self.activate_layer(index);
                self.zoom_to_node(NodeId::Layer(index));
            }
            Message::ShowAllLayers => self.set_all_layers_visible(true),
            Message::HideAllLayers => self.set_all_layers_visible(false),

            Message::TreeSelected(node) => {
                self.layer_tree.selected = Some(node);
                self.tree_focused = true;

                if let Some(layer) = node.layer() {
                    self.active_layer = layer;
                }
            }
            Message::TreeToggled(node) => self.layer_tree.toggle(node),
            Message::TreeMoved(from, to, place) => self.move_entry(from, to, place),
            Message::LayerLocked(layer) => {
                if let Some(locked) = self.layer_tree.locked.get_mut(layer) {
                    *locked = !*locked;
                    let locked = *locked;
                    let name = self.layer_name(layer);

                    self.log(if locked {
                        format!("{name} kilitlendi: öğeleri eklenmez, silinmez, düzenlenmez.")
                    } else {
                        format!("{name} kilidi açıldı.")
                    });
                }
            }
            Message::LayerSelectable(layer) => {
                if let Some(selectable) = self.layer_tree.selectable.get_mut(layer) {
                    *selectable = !*selectable;
                    let selectable = *selectable;
                    let name = self.layer_name(layer);

                    self.log(if selectable {
                        format!("{name} yeniden seçilebilir.")
                    } else {
                        format!("{name} seçilemez: haritada tıklanınca seçilmez.")
                    });
                }
            }
            Message::RenameStarted(node) => return self.start_rename(node),
            Message::RenameInput(text) => {
                if let Some((_, buffer)) = &mut self.renaming {
                    *buffer = text;
                }
            }
            Message::RenameSubmitted => self.finish_rename(),
            Message::RenameCancelled => self.cancel_rename(),
            Message::F2Pressed => {
                // Son tıklanan ağaçtaki seçili düğüm adlandırılır: galeride
                // örnek ağaç, çalışma alanında katman ağacı.
                let gallery = self.ribbon_tab == RibbonTab::Gallery;

                if self.tree_focused
                    && gallery
                    && self.gallery.page == Page::Data
                    && let Some(row) = self.gallery.outline_selected
                {
                    return self.update(Message::Gallery(Demo::OutlineRename(row)));
                }

                if self.tree_focused
                    && !gallery
                    && let Some(node @ (NodeId::Group(_) | NodeId::Layer(_))) =
                        self.layer_tree.selected
                {
                    return self.start_rename(node);
                }

                self.command_expanded = !self.command_expanded;
            }
            Message::RadialOpened => {
                // Komut yazılırken ve pencere ya da menü açıkken açılmaz.
                let blocked = self.app_menu_open
                    || self.query.is_some()
                    || self.help_open
                    || self.confirm.is_some()
                    || self.import.is_some()
                    || self.properties.is_some();

                if self.ribbon_tab != RibbonTab::Gallery
                    && self.command_input.is_empty()
                    && !blocked
                {
                    self.radial_open = true;
                }
            }
            Message::RadialClosed => self.radial_open = false,
            Message::TreeChecked(node, checked) => self.check_node(node, checked),
            Message::TreeExpandAll(group, expanded) => self.layer_tree.expand_all(group, expanded),
            Message::ShowOnly(node) => self.show_only(node),
            Message::SublayersShown(index, visible) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    for sublayer in &mut layer.sublayers {
                        sublayer.visible = visible;
                    }
                }
            }
            Message::ZoomToNode(node) => self.zoom_to_node(node),
            Message::SelectNode(node) => self.select_node(node),
            Message::OpenTable(index) => {
                self.activate_layer(index);
                self.docks.show(DockPanel::Table, DockPanel::Table.side());
            }
            Message::ClearDrawings => {
                if self
                    .layers
                    .get(DRAWING_LAYER)
                    .is_some_and(|layer| !layer.features.is_empty())
                {
                    self.confirm = Some(Confirmation::ClearDrawings);
                }
            }

            Message::SelectFeature(reference, mode) => {
                self.selection.apply(mode, [reference]);

                if mode == SelectionMode::Add {
                    self.selection.focus(reference);
                }

                self.selection_changed(true);
            }
            Message::CenterAt(location) => self.viewport.center = location,
            Message::CopyCoordinates(location) => {
                let text = format::decimal(location);
                self.log(format!("Koordinat panoya kopyalandı: {text}"));
                self.toasts
                    .push(Toast::info("Koordinat kopyalandı").body(text.clone()));

                return iced::clipboard::write(text);
            }
            Message::CopyRow(reference) => {
                if let Some(text) = self.row_text(reference) {
                    self.log("Satır panoya kopyalandı (sekmeyle ayrılmış).");
                    self.toasts.push(
                        Toast::info("Satır kopyalandı")
                            .body("Değerler sekmeyle ayrılmış; tabloya yapıştırılabilir."),
                    );

                    return iced::clipboard::write(text);
                }
            }

            Message::SelectAll => self.select_all(),
            Message::InvertSelection => self.invert_selection(),
            Message::ClearSelection => {
                self.selection.clear();
                self.sync_inspector();
            }
            Message::DeleteSelection => self.delete_selection(),
            Message::ClearMeasurement => self.measurement.clear(),
            Message::SelectionStep(forward) => {
                self.selection.step(forward);
                self.selection_changed(true);
            }
            Message::Inspector(event) => {
                if let Some(action) = self.inspector.update(event) {
                    match action {
                        inspector::Action::Change { id, value } => {
                            if self.editable(self.selection.primary().map(|item| item.layer)) {
                                self.set_attribute(id, value);
                            }
                        }
                        inspector::Action::Pick(field) => self.start_pick(field),
                        inspector::Action::CancelPick => self.cancel_pick(),
                        inspector::Action::Navigate { id, object } => self.navigate(id, object),
                    }
                }
            }

            Message::TableToggled => {
                self.docks.toggle(DockPanel::Table, DockPanel::Table.side());
                self.save_settings();
            }
            Message::TableRowPressed(reference) => {
                self.tree_focused = false;
                self.press_row(reference);
            }
            Message::TableSearch(search) => {
                if let Some(table) = self.tables.get_mut(self.active_layer) {
                    table.search = search;
                }
            }
            Message::TableSelectedOnly => {
                if let Some(table) = self.tables.get_mut(self.active_layer) {
                    table.selected_only = !table.selected_only;
                }
            }
            Message::TableSort(column) => {
                if let Some(table) = self.tables.get_mut(self.active_layer) {
                    table.sort_by(column);
                }
            }
            Message::FilterCleared => {
                if let Some(table) = self.tables.get_mut(self.active_layer) {
                    table.filter = Query::default();
                    self.log(format!(
                        "{} tablosunun filtresi kaldırıldı.",
                        self.active_layer_name()
                    ));
                }
            }

            Message::QueryOpened(purpose) => self.open_query(purpose, self.active_layer),
            Message::QueryOpenedFor(purpose, layer) => self.open_query(purpose, layer),
            Message::QueryLayerSelected(index) => {
                if let Some(dialog) = &mut self.query
                    && dialog.layer != index
                    && let Some(layer) = self.layers.get(index)
                {
                    dialog.layer = index;
                    dialog.query = starter_query(&layer.schema);
                }
            }
            Message::QueryEdited(edit) => {
                if let Some(dialog) = &mut self.query
                    && let Some(layer) = self.layers.get(dialog.layer)
                {
                    dialog.query.apply(edit, &layer.schema);
                }
            }
            Message::QueryModeSelected(mode) => {
                if let Some(dialog) = &mut self.query {
                    dialog.mode = mode;
                }
            }
            Message::QueryApplied => self.apply_query(),
            Message::QueryClosed => self.query = None,

            Message::Toggle(setting) => {
                let enabled = !self.setting(setting);
                self.set_setting(setting, enabled);
                self.log(format!(
                    "{} {}.",
                    setting.label(),
                    if enabled { "açık" } else { "kapalı" }
                ));
            }
            Message::ToggleTheme => self.set_mode(self.mode.toggled()),

            Message::RibbonTabSelected(tab) => {
                self.ribbon_tab = tab;
                self.ribbon_collapsed = false;

                // Galeride model alanı görünmez; imleç bilgisi eskimesin,
                // haritadan seçim de sürmesin.
                if tab == RibbonTab::Gallery {
                    self.cursor = None;
                    self.hover = None;
                    self.cancel_pick();
                }
            }
            Message::RibbonCollapsed => self.ribbon_collapsed = !self.ribbon_collapsed,
            Message::QuickToggled(index) => {
                if let Some(shown) = self.quick.get_mut(index) {
                    *shown = !*shown;
                }
            }
            Message::AppMenuToggled => {
                self.app_menu_open = !self.app_menu_open;
                self.app_menu_hover = None;
            }
            Message::AppMenuHovered(command) => self.app_menu_hover = Some(command),
            Message::AppCommandPressed(command) if command.has_submenu() => {
                self.app_menu_hover = Some(command);
            }
            Message::AppCommandPressed(command) => {
                self.app_menu_open = false;
                self.run_app_command(command);
            }
            Message::ExportPressed(format) => {
                self.app_menu_open = false;

                let features = self
                    .layers
                    .iter()
                    .map(|layer| layer.features.len())
                    .sum::<usize>();
                self.jobs.push(JobKind::Export(format), features as u32);
                self.log(format!(
                    "{} olarak dışa aktarma başladı; ilerleme durum çubuğunda.",
                    format
                ));
            }
            Message::RecentPressed(index) => {
                self.app_menu_open = false;

                if let Some((name, _, _)) = RECENT_DRAWINGS.get(index) {
                    self.log(if index == 0 {
                        format!("{name} zaten açık.")
                    } else {
                        format!("{name} açılamadı: dosya işlemleri bu sürümde henüz yok.")
                    });
                }
            }
            Message::HelpToggled => self.help_open = !self.help_open,
            Message::CommandListRequested => {
                self.command_input.clear();

                return command_line::show_commands(COMMAND_INPUT);
            }

            Message::GalleryPageSelected(page) => self.gallery.page = page,
            Message::Gallery(demo) => {
                if let Demo::Notify(index) = demo {
                    let samples = crate::gallery::sample_toasts();

                    match samples.into_iter().nth(index) {
                        Some((_, toast)) => {
                            self.toasts.push(toast);
                        }
                        None => {
                            for (_, toast) in crate::gallery::sample_toasts() {
                                self.toasts.push(toast);
                            }
                        }
                    }
                } else {
                    let rename = matches!(demo, Demo::OutlineRename(_));

                    if matches!(demo, Demo::OutlineSelected(_)) {
                        self.tree_focused = true;
                    }

                    if let Some(output) = self.gallery.update(demo) {
                        self.log(output);
                    }

                    if rename && self.gallery.outline_renaming.is_some() {
                        return focus_rename();
                    }
                }
            }

            Message::CommandInput(value) => self.command_input = value,
            Message::CommandSubmitted => {
                let input = std::mem::take(&mut self.command_input);

                return self.submit(input.trim());
            }
            Message::CommandRun(name) => {
                self.command_input.clear();

                return self.submit(&name);
            }
            Message::CommandTyped(text) => {
                self.tree_focused = false;

                // Pencere ya da menü açıkken yazılanlar komut kutusuna gitmez.
                if self.app_menu_open
                    || self.query.is_some()
                    || self.help_open
                    || self.confirm.is_some()
                    || self.import.is_some()
                    || self.properties.is_some()
                {
                    return Task::none();
                }

                self.command_input.push_str(&text);

                return Task::batch([
                    iced::widget::operation::focus(COMMAND_INPUT),
                    iced::widget::operation::move_cursor_to_end(COMMAND_INPUT),
                ]);
            }
            Message::CommandHistoryToggled => self.command_expanded = !self.command_expanded,
            Message::Keyword(keyword) => self.keyword(keyword),

            Message::ThemeSelected(mode) => {
                if mode != self.mode {
                    self.set_mode(mode);
                }
            }
            Message::AccentChanged(accent) => {
                self.pending = None;

                if accent != self.accent {
                    self.accent = accent;
                    self.log(format!("Vurgu rengi: {}.", accent.name()));
                    self.save_settings();
                }
            }
            Message::BackdropChanged(backdrop) => {
                self.pending = None;

                if backdrop != self.backdrop {
                    self.backdrop = backdrop;
                    self.log(format!("Harita zemini: {}.", backdrop.name()));
                    self.save_settings();
                }
            }
            Message::TypographyChanged(typography) => self.set_typography(typography),
            Message::TextSize(step) => {
                let size = match step {
                    SizeStep::Larger => self.typography.size + 1.0,
                    SizeStep::Smaller => self.typography.size - 1.0,
                    SizeStep::Default => Typography::DEFAULT.size,
                };

                self.set_typography(Typography {
                    size,
                    ..self.typography
                });
            }

            Message::Window(event) => {
                let closed = event == floating::Event::Closed(Pane::Measure);

                self.windows.update(event);

                // Ölçüm penceresi Ölç aracının kendisidir: kapatmak araçtan
                // çıkar.
                if closed && self.tool == Tool::Measure {
                    self.select_tool(Tool::Select);
                }
            }
            Message::PaneToggled(pane) => self.toggle_pane(pane),
            Message::StyleOpened(index) => {
                self.activate_layer(index);
                self.open_pane(Pane::Style);
            }
            Message::GoToLatitude(text) => self.go_to.latitude = text,
            Message::GoToLongitude(text) => self.go_to.longitude = text,
            Message::GoToCentered => {
                if let Some(location) = self.go_to.location() {
                    self.viewport.center = location;
                    self.log(format!(
                        "Görünüm {} noktasına ortalandı.",
                        format::decimal(location)
                    ));
                }
            }
            Message::GoToPlaced => {
                if let Some(location) = self.go_to.location()
                    && self.tool.takes_points()
                {
                    self.handle_model_space(model_space::Event::PointPicked(location));
                }
            }
            Message::CopyMeasurement => {
                if !self.measurement.is_empty() {
                    self.log("Ölçüm panoya kopyalandı.");
                    self.toasts
                        .push(Toast::info("Ölçüm kopyalandı").body(format!(
                            "{} kenar, toplam {}",
                            self.measurement.segment_count(),
                            format::distance(self.measurement.total_meters())
                        )));

                    return iced::clipboard::write(self.measurement_text());
                }
            }
            Message::ToastClosed(id) => {
                self.toasts.dismiss(id);
            }
            Message::UndoDelete => self.undo_delete(),

            Message::JobTick => self.step_jobs(),
            Message::JobCancelled(id) => {
                if let Some((title, detail)) =
                    self.jobs.cancel(id).map(|job| (job.title(), job.detail()))
                {
                    self.log(format!("{title}: iptal edildi."));
                    self.toasts
                        .push(Toast::info(format!("{title} iptal edildi")).body(detail));
                }
            }
            Message::JobRetried(id) => {
                if self.jobs.retry(id) {
                    self.log("İş yeniden sıraya kondu.");
                }
            }
            Message::JobDismissed(id) => self.jobs.dismiss(id),
            Message::JobsCleared => self.jobs.clear_finished(),
            Message::IndexRequested => {
                self.jobs.push(JobKind::Index, 28);
                self.log("Uzamsal dizin oluşturuluyor.");
            }

            Message::Dock(event) => {
                // Sürüklerken gelen boyut ve konumlar, bırakılınca gelen
                // `Settled` ile saklanır.
                let live = matches!(
                    event,
                    docking::Event::Resized(..)
                        | docking::Event::Shared(..)
                        | docking::Event::Placed(..)
                );

                self.docks.update(event);

                if !live {
                    self.save_settings();
                }
            }
            Message::PanelToggled(panel) => {
                self.docks.toggle(panel, panel.side());
                self.save_settings();
            }
            Message::PanelShown(panel) => {
                self.docks.show(panel, panel.side());
                self.save_settings();
            }

            Message::CoordinateFormatSelected(format) => self.coordinate_format = format,
            Message::ScaleSelected(denominator) => {
                self.viewport.set_scale(denominator);
                self.log(format!(
                    "Ölçek 1:{}.",
                    format::integer(self.viewport.scale_denominator())
                ));
            }

            Message::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            Message::Escape => self.escape(),
            Message::Tick => {
                self.cube_rotation = (self.cube_rotation + CUBE_SPEED) % std::f32::consts::TAU;
            }
            Message::Quit => {
                // Kaydedilmemiş çizim varken çıkış onay ister.
                if self
                    .layers
                    .get(DRAWING_LAYER)
                    .is_some_and(|layer| !layer.features.is_empty())
                {
                    self.app_menu_open = false;
                    self.confirm = Some(Confirmation::Quit);
                } else {
                    return window::latest().and_then(window::close);
                }
            }
            Message::ConfirmAccepted => match self.confirm.take() {
                Some(Confirmation::ClearDrawings) => self.clear_drawings(),
                Some(Confirmation::Quit) => return window::latest().and_then(window::close),
                None => {}
            },
            Message::ConfirmCancelled => self.confirm = None,
            Message::EnterPressed => {
                if self.confirm.is_some() {
                    return self.update(Message::ConfirmAccepted);
                }

                if self.import.is_some() {
                    self.import_next();
                }
            }
            Message::BannerDismissed => self.read_only_notice = false,

            Message::ImportOpened => {
                self.import = Some(ImportWizard::default());
                self.app_menu_open = false;
            }
            Message::ImportSource(source) => {
                if let Some(wizard) = &mut self.import {
                    wizard.select(source);
                }
            }
            Message::ImportX(column) => {
                if let Some(wizard) = &mut self.import {
                    wizard.x = Some(column);
                }
            }
            Message::ImportY(column) => {
                if let Some(wizard) = &mut self.import {
                    wizard.y = Some(column);
                }
            }
            Message::ImportSystem(system) => {
                if let Some(wizard) = &mut self.import {
                    wizard.system = system;
                }
            }
            Message::ImportName(name) => {
                if let Some(wizard) = &mut self.import {
                    wizard.name = name;
                }
            }
            Message::ImportBack => {
                if let Some(wizard) = &mut self.import {
                    wizard.step = wizard.step.saturating_sub(1);
                }
            }
            Message::ImportNext => self.import_next(),
            Message::ImportClosed => self.import = None,

            Message::PropertiesOpened(index) => {
                if let Some(layer) = self.layers.get(index) {
                    let visible = self.layer_tree.visible.get(index).copied().unwrap_or(true);

                    self.properties = Some(LayerProperties::new(
                        index,
                        properties::Draft::of(layer, visible),
                    ));
                }
            }
            Message::PropertiesSection(section) => {
                if let Some(properties) = &mut self.properties {
                    properties.section = section;
                }
            }
            Message::PropertiesEdited(edit) => {
                if let Some(properties) = &mut self.properties {
                    properties.edit(edit);
                }
            }
            Message::PropertiesApplied => {
                self.apply_properties();
            }
            Message::PropertiesAccepted => {
                if self.apply_properties() {
                    self.properties = None;
                }
            }
            Message::PropertiesClosed => self.properties = None,
        }

        Task::none()
    }

    // --- Model alanı -----------------------------------------------------

    /// Düzendeki harita çerçevesi yalnızca gezinir: seçim ve çizim model
    /// alanındadır. İmlecin koordinatı durum çubuğunda yine görünür.
    fn handle_sheet_view(&mut self, event: model_space::Event) {
        let Some(sheet) = self.sheets.sheet_mut() else {
            return;
        };

        match event {
            model_space::Event::Resized(size) => {
                sheet.viewport.size = size;

                // Yeni paftanın çerçevesi görünür katmanlara sığdırılır.
                if !sheet.fitted {
                    sheet.fitted = true;

                    if let Some(bounds) = feature::visible_bounds(&self.layers) {
                        sheet.viewport.fit_bounds(bounds, 12.0);
                    }
                }
            }
            model_space::Event::Panned { delta, cursor } => {
                sheet.viewport.pan_by(delta);
                self.cursor = Some(cursor);
                self.last_cursor = Some(cursor);
            }
            model_space::Event::Zoomed { delta, anchor } => sheet.viewport.zoom_by(delta, anchor),
            model_space::Event::CursorMoved(location) => {
                self.cursor = Some(location);
                self.last_cursor = Some(location);
            }
            model_space::Event::CursorLeft => self.cursor = None,
            model_space::Event::Clicked { .. }
            | model_space::Event::BoxSelected { .. }
            | model_space::Event::PointPicked(_)
            | model_space::Event::Finished => {}
        }
    }

    fn handle_model_space(&mut self, event: model_space::Event) {
        match event {
            model_space::Event::Resized(size) => self.viewport.size = size,
            model_space::Event::CursorMoved(location) => {
                self.cursor = Some(location);
                self.last_cursor = Some(location);
                self.hover = self.hit(self.viewport.project(location));
            }
            model_space::Event::CursorLeft => {
                self.cursor = None;
                self.hover = None;
            }
            model_space::Event::Panned { delta, cursor } => {
                self.viewport.pan_by(delta);
                self.cursor = Some(cursor);
                self.last_cursor = Some(cursor);
            }
            model_space::Event::Zoomed { delta, anchor } => self.viewport.zoom_by(delta, anchor),
            model_space::Event::Clicked {
                location,
                modifiers,
            } => {
                let point = self.viewport.project(location);

                if self.picking.is_some() {
                    self.complete_pick(point);
                } else {
                    self.click_select(point, modifiers);
                }
            }
            model_space::Event::BoxSelected {
                bounds,
                crossing,
                modifiers,
            } => {
                if self.picking.is_none() {
                    self.box_select(bounds, crossing, modifiers);
                }
            }
            model_space::Event::PointPicked(location) => {
                if self.tool == Tool::Measure {
                    self.measurement.push(location);
                } else if let Some(geometry) = self.draft.push(self.tool, location, &self.viewport)
                {
                    self.commit_drawing(geometry);
                }
            }
            model_space::Event::Finished => {
                if self.tool == Tool::Measure {
                    self.measurement.clear();
                } else {
                    self.finish_draft();
                }
            }
        }
    }

    /// İmlecin altındaki öğe; haritadan seçim sürerken yalnızca hedef
    /// katmanda aranır. Seçilemeyen katmanların öğeleri sayılmaz.
    fn hit(&self, point: Point) -> Option<FeatureRef> {
        match &self.picking {
            Some(pick) => query::hit_test_in(&self.layers, pick.target, &self.viewport, point),
            None => query::hit_test(&self.layers, &self.viewport, point)
                .filter(|hit| self.is_selectable(hit.layer)),
        }
    }

    fn is_selectable(&self, layer: usize) -> bool {
        self.layer_tree
            .selectable
            .get(layer)
            .copied()
            .unwrap_or(true)
    }

    /// Seçimde seçilemeyen katmanlar gizliymiş gibi aranır: altlarındaki
    /// seçilebilir öğe bulunur.
    fn with_selectable<R>(&mut self, search: impl FnOnce(&[Layer]) -> R) -> R {
        let hidden: Vec<usize> = (0..self.layers.len())
            .filter(|layer| self.layers[*layer].visible && !self.is_selectable(*layer))
            .collect();

        for layer in &hidden {
            self.layers[*layer].visible = false;
        }

        let result = search(&self.layers);

        for layer in hidden {
            self.layers[layer].visible = true;
        }

        result
    }

    /// Katmanın öğeleri değiştirilebilir mi; kilitliyse nedenini yazar.
    fn editable(&mut self, layer: Option<usize>) -> bool {
        let Some(layer) = layer else {
            return true;
        };

        if self.layer_tree.locked.get(layer).copied().unwrap_or(false) {
            let name = self.layer_name(layer);
            self.log(format!("{name} kilitli; değişiklik yapılmadı."));
            return false;
        }

        true
    }

    /// Katmanın adı.
    fn layer_name(&self, layer: usize) -> String {
        self.layers
            .get(layer)
            .map_or_else(String::new, |layer| layer.name.clone())
    }

    /// Ağaçta sürüklenip bırakılan girdiyi taşır.
    fn move_entry(&mut self, from: usize, to: usize, place: Place) {
        let (entry, target) = (
            layer_tree::Entry::from_key(from),
            layer_tree::Entry::from_key(to),
        );
        let name = |tree: &Self, entry: layer_tree::Entry| match entry {
            layer_tree::Entry::Group(group) => tree.layer_tree.groups[group].name.clone(),
            layer_tree::Entry::Layer(layer) => tree.layer_name(layer),
        };

        if self.layer_tree.move_entry(entry, target, place) {
            self.sync_visibility();

            let (moved, onto) = (name(self, entry), name(self, target));
            self.log(match place {
                Place::Into => format!("{moved}, {onto} grubuna taşındı."),
                Place::Before => format!("{moved}, {onto} önüne taşındı."),
                Place::After => format!("{moved}, {onto} ardına taşındı."),
            });
        }
    }

    /// Grubu ya da katmanı yerinde yeniden adlandırmaya başlar; kutu
    /// odaklanır.
    fn start_rename(&mut self, node: NodeId) -> Task<Message> {
        let name = match node {
            NodeId::Group(group) => self
                .layer_tree
                .groups
                .get(group)
                .map(|group| group.name.clone()),
            NodeId::Layer(layer) => self.layers.get(layer).map(|layer| layer.name.clone()),
            NodeId::Sublayer(..) => None,
        };

        let Some(name) = name else {
            return Task::none();
        };

        self.layer_tree.selected = Some(node);
        self.renaming = Some((node, name));

        focus_rename()
    }

    /// Mini araç çubuğunun yeri: seçimin model alanındaki kutusu. Seç
    /// aracında, çizim ya da haritadan seçim sürmüyorken vardır.
    pub(crate) fn selection_anchor(&self) -> Option<iced::Rectangle> {
        if self.tool != Tool::Select || !self.draft.is_empty() || self.picking.is_some() {
            return None;
        }

        let bounds = bounds_of(&self.layers, self.selection.iter())?;
        let south_west = self.viewport.project(bounds.south_west);
        let north_east = self.viewport.project(bounds.north_east);

        Some(iced::Rectangle::new(
            Point::new(
                south_west.x.min(north_east.x),
                south_west.y.min(north_east.y),
            ),
            Size::new(
                (north_east.x - south_west.x).abs(),
                (north_east.y - south_west.y).abs(),
            ),
        ))
    }

    /// Seçimdeki çizimler silinebilir mi: Çizimler katmanında ve katman
    /// kilitli değil.
    pub(crate) fn can_delete_selection(&self) -> bool {
        self.selection
            .iter()
            .any(|item| item.layer == DRAWING_LAYER)
            && !self
                .layer_tree
                .locked
                .get(DRAWING_LAYER)
                .copied()
                .unwrap_or(false)
    }

    /// Yeniden adlandırmadan vazgeçer; ad değişmez.
    fn cancel_rename(&mut self) {
        self.gallery.outline_renaming = None;

        if self.renaming.take().is_some() {
            self.log("Yeniden adlandırma iptal edildi.");
        }
    }

    /// Yazılan adı uygular; boş ad eski adı bırakır.
    fn finish_rename(&mut self) {
        let Some((node, name)) = self.renaming.take() else {
            return;
        };
        let name = name.trim().to_owned();

        if name.is_empty() {
            self.log("Ad boş olamaz; eski ad kaldı.");
            return;
        }

        let previous = match node {
            NodeId::Group(group) => self
                .layer_tree
                .groups
                .get_mut(group)
                .map(|group| std::mem::replace(&mut group.name, name.clone())),
            NodeId::Layer(layer) => self
                .layers
                .get_mut(layer)
                .map(|layer| std::mem::replace(&mut layer.name, name.clone())),
            NodeId::Sublayer(..) => None,
        };

        if let Some(previous) = previous
            && previous != name
        {
            self.log(format!("{previous} yeniden adlandırıldı: {name}."));
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.draft.clear();
        // Çizim araçlarında tıklama nokta girişidir; haritadan seçim sürmez.
        self.cancel_pick();

        // Ölçüm penceresi Ölç aracıyla açılır, başka araca geçince kapanır.
        if tool == Tool::Measure {
            self.windows.open(Pane::Measure, Pane::Measure.placement());
        } else {
            self.measurement.clear();
            self.windows.close(Pane::Measure);
        }

        self.log(format!("{}: {}", tool.label(), tool.description()));
    }

    /// Açık uçlu çizimi (çoklu çizgi, alan) tamamlar; yarım kalan diğer
    /// çizimleri atar.
    fn finish_draft(&mut self) {
        if let Some(geometry) = self.draft.finish(self.tool) {
            self.commit_drawing(geometry);
        }
    }

    /// Tamamlanan geometriyi çizim katmanına ekler ve seçer; katman
    /// kilitliyse eklemez.
    fn commit_drawing(&mut self, geometry: Geometry) {
        if !self.editable(Some(DRAWING_LAYER)) {
            return;
        }

        let Some(layer) = self.layers.get_mut(DRAWING_LAYER) else {
            return;
        };

        self.drawn_count += 1;
        let name = format!("{} {}", self.tool.label(), self.drawn_count);

        let id = layer.insert(Feature::new(geometry).with_values([
            Value::from(name.as_str()),
            Value::from(self.tool.label()),
            Value::from("Taslak"),
            Value::DateTime(DateTime::now(TIME_ZONE)),
            Value::Null,
        ]));

        // Çizim görünsün diye katman ve üst grupları açılır.
        self.layer_tree.reveal(DRAWING_LAYER);
        self.sync_visibility();

        self.selection.select(FeatureRef::new(DRAWING_LAYER, id));
        self.selection_changed(true);
        self.log(format!("{name} çizildi."));
    }

    // --- Kayan pencereler -----------------------------------------------

    /// Pencereyi açar ya da kapatır. Ölçüm penceresi Ölç aracıyla birlikte
    /// açılır ve kapanır.
    fn toggle_pane(&mut self, pane: Pane) {
        if self.windows.is_open(pane) {
            if pane == Pane::Measure {
                self.select_tool(Tool::Select);
            } else {
                self.windows.close(pane);
            }
        } else {
            self.open_pane(pane);
        }
    }

    /// Pencereyi açar ya da öne getirir. Koordinata git boşsa görünümün
    /// merkeziyle dolar.
    fn open_pane(&mut self, pane: Pane) {
        match pane {
            Pane::Measure => {
                if self.tool == Tool::Measure {
                    self.windows.raise(pane);
                } else {
                    self.select_tool(Tool::Measure);
                }
            }
            Pane::GoTo => {
                if self.go_to == GoTo::default() {
                    let center = self.viewport.center;

                    self.go_to = GoTo {
                        latitude: format!("{:.5}", center.lat),
                        longitude: format!("{:.5}", center.lon),
                    };
                }

                self.windows.open(pane, pane.placement());
            }
            Pane::Style => self.windows.open(pane, pane.placement()),
        }
    }

    /// Ölçümün kenarları ve toplamı, satır satır.
    fn measurement_text(&self) -> String {
        self.measurement
            .segments()
            .enumerate()
            .map(|(index, meters)| format!("Kenar {}: {}", index + 1, format::distance(meters)))
            .chain(std::iter::once(format!(
                "Toplam: {}",
                format::distance(self.measurement.total_meters())
            )))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // --- İçe aktarma ve katman özellikleri -----------------------------

    /// Sihirbazda sonraki adım; son adımda içe aktarmayı başlatır. Adım
    /// tamamlanmadıysa bir şey olmaz.
    fn import_next(&mut self) {
        let Some(wizard) = &mut self.import else {
            return;
        };

        if !wizard.is_ready(&self.layers) {
            return;
        }

        if wizard.step + 1 < import::STEPS.len() {
            wizard.step += 1;
            return;
        }

        let Some(wizard) = self.import.take() else {
            return;
        };
        let Some(source) = wizard.source else {
            return;
        };

        let name = wizard.name.trim().to_owned();

        self.jobs.push(
            JobKind::Import {
                source,
                name: name.clone(),
                x: wizard.x.unwrap_or(3),
                y: wizard.y.unwrap_or(2),
            },
            source.count() as u32,
        );
        self.log(format!(
            "{} içe aktarılıyor; {name} katmanı bitince eklenecek.",
            source.file()
        ));
    }

    /// İçe aktarılan katmanı ağacın en üstüne, çizimlerin altına ekler.
    fn add_layer(&mut self, layer: kentos_rc::spatial::Layer, source: &str) -> usize {
        let index = self.layers.len();

        self.layers.push(layer);
        self.tables.push(TableView::default());
        self.sources.push(format!("{source}; içe aktarıldı"));
        self.layer_tree.visible.push(true);
        self.layer_tree.expanded.push(false);
        self.layer_tree.locked.push(false);
        self.layer_tree.selectable.push(true);

        let position = self.layer_tree.roots.len().min(1);
        self.layer_tree
            .roots
            .insert(position, crate::layer_tree::Entry::Layer(index));

        self.sync_visibility();
        index
    }

    /// Taslağı katmana yazar; ad geçersizse yazmaz.
    fn apply_properties(&mut self) -> bool {
        let Some(properties) = &mut self.properties else {
            return false;
        };

        let index = properties.layer;

        if properties.draft.problem(&self.layers, index).is_some() {
            return false;
        }

        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };

        properties.draft.apply(layer);
        properties.original = properties.draft.clone();

        if let Some(visible) = self.layer_tree.visible.get_mut(index) {
            *visible = properties.draft.visible;
        }

        let name = layer.name.clone();
        self.sync_visibility();
        self.log(format!("{name}: katman özellikleri uygulandı."));
        true
    }

    // --- Arka plandaki işler -------------------------------------------

    /// İşleri bir adım ilerletir; biten ya da başarısız olan işi bildirir.
    fn step_jobs(&mut self) {
        match self.jobs.tick() {
            Some(Outcome::Done(job)) => {
                self.log(format!("{}: {}", job.title(), job.detail()));

                let toast = match &job.kind {
                    JobKind::Export(_) => Toast::success("Dışa aktarıldı").body(job.detail()),
                    JobKind::Index => Toast::success("Uzamsal dizin hazır").body(job.detail()),
                    JobKind::Import { source, name, x, y } => {
                        let index =
                            self.add_layer(import::layer(*source, name, *x, *y), source.file());

                        Toast::success(format!("{name} eklendi"))
                            .body(format!("{} öğe, EPSG:4326 WGS 84.", source.count()))
                            .action("Katmana git", Message::ZoomToLayer(index))
                    }
                };

                self.toasts.push(toast);
            }
            Some(Outcome::Failed(job)) => {
                self.error(format!("{}: {}", job.title(), job.detail()));
                self.toasts.push(
                    Toast::error(format!("{} başarısız", job.title()))
                        .body(job.detail())
                        .action("Yeniden dene", Message::JobRetried(job.id)),
                );
            }
            None => {}
        }
    }

    // --- Seçim -----------------------------------------------------------

    /// Seç aracında tıklama: Shift seçime ekler, Ctrl seçimden çıkarır;
    /// değiştirici yoksa boş yere tıklamak seçimi kaldırır.
    fn click_select(&mut self, point: Point, modifiers: Modifiers) {
        self.tree_focused = false;

        let viewport = self.viewport;
        let hit = self.with_selectable(|layers| query::hit_test(layers, &viewport, point));
        let mode = selection_mode(modifiers);

        match hit {
            Some(hit) => {
                self.selection.apply(mode, [hit]);

                if mode == SelectionMode::Add {
                    self.selection.focus(hit);
                }
            }
            None if mode == SelectionMode::New => self.selection.clear(),
            None => return,
        }

        self.selection_changed(true);
    }

    /// Seçim penceresi: soldan sağa pencere, sağdan sola kesişen seçim.
    fn box_select(&mut self, bounds: Bounds, crossing: bool, modifiers: Modifiers) {
        let viewport = self.viewport;
        let found =
            self.with_selectable(|layers| query::in_bounds(layers, &viewport, bounds, crossing));
        let count = found.len();
        let mode = selection_mode(modifiers);

        self.selection.apply(mode, found);
        self.selection_changed(true);

        let kind = if crossing {
            "Kesişen seçim"
        } else {
            "Pencere seçimi"
        };

        self.log(match mode {
            SelectionMode::New => format!("{kind}: {count} öğe seçildi."),
            _ => format!(
                "{kind} ({}): {count} öğe; toplam {} seçili.",
                text::to_lowercase(&mode.to_string()),
                self.selection.len()
            ),
        });
    }

    /// Tablo satırına tıklama: Shift birincil satırdan tıklanan satıra kadar
    /// aralığı seçer, Ctrl satırı seçime ekler ya da çıkarır.
    fn press_row(&mut self, reference: FeatureRef) {
        if self.modifiers.shift()
            && let Some(anchor) = self.selection.primary()
        {
            let rows = self.table_rows();
            let position = |target: FeatureRef| rows.iter().position(|row| *row == target);

            if let (Some(start), Some(end)) = (position(anchor), position(reference)) {
                let range = rows[start.min(end)..=start.max(end)].to_vec();

                self.selection.apply(SelectionMode::New, range);
                self.selection.focus(anchor);
                self.sync_inspector();
                return;
            }
        }

        if self.modifiers.command() {
            self.selection.toggle(reference);
        } else {
            self.selection.select(reference);
        }

        self.sync_inspector();
    }

    /// Aktif katmanda filtreye ve aramaya uyan bütün kayıtları seçer; diğer
    /// katmanlardaki seçim korunur.
    fn select_all(&mut self) {
        let rows = self.matching_rows();
        let count = rows.len();

        self.replace_in_active_layer(rows);
        self.log(format!(
            "{}: {count} öğe seçildi.",
            self.active_layer_name()
        ));
    }

    /// Aktif katmanda filtreye ve aramaya uyan kayıtlar arasında seçimi
    /// tersine çevirir.
    fn invert_selection(&mut self) {
        let inverted: Vec<FeatureRef> = self
            .matching_rows()
            .into_iter()
            .filter(|row| !self.selection.contains(row))
            .collect();
        let count = inverted.len();

        self.replace_in_active_layer(inverted);
        self.log(format!(
            "{}: seçim tersine çevrildi, {count} öğe seçili.",
            self.active_layer_name()
        ));
    }

    /// Aktif katmandaki seçimi verilen öğelerle değiştirir; ilki birincil
    /// olur.
    fn replace_in_active_layer(&mut self, references: Vec<FeatureRef>) {
        let layer = self.active_layer;

        self.selection.retain(|item| item.layer != layer);
        self.selection
            .apply(SelectionMode::Add, references.iter().copied());

        if let Some(first) = references.first() {
            self.selection.focus(*first);
        }

        self.sync_inspector();
    }

    /// Seçim değişti: nesne inceleyiciyi birincil öğeye eşitler. `follow`
    /// ise aktif katman da birincil öğenin katmanı olur; öznitelik tablosu
    /// seçimi izler.
    fn selection_changed(&mut self, follow: bool) {
        if follow && let Some(primary) = self.selection.primary() {
            self.activate_layer(primary.layer);
        }

        self.sync_inspector();
    }

    /// Nesne inceleyici birincil öğeyi gösterir; öğe değişince yazılmakta
    /// olan metinler ve haritadan seçim bırakılır.
    fn sync_inspector(&mut self) {
        let primary = self.selection.primary();
        self.inspector.inspect(primary.map(inspector_key));

        if self
            .picking
            .as_ref()
            .is_some_and(|pick| Some(pick.subject) != primary)
        {
            self.picking = None;
        }
    }

    /// Seçili çizimleri siler. Örnek veri katmanlarının öğeleri silinmez.
    fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            self.log("Silinecek seçili öğe yok.");
            return;
        }

        let (drawings, others): (Vec<FeatureRef>, Vec<FeatureRef>) = self
            .selection
            .iter()
            .partition(|item| item.layer == DRAWING_LAYER);

        if !drawings.is_empty() && !self.editable(Some(DRAWING_LAYER)) {
            return;
        }

        if drawings.is_empty() {
            // Süren bir durum: bildirim değil, haritanın üstünde şerit.
            self.log("Yalnızca Çizimler katmanındaki öğeler silinebilir.");
            self.read_only_notice = true;
            return;
        }

        let removed: Vec<Feature> = self
            .layers
            .get_mut(DRAWING_LAYER)
            .map(|layer| {
                drawings
                    .iter()
                    .filter_map(|drawing| layer.remove(drawing.id))
                    .collect()
            })
            .unwrap_or_default();

        self.selection.retain(|item| item.layer != DRAWING_LAYER);
        self.hover = None;
        self.sync_inspector();

        let mut output = format!("{} çizim silindi.", removed.len());

        if !others.is_empty() {
            output.push_str(&format!(
                " Diğer katmanlardaki {} öğe silinmedi: yalnızca çizimler silinebilir.",
                others.len()
            ));
        }

        self.log(output);
        self.forget(removed);
    }

    /// Silinen çizimleri "Geri al" için saklar ve bildirir.
    fn forget(&mut self, removed: Vec<Feature>) {
        if removed.is_empty() {
            return;
        }

        let toast = Toast::success(match removed.len() {
            1 => "Çizim silindi".to_owned(),
            count => format!("{count} çizim silindi"),
        })
        .action("Geri al", Message::UndoDelete);

        self.deleted = removed;
        self.toasts.push(toast);
    }

    /// Son silinen çizimleri numaralarıyla geri koyar ve seçer.
    /// Geri alınabilecek silme var mı.
    pub(crate) fn can_undo(&self) -> bool {
        !self.deleted.is_empty()
    }

    fn undo_delete(&mut self) {
        let deleted = std::mem::take(&mut self.deleted);
        let Some(layer) = self.layers.get_mut(DRAWING_LAYER) else {
            return;
        };

        let restored: Vec<FeatureRef> = deleted
            .into_iter()
            .filter_map(|feature| {
                let id = feature.id;
                layer
                    .restore(feature)
                    .then(|| FeatureRef::new(DRAWING_LAYER, id))
            })
            .collect();

        if restored.is_empty() {
            return;
        }

        self.layer_tree.reveal(DRAWING_LAYER);
        self.sync_visibility();
        self.selection
            .apply(SelectionMode::New, restored.iter().copied());
        self.selection_changed(true);
        self.log(format!("{} çizim geri alındı.", restored.len()));
    }

    // --- Öznitelikler ----------------------------------------------------

    /// Nesne inceleyicideki değişikliği birincil öğeye uygular.
    fn set_attribute(&mut self, field: usize, value: Value) {
        if let Some(subject) = self.selection.primary() {
            self.set_value(subject, field, value);
        }
    }

    /// Öğenin alanını değiştirir. Alan salt okunursa ya da değer alanın
    /// kısıtlarına uymuyorsa değiştirmez ve nedenini komut satırına yazar.
    fn set_value(&mut self, subject: FeatureRef, field: usize, value: Value) -> bool {
        let Some(definition) = self
            .layers
            .get(subject.layer)
            .and_then(|layer| layer.schema.get(field))
        else {
            return false;
        };

        let check = if definition.editable {
            definition.validate(&value)
        } else {
            Err("Bu alan salt okunur.".to_owned())
        };

        if let Err(error) = check {
            let name = definition.name.clone();
            self.log(format!("{name}: {error}"));
            return false;
        }

        self.layers
            .get_mut(subject.layer)
            .and_then(|layer| layer.feature_mut(subject.id))
            .and_then(|feature| feature.values.get_mut(field))
            .map(|slot| *slot = value)
            .is_some()
    }

    /// Başvuru alanının değerini haritadan seçmeye başlar (varlık seçici).
    fn start_pick(&mut self, field: usize) {
        let pick = self.selection.primary().and_then(|subject| {
            let (layer, _) = subject.resolve(&self.layers)?;
            let definition = layer.schema.get(field)?;
            let FieldKind::Object { target } = &definition.kind else {
                return None;
            };
            let index = self.layers.iter().position(|layer| &layer.name == target)?;

            Some((subject, index, definition.name.clone(), target.clone()))
        });

        let Some((subject, target, name, target_name)) = pick else {
            self.inspector.stop_picking();
            return;
        };

        // Tıklamalar yalnızca Seç ve Kaydır araçlarında seçimdir.
        if self.tool.takes_points() {
            self.tool = Tool::Select;
            self.draft.clear();
            self.measurement.clear();
        }

        if self.layers.get(target).is_some_and(|layer| !layer.visible) {
            self.layer_tree.reveal(target);
            self.sync_visibility();
            self.log(format!("{target_name} katmanı gösterildi."));
        }

        self.picking = Some(Pick {
            subject,
            field,
            target,
            prompt: format!("{target_name} katmanından bir öğe seçin"),
        });
        self.hover = None;
        self.log(format!(
            "{name}: haritada {target_name} katmanından bir öğe seçin. İptal için Esc."
        ));
    }

    /// Haritadan seçimi tıklanan öğeyle tamamlar; hedef katmanda öğe yoksa
    /// seçim sürer.
    fn complete_pick(&mut self, point: Point) {
        let Some(pick) = self.picking.clone() else {
            return;
        };

        let Some(hit) = query::hit_test_in(&self.layers, pick.target, &self.viewport, point) else {
            let target = self
                .layers
                .get(pick.target)
                .map_or("", |layer| layer.name.as_str());
            self.log(format!("Tıklanan yerde {target} öğesi yok."));
            return;
        };

        self.cancel_pick();

        if self.set_value(pick.subject, pick.field, Value::Object(hit.id)) {
            let field = pick
                .subject
                .resolve(&self.layers)
                .and_then(|(layer, _)| layer.schema.get(pick.field))
                .map_or_else(String::new, |field| field.name.clone());
            let chosen = hit
                .resolve(&self.layers)
                .map_or_else(String::new, |(layer, feature)| layer.label(feature));

            self.log(format!("{field}: {chosen} seçildi."));
        }
    }

    /// Birincil öğenin başvuru alanının gösterdiği nesneyi seçer ve ona
    /// odaklanır.
    fn navigate(&mut self, field: usize, object: ObjectId) {
        let target = self
            .selection
            .primary()
            .and_then(|subject| subject.resolve(&self.layers))
            .and_then(|(layer, _)| match &layer.schema.get(field)?.kind {
                FieldKind::Object { target } => Some(target.clone()),
                _ => None,
            })
            .and_then(|target| self.layers.iter().position(|layer| layer.name == target));

        let Some(reference) = target.map(|layer| FeatureRef::new(layer, object)) else {
            return;
        };

        if reference.resolve(&self.layers).is_none() {
            self.log(format!("#{object} bulunamadı."));
            return;
        }

        self.selection.select(reference);
        self.selection_changed(true);

        if let Some(bounds) = bounds_of(&self.layers, [reference]) {
            self.viewport.focus(bounds, FOCUS_PADDING, FOCUS_ZOOM);
        }
    }

    fn cancel_pick(&mut self) {
        self.picking = None;
        self.inspector.stop_picking();
    }

    // --- Öznitelik tablosu ve sorgular -----------------------------------

    /// Aktif katmanın tablo ayarları.
    pub(crate) fn active_table(&self) -> Option<&TableView> {
        self.tables.get(self.active_layer)
    }

    /// Tabloda görünen satırlar, görünen sırayla.
    pub(crate) fn table_rows(&self) -> Vec<FeatureRef> {
        self.active_table().map_or_else(Vec::new, |table| {
            table.rows(&self.layers, self.active_layer, &self.selection)
        })
    }

    /// Filtreye ve aramaya uyan satırlar, seçimden bağımsız.
    fn matching_rows(&self) -> Vec<FeatureRef> {
        self.active_table().map_or_else(Vec::new, |table| {
            table.matching(&self.layers, self.active_layer)
        })
    }

    /// Sorgu penceresini açar. Filtrede mevcut filtreyle, seçimde tek boş
    /// koşulla başlar.
    fn open_query(&mut self, purpose: QueryPurpose, layer: usize) {
        let Some(schema) = self.layers.get(layer).map(|layer| &layer.schema) else {
            return;
        };

        let current = match purpose {
            QueryPurpose::Filter => self
                .tables
                .get(layer)
                .map(|table| table.filter.clone())
                .filter(|filter| !filter.is_empty()),
            QueryPurpose::Select => None,
        };

        self.query = Some(QueryDialog {
            purpose,
            layer,
            query: current.unwrap_or_else(|| starter_query(schema)),
            mode: SelectionMode::New,
        });
        self.app_menu_open = false;
        self.help_open = false;
    }

    /// Sorgu penceresini uygular: eşleşen öğeleri seçer ya da tabloyu
    /// filtreler.
    fn apply_query(&mut self) {
        let Some(dialog) = self.query.take() else {
            return;
        };

        let Some(layer) = self.layers.get(dialog.layer) else {
            return;
        };

        if !dialog.query.errors(&layer.schema).is_empty() {
            self.query = Some(dialog);
            return;
        }

        let name = layer.name.clone();
        let description = match dialog.query.describe(&layer.schema) {
            description if description.is_empty() => "bütün kayıtlar".to_owned(),
            description => description,
        };

        match dialog.purpose {
            QueryPurpose::Select => {
                let found: Vec<FeatureRef> = layer
                    .features
                    .iter()
                    .filter(|feature| dialog.query.matches(&layer.schema, &feature.values))
                    .map(|feature| FeatureRef::new(dialog.layer, feature.id))
                    .collect();
                let count = found.len();

                self.selection.apply(dialog.mode, found);
                self.activate_layer(dialog.layer);
                self.sync_inspector();
                self.log(format!(
                    "Öznitelikle seç, {name}: {description}. {count} öğe eşleşti; {} öğe seçili.",
                    self.selection.len()
                ));
            }
            QueryPurpose::Filter => {
                let cleared = dialog.query.is_empty();

                if let Some(table) = self.tables.get_mut(dialog.layer) {
                    table.filter = dialog.query;
                }

                self.activate_layer(dialog.layer);
                self.docks.show(DockPanel::Table, DockPanel::Table.side());
                self.log(if cleared {
                    format!("{name} tablosunun filtresi kaldırıldı.")
                } else {
                    format!("{name} tablosu filtrelendi: {description}.")
                });
            }
        }
    }

    // --- Katman ağacı ----------------------------------------------------

    /// Katmanı aktif yapar; ağaçta da o katman seçilir.
    fn activate_layer(&mut self, index: usize) {
        if index < self.layers.len() {
            self.active_layer = index;
            self.layer_tree.selected = Some(NodeId::Layer(index));
        }
    }

    /// Katmanların çizilip çizilmeyeceğini ağaçtaki işaretlerden hesaplar:
    /// katman, kendisi ve bütün üst grupları görünürse çizilir.
    fn sync_visibility(&mut self) {
        for (layer, visible) in self.layers.iter_mut().zip(self.layer_tree.effective()) {
            layer.visible = visible;
        }
    }

    /// Ağaçtaki onay kutusu. Bazı alt katmanları gizli olan görünür
    /// katmanın kutusu karışıktır; ona tıklamak bütün alt katmanları gösterir.
    fn check_node(&mut self, node: NodeId, checked: bool) {
        match node {
            NodeId::Group(group) => {
                if let Some(group) = self.layer_tree.groups.get_mut(group) {
                    group.visible = checked;
                }
            }
            NodeId::Layer(index) => {
                let own = self.layer_tree.visible.get(index).copied().unwrap_or(false);

                if checked && own {
                    if let Some(layer) = self.layers.get_mut(index) {
                        for sublayer in &mut layer.sublayers {
                            sublayer.visible = true;
                        }
                    }
                } else if let Some(visible) = self.layer_tree.visible.get_mut(index) {
                    *visible = checked;
                }
            }
            NodeId::Sublayer(layer, sublayer) => {
                if let Some(sublayer) = self
                    .layers
                    .get_mut(layer)
                    .and_then(|layer| layer.sublayers.get_mut(sublayer))
                {
                    sublayer.visible = checked;
                }
            }
        }

        self.sync_visibility();
    }

    /// Yalnızca düğümü gösterir: katman ya da grubun katmanları görünür,
    /// diğerleri gizlenir; alt katmanda katmanın diğer alt katmanları gizlenir.
    fn show_only(&mut self, node: NodeId) {
        match node {
            NodeId::Group(group) => {
                let layers = self.layer_tree.layers_in(group);
                self.layer_tree.show_only(&layers);
                self.layer_tree.groups[group].visible = true;
            }
            NodeId::Layer(layer) => self.layer_tree.show_only(&[layer]),
            NodeId::Sublayer(index, only) => {
                self.layer_tree.reveal(index);

                if let Some(layer) = self.layers.get_mut(index) {
                    for (sublayer, entry) in layer.sublayers.iter_mut().enumerate() {
                        entry.visible = sublayer == only;
                    }
                }
            }
        }

        self.sync_visibility();
        self.log(format!("Yalnızca {} gösteriliyor.", self.node_name(node)));
    }

    /// Düğümün öğeleri: katmanın, alt katmanın ya da grubun bütün katmanlarının.
    pub(crate) fn node_features(&self, node: NodeId) -> Vec<FeatureRef> {
        let layer_features = |index: usize| {
            self.layers
                .get(index)
                .into_iter()
                .flat_map(move |layer| layer.features.iter())
                .map(move |feature| FeatureRef::new(index, feature.id))
        };

        match node {
            NodeId::Group(group) => self
                .layer_tree
                .layers_in(group)
                .into_iter()
                .flat_map(layer_features)
                .collect(),
            NodeId::Layer(index) => layer_features(index).collect(),
            NodeId::Sublayer(index, sublayer) => self
                .layers
                .get(index)
                .into_iter()
                .flat_map(|layer| layer.sublayer_features(sublayer))
                .map(|feature| FeatureRef::new(index, feature.id))
                .collect(),
        }
    }

    fn zoom_to_node(&mut self, node: NodeId) {
        if let Some(bounds) = bounds_of(&self.layers, self.node_features(node)) {
            self.viewport.focus(bounds, 48.0, 9.0);
        }
    }

    fn select_node(&mut self, node: NodeId) {
        let found = self.node_features(node);
        let count = found.len();

        self.selection.apply(SelectionMode::New, found);
        self.selection_changed(true);
        self.log(format!("{}: {count} öğe seçildi.", self.node_name(node)));
    }

    /// Düğümün ağaçta gösterilen adı.
    pub(crate) fn node_name(&self, node: NodeId) -> String {
        match node {
            NodeId::Group(group) => self
                .layer_tree
                .groups
                .get(group)
                .map_or_else(String::new, |group| group.name.clone()),
            NodeId::Layer(layer) => self
                .layers
                .get(layer)
                .map_or_else(String::new, |layer| layer.name.clone()),
            NodeId::Sublayer(layer, sublayer) => self
                .layers
                .get(layer)
                .and_then(|layer| layer.sublayers.get(sublayer))
                .map_or_else(String::new, |sublayer| sublayer.name.clone()),
        }
    }

    /// Çizimler katmanındaki bütün öğeleri siler.
    fn clear_drawings(&mut self) {
        let removed = self
            .layers
            .get_mut(DRAWING_LAYER)
            .map(|layer| std::mem::take(&mut layer.features))
            .unwrap_or_default();

        self.selection.retain(|item| item.layer != DRAWING_LAYER);
        self.hover = None;
        self.sync_inspector();
        self.log(format!("{} çizim silindi.", removed.len()));
        self.forget(removed);
    }

    /// Satırın değerleri, sekmeyle ayrılmış: OBJECTID ve şemadaki alanlar.
    fn row_text(&self, reference: FeatureRef) -> Option<String> {
        let (layer, feature) = reference.resolve(&self.layers)?;

        let values = (0..layer.schema.len())
            .map(|field| table::cell_text(&self.layers, layer, feature, field));

        Some(
            std::iter::once(feature.id.to_string())
                .chain(values)
                .collect::<Vec<_>>()
                .join("\t"),
        )
    }

    // --- Görünüm ---------------------------------------------------------

    fn zoom(&mut self, delta: f64) {
        let anchor = self.viewport.center_point();
        self.viewport.zoom_by(delta, anchor);
    }

    fn set_all_layers_visible(&mut self, visible: bool) {
        self.layer_tree.visible.fill(visible);

        if visible {
            for group in &mut self.layer_tree.groups {
                group.visible = true;
            }
        }

        self.sync_visibility();
        self.log(if visible {
            "Tüm katmanlar gösterildi."
        } else {
            "Tüm katmanlar gizlendi."
        });
    }

    /// Esc: önce açık menüyü, pencereyi ya da haritadan seçimi kapatır;
    /// sonra yarım çizimi bitirir; en son ölçümü ve seçimi temizler.
    fn escape(&mut self) {
        if self.renaming.is_some() || self.gallery.outline_renaming.is_some() {
            self.cancel_rename();
            return;
        }

        if self.confirm.take().is_some()
            || self.import.take().is_some()
            || self.properties.take().is_some()
        {
            return;
        }

        if let Some(pending) = self.pending.take() {
            let name = match pending {
                Pending::Typeface => command::name(Command::Typeface),
                Pending::TextSize => command::name(Command::TextSize),
                Pending::Accent => command::name(Command::Accent),
                Pending::Backdrop => command::name(Command::Backdrop),
            };

            self.log(format!("{name} iptal edildi."));
        } else if self.app_menu_open {
            self.app_menu_open = false;
        } else if self.query.is_some() {
            self.query = None;
        } else if self.help_open {
            self.help_open = false;
        } else if self.picking.is_some() {
            self.cancel_pick();
            self.log("Haritadan seçim iptal edildi.");
        } else if !self.draft.is_empty() {
            self.finish_draft();
        } else if self.tool == Tool::Measure && !self.measurement.is_empty() {
            self.measurement.clear();
        } else if self.tool != Tool::Select {
            // CAD'deki gibi: Esc etkin komuttan çıkar, seçime dönülür.
            self.select_tool(Tool::Select);
        } else {
            self.selection.clear();
            self.sync_inspector();
        }
    }

    // --- Ayarlar ---------------------------------------------------------

    pub(crate) fn setting(&self, setting: Setting) -> bool {
        match setting {
            Setting::Grid => self.options.grid,
            Setting::Snap => self.options.snap,
            Setting::FullCrosshair => self.options.full_crosshair,
            Setting::Labels => self.options.labels,
            Setting::ViewCube => self.view_cube,
        }
    }

    fn set_setting(&mut self, setting: Setting, enabled: bool) {
        match setting {
            Setting::Grid => self.options.grid = enabled,
            Setting::Snap => self.options.snap = enabled,
            Setting::FullCrosshair => self.options.full_crosshair = enabled,
            Setting::Labels => self.options.labels = enabled,
            Setting::ViewCube => self.view_cube = enabled,
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.log(format!("Tema: {}.", mode.label()));
        self.save_settings();
    }

    /// Yazı ayarını değiştirir; bekleyen yazı komutu biter.
    fn set_typography(&mut self, typography: Typography) {
        typography::set(typography);

        let previous = self.typography;
        self.typography = typography::current();
        self.pending = None;

        if self.typography != previous {
            self.log(format!(
                "Yazı: {}, {} piksel; eş aralıklı {}.",
                self.typography.family.name(),
                self.typography.size,
                self.typography.mono.name(),
            ));
            self.save_settings();
        }
    }

    /// Tema ve yazı ayarını dosyaya yazar.
    fn save_settings(&mut self) {
        let Some(path) = &self.settings_path else {
            return;
        };

        let settings = Settings {
            mode: self.mode,
            accent: self.accent,
            backdrop: self.backdrop,
            typography: self.typography,
            docks: self.docks.clone(),
        };

        if let Err(error) = settings.save(path) {
            let file = path.display().to_string();

            self.error(format!("Ayarlar {file} dosyasına yazılamadı: {error}"));
            self.toasts
                .push(Toast::error("Ayarlar kaydedilemedi").body(format!(
                    "{file}: {error}. Değişiklikler bu oturumda geçerli."
                )));
        }
    }

    // --- Komutlar --------------------------------------------------------

    fn run_app_command(&mut self, command: AppCommand) {
        if command == AppCommand::New {
            self.viewport.center = INITIAL_CENTER;
            self.viewport.zoom = INITIAL_ZOOM;
            self.selection.clear();
            self.sync_inspector();
            self.cancel_pick();
            self.measurement.clear();
            self.draft.clear();
            self.log("Yeni: görünüm, seçim ve ölçüm sıfırlandı.");
        } else {
            self.log(format!(
                "{}: dosya işlemleri bu sürümde henüz yok.",
                command.label()
            ));
        }
    }

    /// Komut kutusuna yazılanı çalıştırır: önce etkin istemin seçenekleri,
    /// sonra "enlem, boylam", sonra komutlar. Boş Enter yarım kalan çizimi
    /// bitirir; etkin komut yokken son komutu yineler (AutoCAD'deki gibi).
    fn submit(&mut self, input: &str) -> Task<Message> {
        if input.is_empty() {
            if !self.draft.is_empty() {
                self.finish_draft();
            } else if self.prompt().is_none()
                && let Some(command) = self.last_command()
            {
                self.push(Entry::Input(command::name(command).to_owned()));
                return self.run_command(command);
            }

            return Task::none();
        }

        self.push(Entry::Input(input.to_owned()));

        if let Some(keyword) = self.prompt().and_then(|prompt| prompt.find(input).cloned()) {
            return self.update(keyword);
        }

        // VURGU beklerken seçeneklerin dışında #RRGGBB de yazılabilir.
        if self.pending == Some(Pending::Accent) {
            return match Accent::parse(input) {
                Some(accent) => self.update(Message::AccentChanged(accent)),
                None => {
                    self.error(format!(
                        "{input}: renk #RRGGBB biçiminde yazılır (ör. #e8618c) ya da hazır renklerden biri seçilir."
                    ));
                    Task::none()
                }
            };
        }

        if let Some(location) = command::coordinates(input) {
            self.enter_point(location);
            return Task::none();
        }

        match command::parse(input) {
            Some(command) => self.run_command(command),
            None => {
                self.error(format!(
                    "Bilinmeyen komut: {}. Bütün komutlar için giriş boşken ↓ tuşuna basın.",
                    command::normalize(input)
                ));
                Task::none()
            }
        }
    }

    /// Geçmişteki son geçerli komut.
    fn last_command(&self) -> Option<Command> {
        self.history.iter().rev().find_map(|entry| match entry {
            Entry::Input(input) => command::parse(input),
            _ => None,
        })
    }

    /// Yazılan koordinat: haritadan seçim ya da çizim sürüyorsa nokta
    /// girişidir, yoksa görünüm oraya ortalanır.
    fn enter_point(&mut self, location: LonLat) {
        if self.picking.is_some() {
            self.complete_pick(self.viewport.project(location));
        } else if self.tool.takes_points() {
            self.handle_model_space(model_space::Event::PointPicked(location));
        } else {
            self.viewport.center = location;
            self.log(format!(
                "Görünüm {} noktasına ortalandı.",
                format::decimal(location)
            ));
        }
    }

    /// İstemdeki seçenek.
    fn keyword(&mut self, keyword: Keyword) {
        match keyword {
            Keyword::Undo => {
                let removed = if self.tool == Tool::Measure {
                    self.measurement.undo()
                } else {
                    self.draft.undo()
                };

                if removed {
                    self.log("Son nokta geri alındı.");
                }
            }
            Keyword::Finish | Keyword::Close => self.finish_draft(),
            Keyword::Clear => {
                self.measurement.clear();
                self.log("Ölçüm temizlendi.");
            }
            Keyword::Cancel => {
                if self.picking.is_some() {
                    self.cancel_pick();
                    self.log("Haritadan seçim iptal edildi.");
                } else {
                    self.draft.clear();
                    self.log("Çizim iptal edildi.");
                }
            }
        }
    }

    /// Komut satırından gelen komutu çalıştırır. Kendi başına iz bırakmayan
    /// komutlar için sonucu geçmişe yazar.
    fn run_command(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::Tool(tool) => self.select_tool(tool),
            Command::ZoomIn | Command::ZoomOut => {
                self.zoom(if command == Command::ZoomIn {
                    ZOOM_STEP
                } else {
                    -ZOOM_STEP
                });
                self.log(format!("Yakınlaştırma: {:.2}", self.viewport.zoom));
            }
            Command::FitAll => {
                let _ = self.update(Message::FitAll);
                self.log("Görünüm tüm katmanlara sığdırıldı.");
            }
            Command::ResetView => {
                let _ = self.update(Message::ResetView);
                self.log("Görünüm sıfırlandı.");
            }
            Command::FocusSelection => {
                if self.selection.is_empty() {
                    self.log("Odaklanacak seçili öğe yok.");
                } else {
                    let _ = self.update(Message::FocusSelection);
                    self.log(format!(
                        "Seçili {} öğeye odaklanıldı.",
                        self.selection.len()
                    ));
                }
            }
            Command::Toggle(setting) => return self.update(Message::Toggle(setting)),
            Command::Theme(mode) => self.set_mode(mode.unwrap_or(self.mode.toggled())),
            Command::ShowAllLayers => self.set_all_layers_visible(true),
            Command::HideAllLayers => self.set_all_layers_visible(false),
            Command::ListLayers => {
                let names: Vec<&str> = self
                    .layers
                    .iter()
                    .filter(|layer| layer.visible)
                    .map(|layer| layer.name.as_str())
                    .collect();

                self.log(format!("Görünür katmanlar: {}", names.join(", ")));
            }
            Command::ActiveLayer => {
                self.log(format!("Aktif katman: {}", self.active_layer_name()));
            }
            Command::SelectAll => self.select_all(),
            Command::InvertSelection => self.invert_selection(),
            Command::AttributeTable => {
                let open = self.docks.toggle(DockPanel::Table, DockPanel::Table.side());
                self.save_settings();
                self.log(if open {
                    "Öznitelik tablosu açıldı."
                } else {
                    "Öznitelik tablosu kapatıldı."
                });
            }
            Command::SelectByAttributes => self.open_query(QueryPurpose::Select, self.active_layer),
            Command::Filter => self.open_query(QueryPurpose::Filter, self.active_layer),
            Command::Delete => self.delete_selection(),
            Command::Clear => {
                self.draft.clear();
                self.measurement.clear();
                self.selection.clear();
                self.sync_inspector();
                self.log("Seçim ve ölçüm temizlendi.");
            }
            Command::New => self.run_app_command(AppCommand::New),
            Command::Pane(pane) => self.open_pane(pane),
            Command::Import => return self.update(Message::ImportOpened),
            Command::Accent => self.pending = Some(Pending::Accent),
            Command::Backdrop => self.pending = Some(Pending::Backdrop),
            Command::Typeface => self.pending = Some(Pending::Typeface),
            Command::TextSize => self.pending = Some(Pending::TextSize),
            Command::Help => {
                self.help_open = true;
                self.log("Kısayollar açıldı.");
            }
            Command::Quit => return self.update(Message::Quit),
        }

        Task::none()
    }

    pub(crate) fn active_layer_name(&self) -> &str {
        self.layers
            .get(self.active_layer)
            .map_or("—", |layer| layer.name.as_str())
    }

    fn log(&mut self, output: impl Into<String>) {
        self.push(Entry::Output(output.into()));
    }

    fn error(&mut self, error: impl Into<String>) {
        self.push(Entry::Error(error.into()));
    }

    fn push(&mut self, entry: Entry) {
        self.history.push(entry);

        if self.history.len() > HISTORY_LIMIT {
            self.history.drain(..HISTORY_LIMIT / 2);
        }
    }
}

/// Değiştirici tuşlara göre seçim yöntemi: Shift ekler, Ctrl çıkarır.
fn selection_mode(modifiers: Modifiers) -> SelectionMode {
    if modifiers.shift() {
        SelectionMode::Add
    } else if modifiers.command() {
        SelectionMode::Remove
    } else {
        SelectionMode::New
    }
}

/// Nesne inceleyicinin öğe anahtarı: katman sırası ve öğe numarası.
fn inspector_key(reference: FeatureRef) -> u64 {
    ((reference.layer as u64) << 48) | reference.id.0
}

/// Öğelerin tamamını kapsayan kutu.
fn bounds_of(layers: &[Layer], references: impl IntoIterator<Item = FeatureRef>) -> Option<Bounds> {
    references
        .into_iter()
        .filter_map(|reference| reference.resolve(layers))
        .filter_map(|(_, feature)| feature.bounds())
        .reduce(Bounds::union)
}

/// Tek, boş koşullu sorgu; pencere boş açılmasın.
fn starter_query(schema: &[Field]) -> Query {
    let mut query = Query::default();
    query.apply(Edit::Add, schema);
    query
}

/// Yerinde adlandırma kutusunu odaklar ve adı seçer.
fn focus_rename() -> Task<Message> {
    Task::batch([
        iced::widget::operation::focus(tree_view::RENAME),
        iced::widget::operation::select_all(tree_view::RENAME),
    ])
}

/// Klavye: değiştirici tuşlar her zaman izlenir; kısayollar yalnızca olayı
/// bir bileşen (ör. metin girişi) kullanmadıysa çalışır.
fn keyboard_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    let Event::Keyboard(event) = event else {
        return None;
    };

    match event {
        keyboard::Event::ModifiersChanged(modifiers) => Some(Message::ModifiersChanged(modifiers)),
        keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Space),
            repeat,
            modifiers,
            ..
        } if status == event::Status::Ignored => {
            (!repeat && modifiers.is_empty()).then_some(Message::RadialOpened)
        }
        keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        } if status == event::Status::Ignored => {
            shortcut(key.as_ref(), modifiers).or_else(|| typed(text.as_deref(), modifiers))
        }
        _ => None,
    }
}

/// CAD'deki gibi komut kutusu odakta değilken de yazılanlar komut kutusuna
/// gider: "l" yazıp Enter'a basmak çizgi aracını seçer.
fn typed(text: Option<&str>, modifiers: Modifiers) -> Option<Message> {
    let text = text?;

    let printable = !text.is_empty()
        && text
            .chars()
            .all(|character| !character.is_control() && !character.is_whitespace());

    (printable && !modifiers.command() && !modifiers.alt() && !modifiers.logo())
        .then(|| Message::CommandTyped(text.to_owned()))
}

/// CAD kısayolları: Esc, Delete, Ctrl+A (tümünü seç), F1 (yardım), Ctrl+F1
/// (şeridi daralt), F2 (komut geçmişi; ağaçta seçili düğümü adlandırır), F3
/// (yakalama), F7 (ızgara); Ctrl +, Ctrl − ve Ctrl 0 yazı boyutunu
/// değiştirir; Ctrl+R düzenin cetvellerini açıp kapatır. Boşluk dairesel
/// araç menüsünü açar (`keyboard_event`).
fn shortcut(key: keyboard::Key<&str>, modifiers: Modifiers) -> Option<Message> {
    use keyboard::Key;
    use keyboard::key::Named;

    match key {
        Key::Named(Named::Escape) => Some(Message::Escape),
        Key::Named(Named::Enter) => Some(Message::EnterPressed),
        Key::Named(Named::Delete) => Some(Message::DeleteSelection),
        Key::Named(Named::F1) if modifiers.command() => Some(Message::RibbonCollapsed),
        Key::Character("r" | "R") if modifiers.command() => Some(Message::RulersToggled),
        Key::Named(Named::F1) => Some(Message::HelpToggled),
        Key::Named(Named::F2) => Some(Message::F2Pressed),
        Key::Named(Named::F3) => Some(Message::Toggle(Setting::Snap)),
        Key::Named(Named::F7) => Some(Message::Toggle(Setting::Grid)),
        Key::Character("a" | "A") if modifiers.command() => Some(Message::SelectAll),
        Key::Character("+" | "=") if modifiers.command() => {
            Some(Message::TextSize(SizeStep::Larger))
        }
        Key::Character("-") if modifiers.command() => Some(Message::TextSize(SizeStep::Smaller)),
        Key::Character("0") if modifiers.command() => Some(Message::TextSize(SizeStep::Default)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submit(app: &mut Showcase, input: &str) {
        let _ = app.update(Message::CommandInput(input.to_owned()));
        let _ = app.update(Message::CommandSubmitted);
    }

    fn picked(app: &mut Showcase, lon: f64, lat: f64) {
        let _ = app.update(Message::ModelSpace(model_space::Event::PointPicked(
            LonLat::new(lon, lat),
        )));
    }

    #[test]
    fn typed_keywords_answer_the_prompt() {
        let mut app = Showcase::new();

        submit(&mut app, "pl");
        assert_eq!(app.tool, Tool::Polyline);

        picked(&mut app, 32.85, 39.93);
        picked(&mut app, 35.48, 38.72);
        submit(&mut app, "g");
        assert_eq!(app.draft.points().len(), 1);

        // Boş Enter çizimi bitirir; tek noktalı çoklu çizgi atılır.
        submit(&mut app, "");
        assert!(app.draft.is_empty());

        // Esc etkin komuttan çıkar.
        let _ = app.update(Message::Escape);
        assert_eq!(app.tool, Tool::Select);
    }

    #[test]
    fn coordinates_add_points_or_center_the_view() {
        let mut app = Showcase::new();

        submit(&mut app, "39.92, 32.85");
        assert_eq!(app.viewport.center, LonLat::new(32.85, 39.92));

        submit(&mut app, "nokta");
        submit(&mut app, "41,01 28,98");

        let drawings = &app.layers[DRAWING_LAYER].features;
        assert_eq!(drawings.len(), 1);
    }

    #[test]
    fn typeface_command_waits_for_an_option() {
        let mut app = Showcase::new();

        submit(&mut app, "yazitipi");
        assert_eq!(app.pending, Some(Pending::Typeface));
        assert!(
            app.prompt()
                .is_some_and(|prompt| prompt.find("inter").is_some())
        );

        // Esc bekleyen komutu bırakır; yazı ayarı değişmez.
        let _ = app.update(Message::Escape);
        assert_eq!(app.pending, None);

        submit(&mut app, "punto");
        assert_eq!(app.pending, Some(Pending::TextSize));
    }

    #[test]
    fn tree_nodes_are_renamed_moved_locked_and_made_unselectable() {
        let mut app = Showcase::new();
        let key = |entry: layer_tree::Entry| entry.key();

        // Menüden yeniden adlandırma; boş ad eski adı bırakır.
        let _ = app.update(Message::RenameStarted(NodeId::Layer(DRAWING_LAYER)));
        let _ = app.update(Message::RenameInput("  Taslaklar ".to_owned()));
        let _ = app.update(Message::RenameSubmitted);
        assert_eq!(app.layers[DRAWING_LAYER].name, "Taslaklar");

        let _ = app.update(Message::RenameStarted(NodeId::Group(0)));
        let _ = app.update(Message::RenameInput(String::new()));
        let _ = app.update(Message::RenameSubmitted);
        assert_eq!(app.layer_tree.groups[0].name, "Yerleşim");

        // F2: ağaçta seçili düğüm varsa adlandırır, yoksa geçmişi açar; Esc
        // vazgeçer.
        let _ = app.update(Message::F2Pressed);
        assert!(app.command_expanded);
        let _ = app.update(Message::TreeSelected(NodeId::Group(1)));
        let _ = app.update(Message::F2Pressed);
        assert!(matches!(app.renaming, Some((NodeId::Group(1), _))));
        let _ = app.update(Message::Escape);
        assert!(app.renaming.is_none());

        // Karayolları, Yerleşim grubunun içine; grup kendi altına gidemez.
        let roads = key(layer_tree::Entry::Layer(3));
        let settlement = key(layer_tree::Entry::Group(0));
        let _ = app.update(Message::TreeMoved(roads, settlement, Place::Into));
        assert_eq!(app.layer_tree.parent(layer_tree::Entry::Layer(3)), Some(0));

        let country = key(layer_tree::Entry::Group(5));
        let _ = app.update(Message::TreeMoved(country, settlement, Place::Into));
        assert_eq!(app.layer_tree.parent(layer_tree::Entry::Group(5)), None);

        // Kilitli çizim katmanına nokta eklenmez.
        let _ = app.update(Message::LayerLocked(DRAWING_LAYER));
        submit(&mut app, "nokta");
        picked(&mut app, 32.85, 39.93);
        assert!(app.layers[DRAWING_LAYER].features.is_empty());

        let _ = app.update(Message::LayerLocked(DRAWING_LAYER));
        picked(&mut app, 32.85, 39.93);
        assert_eq!(app.layers[DRAWING_LAYER].features.len(), 1);

        // Seçilemeyen katmanın öğesine tıklamak seçmez.
        let _ = app.update(Message::LayerSelectable(DRAWING_LAYER));
        let _ = app.update(Message::ToolSelected(Tool::Select));
        app.selection.clear();
        let point = app.viewport.project(LonLat::new(32.85, 39.93));
        let hit = app.hit(point);
        assert!(hit.is_none_or(|hit| hit.layer != DRAWING_LAYER));
    }

    #[test]
    fn the_radial_menu_opens_only_on_the_drawing_and_closes() {
        let mut app = Showcase::new();

        let _ = app.update(Message::RadialOpened);
        assert!(app.radial_open);
        let _ = app.update(Message::ToolSelected(Tool::Line));
        let _ = app.update(Message::RadialClosed);
        assert!(!app.radial_open);
        assert_eq!(app.tool, Tool::Line);

        // Komut yazılırken Boşluk menüyü açmaz.
        let _ = app.update(Message::CommandInput("koordinat".to_owned()));
        let _ = app.update(Message::RadialOpened);
        assert!(!app.radial_open);

        let _ = app.update(Message::CommandInput(String::new()));
        let _ = app.update(Message::RibbonTabSelected(RibbonTab::Gallery));
        let _ = app.update(Message::RadialOpened);
        assert!(!app.radial_open);
    }

    #[test]
    fn the_mini_toolbar_follows_the_selection_in_the_select_tool() {
        let mut app = Showcase::new();

        assert!(app.selection_anchor().is_none());

        let road = FeatureRef::new(3, ObjectId(2));
        let _ = app.update(Message::LayerActivated(3));
        let _ = app.update(Message::TableRowPressed(road));
        let anchor = app.selection_anchor().expect("seçim var");
        assert!(anchor.width > 0.0 && anchor.height > 0.0);

        // Örnek veri silinemez; araç değişince çubuk gizlenir.
        assert!(!app.can_delete_selection());
        let _ = app.update(Message::ToolSelected(Tool::Measure));
        assert!(app.selection_anchor().is_none());
    }

    #[test]
    fn layout_sheets_keep_their_own_guides() {
        use kentos_rc::widget::rulers::{Event, Guide};

        let mut app = Showcase::new();

        // Model alanında kılavuz yok.
        let _ = app.update(Message::SheetGuide(Event::Added(Guide::vertical(20.0))));
        assert!(app.sheets.iter().all(|sheet| sheet.guides.is_empty()));

        let _ = app.update(Message::SheetSelected(1));
        let _ = app.update(Message::SheetGuide(Event::Added(Guide::vertical(20.0))));
        let _ = app.update(Message::SheetGuide(Event::Added(Guide::horizontal(15.0))));
        let _ = app.update(Message::SheetGuide(Event::Moved(0, 25.0)));
        let guides = &app.sheets.sheet().expect("düzen açık").guides;
        assert_eq!(
            guides.as_slice(),
            [Guide::vertical(25.0), Guide::horizontal(15.0)]
        );

        // Öbür düzenin kılavuzları ayrıdır.
        let _ = app.update(Message::SheetSelected(2));
        assert!(app.sheets.sheet().expect("düzen açık").guides.is_empty());

        let _ = app.update(Message::SheetSelected(1));
        let _ = app.update(Message::GuidesCleared);
        assert!(app.sheets.sheet().expect("düzen açık").guides.is_empty());

        assert!(app.rulers);
        let _ = app.update(Message::RulersToggled);
        assert!(!app.rulers);
    }

    #[test]
    fn gallery_timelines_play_with_the_ticks() {
        use kentos_rc::widget::timeline::Event;

        let mut app = Showcase::new();
        let start = iced::time::Instant::now();

        let _ = app.update(Message::Gallery(Demo::Animation(Event::Play(true))));
        assert!(app.gallery.is_playing());

        // İlk tur yalnızca anı kaydeder; yarım saniye 24 kare/saniyede 12
        // karedir.
        let _ = app.update(Message::Gallery(Demo::Tick(start)));
        let _ = app.update(Message::Gallery(Demo::Tick(
            start + Duration::from_millis(500),
        )));
        assert!((app.gallery.animation.current - 12.0).abs() < 1e-6);

        let _ = app.update(Message::Gallery(Demo::Animation(Event::Play(false))));
        assert!(!app.gallery.is_playing());
    }

    #[test]
    fn f2_renames_the_gallery_tree_row_on_the_data_page() {
        let mut app = Showcase::new();

        let _ = app.update(Message::RibbonTabSelected(RibbonTab::Gallery));
        let _ = app.update(Message::GalleryPageSelected(Page::Data));
        let _ = app.update(Message::Gallery(Demo::OutlineSelected(2)));
        let _ = app.update(Message::F2Pressed);
        assert_eq!(
            app.gallery.outline_renaming,
            Some((2, "Kapılar".to_owned()))
        );
        assert!(!app.command_expanded);

        let _ = app.update(Message::Gallery(Demo::OutlineInput(
            "Kapı doğramaları ".to_owned(),
        )));
        let _ = app.update(Message::Gallery(Demo::OutlineRenamed));
        assert_eq!(app.gallery.outline[2].name, "Kapı doğramaları");

        // Esc adlandırmadan vazgeçer; ad değişmez.
        let _ = app.update(Message::F2Pressed);
        let _ = app.update(Message::Gallery(Demo::OutlineInput("Başka".to_owned())));
        let _ = app.update(Message::Escape);
        assert!(app.gallery.outline_renaming.is_none());
        assert_eq!(app.gallery.outline[2].name, "Kapı doğramaları");

        // Başka bir yere yazmaya başlamak F2'yi komut geçmişine döndürür.
        let _ = app.update(Message::CommandTyped("l".to_owned()));
        let _ = app.update(Message::F2Pressed);
        assert!(app.gallery.outline_renaming.is_none());
        assert!(app.command_expanded);
    }

    #[test]
    fn the_ribbon_collapses_and_quick_buttons_hide() {
        let mut app = Showcase::new();

        assert!(matches!(
            shortcut(
                keyboard::Key::Named(keyboard::key::Named::F1),
                Modifiers::CTRL
            ),
            Some(Message::RibbonCollapsed)
        ));

        let _ = app.update(Message::RibbonCollapsed);
        assert!(app.ribbon_collapsed);

        // Sekme seçmek şeridi açar.
        let _ = app.update(Message::RibbonTabSelected(RibbonTab::View));
        assert!(!app.ribbon_collapsed);

        let _ = app.update(Message::QuickToggled(1));
        assert_eq!(app.quick, [true, false, true, true]);
        assert!(!app.can_undo());
    }

    #[test]
    fn dock_layout_changes_with_messages() {
        let mut app = Showcase::new();
        assert!(app.docks.is_shown(DockPanel::Table));
        assert!(!app.docks.is_shown(DockPanel::Tasks));

        let _ = app.update(Message::Dock(docking::Event::Resized(
            docking::Side::Right,
            420.0,
        )));
        let _ = app.update(Message::Dock(docking::Event::Settled));
        assert_eq!(app.docks.size(docking::Side::Right), 420.0);

        // Panel kapanır, yeniden açılınca aynı kenara döner.
        let _ = app.update(Message::PanelToggled(DockPanel::Layers));
        assert!(!app.docks.contains(DockPanel::Layers));
        let _ = app.update(Message::PanelToggled(DockPanel::Layers));
        assert_eq!(
            app.docks.slot(DockPanel::Layers),
            Some(docking::Slot::Docked(docking::Side::Right, 1))
        );

        // Görevler arkadaki sekmesinden öne gelir. Tablo komutu arkadaki
        // tabloyu öne getirir, öndeyse kapatır.
        let _ = app.update(Message::PanelShown(DockPanel::Tasks));
        assert!(app.docks.is_shown(DockPanel::Tasks));

        submit(&mut app, "tablo");
        assert!(app.docks.is_shown(DockPanel::Table));
        submit(&mut app, "tablo");
        assert!(!app.docks.contains(DockPanel::Table));
        let _ = app.update(Message::OpenTable(2));
        assert!(app.docks.is_shown(DockPanel::Table));
    }

    #[test]
    fn the_measure_window_follows_the_measure_tool() {
        let mut app = Showcase::new();

        submit(&mut app, "olc");
        assert!(app.windows.is_open(Pane::Measure));

        picked(&mut app, 32.85, 39.93);
        picked(&mut app, 35.48, 38.72);
        assert_eq!(app.measurement.segment_count(), 1);

        // Pencereyi kapatmak araçtan çıkar ve ölçümü temizler.
        let _ = app.update(Message::Window(floating::Event::Closed(Pane::Measure)));
        assert_eq!(app.tool, Tool::Select);
        assert!(app.measurement.is_empty());

        // Başka araca geçmek de pencereyi kapatır.
        let _ = app.update(Message::PaneToggled(Pane::Measure));
        assert_eq!(app.tool, Tool::Measure);
        let _ = app.update(Message::ToolSelected(Tool::Line));
        assert!(!app.windows.is_open(Pane::Measure));
    }

    #[test]
    fn go_to_validates_and_centers_or_adds_points() {
        let mut app = Showcase::new();

        // Açılınca görünümün merkeziyle dolar.
        submit(&mut app, "git");
        assert!(app.windows.is_open(Pane::GoTo));
        assert_eq!(app.go_to.latitude, "39.00000");

        let _ = app.update(Message::GoToLatitude("95".to_owned()));
        assert!(app.go_to.latitude().is_err());
        assert_eq!(app.go_to.location(), None);

        let _ = app.update(Message::GoToLatitude("41,01".to_owned()));
        let _ = app.update(Message::GoToLongitude("28.98".to_owned()));
        let _ = app.update(Message::GoToCentered);
        assert_eq!(app.viewport.center, LonLat::new(28.98, 41.01));

        // Çizim aracı yokken nokta eklenmez; nokta aracında eklenir.
        let _ = app.update(Message::GoToPlaced);
        assert!(app.layers[DRAWING_LAYER].features.is_empty());

        let _ = app.update(Message::ToolSelected(Tool::Point));
        let _ = app.update(Message::GoToPlaced);
        assert_eq!(app.layers[DRAWING_LAYER].features.len(), 1);
    }

    #[test]
    fn the_style_window_edits_the_active_layer() {
        let mut app = Showcase::new();

        let _ = app.update(Message::StyleOpened(3));
        assert!(app.windows.is_open(Pane::Style));
        assert_eq!(app.active_layer, 3);

        let red = iced::Color::from_rgb(1.0, 0.0, 0.0);
        let _ = app.update(Message::LayerColor(3, red));
        let _ = app.update(Message::LayerStroke(3, 3.0));
        assert_eq!(app.layers[3].color, red);
        assert_eq!(app.layers[3].stroke_width, 3.0);

        // Ribbondaki düğme açık pencereyi kapatır; komut yalnızca açar.
        let _ = app.update(Message::PaneToggled(Pane::Style));
        assert!(!app.windows.is_open(Pane::Style));
        submit(&mut app, "stil");
        submit(&mut app, "stil");
        assert!(app.windows.is_open(Pane::Style));
    }

    #[test]
    fn deleting_drawings_can_be_undone_from_the_notification() {
        let mut app = Showcase::new();

        submit(&mut app, "nokta");
        picked(&mut app, 32.85, 39.93);
        picked(&mut app, 35.48, 38.72);
        let _ = app.update(Message::SelectNode(NodeId::Layer(DRAWING_LAYER)));
        let _ = app.update(Message::DeleteSelection);

        assert!(app.layers[DRAWING_LAYER].features.is_empty());
        let (_, toast, _) = app.toasts.iter().last().expect("bildirim yok");
        assert_eq!(toast.title(), "2 çizim silindi");

        // "Geri al" çizimleri numaralarıyla geri koyar ve seçer.
        let _ = app.update(Message::UndoDelete);
        let ids: Vec<ObjectId> = app.layers[DRAWING_LAYER]
            .features
            .iter()
            .map(|feature| feature.id)
            .collect();
        assert_eq!(ids, [ObjectId(1), ObjectId(2)]);
        assert_eq!(app.selection.len(), 2);

        // Örnek veri silinmez; haritanın üstünde salt okunur şeridi açılır.
        let _ = app.update(Message::SelectFeature(
            FeatureRef::new(1, ObjectId(1)),
            SelectionMode::New,
        ));
        let _ = app.update(Message::DeleteSelection);
        assert!(app.read_only_notice);

        let _ = app.update(Message::BannerDismissed);
        assert!(!app.read_only_notice);
    }

    #[test]
    fn clearing_drawings_and_quitting_ask_for_confirmation() {
        let mut app = Showcase::new();

        // Çizim yokken sorulacak bir şey yok.
        let _ = app.update(Message::ClearDrawings);
        assert_eq!(app.confirm, None);

        submit(&mut app, "nokta");
        picked(&mut app, 32.85, 39.93);

        let _ = app.update(Message::ClearDrawings);
        assert_eq!(app.confirm, Some(Confirmation::ClearDrawings));

        // Esc vazgeçer; çizim yerinde kalır.
        let _ = app.update(Message::Escape);
        assert_eq!(app.confirm, None);
        assert_eq!(app.layers[DRAWING_LAYER].features.len(), 1);

        // Enter onaylar.
        let _ = app.update(Message::ClearDrawings);
        let _ = app.update(Message::EnterPressed);
        assert_eq!(app.confirm, None);
        assert!(app.layers[DRAWING_LAYER].features.is_empty());

        // Kaydedilmemiş çizim varken çıkış onay ister.
        picked(&mut app, 35.48, 38.72);
        let _ = app.update(Message::Quit);
        assert_eq!(app.confirm, Some(Confirmation::Quit));
    }

    #[test]
    fn the_import_wizard_adds_a_layer_through_a_background_job() {
        let mut app = Showcase::new();
        let layers = app.layers.len();

        submit(&mut app, "iceaktar");
        assert!(app.import.is_some());

        // Dosya seçilmeden ilerlenmez; bozuk dosyada da.
        let _ = app.update(Message::ImportNext);
        assert_eq!(app.import.as_ref().map(|wizard| wizard.step), Some(0));

        let _ = app.update(Message::ImportSource(import::Source::Broken));
        let _ = app.update(Message::ImportNext);
        assert_eq!(app.import.as_ref().map(|wizard| wizard.step), Some(0));

        let _ = app.update(Message::ImportSource(import::Source::Stations));

        for _ in 0..4 {
            let _ = app.update(Message::EnterPressed);
        }

        // Son adım işi başlatır; sihirbaz kapanır.
        assert!(app.import.is_none());
        assert!(app.jobs.is_busy());

        for _ in 0..30 {
            let _ = app.update(Message::JobTick);
        }

        assert_eq!(app.layers.len(), layers + 1);
        assert_eq!(app.layers[layers].name, "Meteoroloji istasyonları");
        assert_eq!(app.layers[layers].features.len(), 24);
        assert!(app.layers[layers].visible);
        assert_eq!(app.tables.len(), app.layers.len());
        assert_eq!(
            app.layer_tree.roots[1],
            crate::layer_tree::Entry::Layer(layers)
        );
    }

    #[test]
    fn layer_properties_apply_only_valid_drafts() {
        let mut app = Showcase::new();

        let _ = app.update(Message::PropertiesOpened(1));
        let _ = app.update(Message::PropertiesEdited(properties::Edit::Name(
            "Karayolları".to_owned(),
        )));

        // Başka katmanın adı verilemez.
        let _ = app.update(Message::PropertiesAccepted);
        assert!(app.properties.is_some());
        assert_eq!(app.layers[1].name, "Şehirler");

        let _ = app.update(Message::PropertiesEdited(properties::Edit::Name(
            "İller".to_owned(),
        )));
        let _ = app.update(Message::PropertiesEdited(properties::Edit::Visible(false)));
        let _ = app.update(Message::PropertiesApplied);
        assert_eq!(app.layers[1].name, "İller");
        assert!(!app.layers[1].visible);
        assert!(app.properties.as_ref().is_some_and(|open| !open.is_dirty()));

        // İptal taslağı atar.
        let _ = app.update(Message::PropertiesEdited(properties::Edit::Opacity(0.3)));
        let _ = app.update(Message::PropertiesClosed);
        assert_eq!(app.layers[1].opacity, 1.0);
    }

    #[test]
    fn accent_and_backdrop_commands_offer_presets_and_hex_codes() {
        let mut app = Showcase::new();

        submit(&mut app, "vurgu");
        assert_eq!(app.pending, Some(Pending::Accent));

        // Hazır renk adıyla ya da baş harfleriyle seçilir.
        submit(&mut app, "tur");
        assert_eq!(app.accent, Accent::Turquoise);
        assert_eq!(app.pending, None);

        // #RRGGBB de yazılabilir; yanlış yazım hata verir, komut sürer.
        submit(&mut app, "renk");
        submit(&mut app, "#zz0000");
        assert!(matches!(app.history.last(), Some(Entry::Error(_))));
        assert_eq!(app.pending, Some(Pending::Accent));

        submit(&mut app, "#e8618c");
        assert_eq!(app.accent, Accent::Custom(0xe8618c));
        assert_eq!(
            kentos_rc::theme::Tokens::of(&app.theme()).accent,
            Accent::Custom(0xe8618c).color(Mode::Dark)
        );

        submit(&mut app, "zemin");
        assert_eq!(app.pending, Some(Pending::Backdrop));
        submit(&mut app, "siyah");
        assert_eq!(app.backdrop, Backdrop::Black);
    }

    #[test]
    fn themes_are_chosen_from_commands_and_messages() {
        let mut app = Showcase::new();

        submit(&mut app, "gece");
        assert_eq!(app.mode, Mode::Night);

        submit(&mut app, "karsitlik");
        assert_eq!(app.mode, Mode::HighContrast);
        assert_eq!(
            kentos_rc::theme::Tokens::of(&app.theme()).mode,
            Mode::HighContrast
        );

        // TEMA koyu ile aydınlık arasında geçer.
        submit(&mut app, "tema");
        assert_eq!(app.mode, Mode::Light);

        let _ = app.update(Message::ThemeSelected(Mode::Night));
        assert_eq!(app.mode, Mode::Night);
    }

    #[test]
    fn unknown_commands_are_errors_and_enter_repeats_the_last_command() {
        let mut app = Showcase::new();

        submit(&mut app, "merhaba");
        assert!(matches!(app.history.last(), Some(Entry::Error(_))));

        let grid = app.options.grid;
        submit(&mut app, "izgara");
        submit(&mut app, "");
        assert_eq!(app.options.grid, grid);
        assert!(matches!(
            &app.history[app.history.len() - 2],
            Entry::Input(input) if input == "IZGARA"
        ));
    }
}
