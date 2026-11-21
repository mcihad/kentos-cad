//! Dosya › Veri kaynağı (docs/adr/9107 §5): a project kept in a GeoPackage file or a PostgreSQL schema, with its data.
//! One window saves and opens: the source (a GeoPackage's file, or a database's address and schema), the projects kept
//! there (name, who wrote it last and when, its size), the project's name. Kaydet writes the drawing there through
//! the save pipeline (`saving::Target::Source`: read back before it counts) and the drawing becomes that project, which
//! Kaydet writes again; Aç reads one off the interface's thread and opens it in a tab of its own. Saving to a file
//! stays as it was: keeping a project in a source is a choice, never required.

use std::path::PathBuf;

use iced::widget::{Column, column, container, row};
use iced::{Element, Length, Task};
use kentos_data::StoredProject;
use kentos_ui::widget::app_menu::Item;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Form, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::document::{Document, Source, StoredSource};
use crate::exchange::words;

/// Where the window looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A GeoPackage file.
    File,
    /// A PostgreSQL database's schema.
    Database,
}

/// The window's state.
#[derive(Debug, Clone)]
pub(crate) struct StoreWindow {
    /// Kaydet (true) or Aç.
    save: bool,
    kind: Kind,
    path: String,
    /// The saved connection picked (it fills the address).
    connection: Option<usize>,
    address: String,
    schema: String,
    name: String,
    /// The projects there, as last read; why not.
    listed: Option<Result<Vec<StoredProject>, String>>,
    /// Which read the list is of: a late answer of an earlier one is dropped.
    asked: u64,
    selected: Option<usize>,
    busy: bool,
    said: Option<String>,
}

/// What the window says.
#[derive(Debug, Clone)]
pub enum Event {
    Kind(usize),
    Path(String),
    Browse,
    Browsed(Option<PathBuf>),
    Connection(usize),
    Address(String),
    Schema(String),
    Name(String),
    /// Listele, or Enter in the place's fields.
    List,
    Listed(u64, Result<Vec<StoredProject>, String>),
    Select(usize),
    Run,
    /// A project read and opened, or why not.
    Read(Box<Result<Document, String>>),
    Close,
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::ProjectStore(e))
}

impl StoreWindow {
    /// The source's address as the drivers take it.
    fn source(&self) -> String {
        match self.kind {
            Kind::File => self.path.trim().to_owned(),
            Kind::Database => self.address.trim().to_owned(),
        }
    }

    fn schema(&self) -> Option<String> {
        (self.kind == Kind::Database).then(|| {
            let s = self.schema.trim();
            if s.is_empty() { "public".to_owned() } else { s.to_owned() }
        })
    }

    fn there(&self) -> &[StoredProject] {
        match &self.listed {
            Some(Ok(list)) => list,
            _ => &[],
        }
    }

    /// The file's path as the window holds it.
    #[cfg(test)]
    pub(crate) fn path_for_test(&self) -> String {
        self.path.clone()
    }

    /// The projects' names listed, none before a read.
    #[cfg(test)]
    pub(crate) fn listed_for_test(&self) -> Option<Vec<String>> {
        self.listed.as_ref().and_then(|l| l.as_ref().ok()).map(|l| l.iter().map(|p| p.name.clone()).collect())
    }

    /// The name typed is a project kept there already: Kaydet writes over it.
    fn over(&self) -> bool {
        let name = self.name.trim();
        self.there().iter().any(|p| p.name == name)
    }
}

/// A size in Turkish units, rounded (at least 1 KB).
fn size_words(bytes: u64) -> String {
    if bytes >= 1 << 20 {
        format!("{:.1} MB", bytes as f64 / (1 << 20) as f64).replace('.', ",")
    } else {
        format!("{} KB", (bytes as f64 / 1024.0).round().max(1.0) as u64)
    }
}

/// When it was written (UTC, as the source's clock said it) in the device's time, as the cloud lists say it:
/// “21.11.2026 15:41”; the text as it is when it cannot be read.
fn when_words(at: &str) -> String {
    let local = crate::cloud::local_time::when(at, crate::cloud::local_time::Zone::system());
    if local.is_empty() { at.to_owned() } else { local }
}

impl App {
    /// Opens the window (Dosya › Veri kaynağı's rows): Kaydet with the drawing's name (or the project it is), Aç
    /// empty.
    pub(crate) fn open_project_store(&mut self, save: bool) -> Task<Message> {
        let stored = self.document.as_ref().and_then(|d| d.stored_source().cloned());
        let name = match (&stored, &self.document) {
            (Some(s), _) => s.name.clone(),
            (None, Some(d)) => d.name().trim_end_matches(".kcad").to_owned(),
            _ => String::new(),
        };
        let mut w = StoreWindow {
            save,
            kind: Kind::File,
            path: String::new(),
            connection: None,
            address: String::new(),
            schema: "public".into(),
            name,
            listed: None,
            asked: 0,
            selected: None,
            busy: false,
            said: None,
        };
        // Where the drawing is kept, or the GeoPackage of its first data layer: the likely place.
        if let Some(s) = stored {
            match s.file() {
                Some(f) => w.path = f.display().to_string(),
                None => {
                    w.kind = Kind::Database;
                    w.address = s.address.clone();
                    w.schema = s.schema.clone().unwrap_or_else(|| "public".into());
                }
            }
        } else if let Some(file) = self.first_data_file() {
            w.path = file;
        }
        let list = !w.source().is_empty();
        self.gis.project_store = Some(w);
        self.dialog = Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::ProjectStore));
        if list { self.list_projects() } else { Task::none() }
    }

    /// The GeoPackage file of the drawing's first data layer read from one.
    fn first_data_file(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        doc.model.layers().leaves().iter().find_map(|n| {
            n.data.as_ref().filter(|d| d.driver == "gpkg").map(|d| d.address.trim_start_matches("gpkg:").to_owned())
        })
    }

    /// Reads the projects kept at the place off the interface's thread.
    fn list_projects(&mut self) -> Task<Message> {
        let Some(w) = self.gis.project_store.as_mut() else {
            return Task::none();
        };
        let (address, schema) = (w.source(), w.schema());
        if address.is_empty() {
            w.listed = None;
            return Task::none();
        }
        w.asked += 1;
        let asked = w.asked;
        crate::db_connections::off_thread(
            move || kentos_native_application::project_store::list(&address, schema.as_deref()),
            move |answer| message(Event::Listed(asked, answer)),
        )
    }

    pub(crate) fn project_store_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Read(read) = event {
            return self.project_store_read(*read);
        }
        let Some(w) = self.gis.project_store.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Kind(i) => {
                w.kind = if i == 1 { Kind::Database } else { Kind::File };
                w.listed = None;
                w.selected = None;
                return self.list_projects();
            }
            Event::Path(p) => {
                w.path = p;
                w.listed = None;
                w.selected = None;
            }
            Event::Browse => {
                let save = w.save;
                return Task::perform(
                    async move {
                        let dialog = rfd::AsyncFileDialog::new()
                            .set_title(if save { "Projenin saklanacağı GeoPackage" } else { "Projenin saklandığı GeoPackage" })
                            .add_filter("GeoPackage (.gpkg)", &["gpkg"]);
                        let file = if save { dialog.save_file().await } else { dialog.pick_file().await };
                        file.map(|f| f.path().to_path_buf())
                    },
                    |p| message(Event::Browsed(p)),
                );
            }
            Event::Browsed(Some(p)) => {
                let p = if kentos_native_application::project_store::is_geopackage(&p) { p } else { p.with_extension("gpkg") };
                w.path = p.display().to_string();
                return self.list_projects();
            }
            Event::Browsed(None) => {}
            Event::Connection(i) => {
                w.connection = Some(i);
                if let Some(c) = kentos_postgis::connections::store().list().get(i) {
                    w.address = c.address();
                }
                return self.list_projects();
            }
            Event::Address(a) => {
                w.address = a;
                w.connection = None;
                w.listed = None;
                w.selected = None;
            }
            Event::Schema(s) => {
                w.schema = s;
                w.listed = None;
                w.selected = None;
            }
            Event::Name(n) => {
                w.name = n;
                w.selected = None;
            }
            Event::List => return self.list_projects(),
            Event::Listed(asked, answer) => {
                if asked == w.asked {
                    w.listed = Some(answer);
                    w.selected = None;
                }
            }
            Event::Select(i) => {
                if let Some(p) = w.there().get(i) {
                    w.name = p.name.clone();
                    w.selected = Some(i);
                }
            }
            Event::Run => return self.run_project_store(),
            Event::Close => {
                self.gis.project_store = None;
                self.dialog = None;
            }
            Event::Read(_) => {}
        }
        Task::none()
    }

    /// Kaydet: the drawing written there (the save pipeline); Aç: the project read off the thread.
    fn run_project_store(&mut self) -> Task<Message> {
        let Some(w) = self.gis.project_store.as_mut() else {
            return Task::none();
        };
        let address = w.source();
        let name = w.name.trim().to_owned();
        if address.is_empty() {
            w.said = Some(match w.kind {
                Kind::File => "GeoPackage dosyasının yolunu yazın ya da Gözat… ile seçin.".into(),
                Kind::Database => "Bir bağlantı seçin ya da veritabanının adresini yazın.".into(),
            });
            return Task::none();
        }
        if let Err(e) = kentos_data::projects::checked_name(&name) {
            w.said = Some(e.to_string());
            return Task::none();
        }
        let at = StoredSource { address: address.clone(), schema: w.schema(), name: name.clone() };
        if w.save {
            self.gis.project_store = None;
            self.dialog = None;
            return self.start_save(crate::saving::Target::Source(at));
        }
        w.busy = true;
        w.said = None;
        crate::db_connections::off_thread(
            move || {
                let bytes = kentos_native_application::project_store::read(&at.address, at.schema.as_deref(), &at.name)?;
                let snapshot = kentos_kcad::decode(&bytes).map_err(|e| format!("“{}” projesi okunamadı: {}", at.name, e.message))?;
                let mut doc = Document::from_v2(snapshot, None)?;
                doc.source = Source::Stored(Box::new(at));
                Ok(doc)
            },
            |read| message(Event::Read(Box::new(read))),
        )
    }

    /// A project read: opened in a tab of its own, the window closed; why not, in the window.
    fn project_store_read(&mut self, read: Result<Document, String>) -> Task<Message> {
        match read {
            Ok(doc) => {
                self.gis.project_store = None;
                self.dialog = None;
                self.update(Message::Opened(Some(Ok(Box::new(doc)))))
            }
            Err(why) => {
                if let Some(w) = self.gis.project_store.as_mut() {
                    w.busy = false;
                    w.said = Some(why);
                }
                Task::none()
            }
        }
    }

    /// The window.
    pub(crate) fn project_store_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.project_store else {
            return column![].into();
        };
        let kind = container(
            Select::new(
                ["GeoPackage dosyası", "PostgreSQL veritabanı"].into_iter().map(Choice::new),
                Some(usize::from(w.kind == Kind::Database)),
                |i| message(Event::Kind(i)),
            )
            .searchable(false),
        )
        .width(320);
        let mut form = Form::new().label_width(150.0).field("Kaynak", kind);
        match w.kind {
            Kind::File => {
                form = form.field(
                    "Dosya",
                    row![
                        container(
                            kentos_ui::widget::text_field("GeoPackage dosyasının yolu (.gpkg)", &w.path)
                                .on_input(|t| message(Event::Path(t)))
                                .on_submit(message(Event::List))
                        )
                        .width(Length::Fill),
                        words::secondary("Gözat…", Some(message(Event::Browse))),
                        words::ghost("Listele", Some(message(Event::List))),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
            }
            Kind::Database => {
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
                    .field(
                        "Adres",
                        kentos_ui::widget::text_field("postgresql://kullanıcı@sunucu:5432/veritabanı", &w.address)
                            .on_input(|t| message(Event::Address(t)))
                            .on_submit(message(Event::List)),
                    )
                    .help("Parola adreste yazılmaz; kayıtlı bağlantının ya da sistemin parolası kullanılır.")
                    .field(
                        "Şema",
                        row![
                            container(
                                kentos_ui::widget::text_field("public", &w.schema)
                                    .on_input(|t| message(Event::Schema(t)))
                                    .on_submit(message(Event::List))
                            )
                            .width(Length::Fill),
                            words::ghost("Listele", Some(message(Event::List))),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    );
            }
        }
        // The projects kept there.
        let projects: Element<'_, Message> = match &w.listed {
            None => label::muted(match w.kind {
                Kind::File => "Dosyayı seçince içindeki projeler burada listelenir.",
                Kind::Database => "Bağlantıyı seçince şemadaki projeler burada listelenir.",
            })
            .into(),
            Some(Err(why)) => words::text_line(words::Kind::Error, why.clone()),
            Some(Ok(list)) if list.is_empty() => label::muted(if w.save {
                "Burada saklanan proje yok; Kaydet ilkini yazar."
            } else {
                "Burada saklanan proje yok."
            })
            .into(),
            Some(Ok(list)) => {
                let rows = list.iter().enumerate().fold(Column::new().spacing(1), |c, (i, p)| {
                    let mut line = vec![size_words(p.size)];
                    if let Some(at) = &p.modified {
                        line.insert(0, when_words(at));
                    }
                    if let Some(user) = &p.user {
                        line.insert(0, user.clone());
                    }
                    c.push(
                        Item::new(p.name.as_str())
                            .subtitle(line.join(", "))
                            .on_press(message(Event::Select(i)))
                            .current(w.selected == Some(i)),
                    )
                });
                container(iced::widget::scrollable(rows).direction(style::field::body_scrollbar()).height(Length::Shrink))
                    .max_height(kentos_ui::theme::typography::scaled(220.0))
                    .width(Length::Fill)
                    .padding(1)
                    .style(style::container::bordered)
                    .into()
            }
        };
        form = form.section("Burada saklanan projeler").wide(projects);
        form = form.section("Proje").field(
            "Proje adı",
            kentos_ui::widget::text_field("Projenin adı", &w.name)
                .on_input(|t| message(Event::Name(t)))
                .on_submit(message(Event::Run)),
        );
        if w.save && w.over() {
            form = form.help("Bu adla saklanan bir proje var; Kaydet onun üstüne yazar.");
        }
        let title = if w.save { "Veri kaynağına kaydet" } else { "Veri kaynağından aç" };
        let mut dialog = Dialog::new(title).scroll(form);
        if let Some(said) = &w.said {
            dialog = dialog.status(words::text_line(words::Kind::Error, said.clone()));
        } else if w.busy {
            dialog = dialog.status(label::caption("Proje okunuyor…"));
        }
        let can = !w.busy && !w.source().is_empty() && !w.name.trim().is_empty();
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Event::Close))))
                .action(words::primary(if w.save { "Kaydet" } else { "Aç" }, can.then(|| message(Event::Run))))
                .width(640.0)
                .max_height(720.0)
                .on_close(Some(message(Event::Close))),
        )
    }
}
