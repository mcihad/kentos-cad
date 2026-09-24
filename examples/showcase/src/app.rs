//! Uygulama durumu ve güncelleme mantığı.

use std::time::Duration;

use iced::keyboard::Modifiers;
use iced::{Event, Point, Size, Subscription, Task, Theme, event, keyboard, window};

use kentos_rc::attribute::query::Edit;
use kentos_rc::attribute::{DateTime, Field, FieldKind, ObjectId, Query, Value, text};
use kentos_rc::spatial::model_space::{self, Options};
use kentos_rc::spatial::{
    Bounds, Draft, Feature, FeatureRef, Geometry, Layer, LonLat, Measurement, Selection,
    SelectionMode, Tool, Viewport, feature, format, query,
};
use kentos_rc::theme::{self, Mode};
use kentos_rc::widget::command_line::Entry;
use kentos_rc::widget::inspector;

use crate::command::{self, Command};
use crate::gallery::Gallery;
use crate::layer_tree::{LayerTree, NodeId};
use crate::message::{AppCommand, Message, QueryPurpose, RECENT_DRAWINGS, RibbonTab, Setting};
use crate::sample;
use crate::table::{self, TableView};

/// Çizim araçlarının geometri eklediği katman; listenin en üstündedir.
pub const DRAWING_LAYER: usize = 0;

/// Yerel saat dilimi: Türkiye (UTC+3), dakika olarak.
pub const TIME_ZONE: i32 = 180;

/// Pencerenin açılış boyutu.
pub const WINDOW_SIZE: Size = Size::new(1440.0, 900.0);

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

/// KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
pub struct Showcase {
    pub(crate) viewport: Viewport,
    pub(crate) layers: Vec<Layer>,
    /// Katmanların gruplar hâlindeki ağacı ve görünürlükleri.
    pub(crate) layer_tree: LayerTree,
    pub(crate) active_layer: usize,
    pub(crate) selection: Selection,
    pub(crate) hover: Option<FeatureRef>,
    pub(crate) cursor: Option<LonLat>,
    /// Basılı değiştirici tuşlar; tabloda Shift ve Ctrl ile seçim için.
    modifiers: Modifiers,

    pub(crate) inspector: inspector::State,
    /// Haritadan varlık seçimi sürüyorsa ayrıntıları.
    pub(crate) picking: Option<Pick>,
    /// Katmanların tablo ayarları; katmanlarla aynı sırada.
    pub(crate) tables: Vec<TableView>,
    pub(crate) table_open: bool,
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

    pub(crate) ribbon_tab: RibbonTab,
    pub(crate) app_menu_open: bool,
    pub(crate) app_menu_hover: Option<AppCommand>,
    pub(crate) help_open: bool,

    pub(crate) command_input: String,
    pub(crate) history: Vec<Entry>,

    pub(crate) gallery: Gallery,
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

        let history = vec![
            Entry::Output(format!(
                "KentOS CAD hazır: {} katman, {feature_count} öğe yüklendi.",
                layers.len()
            )),
            Entry::Output(
                "Komut yazın veya şeritten bir araç seçin. Komut listesi için YARDIM.".to_owned(),
            ),
        ];

        let mut layer_tree = sample::layer_tree();
        layer_tree.selected = Some(NodeId::Layer(DRAWING_LAYER + 1));

        Self {
            viewport: Viewport::new(INITIAL_CENTER, INITIAL_ZOOM, Size::new(900.0, 640.0)),
            layer_tree,
            active_layer: DRAWING_LAYER + 1,
            selection: Selection::new(),
            hover: None,
            cursor: None,
            modifiers: Modifiers::default(),
            inspector: inspector::State::new(),
            picking: None,
            tables: vec![TableView::default(); layers.len()],
            table_open: true,
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
            ribbon_tab: RibbonTab::Home,
            app_menu_open: false,
            app_menu_hover: None,
            help_open: false,
            command_input: String::new(),
            history,
            gallery: Gallery::default(),
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keys = event::listen_with(keyboard_event);

        if self.view_cube {
            Subscription::batch([
                keys,
                iced::time::every(Duration::from_millis(33)).map(|_| Message::Tick),
            ])
        } else {
            keys
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModelSpace(event) => self.handle_model_space(event),
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
            Message::LayerActivated(index) => self.activate_layer(index),
            Message::ZoomToLayer(index) => {
                self.activate_layer(index);
                self.zoom_to_node(NodeId::Layer(index));
            }
            Message::ShowAllLayers => self.set_all_layers_visible(true),
            Message::HideAllLayers => self.set_all_layers_visible(false),

            Message::TreeSelected(node) => {
                self.layer_tree.selected = Some(node);

                if let Some(layer) = node.layer() {
                    self.active_layer = layer;
                }
            }
            Message::TreeToggled(node) => self.layer_tree.toggle(node),
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
                self.table_open = true;
            }
            Message::ClearDrawings => self.clear_drawings(),

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

                return iced::clipboard::write(text);
            }
            Message::CopyRow(reference) => {
                if let Some(text) = self.row_text(reference) {
                    self.log("Satır panoya kopyalandı (sekmeyle ayrılmış).");

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
                        inspector::Action::Change { id, value } => self.set_attribute(id, value),
                        inspector::Action::Pick(field) => self.start_pick(field),
                        inspector::Action::CancelPick => self.cancel_pick(),
                        inspector::Action::Navigate { id, object } => self.navigate(id, object),
                    }
                }
            }

            Message::TableToggled => self.table_open = !self.table_open,
            Message::TableRowPressed(reference) => self.press_row(reference),
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

                // Galeride model alanı görünmez; imleç bilgisi eskimesin,
                // haritadan seçim de sürmesin.
                if tab == RibbonTab::Gallery {
                    self.cursor = None;
                    self.hover = None;
                    self.cancel_pick();
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
                self.log(format!(
                    "{format} olarak dışa aktarma bu sürümde henüz yok."
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
                self.log(format!(
                    "Komutlar: {}",
                    command::names().collect::<Vec<_>>().join(", ")
                ));
            }

            Message::GalleryPageSelected(page) => self.gallery.page = page,
            Message::Gallery(demo) => {
                if let Some(output) = self.gallery.update(demo) {
                    self.log(output);
                }
            }

            Message::CommandInput(value) => self.command_input = value,
            Message::CommandSubmitted => {
                let input = std::mem::take(&mut self.command_input);
                let input = input.trim();

                if input.is_empty() {
                    return Task::none();
                }

                self.history.push(Entry::Input(input.to_owned()));

                return match command::parse(input) {
                    Some(command) => self.run_command(command),
                    None => {
                        self.log(format!("Bilinmeyen komut: {}", command::normalize(input)));
                        Task::none()
                    }
                };
            }

            Message::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            Message::Escape => self.escape(),
            Message::Tick => {
                self.cube_rotation = (self.cube_rotation + CUBE_SPEED) % std::f32::consts::TAU;
            }
            Message::Quit => return window::latest().and_then(window::close),
        }

        Task::none()
    }

    // --- Model alanı -----------------------------------------------------

    fn handle_model_space(&mut self, event: model_space::Event) {
        match event {
            model_space::Event::Resized(size) => self.viewport.size = size,
            model_space::Event::CursorMoved(location) => {
                self.cursor = Some(location);
                self.hover = self.hit(self.viewport.project(location));
            }
            model_space::Event::CursorLeft => {
                self.cursor = None;
                self.hover = None;
            }
            model_space::Event::Panned { delta, cursor } => {
                self.viewport.pan_by(delta);
                self.cursor = Some(cursor);
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
    /// katmanda aranır.
    fn hit(&self, point: Point) -> Option<FeatureRef> {
        match &self.picking {
            Some(pick) => query::hit_test_in(&self.layers, pick.target, &self.viewport, point),
            None => query::hit_test(&self.layers, &self.viewport, point),
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.draft.clear();
        // Çizim araçlarında tıklama nokta girişidir; haritadan seçim sürmez.
        self.cancel_pick();

        if tool != Tool::Measure {
            self.measurement.clear();
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

    /// Tamamlanan geometriyi çizim katmanına ekler ve seçer.
    fn commit_drawing(&mut self, geometry: Geometry) {
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

    // --- Seçim -----------------------------------------------------------

    /// Seç aracında tıklama: Shift seçime ekler, Ctrl seçimden çıkarır;
    /// değiştirici yoksa boş yere tıklamak seçimi kaldırır.
    fn click_select(&mut self, point: Point, modifiers: Modifiers) {
        let hit = query::hit_test(&self.layers, &self.viewport, point);
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
        let found = query::in_bounds(&self.layers, &self.viewport, bounds, crossing);
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

        if drawings.is_empty() {
            self.log("Yalnızca Çizimler katmanındaki öğeler silinebilir.");
            return;
        }

        if let Some(layer) = self.layers.get_mut(DRAWING_LAYER) {
            for drawing in &drawings {
                layer.remove(drawing.id);
            }
        }

        self.selection.retain(|item| item.layer != DRAWING_LAYER);
        self.hover = None;
        self.sync_inspector();

        let mut output = format!("{} çizim silindi.", drawings.len());

        if !others.is_empty() {
            output.push_str(&format!(
                " Diğer katmanlardaki {} öğe silinmedi: yalnızca çizimler silinebilir.",
                others.len()
            ));
        }

        self.log(output);
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
                self.table_open = true;
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
            .map_or(0, |layer| std::mem::take(&mut layer.features).len());

        self.selection.retain(|item| item.layer != DRAWING_LAYER);
        self.hover = None;
        self.sync_inspector();
        self.log(format!("{removed} çizim silindi."));
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
        if self.app_menu_open {
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
        } else {
            self.measurement.clear();
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
        self.log(format!("{} tema etkin.", mode.label()));
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
                self.table_open = !self.table_open;
                self.log(if self.table_open {
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
        self.history.push(Entry::Output(output.into()));

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

/// Klavye: değiştirici tuşlar her zaman izlenir; kısayollar yalnızca olayı
/// bir bileşen (ör. metin girişi) kullanmadıysa çalışır.
fn keyboard_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    let Event::Keyboard(event) = event else {
        return None;
    };

    match event {
        keyboard::Event::ModifiersChanged(modifiers) => Some(Message::ModifiersChanged(modifiers)),
        keyboard::Event::KeyPressed { key, modifiers, .. } if status == event::Status::Ignored => {
            shortcut(key.as_ref(), modifiers)
        }
        _ => None,
    }
}

/// CAD kısayolları: Esc, Delete, Ctrl+A (tümünü seç), F1 (yardım), F3
/// (yakalama), F7 (ızgara).
fn shortcut(key: keyboard::Key<&str>, modifiers: Modifiers) -> Option<Message> {
    use keyboard::Key;
    use keyboard::key::Named;

    match key {
        Key::Named(Named::Escape) => Some(Message::Escape),
        Key::Named(Named::Delete) => Some(Message::DeleteSelection),
        Key::Named(Named::F1) => Some(Message::HelpToggled),
        Key::Named(Named::F3) => Some(Message::Toggle(Setting::Snap)),
        Key::Named(Named::F7) => Some(Message::Toggle(Setting::Grid)),
        Key::Character("a" | "A") if modifiers.command() => Some(Message::SelectAll),
        _ => None,
    }
}
