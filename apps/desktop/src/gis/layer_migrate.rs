//! Veri kaynağına taşı (docs/adr/9107 §8, 9321): a CBS project's drawing layers whose objects are kept in the project
//! file moved into a data source, one or many at once. The window asks where once (a GeoPackage file, made when it is
//! not there, or a PostgreSQL connection and schema) and lists the layers: each one's objects, the kind its table
//! holds and its table's name. The tables are written in the project's system.
//!
//! - Katmanlar › Veri kaynağına taşı… opens it on one layer.
//! - Kaydet of a CBS project whose file would hold layers' objects opens it on every one of them; Taşı ve kaydet moves
//!   them and goes on with the save.
//! - The first import or run output of a project whose data has no place yet opens it on what came in: the place is
//!   chosen once, and later imports go there.
//!
//! Taşı first asks each layer's move without writing (`cad.layers.migrate`'s plan). A table that is there, a name two
//! layers would share, objects no table holds are said at their rows, with a free name offered, and nothing is
//! written. Then the layers move one by one, each `cad.layers.migrate`'s step: the layer keeps its id and every
//! reference to it, and leaves the list. A failure stops there with its words at its row; Taşı again goes on with
//! those left, no table made twice.

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

/// A layer the window moves.
#[derive(Debug, Clone)]
pub(crate) struct MigrateRow {
    layer: String,
    title: String,
    objects: usize,
    /// Its texts, written into a table of their own beside its table (docs/adr/9114 §2).
    texts: usize,
    /// What its table holds, in words: nokta, çizgi, alan, yazı or karışık.
    kind: &'static str,
    table: String,
    /// What holds it back, at its row: a table that is there (a free name offered), a refusal.
    said: Option<String>,
}

/// What the window does once every layer is moved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum After {
    /// Katmanlar › Veri kaynağına taşı…: nothing more.
    Nothing,
    /// Kaydet: the save it held back goes on to its target.
    Save(crate::saving::Target),
    /// An import or a run brought layers into a project whose data had no place: the place is chosen now, once.
    Place,
}

/// The window's state.
#[derive(Debug, Clone)]
pub(crate) struct MigrateWindow {
    rows: Vec<MigrateRow>,
    target: CreateTarget,
    path: String,
    connection: Option<usize>,
    address: String,
    schema: String,
    said: Option<String>,
    after: After,
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
    /// A row's table name.
    Table(usize, String),
    Run,
    Close,
}

#[cfg(test)]
impl MigrateWindow {
    /// The layers the window moves, in its order.
    pub(crate) fn layers_for_test(&self) -> Vec<String> {
        self.rows.iter().map(|r| r.layer.clone()).collect()
    }

    /// A row's words, when it has some.
    pub(crate) fn said_for_test(&self, at: usize) -> Option<String> {
        self.rows.get(at).and_then(|r| r.said.clone())
    }

    /// A row's table name.
    pub(crate) fn table_for_test(&self, at: usize) -> String {
        self.rows.get(at).map(|r| r.table.clone()).unwrap_or_default()
    }

    /// What the window does after the move.
    pub(crate) fn after_for_test(&self) -> &After {
        &self.after
    }
}

impl MigrateWindow {
    /// The move of `row` into the window's place.
    fn input(&self, row: &MigrateRow) -> LayersMigrate {
        LayersMigrate {
            layer: row.layer.clone(),
            target: self.target,
            path: (self.target == CreateTarget::Gpkg).then(|| self.path.trim().to_owned()),
            address: (self.target == CreateTarget::Postgis).then(|| self.address.trim().to_owned()),
            schema: (self.target == CreateTarget::Postgis).then(|| {
                let s = self.schema.trim();
                if s.is_empty() { "public".to_owned() } else { s.to_owned() }
            }),
            table: Some(row.table.trim().to_owned()).filter(|t| !t.is_empty()),
        }
    }
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

    /// The layers of a CBS project whose objects would go into its project file (docs/adr/9196 §2), each id and name in
    /// the tree's order: the shared rule every client's save follows (`kentos_native_application::layer_kind`).
    pub(crate) fn data_held_in_file(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        kentos_native_application::layer_kind::held_in_file(&doc.model)
            .into_iter()
            .map(|h| (h.id, h.name))
            .collect()
    }

    /// Where a CBS project's tables are (docs/adr/9196 §2): the GeoPackage or schema it is kept in, else its first data
    /// layer's source, else the GeoPackage beside its file (the wizard's place, wizard/data_place.rs); none when nothing
    /// says.
    pub(crate) fn cbs_data_place(&self) -> Option<(CreateTarget, String, Option<String>)> {
        let doc = self.document.as_ref()?;
        if let Some(s) = doc.stored_source() {
            return Some(match s.file() {
                Some(f) => (CreateTarget::Gpkg, f.display().to_string(), None),
                None => (CreateTarget::Postgis, s.address.clone(), s.schema.clone()),
            });
        }
        let first = doc.model.layers().leaves().iter().find_map(|n| n.data.clone());
        if let Some(d) = first {
            return Some(match d.driver.as_str() {
                "postgis" => {
                    let schema = d.table.rsplit_once('.').map(|(s, _)| s.to_owned());
                    (CreateTarget::Postgis, d.address, schema)
                }
                _ => (CreateTarget::Gpkg, d.address.trim_start_matches("gpkg:").to_owned(), None),
            });
        }
        doc.path.as_ref().map(|p| (CreateTarget::Gpkg, p.with_extension("gpkg").display().to_string(), None))
    }

    /// A CBS project's layers its data would keep in the project file, made its data place's tables at once
    /// (docs/adr/9196 §2): what an import brought in goes where the project's data is, each one `cad.layers.migrate`'s
    /// step. Where nothing says where the data is yet, the window asks for the place once (docs/adr/9321); a layer that
    /// cannot go stays and the save asks for it.
    pub(crate) fn data_into_tables(&mut self, layers: Vec<(String, String)>) {
        if layers.is_empty() {
            return;
        }
        let Some((target, place, schema)) = self.cbs_data_place() else {
            if self.gis.data_place_asked || self.gis.layer_migrate.is_some() {
                return;
            }
            self.gis.data_place_asked = true;
            self.open_layer_migrate(layers.into_iter().map(|(id, _)| id).collect(), After::Place);
            return;
        };
        let mut moved = Vec::new();
        for (id, name) in layers {
            let Some(doc) = self.document.as_mut() else { return };
            let input = LayersMigrate {
                layer: id,
                target,
                path: (target == CreateTarget::Gpkg).then(|| place.clone()),
                address: (target == CreateTarget::Postgis).then(|| place.clone()),
                schema: (target == CreateTarget::Postgis).then(|| schema.clone().unwrap_or_else(|| "public".to_owned())),
                table: None,
            };
            match kentos_native_application::layers_migrate::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => moved.push(format!("“{name}” → “{}”", output.table)),
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    self.warn(format!(
                        "“{name}” tabloya taşınamadı: {} Kaydet, taşınmayan katmanları tek pencerede sorar.",
                        error.message
                    ));
                }
                CommandResult::Queued { .. } | CommandResult::Cancelled => {}
            }
        }
        if !moved.is_empty() {
            let file = std::path::Path::new(&place)
                .file_name()
                .map_or_else(|| place.clone(), |n| n.to_string_lossy().into_owned());
            self.say(
                kentos_interaction::Level::Success,
                format!(
                    "CBS projesinin verisi tablolarda durur: içe alınanlar {file} içinde tablo oldu ({}). Her biri bir \
                     adımda geri alınır.",
                    moved.join(", ")
                ),
            );
        }
    }

    /// A CBS project whose data has no place yet and was not asked for one this session (docs/adr/9321): a layer an
    /// import, a run or a tool brings in asks for the place.
    pub(crate) fn data_place_unasked(&self) -> bool {
        !self.gis.data_place_asked
            && self.gis.layer_migrate.is_none()
            && self.document.as_ref().is_some_and(|d| {
                d.model.settings().project_type() == Some(kentos_commands::Workspace::Gis)
            })
            && self.cbs_data_place().is_none()
    }

    /// Kaydet of a CBS project whose file would hold layers' objects (docs/adr/9196 §2, 9321): the window on every one
    /// of them, the save going on once they are moved.
    pub(crate) fn migrate_then_save(&mut self, held: &[(String, String)], target: crate::saving::Target) {
        self.open_layer_migrate(held.iter().map(|(id, _)| id.clone()).collect(), After::Save(target));
    }

    /// The window on `layers` (those a table can take), its place the project's data place, else the GeoPackage beside
    /// the file the save goes to, else the likely one.
    fn open_layer_migrate(&mut self, layers: Vec<String>, after: After) {
        let Some(doc) = &self.document else {
            return;
        };
        let rows: Vec<MigrateRow> = layers
            .iter()
            .filter_map(|id| doc.model.layers().get(id).cloned())
            .map(|node| {
                let texts = doc
                    .model
                    .by_layer_with_uids(&node.id)
                    .filter(|(_, e)| matches!(e, kentos_commands::Entity::Text(_)))
                    .count();
                let objects = self.objects_below(&node);
                MigrateRow {
                    kind: kind_of(&node, objects, texts),
                    table: kentos_native_application::layer_export::table_name_of(&node.name),
                    title: node.name.clone(),
                    layer: node.id.clone(),
                    objects,
                    texts,
                    said: None,
                }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        let beside = match &after {
            After::Save(crate::saving::Target::File(path)) => Some(path.with_extension("gpkg").display().to_string()),
            _ => None,
        };
        let (target, path, address, schema) = match self.cbs_data_place() {
            Some((CreateTarget::Postgis, address, schema)) => {
                (CreateTarget::Postgis, String::new(), address, schema.unwrap_or_else(|| "public".into()))
            }
            Some((_, path, _)) => (CreateTarget::Gpkg, path, String::new(), "public".into()),
            None => (CreateTarget::Gpkg, beside.unwrap_or_else(|| self.likely_geopackage()), String::new(), "public".into()),
        };
        self.gis.layer_migrate =
            Some(MigrateWindow { rows, target, path, connection: None, address, schema, said: None, after });
        self.dialog = Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::LayerMigrate));
    }

    /// Why a CBS project is not written to its file (docs/adr/9196 §2): the layers named and the way said.
    pub(crate) fn data_held_words(held: &[(String, String)]) -> String {
        let held: Vec<kentos_native_application::layer_kind::Held> = held
            .iter()
            .map(|(id, name)| kentos_native_application::layer_kind::Held { id: id.clone(), name: name.clone() })
            .collect();
        kentos_native_application::layer_kind::held_words(&held)
    }

    pub(crate) fn layer_migrate_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Open(layer) = event {
            self.open_layer_migrate(vec![layer], After::Nothing);
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
            Event::Table(at, t) => {
                if let Some(row) = w.rows.get_mut(at) {
                    row.table = t;
                    row.said = None;
                }
            }
            Event::Run => return self.run_layer_migrate(),
            Event::Close => {
                self.gis.layer_migrate = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Taşı: every layer's move asked first without writing, then the layers moved one by one, each its own undo step;
    /// what holds a layer back said at its row. Every layer moved: the save it held back goes on.
    fn run_layer_migrate(&mut self) -> Task<Message> {
        let Some(mut w) = self.gis.layer_migrate.clone() else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        // A layer moved or removed meanwhile leaves the list.
        w.rows.retain(|r| doc.model.layers().get(&r.layer).is_some_and(|n| n.data.is_none()));
        w.said = None;
        if !checked(&mut doc.model, &mut w) {
            w.said = Some("Satırlarda söylenenleri giderin ya da önerilen adları kabul edip yeniden Taşı'ya basın; hiçbir \
                           şey yazılmadı."
                .to_owned());
            self.gis.layer_migrate = Some(w);
            return Task::none();
        }
        let mut moved: Vec<String> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut place = String::new();
        while let Some(row) = w.rows.first().cloned() {
            let input = w.input(&row);
            match kentos_native_application::layers_migrate::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, warnings: said } => {
                    place = output.address.clone();
                    moved.push(format!("“{}” → “{}” ({} nesne)", row.title, output.table, output.written));
                    warnings.extend(said.into_iter().map(|w| w.message));
                    w.rows.remove(0);
                }
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    w.rows[0].said = Some(error.message);
                    w.said = Some(format!(
                        "{} katman taşındı; “{}” taşınamadı. Satırındaki nedeni giderip yeniden Taşı'ya basın: kalanlar \
                         taşınır, taşınanlar yeniden yazılmaz.",
                        moved.len(),
                        row.title
                    ));
                    break;
                }
                CommandResult::Queued { .. } | CommandResult::Cancelled => break,
            }
        }
        for warning in warnings {
            self.warn(warning);
        }
        if !moved.is_empty() {
            let file = std::path::Path::new(&place)
                .file_name()
                .map_or_else(|| place.clone(), |n| n.to_string_lossy().into_owned());
            let words = format!(
                "Veri kaynağına taşındı ({file}): {}. Katmanlar adları, stilleri ve onları anan ağ, form, ilişki ve \
                 görünümlerle tablolarını okuyor; her biri bir adımda geri alınır.",
                moved.join(", ")
            );
            self.say(kentos_interaction::Level::Success, words.clone());
            self.toast_saved("Kaynağa taşındı", words);
        }
        if !w.rows.is_empty() {
            self.gis.layer_migrate = Some(w);
            return Task::none();
        }
        self.gis.layer_migrate = None;
        self.dialog = None;
        match w.after {
            After::Save(target) => self.start_save(target),
            After::Nothing | After::Place => Task::none(),
        }
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
                    .help("Dosya yoksa yapılır; varsa tablolar onun içine eklenir.");
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
        let srid = self.document.as_ref().map_or(0, |d| d.model.settings().srid);
        let system = match kentos_project::crs::system(srid) {
            Some(s) if srid > 0 => format!("{} (EPSG:{srid})", s.name),
            _ => "projenin sistemi".to_owned(),
        };
        form = form.field("Koordinat sistemi", label::body(format!("{system}; tablolar projenin sisteminde yazılır.")));
        // The layers: each one's objects and kind, its table's name, what holds it back.
        let mut list = column![
            row![
                container(label::caption("Katman").style(muted)).width(Length::FillPortion(2)),
                container(label::caption("Tablonun adı").style(muted)).width(Length::FillPortion(3)),
            ]
            .spacing(10)
        ]
        .spacing(6);
        for (at, r) in w.rows.iter().enumerate() {
            let n = crate::view::thousands(r.objects as f64);
            list = list.push(
                row![
                    container(column![label::strong(r.title.clone()), label::caption(format!("{n} nesne · {}", r.kind)).style(muted)])
                        .width(Length::FillPortion(2)),
                    container(
                        kentos_ui::widget::text_field("katmanın adından", &r.table)
                            .on_input(move |t| message(Event::Table(at, t))),
                    )
                    .width(Length::FillPortion(3)),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            );
            if let Some(said) = &r.said {
                list = list.push(words::text_line(words::Kind::Warn, said.clone()));
            }
        }
        let (title, lead, action) = match &w.after {
            After::Save(_) => (
                "Kaydetmeden önce veriyi tablolara taşı".to_owned(),
                Some(
                    "Bu bir CBS projesi: verisi tablolarda durur, proje dosyası yapıyı, stilleri, ayarları ve açıklamaları \
                     tutar. Aşağıdaki katmanların nesneleri seçtiğiniz yerde tablo olur, sonra proje kaydedilir.",
                ),
                "Taşı ve kaydet",
            ),
            After::Place => (
                "Verinin yeri".to_owned(),
                Some(
                    "Bu CBS projesinin verisinin yeri henüz seçilmedi. Gelen katmanlar burada tablo olur; sonraki içe \
                     almalar da buraya yazılır.",
                ),
                "Taşı",
            ),
            After::Nothing => (
                format!("Veri kaynağına taşı — {}", w.rows.first().map(|r| r.title.as_str()).unwrap_or_default()),
                None,
                "Taşı",
            ),
        };
        let mut body = column![].spacing(12);
        if let Some(lead) = lead {
            body = body.push(label::body(lead));
        }
        body = body.push(form).push(list);
        let mut dialog = Dialog::new(title).scroll(body);
        dialog = match &w.said {
            Some(said) => dialog.status(words::text_line(words::Kind::Error, said.clone())),
            None => dialog.status(label::caption(migrate_words(w))),
        };
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Event::Close))))
                .action(words::primary(action, Some(message(Event::Run))))
                .width(680.0)
                .max_height(620.0)
                .on_close(Some(message(Event::Close))),
        )
    }
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    kentos_ui::style::text::muted(theme)
}

/// What Taşı will write, said under the window: one layer's objects as its table's rows (its texts into a table of
/// their own beside it, docs/adr/9114 §2), or how many layers and objects.
fn migrate_words(w: &MigrateWindow) -> String {
    let n = |v: usize| crate::view::thousands(v as f64);
    match w.rows.as_slice() {
        [] => "Taşınacak katman kalmadı.".to_owned(),
        [r] => {
            let others = r.objects.saturating_sub(r.texts);
            let table = table_of(r);
            match (others, r.texts) {
                (_, 0) => format!(
                    "{} nesne tablonun satırları olur; katman yerinde, kimliği, adı ve stiliyle tabloyu okur.",
                    n(others)
                ),
                (0, texts) => format!(
                    "{} yazı tablonun satırları olur; katman yerinde kalır ve yazılarını tablodan çizer.",
                    n(texts)
                ),
                (others, texts) => format!(
                    "{} nesne tablonun satırları olur; {} yazı yanındaki “{table}_yazilar” tablosuna. Katman yerinde kalır, \
                     yazıları üstündeki katmandan çizilir.",
                    n(others),
                    n(texts)
                ),
            }
        }
        rows => format!(
            "{} katmanın {} nesnesi tablolarına yazılır; her katman yerinde, kimliği, adı ve stiliyle tablosunu okur.",
            rows.len(),
            n(rows.iter().map(|r| r.objects).sum())
        ),
    }
}

/// A row's table: its name as typed, else the layer's name made a table name.
fn table_of(r: &MigrateRow) -> String {
    let t = r.table.trim();
    if t.is_empty() { kentos_native_application::layer_export::table_name_of(&r.title) } else { t.to_owned() }
}

/// What a layer's table holds, in words: its texts alone, else its kind.
fn kind_of(node: &kentos_commands::LayerNode, objects: usize, texts: usize) -> &'static str {
    if objects > 0 && texts == objects {
        return "yazı";
    }
    match node.geometry {
        Some(kentos_commands::LayerGeometry::Point) => "nokta",
        Some(kentos_commands::LayerGeometry::Line) => "çizgi",
        Some(kentos_commands::LayerGeometry::Area) => "alan",
        Some(kentos_commands::LayerGeometry::Annotation) => "açıklama",
        None => "karışık",
    }
}

/// Every row's move asked without writing (`cad.layers.migrate`'s plan), its table's name against the others' too.
/// What holds a row back is said at it; a table that is there or a name two rows share gets a free name in its place.
/// True when every row moves as it is.
fn checked(doc: &mut kentos_domain::Document, w: &mut MigrateWindow) -> bool {
    let fold = kentos_commands::fold;
    let mut ok = true;
    let mut names: Vec<String> = Vec::new();
    for at in 0..w.rows.len() {
        let table = table_of(&w.rows[at]);
        let shared = names.iter().any(|n| fold(n) == fold(&table));
        let refused = if shared { None } else { refusal(doc, &w.input(&w.rows[at])) };
        let there = refused.as_ref().is_some_and(|(code, _)| code == kentos_commands::codes::TARGET_EXISTS);
        if shared || there {
            let free = free_name(doc, w, at, &table, &names);
            w.rows[at].said = Some(if shared {
                format!("“{table}” adını listedeki başka bir katman da alıyor; “{free}” önerildi.")
            } else {
                format!("“{table}” tablosu kaynakta var; “{free}” önerildi.")
            });
            w.rows[at].table = free.clone();
            names.push(free);
            ok = false;
        } else {
            w.rows[at].said = refused.map(|(_, words)| words);
            ok &= w.rows[at].said.is_none();
            names.push(table);
        }
        // A layer's texts beside other objects take a table of their own (docs/adr/9114 §2): its name is taken too.
        let r = &w.rows[at];
        if r.texts > 0 && r.texts < r.objects {
            names.push(format!("{}_yazilar", table_of(r)));
        }
    }
    ok
}

/// Why `input` would not move: its code and words; none when it would.
fn refusal(doc: &mut kentos_domain::Document, input: &LayersMigrate) -> Option<(String, String)> {
    match kentos_native_application::layers_migrate::plan(&ExecutionContext::new(doc), input) {
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
            Some((error.code, error.message))
        }
        CommandResult::Completed { .. } | CommandResult::Queued { .. } | CommandResult::Cancelled => None,
    }
}

/// `table` with the first number after it that neither the source nor the list's other rows have.
fn free_name(doc: &mut kentos_domain::Document, w: &MigrateWindow, at: usize, table: &str, names: &[String]) -> String {
    let fold = kentos_commands::fold;
    for n in 2..100 {
        let name = format!("{table}_{n}");
        if names.iter().any(|x| fold(x) == fold(&name)) {
            continue;
        }
        let mut row = w.rows[at].clone();
        row.table = name.clone();
        let there = refusal(doc, &w.input(&row)).is_some_and(|(code, _)| code == kentos_commands::codes::TARGET_EXISTS);
        if !there {
            return name;
        }
    }
    format!("{table}_{}", names.len() + 100)
}
