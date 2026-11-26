//! Yeni katman in a CBS project (docs/adr/9107 §7): Katmanlar's + ▸ Nokta, Çizgi, Alan katmanı… make a new table of a
//! data source, not a layer of the project file. The window: where (a GeoPackage file, made when it is not there, or
//! a PostgreSQL connection and schema), the layer's and the table's names, the geometry (its kind, multi-part, Z), the
//! system, the columns (name, kind, a text's length). Oluştur runs `cad.layers.create`: the table is made in its
//! source and added as a data layer beside the active layer, chosen, made active and scrolled into view.
//! The likely place is offered: the GeoPackage the project is kept in, the project file's own beside it, or the first
//! data layer's.

use std::path::PathBuf;

use iced::widget::{Column, column, container, row};
use iced::{Element, Length, Task};
use kentos_commands::{CommandResult, CreateGeometry, CreateTarget, LayerField, LayerFieldKind, LayersCreate};
use kentos_native_application::ExecutionContext;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Dialog, Form, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words;

const GEOMETRIES: [(CreateGeometry, &str); 4] = [
    (CreateGeometry::Point, "Nokta"),
    (CreateGeometry::Line, "Çizgi"),
    (CreateGeometry::Area, "Alan"),
    (CreateGeometry::None, "Geometrisiz (kayıt tablosu)"),
];
const KINDS: [LayerFieldKind; 6] = [
    LayerFieldKind::Text,
    LayerFieldKind::Integer,
    LayerFieldKind::Decimal,
    LayerFieldKind::Date,
    LayerFieldKind::DateTime,
    LayerFieldKind::Boolean,
];

/// A column of the table to make.
#[derive(Debug, Clone)]
pub(crate) struct NewField {
    name: String,
    kind: LayerFieldKind,
    length: String,
}

/// The window's state.
#[derive(Debug, Clone)]
pub(crate) struct NewLayerWindow {
    target: CreateTarget,
    path: String,
    connection: Option<usize>,
    address: String,
    schema: String,
    name: String,
    table: String,
    /// The table's name typed by hand: it no longer follows the layer's name.
    table_typed: bool,
    geometry: CreateGeometry,
    multi: bool,
    z: bool,
    /// The table's system; none: the project's (system_select.rs).
    srid: Option<u32>,
    fields: Vec<NewField>,
    said: Option<String>,
    /// Opened by Noktalar's Nokta katmanı oluştur (docs/adr/9320): the table made becomes the new row's layer, its edit
    /// session open.
    for_point_row: bool,
}

/// What the window says.
#[derive(Debug, Clone)]
pub enum Event {
    Target(usize),
    Path(String),
    Browse,
    Browsed(Option<PathBuf>),
    Connection(usize),
    Address(String),
    Schema(String),
    Name(String),
    Table(String),
    Geometry(usize),
    Multi(bool),
    Z(bool),
    Srid(Option<u32>),
    FieldName(usize, String),
    FieldKind(usize, usize),
    FieldLength(usize, String),
    AddField,
    RemoveField(usize),
    Run,
    Close,
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::LayerCreate(e))
}

fn geometry_of(g: kentos_commands::LayerGeometry) -> CreateGeometry {
    match g {
        kentos_commands::LayerGeometry::Point => CreateGeometry::Point,
        kentos_commands::LayerGeometry::Line => CreateGeometry::Line,
        kentos_commands::LayerGeometry::Area | kentos_commands::LayerGeometry::Annotation => CreateGeometry::Area,
    }
}

impl NewLayerWindow {
    /// The command's input for the window as it is; `parent` and `index` place it beside the active layer.
    fn input(&self, parent: Option<String>, index: Option<u32>) -> LayersCreate {
        let fields: Vec<LayerField> = self
            .fields
            .iter()
            .filter(|f| !f.name.trim().is_empty())
            .filter_map(|f| {
                // Built through its contract: only the name, the kind and a text's length are the window's.
                let mut v = serde_json::json!({ "name": f.name.trim(), "kind": f.kind.name() });
                if f.kind == LayerFieldKind::Text
                    && let Ok(n) = f.length.trim().parse::<u32>()
                {
                    v["length"] = serde_json::json!(n);
                }
                serde_json::from_value::<LayerField>(v).ok()
            })
            .collect();
        LayersCreate {
            name: self.name.trim().to_owned(),
            target: self.target,
            path: (self.target == CreateTarget::Gpkg).then(|| self.path.trim().to_owned()),
            address: (self.target == CreateTarget::Postgis).then(|| self.address.trim().to_owned()),
            schema: (self.target == CreateTarget::Postgis).then(|| {
                let s = self.schema.trim();
                if s.is_empty() { "public".to_owned() } else { s.to_owned() }
            }),
            table: Some(self.table.trim().to_owned()).filter(|t| !t.is_empty()),
            geometry: self.geometry,
            multi: self.multi.then_some(true),
            z: self.z.then_some(true),
            srid: self.srid,
            fields: (!fields.is_empty()).then_some(fields),
            key: None,
            geometry_column: None,
            parent,
            index,
        }
    }
}

impl App {
    /// The likely GeoPackage of a new table: the one the project is kept in, the project file's own beside it, or the
    /// first data layer's.
    pub(crate) fn likely_geopackage(&self) -> String {
        let Some(doc) = &self.document else {
            return String::new();
        };
        if let Some(f) = doc.stored_source().and_then(|s| s.file()) {
            return f.display().to_string();
        }
        if let Some(p) = &doc.path {
            return p.with_extension("gpkg").display().to_string();
        }
        doc.model
            .layers()
            .leaves()
            .iter()
            .find_map(|n| n.data.as_ref().filter(|d| d.driver == "gpkg").map(|d| d.address.trim_start_matches("gpkg:").to_owned()))
            .unwrap_or_default()
    }

    /// + ▸ Nokta, Çizgi, Alan katmanı… in a CBS project: the window with that kind.
    pub(crate) fn open_layer_create(&mut self, geometry: kentos_commands::LayerGeometry) {
        if let Some(why) = self.tree_locked() {
            self.warn(why);
            return;
        }
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name(crate::layer_types::new_name(geometry));
        self.gis.layer_create = Some(NewLayerWindow {
            target: CreateTarget::Gpkg,
            path: self.likely_geopackage(),
            connection: None,
            address: String::new(),
            schema: "public".into(),
            table: kentos_native_application::layer_export::table_name_of(&name),
            name,
            table_typed: false,
            geometry: geometry_of(geometry),
            multi: false,
            z: false,
            srid: None,
            fields: vec![NewField { name: String::new(), kind: LayerFieldKind::Text, length: String::new() }],
            said: None,
            for_point_row: false,
        });
        self.dialog = Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::LayerCreate));
    }

    /// Noktalar's Nokta katmanı oluştur (docs/adr/9320): the window for a table of points whose columns keep what the
    /// row has, `ad` and `kod`, and whose geometry keeps the elevation (Z).
    pub(crate) fn open_point_table_for_rows(&mut self) {
        self.open_layer_create(kentos_commands::LayerGeometry::Point);
        if let Some(w) = self.gis.layer_create.as_mut() {
            w.z = true;
            w.fields = ["ad", "kod"]
                .map(|name| NewField { name: name.to_owned(), kind: LayerFieldKind::Text, length: String::new() })
                .to_vec();
            w.for_point_row = true;
        }
    }

    pub(crate) fn layer_create_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.gis.layer_create.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Target(i) => w.target = if i == 1 { CreateTarget::Postgis } else { CreateTarget::Gpkg },
            Event::Path(p) => w.path = p,
            Event::Browse => {
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Tablonun yapılacağı GeoPackage")
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
            Event::Name(n) => {
                if !w.table_typed {
                    w.table = kentos_native_application::layer_export::table_name_of(&n);
                }
                w.name = n;
            }
            Event::Table(t) => {
                w.table_typed = !t.trim().is_empty();
                w.table = t;
            }
            Event::Geometry(i) => w.geometry = GEOMETRIES[i.min(GEOMETRIES.len() - 1)].0,
            Event::Multi(on) => w.multi = on,
            Event::Z(on) => w.z = on,
            Event::Srid(s) => w.srid = s,
            Event::FieldName(i, n) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.name = n;
                }
            }
            Event::FieldKind(i, k) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.kind = KINDS[k.min(KINDS.len() - 1)];
                }
            }
            Event::FieldLength(i, l) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.length = l.chars().filter(char::is_ascii_digit).collect();
                }
            }
            Event::AddField => w.fields.push(NewField { name: String::new(), kind: LayerFieldKind::Text, length: String::new() }),
            Event::RemoveField(i) => {
                if i < w.fields.len() {
                    w.fields.remove(i);
                }
            }
            Event::Run => return self.run_layer_create(),
            Event::Close => {
                self.gis.layer_create = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Oluştur: the table made and added beside the active layer; why not, at the window's foot.
    fn run_layer_create(&mut self) -> Task<Message> {
        let Some(w) = self.gis.layer_create.clone() else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let active = doc.model.layers().active().to_owned();
        let (parent, index) = doc.model.layers().place_of(&active).map_or((None, None), |(p, i)| (p, u32::try_from(i).ok()));
        let input = w.input(parent, index);
        let name = input.name.clone();
        let result = kentos_native_application::layers_create::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, .. } => {
                self.gis.layer_create = None;
                self.dialog = None;
                let id = output.layer.clone();
                if let Some(doc) = self.document.as_mut() {
                    doc.model.set_active_layer(&id);
                }
                let hidden = self.layer_row_hidden(&id);
                self.selected_layer = Some(id.clone());
                self.layers_follow = false;
                self.layer_reveal = Some(id);
                let place = std::path::Path::new(&output.address)
                    .file_name()
                    .map_or_else(|| output.address.clone(), |n| n.to_string_lossy().into_owned());
                let mut words = format!(
                    "“{name}” katmanı yapıldı: {} içinde “{}” tablosu ({}). Nesneleri tablonun satırlarıdır; çizmek için Düzenlemeyi başlat.",
                    place,
                    output.table,
                    output.geometry.as_deref().map_or("geometrisiz", |g| g)
                );
                if hidden {
                    words.push_str(" Katman araması onu gizliyor; görmek için aramayı temizleyin.");
                }
                self.say(kentos_interaction::Level::Success, words);
                // Made for Noktalar's new row: its layer, opened for editing (docs/adr/9320).
                if w.for_point_row {
                    return self.point_table_made(&output.layer);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                if let Some(w) = self.gis.layer_create.as_mut() {
                    w.said = Some(error.message);
                }
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        Task::none()
    }

    /// The window.
    pub(crate) fn layer_create_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.layer_create else {
            return column![].into();
        };
        let text = |placeholder: &str, value: &str, on: fn(String) -> Event| -> Element<'_, Message> {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| message(on(t))).into()
        };
        let select = |choices: Vec<&'static str>, at: Option<usize>, on: fn(usize) -> Event| -> Element<'_, Message> {
            container(Select::new(choices.into_iter().map(Choice::new), at, move |i| message(on(i))).searchable(false))
                .width(320)
                .into()
        };
        let mut form = Form::new().label_width(150.0).section("Kaynak").field(
            "Nerede",
            select(
                vec!["GeoPackage dosyası", "PostgreSQL veritabanı"],
                Some(usize::from(w.target == CreateTarget::Postgis)),
                Event::Target,
            ),
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
        form = form
            .section("Katman")
            .field("Katmanın adı", text("Katmanlar'da görünen ad", &w.name, Event::Name))
            .field("Tablonun adı", text("katmanın adından", &w.table, Event::Table))
            .field(
                "Geometri",
                select(
                    GEOMETRIES.iter().map(|(_, l)| *l).collect(),
                    GEOMETRIES.iter().position(|(g, _)| *g == w.geometry),
                    Event::Geometry,
                ),
            );
        if w.geometry != CreateGeometry::None {
            form = form
                .row(Switch::new(w.multi, |b| message(Event::Multi(b))).label("Çok parçalı"))
                .row(Switch::new(w.z, |b| message(Event::Z(b))).label("Z (kot) ile"))
                .field("Koordinat sistemi", container(self.layer_create_systems(w.srid)).width(420))
                .help("Tablonun geometrisinin sistemi; varsayılan projenin sistemidir.");
        }
        // Alanlar: name, kind, a text's length; + Alan ekle.
        let heads = kentos_ui::widget::table::band(
            row![
                container(label::caption("Alanın adı")).width(220),
                container(label::caption("Türü")).width(170),
                container(label::caption("Uzunluk")).width(90),
            ]
            .spacing(12)
            .padding([0, 8]),
        );
        let mut rows = Column::new().spacing(4).padding([6, 8]);
        for (i, f) in w.fields.iter().enumerate() {
            let kind = container(
                Select::new(
                    KINDS.iter().map(|k| Choice::new(k.label())),
                    KINDS.iter().position(|k| *k == f.kind),
                    move |k| message(Event::FieldKind(i, k)),
                )
                .searchable(false),
            )
            .width(170);
            let length: Element<'_, Message> = if f.kind == LayerFieldKind::Text {
                container(kentos_ui::widget::text_field("sınırsız", &f.length).on_input(move |t| message(Event::FieldLength(i, t))))
                    .width(90)
                    .into()
            } else {
                container(label::muted("—")).width(90).into()
            };
            rows = rows.push(
                row![
                    container(kentos_ui::widget::text_field("ör. durum", &f.name).on_input(move |t| message(Event::FieldName(i, t))))
                        .width(220),
                    kind,
                    length,
                    words::ghost("Kaldır", Some(message(Event::RemoveField(i)))),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            );
        }
        let list = container(column![heads, rows])
            .width(Length::Fill)
            .padding(1)
            .style(style::container::bordered);
        form = form
            .section("Alanlar")
            .wide(column![list, words::ghost("Alan ekle", Some(message(Event::AddField)))].spacing(6));
        let mut dialog = Dialog::new("Yeni katman").scroll(form);
        if let Some(said) = &w.said {
            dialog = dialog.status(words::text_line(words::Kind::Error, said.clone()));
        } else {
            dialog = dialog.status(label::caption("Katmanın nesneleri proje dosyasında değil, bu tablonun satırlarında durur."));
        }
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Event::Close))))
                .action(words::primary("Oluştur", Some(message(Event::Run))))
                .width(680.0)
                .max_height(800.0)
                .on_close(Some(message(Event::Close))),
        )
    }
}

impl App {
    /// Yeni katman's system list: the project's first (the default), then the registry's systems.
    fn layer_create_systems(&self, chosen: Option<u32>) -> Element<'_, Message> {
        let project = self.document.as_ref().map_or(0, |d| d.model.settings().srid);
        let label = match kentos_project::crs::system(project) {
            Some(s) if project > 0 => format!("Projenin sistemi · {} (EPSG:{project})", s.name),
            _ => "Projenin sistemi".to_owned(),
        };
        crate::system_select::select(
            crate::system_select::rows(label, (project > 0).then_some(project), project),
            chosen,
            |s| message(Event::Srid(s)),
        )
    }
}
