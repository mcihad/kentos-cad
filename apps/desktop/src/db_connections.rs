//! Veritabanı bağlantıları (docs/adr/0623; as QGIS's “Create a New PostGIS Connection” and its Browser's PostgreSQL
//! connections, KentOS's own): the connection window (Ad, Sunucu, Bağlantı noktası, Veritabanı, Kullanıcı adı, Parola
//! and Parolayı sakla, SSL, Seçenekler; Adresten doldur; Bağlantıyı dene), the password question when a server asks
//! for a password KentOS does not know, Sil's question, and the connections given to a team and taken from one
//! (Dışa aktar, İçe aktar: KentOS's own file).
//! The connections themselves, their file and how they are tried are `kentos_postgis`'s (`connections`, `probe`):
//! nothing here but the windows. Nobody types a connection string.

use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::mpsc;
use iced::widget::{Column, button, column, row};
use iced::{Center, Element, Task};
use kentos_interaction::Level;
use kentos_postgis::connections::{self, DbConnection, Field, SSL_MODES};
use kentos_postgis::probe::{self, Catalog, Diagnosis, Problem, ServerFacts};
use kentos_ui::widget::{Choice, Confirm, Dialog, Form, InputField, Select, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};

/// What follows a connection saved in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// Kaynaklar opens it.
    Browse,
    /// PostGIS'e bağlan chooses it and lists its tables.
    Postgis,
}

/// Who asked for a password, to be given the tables the password opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retry {
    /// Kaynaklar's row of the connection.
    Browse(String),
    /// PostGIS'e bağlan's list.
    Postgis(String),
}

/// The connection window while it is open.
#[derive(Debug)]
pub struct Editor {
    /// The connection being edited, by its name; none: a new one.
    pub previous: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    /// What is typed; empty with `kept`: the saved password stays.
    pub password: String,
    /// The connection had a saved password.
    pub kept: bool,
    /// That saved password, kept as it was when nothing is typed.
    saved_password: Option<String>,
    pub keep_password: bool,
    pub ssl: connections::SslMode,
    pub public_only: bool,
    pub geometryless: bool,
    /// Adresten doldur's field, while it is open.
    pub paste: Option<String>,
    pub paste_problem: Option<String>,
    pub testing: bool,
    pub tested: Option<Result<ServerFacts, Diagnosis>>,
    /// Kaydet was pressed: the fields' problems are said.
    pub tried: bool,
    pub save_problem: Option<String>,
    pub then: Then,
    /// Bumped by every change of a field: a try's answer for older fields is dropped.
    pub generation: u64,
}

impl Editor {
    fn new(then: Then) -> Self {
        Editor {
            previous: None,
            name: String::new(),
            host: String::new(),
            port: connections::DEFAULT_PORT.to_string(),
            database: String::new(),
            user: String::new(),
            password: String::new(),
            kept: false,
            saved_password: None,
            keep_password: false,
            ssl: connections::SslMode::Prefer,
            public_only: false,
            geometryless: false,
            paste: None,
            paste_problem: None,
            testing: false,
            tested: None,
            tried: false,
            save_problem: None,
            then,
            generation: 0,
        }
    }

    fn of(c: &DbConnection, then: Then) -> Self {
        Editor {
            previous: Some(c.name.clone()),
            name: c.name.clone(),
            host: c.host.clone(),
            port: c.port.to_string(),
            database: c.database.clone(),
            user: c.user.clone(),
            kept: c.password.is_some(),
            saved_password: c.password.clone(),
            keep_password: c.password.is_some(),
            ssl: c.ssl,
            public_only: c.public_only,
            geometryless: c.geometryless,
            ..Editor::new(then)
        }
    }

    /// The connection the fields say; the port's problem apart (it is no number).
    fn connection(&self) -> (DbConnection, Option<String>) {
        let port = self.port.trim().parse::<u16>().ok().filter(|&p| p > 0);
        let saved = match (self.keep_password, self.password.is_empty()) {
            (true, false) => Some(self.password.clone()),
            (true, true) => self.saved_password.clone(),
            (false, _) => None,
        };
        (
            DbConnection {
                name: self.name.trim().to_owned(),
                host: self.host.trim().to_owned(),
                port: port.unwrap_or(connections::DEFAULT_PORT),
                database: self.database.trim().to_owned(),
                user: self.user.trim().to_owned(),
                ssl: self.ssl,
                password: saved,
                public_only: self.public_only,
                geometryless: self.geometryless,
            },
            port.is_none().then(|| {
                "Bağlantı noktası 1 ile 65535 arasında bir sayıdır; PostgreSQL'de genellikle 5432'dir.".to_owned()
            }),
        )
    }

    /// The problem of `field` the window says (only after Kaydet).
    fn problem(&self, field: Field) -> Option<String> {
        if !self.tried {
            return None;
        }
        let (c, port) = self.connection();
        if field == Field::Port && port.is_some() {
            return port;
        }
        c.problems()
            .into_iter()
            .find(|(f, _)| *f == field)
            .map(|(_, why)| why)
    }

    /// The password typed, for a try.
    fn typed(&self) -> Option<String> {
        (!self.password.is_empty()).then(|| self.password.clone())
    }

    fn changed(&mut self) {
        self.generation += 1;
        self.tested = None;
        self.testing = false;
        self.save_problem = None;
    }
}

/// The password question.
#[derive(Debug)]
pub struct PasswordAsk {
    pub connection: String,
    pub password: String,
    pub keep: bool,
    /// Why it is asked (the server's refusal, said).
    pub reason: String,
    pub retry: Retry,
    pub trying: bool,
    pub generation: u64,
}

#[derive(Debug, Clone)]
pub enum Event {
    New(Then),
    Edit(String, Then),
    Duplicate(String),
    AskRemove(String),
    Remove,
    KeepIt,
    Name(String),
    Host(String),
    Port(String),
    Database(String),
    User(String),
    Password(String),
    KeepPassword,
    Ssl(usize),
    PublicOnly,
    Geometryless,
    PasteToggle,
    Paste(String),
    PasteApply,
    Test,
    Tested(u64, Result<ServerFacts, Diagnosis>),
    Save,
    Close,
    /// The password question: typed, kept, tried and its answer, closed.
    Asked(String),
    AskKeep,
    AskSubmit,
    AskTried(u64, Result<Arc<Catalog>, Diagnosis>),
    AskClose,
    /// A connections file: chosen and read, or written (with the passwords when asked).
    Import,
    ImportRead(Option<Result<(PathBuf, String), String>>),
    Export(bool),
    ExportPicked(Option<PathBuf>, bool),
}

pub(crate) fn msg(e: Event) -> Message {
    Message::Db(e)
}

/// Runs `work` on a thread of its own and brings its answer back as a message.
pub(crate) fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
        let answer = done(work());
        let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
    })
}

impl App {
    pub(crate) fn db_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::New(then) => self.db_editor = Some(Editor::new(then)),
            Event::Edit(name, then) => match self.db_store.get(&name) {
                Some(c) => self.db_editor = Some(Editor::of(&c, then)),
                None => self.output(format!("“{name}” bağlantısı artık yok.")),
            },
            Event::Duplicate(name) => {
                if let Some(c) = self.db_store.get(&name) {
                    let mut e = Editor::of(&c, Then::Browse);
                    e.previous = None;
                    e.name = self.db_store.unique_name(&c.name);
                    // The copy is a new connection: its saved password comes with it when kept.
                    e.password = c.password.clone().unwrap_or_default();
                    e.kept = false;
                    self.db_editor = Some(e);
                }
            }
            Event::AskRemove(name) => self.db_removing = Some(name),
            Event::KeepIt => self.db_removing = None,
            Event::Remove => {
                if let Some(name) = self.db_removing.take() {
                    let gone = self.db_store.get(&name);
                    match self.db_store.remove(&name) {
                        Ok(_) => {
                            if let Some(c) = gone {
                                connections::forget(&c);
                            }
                            self.sources.forget_connection(&name);
                            self.output(format!("“{name}” bağlantısı silindi; bu bağlantıyla bağlanmış katmanlar çizimde kalır."));
                        }
                        Err(why) => self.error(why),
                    }
                }
            }
            Event::Name(t) => self.with_editor(|e| e.name = t),
            Event::Host(t) => self.with_editor(|e| e.host = t),
            Event::Port(t) => self.with_editor(|e| {
                e.port = t.chars().filter(char::is_ascii_digit).take(5).collect();
            }),
            Event::Database(t) => self.with_editor(|e| e.database = t),
            Event::User(t) => self.with_editor(|e| e.user = t),
            Event::Password(t) => self.with_editor(|e| e.password = t),
            Event::KeepPassword => self.with_editor(|e| e.keep_password = !e.keep_password),
            Event::Ssl(i) => {
                self.with_editor(|e| e.ssl = SSL_MODES.get(i).copied().unwrap_or_default())
            }
            Event::PublicOnly => self.with_editor(|e| e.public_only = !e.public_only),
            Event::Geometryless => self.with_editor(|e| e.geometryless = !e.geometryless),
            Event::PasteToggle => {
                if let Some(e) = &mut self.db_editor {
                    e.paste = match e.paste.take() {
                        Some(_) => None,
                        None => Some(String::new()),
                    };
                    e.paste_problem = None;
                }
            }
            Event::Paste(t) => {
                if let Some(e) = &mut self.db_editor {
                    e.paste = Some(t);
                    e.paste_problem = None;
                }
            }
            Event::PasteApply => self.paste_into_editor(),
            Event::Test => {
                let Some(e) = &mut self.db_editor else {
                    return Task::none();
                };
                let (c, port) = e.connection();
                if let Some(why) = port.or_else(|| {
                    c.problems()
                        .into_iter()
                        .find(|(f, _)| *f != Field::Name)
                        .map(|(_, why)| why)
                }) {
                    e.tested = Some(Err(Diagnosis {
                        problem: Problem::Other,
                        message: why,
                    }));
                    return Task::none();
                }
                e.testing = true;
                e.tested = None;
                let generation = e.generation;
                let typed = e.typed();
                return off_thread(
                    move || probe::probe(&c, typed.as_deref()),
                    move |r| msg(Event::Tested(generation, r)),
                );
            }
            Event::Tested(generation, result) => {
                let Some(e) = &mut self.db_editor else {
                    return Task::none();
                };
                if e.generation != generation {
                    return Task::none();
                }
                e.testing = false;
                // A password that opened the database stays for this session, saved or not.
                if result.is_ok()
                    && let Some(typed) = e.typed()
                {
                    connections::remember(&e.connection().0, typed);
                }
                e.tested = Some(result);
            }
            Event::Save => return self.save_connection(),
            Event::Close => self.db_editor = None,
            Event::Asked(t) => {
                if let Some(a) = &mut self.db_password {
                    a.password = t;
                }
            }
            Event::AskKeep => {
                if let Some(a) = &mut self.db_password {
                    a.keep = !a.keep;
                }
            }
            Event::AskSubmit => {
                let Some(a) = &mut self.db_password else {
                    return Task::none();
                };
                let Some(c) = self.db_store.get(&a.connection) else {
                    self.db_password = None;
                    return Task::none();
                };
                if a.password.is_empty() {
                    return Task::none();
                }
                a.trying = true;
                a.generation += 1;
                let generation = a.generation;
                let typed = a.password.clone();
                return off_thread(
                    move || probe::browse(&c, Some(&typed)).map(Arc::new),
                    move |r| msg(Event::AskTried(generation, r)),
                );
            }
            Event::AskTried(generation, result) => return self.password_tried(generation, result),
            Event::AskClose => {
                if let Some(a) = self.db_password.take() {
                    // The list that waited for it says why it stays empty.
                    let why = Diagnosis {
                        problem: Problem::PasswordNeeded,
                        message: a.reason,
                    };
                    return self.deliver_catalog(&a.retry, Err(why));
                }
            }
            Event::Import => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Bağlantıları içe aktar")
                            .add_filter("KentOS veritabanı bağlantıları", &["json"])
                            .pick_file()
                            .await?;
                        let path = file.path().to_path_buf();
                        Some(
                            std::fs::read_to_string(&path)
                                .map(|text| (path, text))
                                .map_err(|e| format!("Dosya okunamadı: {e}.")),
                        )
                    },
                    |r| msg(Event::ImportRead(r)),
                );
            }
            Event::ImportRead(None) => {}
            Event::ImportRead(Some(Err(why))) => self.error(why),
            Event::ImportRead(Some(Ok((path, text)))) => self.import_connections(&path, &text),
            Event::Export(passwords) => {
                if self.db_store.list().is_empty() {
                    self.output("Dışa aktarılacak bağlantı yok.");
                    return Task::none();
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Bağlantıları dışa aktar")
                            .add_filter("KentOS veritabanı bağlantıları", &["json"])
                            .set_file_name("veritabani-baglantilari.json")
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |path| msg(Event::ExportPicked(path, passwords)),
                );
            }
            Event::ExportPicked(None, _) => {}
            Event::ExportPicked(Some(path), passwords) => {
                let list = self.db_store.list();
                let text = connections::export(&list, passwords);
                match std::fs::write(&path, text) {
                    Ok(()) => self.announce(
                        Level::Success,
                        format!(
                            "{} bağlantı {} dosyasına yazıldı{}; başka bir KentOS'ta Kaynaklar › Veritabanları › Bağlantıları içe aktar ile alınır.",
                            list.len(),
                            path.display(),
                            if passwords {
                                ", kaydedilmiş parolalarıyla (dosyayı paylaşırken dikkat edin)"
                            } else {
                                ", parolasız"
                            }
                        ),
                    ),
                    Err(e) => self.error(format!("{} yazılamadı: {e}.", path.display())),
                }
            }
        }
        Task::none()
    }

    fn with_editor(&mut self, change: impl FnOnce(&mut Editor)) {
        if let Some(e) = &mut self.db_editor {
            change(e);
            e.changed();
        }
    }

    /// Adresten doldur: the pasted address's or connection string's fields into the window, its password into the
    /// password field (never into the address).
    fn paste_into_editor(&mut self) {
        let Some(e) = &mut self.db_editor else { return };
        let Some(text) = e.paste.clone() else { return };
        match connections::parse(&text) {
            Ok(parsed) => {
                let c = parsed.connection;
                if !c.host.is_empty() {
                    e.host = c.host;
                }
                e.port = c.port.to_string();
                if !c.database.is_empty() {
                    e.database = c.database.clone();
                }
                if !c.user.is_empty() {
                    e.user = c.user;
                }
                e.ssl = c.ssl;
                if let Some(p) = parsed.password {
                    e.password = p;
                }
                if e.name.trim().is_empty() && !c.database.is_empty() {
                    e.name = self.db_store.unique_name(&c.database);
                }
                e.paste = None;
                e.paste_problem = None;
                e.changed();
            }
            Err(why) => e.paste_problem = Some(why),
        }
    }

    fn save_connection(&mut self) -> Task<Message> {
        let Some(e) = &mut self.db_editor else {
            return Task::none();
        };
        e.tried = true;
        let (c, port) = e.connection();
        if port.is_some() || !c.problems().is_empty() {
            return Task::none();
        }
        if let Err(why) = self.db_store.put(e.previous.as_deref(), c.clone()) {
            e.save_problem = Some(why);
            return Task::none();
        }
        // A password typed but not saved stays for this session.
        if !e.keep_password
            && let Some(typed) = e.typed()
        {
            connections::remember(&c, typed);
        }
        let renamed = e.previous.as_ref().filter(|p| **p != c.name).cloned();
        let then = e.then;
        let new = e.previous.is_none();
        self.db_editor = None;
        if let Some(old) = renamed {
            self.sources.forget_connection(&old);
        }
        self.output(format!(
            "“{}” bağlantısı {}: {}.",
            c.name,
            if new { "kaydedildi" } else { "güncellendi" },
            c.summary()
        ));
        match then {
            Then::Browse => self.browse_connection_in_sources(&c.name),
            Then::Postgis => self.postgis_choose(&c.name),
        }
    }

    fn import_connections(&mut self, path: &std::path::Path, text: &str) {
        let read = match connections::import(text) {
            Ok(r) => r,
            Err(why) => return self.error(why),
        };
        let (mut taken, mut left, mut without) = (Vec::new(), Vec::new(), 0usize);
        for i in read {
            if let Some(why) = i.skipped {
                left.push(why);
                continue;
            }
            let mut c = i.connection;
            c.name = self.db_store.unique_name(&c.name);
            if c.password.is_none() {
                without += 1;
            }
            match self.db_store.put(None, c.clone()) {
                Ok(()) => taken.push(c.name),
                Err(why) => left.push(format!("“{}”: {why}", c.name)),
            }
        }
        if taken.is_empty() && left.is_empty() {
            return self.output(format!("{} dosyasında bağlantı yok.", path.display()));
        }
        let mut said = format!("{} bağlantı alındı: {}.", taken.len(), taken.join(", "));
        if without > 0 {
            said.push_str(&format!(
                " {without} bağlantının parolası dosyada yok; ilk bağlanışta sorulur."
            ));
        }
        self.announce(
            if left.is_empty() {
                Level::Success
            } else {
                Level::Warn
            },
            said,
        );
        for why in left {
            self.say(Level::Warn, why);
        }
        self.sources.open_databases();
    }

    /// The password `connection` needs, asked for `retry`, with why.
    pub(crate) fn ask_db_password(&mut self, connection: &str, reason: String, retry: Retry) {
        if self
            .db_password
            .as_ref()
            .is_some_and(|a| a.connection == connection)
        {
            return;
        }
        self.db_password = Some(PasswordAsk {
            connection: connection.to_owned(),
            password: String::new(),
            keep: false,
            reason,
            retry,
            trying: false,
            generation: 0,
        });
    }

    fn password_tried(
        &mut self,
        generation: u64,
        result: Result<Arc<Catalog>, Diagnosis>,
    ) -> Task<Message> {
        let Some(a) = &mut self.db_password else {
            return Task::none();
        };
        if a.generation != generation {
            return Task::none();
        }
        a.trying = false;
        match result {
            Err(d) if matches!(d.problem, Problem::Password | Problem::PasswordNeeded) => {
                a.reason = "Parola kabul edilmedi; yeniden yazın.".to_owned();
                a.password.clear();
                Task::none()
            }
            other => {
                let Some(a) = self.db_password.take() else {
                    return Task::none();
                };
                if other.is_ok()
                    && let Some(mut c) = self.db_store.get(&a.connection)
                {
                    connections::remember(&c, a.password.clone());
                    if a.keep {
                        c.password = Some(a.password.clone());
                        if let Err(why) = self.db_store.put(Some(&a.connection), c) {
                            self.error(why);
                        }
                    }
                }
                self.deliver_catalog(&a.retry, other)
            }
        }
    }

    /// A catalogue read for `retry`'s list, there.
    fn deliver_catalog(
        &mut self,
        retry: &Retry,
        result: Result<Arc<Catalog>, Diagnosis>,
    ) -> Task<Message> {
        match retry {
            Retry::Browse(name) => {
                self.sources_event(crate::sources::Event::Browsed(name.clone(), result, false))
            }
            Retry::Postgis(name) => self.exchange_event(crate::exchange::Event::Postgis(
                crate::exchange::postgis::Event::Listed(name.clone(), result, false),
            )),
        }
    }

    /// The windows over everything else: Sil's question, the password question, the connection window.
    pub(crate) fn db_view(&self) -> Option<Element<'_, Message>> {
        if let Some(name) = &self.db_removing {
            return Some(overlay::modal(
                Confirm::new("Bağlantıyı sil", msg(Event::Remove), msg(Event::KeepIt))
                    .message(format!("“{name}” bağlantısı silinsin mi?"))
                    .detail("Kaydedilmiş parolası da silinir. Bu bağlantıyla bağlanmış katmanlar çizimde kalır; yeniden bağlanmak için bağlantıyı yeniden tanımlamanız gerekir.")
                    .confirm("Sil")
                    .destructive(),
                msg(Event::KeepIt),
            ));
        }
        if let Some(a) = &self.db_password {
            return Some(self.password_view(a));
        }
        self.db_editor.as_ref().map(|e| self.editor_view(e))
    }

    fn password_view<'a>(&'a self, a: &'a PasswordAsk) -> Element<'a, Message> {
        let c = self.db_store.get(&a.connection);
        let summary = c.as_ref().map_or_else(String::new, DbConnection::summary);
        let field = InputField::new("Parola", &a.password)
            .secure(true)
            .on_input(|t| msg(Event::Asked(t)))
            .on_submit(msg(Event::AskSubmit))
            .id("db-parola");
        let body = column![
            label::body(format!(
                "“{}” ({summary}) bağlanmak için parola istiyor.",
                a.connection
            )),
            words::field("Parola", field, None),
            words::check(
                a.keep,
                "Parolayı bu bilgisayarda sakla",
                Some(msg(Event::AskKeep)),
            ),
            words::text_line(
                if a.trying { Line::Info } else { Line::Warn },
                if a.trying {
                    "Bağlanılıyor…".to_owned()
                } else {
                    a.reason.clone()
                },
            ),
        ]
        .spacing(12);
        overlay::blocking(
            Dialog::new("Parola gerekli")
                .push(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::AskClose))))
                .action(words::primary(
                    "Bağlan",
                    (!a.password.is_empty() && !a.trying).then(|| msg(Event::AskSubmit)),
                ))
                .width(460.0)
                .on_close(Some(msg(Event::AskClose))),
        )
    }

    fn editor_view<'a>(&'a self, e: &'a Editor) -> Element<'a, Message> {
        let input =
            |placeholder: &str, value: &'a str, on: fn(String) -> Event, field: Option<Field>| {
                InputField::new(placeholder, value)
                    .on_input(move |t| msg(on(t)))
                    .on_submit(msg(Event::Save))
                    .invalid(field.is_some_and(|f| e.problem(f).is_some()))
            };
        let mut form = Form::new()
            .label_width(130.0)
            .section("Bağlantı")
            .field("Ad", input("Örneğin Belediye", &e.name, Event::Name, Some(Field::Name)))
            .required()
            .help("Listelerde bu adla görünür.")
            .error(e.problem(Field::Name))
            .field("Sunucu", input("db.belediye.local ya da 10.0.0.5", &e.host, Event::Host, Some(Field::Host)))
            .required()
            .help("Veritabanı sunucusunun adı ya da IP adresi.")
            .error(e.problem(Field::Host))
            .field(
                "Bağlantı noktası",
                input("5432", &e.port, Event::Port, Some(Field::Port)).width(110.0),
            )
            .help("PostgreSQL'de genellikle 5432.")
            .error(e.problem(Field::Port))
            .field("Veritabanı", input("Örneğin kent", &e.database, Event::Database, Some(Field::Database)))
            .required()
            .error(e.problem(Field::Database))
            .section("Kimlik")
            .field("Kullanıcı adı", input("Örneğin ali", &e.user, Event::User, Some(Field::User)))
            .error(e.problem(Field::User))
            .field(
                "Parola",
                InputField::new(
                    if e.kept {
                        "Kayıtlı parola değişmez"
                    } else {
                        ""
                    },
                    &e.password,
                )
                .secure(true)
                .on_input(|t| msg(Event::Password(t)))
                .on_submit(msg(Event::Save)),
            )
            .row(words::check(
                e.keep_password,
                "Parolayı bu bilgisayarda sakla",
                Some(msg(Event::KeepPassword)),
            ))
            .row(label::caption(if e.keep_password {
                "Parola yalnız sizin okuyabildiğiniz bir dosyada (veritabani-baglantilari.json) düz metin olarak saklanır; çizime hiç yazılmaz."
            } else {
                "Parola saklanmaz: bu oturumda yazdığınız kullanılır, bir dahaki açılışta sorulur. Çizime hiç yazılmaz."
            }))
            .section("Güvenlik")
            .field(
                "SSL",
                Select::new(
                    SSL_MODES.iter().map(|m| Choice::new(m.label()).detail(m.libpq())),
                    SSL_MODES.iter().position(|m| *m == e.ssl),
                    |i| msg(Event::Ssl(i)),
                )
                .searchable(false),
            )
            .help(e.ssl.note())
            .section("Seçenekler")
            .row(words::check(
                e.public_only,
                "Yalnız public şemasını listele",
                Some(msg(Event::PublicOnly)),
            ))
            .row(words::check(
                e.geometryless,
                "Geometrisiz tabloları da listele",
                Some(msg(Event::Geometryless)),
            ));
        if let Some(why) = &e.save_problem {
            form = form.row(words::text_line(Line::Error, why.clone()));
        }
        let paste: Element<'a, Message> = match &e.paste {
            None => button(
                row![
                    kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronRight).size(12.0),
                    label::body("Adresten doldur")
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(msg(Event::PasteToggle))
            .padding([2, 4])
            .style(style::button::ghost)
            .into(),
            Some(text) => {
                let mut c = Column::new().spacing(6).push(
                    row![
                        InputField::new(
                            "postgresql://kullanıcı@sunucu:5432/veritabanı ya da host=… dbname=… user=…",
                            text,
                        )
                        .on_input(|t| msg(Event::Paste(t)))
                        .on_submit(msg(Event::PasteApply)),
                        words::secondary("Doldur", (!text.trim().is_empty()).then(|| msg(Event::PasteApply))),
                        words::ghost("Kapat", Some(msg(Event::PasteToggle))),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
                c = c.push(label::caption(
                    "Elinizde bir PostgreSQL adresi ya da bağlantı dizesi varsa yapıştırın; alanlara dağıtılır, içindeki parola parola alanına gider. Zorunlu değildir: alanları elle doldurmak yeter.",
                ));
                if let Some(why) = &e.paste_problem {
                    c = c.push(words::text_line(Line::Error, why.clone()));
                }
                c.into()
            }
        };
        let tested: Element<'a, Message> = match (&e.tested, e.testing) {
            (_, true) => words::text_line(Line::Info, "Bağlanılıyor…"),
            (Some(Ok(facts)), _) => {
                let mut lines = vec![words::text_line(Line::Ok, format!("Bağlandı: {}", facts.line()))];
                if facts.postgis.is_none() {
                    lines.push(words::text_line(
                        Line::Warn,
                        "Bu veritabanında PostGIS kurulu değil: geometri sütunları okunamaz. Veritabanı yöneticiniz “CREATE EXTENSION postgis;” ile kurabilir.",
                    ));
                }
                Column::with_children(lines).spacing(4).into()
            }
            (Some(Err(d)), _) => words::text_line(Line::Error, d.message.clone()),
            (None, _) => label::caption(
                "Bağlantıyı dene, sunucuya bu bilgilerle bağlanıp PostgreSQL'in ve PostGIS'in sürümünü ve tabloların sayısını söyler; kaydetmez.",
            )
            .into(),
        };
        let test = row![
            words::secondary("Bağlantıyı dene", (!e.testing).then(|| msg(Event::Test))),
            tested
        ]
        .spacing(12)
        .align_y(Center);
        let body = column![paste, form, test].spacing(14);
        overlay::blocking(
            Dialog::new(if e.previous.is_some() {
                "PostgreSQL bağlantısını düzenle"
            } else {
                "Yeni PostgreSQL bağlantısı"
            })
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
            .action(words::primary("Kaydet", Some(msg(Event::Save))))
            .width(620.0)
            .max_height(760.0)
            .on_close(Some(msg(Event::Close))),
        )
    }
}
