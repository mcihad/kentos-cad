//! Katmanlar › Veri kaynağına taşı… (docs/adr/9107 §8): a drawing layer of a CBS project whose objects are kept in the
//! project file moved into a data source in one step. The window asks where (a GeoPackage file, made when it is not
//! there, or a PostgreSQL connection and schema) and the table's name; Taşı runs `cad.layers.migrate`: the objects
//! become the table's rows and a data layer reading it takes the layer's place with its name and style. One undo step.

use std::path::PathBuf;

use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use kentos_commands::{CommandResult, CreateTarget, LayersMigrate};
use kentos_native_application::ExecutionContext;
use kentos_ui::label;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Form, overlay};

use crate::app::{App, Message};
use crate::exchange::words;

/// The window's state.
#[derive(Debug, Clone)]
pub(crate) struct MigrateWindow {
    layer: String,
    title: String,
    objects: usize,
    /// Its texts, written into a table of their own beside its table (docs/adr/9114 §2).
    texts: usize,
    target: CreateTarget,
    path: String,
    connection: Option<usize>,
    address: String,
    schema: String,
    table: String,
    said: Option<String>,
}

/// What the window says.
#[derive(Debug, Clone)]
pub enum Event {
    /// Veri kaynağına taşı… on a layer.
    Open(String),
    Target(usize),
    Path(String),
    Browse,
    Browsed(Option<PathBuf>),
    Connection(usize),
    Address(String),
    Schema(String),
    Table(String),
    Run,
    Close,
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::LayerMigrate(e))
}

impl App {
    /// Whether a layer is one Veri kaynağına taşı… moves: a CBS project's drawing layer holding objects.
    pub(crate) fn migrates(&self, node: &kentos_commands::LayerNode) -> bool {
        let Some(doc) = &self.document else {
            return false;
        };
        doc.model.settings().project_type() == Some(kentos_commands::Workspace::Gis)
            && node.kind == kentos_commands::LayerNodeType::Layer
            && node.data.is_none()
            && node.service.is_none()
            && self.objects_below(node) > 0
    }

    pub(crate) fn layer_migrate_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Open(layer) = event {
            let Some(node) = self.document.as_ref().and_then(|d| d.model.layers().get(&layer)).cloned() else {
                return Task::none();
            };
            let texts = self.document.as_ref().map_or(0, |d| {
                d.model
                    .by_layer_with_uids(&node.id)
                    .filter(|(_, e)| matches!(e, kentos_commands::Entity::Text(_)))
                    .count()
            });
            self.gis.layer_migrate = Some(MigrateWindow {
                objects: self.objects_below(&node),
                texts,
                layer: node.id.clone(),
                table: kentos_native_application::layer_export::table_name_of(&node.name),
                title: node.name.clone(),
                target: CreateTarget::Gpkg,
                path: self.likely_geopackage(),
                connection: None,
                address: String::new(),
                schema: "public".into(),
                said: None,
            });
            self.dialog = Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::LayerMigrate));
            return Task::none();
        }
        let Some(w) = self.gis.layer_migrate.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Open(_) => {}
            Event::Target(i) => w.target = if i == 1 { CreateTarget::Postgis } else { CreateTarget::Gpkg },
            Event::Path(p) => w.path = p,
            Event::Browse => {
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Katmanın taşınacağı GeoPackage")
                            .add_filter("GeoPackage (.gpkg)", &["gpkg"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| message(Event::Browsed(p)),
                );
            }
            Event::Browsed(Some(p)) => {
                let p = if kentos_native_application::project_store::is_geopackage(&p) { p } else { p.with_extension("gpkg") };
                w.path = p.display().to_string();
            }
            Event::Browsed(None) => {}
            Event::Connection(i) => {
                w.connection = Some(i);
                if let Some(c) = kentos_postgis::connections::store().list().get(i) {
                    w.address = c.address();
                }
            }
            Event::Address(a) => {
                w.address = a;
                w.connection = None;
            }
            Event::Schema(s) => w.schema = s,
            Event::Table(t) => w.table = t,
            Event::Run => return self.run_layer_migrate(),
            Event::Close => {
                self.gis.layer_migrate = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Taşı: the layer moved into the source, one undo step; why not, at the window's foot.
    fn run_layer_migrate(&mut self) -> Task<Message> {
        let Some(w) = self.gis.layer_migrate.clone() else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = LayersMigrate {
            layer: w.layer.clone(),
            target: w.target,
            path: (w.target == CreateTarget::Gpkg).then(|| w.path.trim().to_owned()),
            address: (w.target == CreateTarget::Postgis).then(|| w.address.trim().to_owned()),
            schema: (w.target == CreateTarget::Postgis).then(|| {
                let s = w.schema.trim();
                if s.is_empty() { "public".to_owned() } else { s.to_owned() }
            }),
            table: Some(w.table.trim().to_owned()).filter(|t| !t.is_empty()),
        };
        match kentos_native_application::layers_migrate::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                self.gis.layer_migrate = None;
                self.dialog = None;
                self.selected_layer = Some(output.layer.clone());
                let place = std::path::Path::new(&output.address)
                    .file_name()
                    .map_or_else(|| output.address.clone(), |n| n.to_string_lossy().into_owned());
                // Its texts in a table of their own beside it (docs/adr/9114 §2): said with the table they went into.
                let texts = output.texts.as_ref().map_or_else(String::new, |t| {
                    format!(
                        " Yazıları ({}) “{}” tablosunda; üstündeki “{} (yazılar)” katmanı onları yazı olarak çizer.",
                        crate::view::thousands(t.written as f64),
                        t.table,
                        w.title
                    )
                });
                let words = format!(
                    "“{}” veri kaynağına taşındı: {} nesne {} içinde “{}” tablosunun satırları oldu.{texts} Katman artık tabloyu okuyor; geri al (Ctrl+Z) çizimdeki hâline döndürür.",
                    w.title,
                    crate::view::thousands(output.written as f64),
                    place,
                    output.table
                );
                self.say(kentos_interaction::Level::Success, words.clone());
                self.toast_saved("Kaynağa taşındı", words);
                for warning in warnings {
                    self.warn(warning.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                if let Some(w) = self.gis.layer_migrate.as_mut() {
                    w.said = Some(error.message);
                }
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        Task::none()
    }

    /// The window.
    pub(crate) fn layer_migrate_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.layer_migrate else {
            return column![].into();
        };
        let text = |placeholder: &str, value: &str, on: fn(String) -> Event| -> Element<'_, Message> {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| message(on(t))).into()
        };
        let mut form = Form::new().label_width(150.0).field(
            "Nerede",
            container(
                Select::new(
                    ["GeoPackage dosyası", "PostgreSQL veritabanı"].into_iter().map(Choice::new),
                    Some(usize::from(w.target == CreateTarget::Postgis)),
                    |i| message(Event::Target(i)),
                )
                .searchable(false),
            )
            .width(320),
        );
        match w.target {
            CreateTarget::Gpkg => {
                form = form
                    .field(
                        "Dosya",
                        row![
                            container(text("GeoPackage dosyasının yolu (.gpkg)", &w.path, Event::Path)).width(Length::Fill),
                            words::secondary("Gözat…", Some(message(Event::Browse))),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    )
                    .help("Dosya yoksa yapılır; varsa tablo onun içine eklenir.");
            }
            CreateTarget::Postgis => {
                let list = kentos_postgis::connections::store().list();
                if !list.is_empty() {
                    let names: Vec<String> = list.iter().map(|c| c.name.clone()).collect();
                    form = form.field(
                        "Bağlantı",
                        container(
                            Select::new(names.into_iter().map(Choice::new), w.connection, |i| message(Event::Connection(i)))
                                .searchable(false),
                        )
                        .width(320),
                    );
                }
                form = form
                    .field("Adres", text("postgresql://kullanıcı@sunucu:5432/veritabanı", &w.address, Event::Address))
                    .field("Şema", text("public", &w.schema, Event::Schema));
            }
        }
        form = form.field("Tablonun adı", text("katmanın adından", &w.table, Event::Table));
        let mut dialog = Dialog::new(format!("Veri kaynağına taşı — {}", w.title)).scroll(form);
        dialog = match &w.said {
            Some(said) => dialog.status(words::text_line(words::Kind::Error, said.clone())),
            None => dialog.status(label::caption(migrate_words(w))),
        };
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Event::Close))))
                .action(words::primary("Taşı", Some(message(Event::Run))))
                .width(620.0)
                .max_height(560.0)
                .on_close(Some(message(Event::Close))),
        )
    }
}

/// What Taşı will write, said under the window: the layer's objects as its table's rows, its texts into a table of their
/// own beside it (docs/adr/9114 §2), or a layer of texts only as its table.
fn migrate_words(w: &MigrateWindow) -> String {
    let others = w.objects.saturating_sub(w.texts);
    let n = |v: usize| crate::view::thousands(v as f64);
    let table = if w.table.trim().is_empty() {
        kentos_native_application::layer_export::table_name_of(&w.title)
    } else {
        w.table.trim().to_owned()
    };
    match (others, w.texts) {
        (_, 0) => format!("{} nesne tablonun satırları olur; katman yerinde kalır, adı ve stiliyle tabloyu okur.", n(others)),
        (0, texts) => format!("{} yazı tablonun satırları olur; katman yerinde kalır ve yazılarını tablodan çizer.", n(texts)),
        (others, texts) => format!(
            "{} nesne tablonun satırları olur; {} yazı yanındaki “{table}_yazilar” tablosuna. Katman yerinde kalır, yazıları üstündeki katmandan çizilir.",
            n(others),
            n(texts)
        ),
    }
}
