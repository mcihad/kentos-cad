//! Uygulama durumu ve güncelleme mantığı.

use std::time::Duration;

use iced::{Event, Size, Subscription, Task, Theme, event, keyboard, window};

use kentos_rc::spatial::model_space::{self, Options};
use kentos_rc::spatial::{
    Draft, FeatureRef, Geometry, Layer, LonLat, Measurement, Tool, Viewport, feature, format, query,
};
use kentos_rc::theme::{self, Mode};
use kentos_rc::widget::command_line::Entry;

use crate::command::{self, Command};
use crate::message::{AppCommand, Message, RECENT_DRAWINGS, RibbonTab, Setting};
use crate::sample;

/// Çizim araçlarının geometri eklediği katman; listenin en üstündedir.
pub const DRAWING_LAYER: usize = 0;

const INITIAL_CENTER: LonLat = LonLat::new(32.0, 39.0);
const INITIAL_ZOOM: f64 = 6.0;

/// Yakınlaştır/uzaklaştır düğmelerinin adımı.
const ZOOM_STEP: f64 = 0.8;

/// Komut geçmişinde tutulan en fazla satır.
const HISTORY_LIMIT: usize = 80;

/// ViewCube'un her karede döndüğü açı (radyan).
const CUBE_SPEED: f32 = 0.012;

/// KentOS CAD: kentos-rc bileşenlerinin vitrin uygulaması.
pub struct Showcase {
    pub(crate) viewport: Viewport,
    pub(crate) layers: Vec<Layer>,
    pub(crate) active_layer: usize,
    pub(crate) selection: Option<FeatureRef>,
    pub(crate) hover: Option<FeatureRef>,
    pub(crate) cursor: Option<LonLat>,

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

        Self {
            viewport: Viewport::new(INITIAL_CENTER, INITIAL_ZOOM, Size::new(900.0, 640.0)),
            layers,
            active_layer: DRAWING_LAYER + 1,
            selection: None,
            hover: None,
            cursor: None,
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
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keys = event::listen_with(keyboard_shortcut);

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
            Message::FocusSelection => self.focus_selection(),

            Message::LayerVisibility(index, visible) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    layer.visible = visible;
                }
            }
            Message::LayerOpacity(index, opacity) => {
                if let Some(layer) = self.layers.get_mut(index) {
                    layer.opacity = opacity;
                }
            }
            Message::LayerActivated(index) => {
                if index < self.layers.len() {
                    self.active_layer = index;
                }
            }
            Message::ZoomToLayer(index) => {
                if let Some(layer) = self.layers.get(index) {
                    self.active_layer = index;

                    if let Some(bounds) = layer.bounds() {
                        self.viewport.fit_bounds(bounds, 48.0);
                    }
                }
            }
            Message::ShowAllLayers => self.set_all_layers_visible(true),
            Message::HideAllLayers => self.set_all_layers_visible(false),

            Message::FeatureSelected(reference) => {
                self.selection = Some(reference);
                self.active_layer = reference.layer;
                self.focus_selection();
            }
            Message::ClearSelection => self.selection = None,
            Message::DeleteSelection => self.delete_selection(),
            Message::ClearMeasurement => self.measurement.clear(),

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

            Message::RibbonTabSelected(tab) => self.ribbon_tab = tab,
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

            Message::Escape => {
                self.finish_draft();
                self.measurement.clear();
                self.selection = None;
                self.help_open = false;
                self.app_menu_open = false;
            }
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
                self.hover = query::hit_test(
                    &self.layers,
                    &self.viewport,
                    self.viewport.project(location),
                );
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
            model_space::Event::Clicked(location) => {
                self.selection = query::hit_test(
                    &self.layers,
                    &self.viewport,
                    self.viewport.project(location),
                );

                if let Some(selection) = self.selection {
                    self.active_layer = selection.layer;
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

    fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.draft.clear();

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

        let (key, value) = match &geometry {
            Geometry::Point(location) => ("Konum", format::decimal(*location)),
            Geometry::Line(_) => ("Uzunluk", format::distance(geometry.length_meters())),
            Geometry::Polygon(_) => ("Çevre", format::distance(geometry.length_meters())),
        };

        self.drawn_count += 1;
        let name = format!("{} {}", self.tool.label(), self.drawn_count);

        layer.visible = true;
        layer.features.push(
            kentos_rc::spatial::Feature::new(name.clone(), geometry)
                .with("Tür", self.tool.label())
                .with(key, value),
        );

        self.selection = Some(FeatureRef::new(DRAWING_LAYER, layer.features.len() - 1));
        self.active_layer = DRAWING_LAYER;
        self.log(format!("{name} çizildi."));
    }

    fn delete_selection(&mut self) {
        let Some(selection) = self.selection else {
            return;
        };

        if selection.layer != DRAWING_LAYER {
            self.log("Yalnızca Çizimler katmanındaki öğeler silinebilir.");
            return;
        }

        if let Some(layer) = self.layers.get_mut(DRAWING_LAYER)
            && selection.feature < layer.features.len()
        {
            let removed = layer.features.remove(selection.feature);
            self.selection = None;
            self.hover = None;
            self.log(format!("{} silindi.", removed.name));
        }
    }

    // --- Görünüm ---------------------------------------------------------

    fn zoom(&mut self, delta: f64) {
        let anchor = self.viewport.center_point();
        self.viewport.zoom_by(delta, anchor);
    }

    /// Seçili öğeyi görünümün ortasına getirir; büyük öğeyi sınırlarına
    /// sığdırır, noktaya sabit bir yakınlaştırmayla odaklanır.
    fn focus_selection(&mut self) {
        let bounds = self
            .selection
            .and_then(|selection| selection.resolve(&self.layers))
            .and_then(|(_, feature)| feature.bounds());

        if let Some(bounds) = bounds {
            self.viewport.focus(bounds, 140.0, 12.0);
        }
    }

    fn set_all_layers_visible(&mut self, visible: bool) {
        for layer in &mut self.layers {
            layer.visible = visible;
        }

        self.log(if visible {
            "Tüm katmanlar gösterildi."
        } else {
            "Tüm katmanlar gizlendi."
        });
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
            self.selection = None;
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
                if self.selection.is_some() {
                    self.focus_selection();
                    self.log("Seçili öğeye odaklanıldı.");
                } else {
                    self.log("Odaklanacak seçili öğe yok.");
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
            Command::Delete => {
                if self.selection.is_some() {
                    self.delete_selection();
                } else {
                    self.log("Silinecek seçili öğe yok.");
                }
            }
            Command::Clear => {
                self.draft.clear();
                self.measurement.clear();
                self.selection = None;
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

/// CAD kısayolları: Esc, Delete, F1 (yardım), F3 (yakalama), F7 (ızgara).
fn keyboard_shortcut(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(key),
        ..
    }) = event
    else {
        return None;
    };

    match key {
        keyboard::key::Named::Escape => Some(Message::Escape),
        keyboard::key::Named::Delete => Some(Message::DeleteSelection),
        keyboard::key::Named::F1 => Some(Message::HelpToggled),
        keyboard::key::Named::F3 => Some(Message::Toggle(Setting::Snap)),
        keyboard::key::Named::F7 => Some(Message::Toggle(Setting::Grid)),
        _ => None,
    }
}
