//! Bağlantılar (docs/adr/0208 §12; the web's `ui/services/ConnectionsDialog.ts`):
//! the project's connections on the left, the one chosen on the right: its
//! name, its origin (where its proof goes), how it proves itself and its
//! secret values. Kaydet writes the connections into the project's settings
//! (not an undo step, as the other settings) and the secrets into this
//! device's file (`secrets.rs`): a secret never goes into the drawing. Dene
//! asks a service that uses the connection (its capabilities, a tile, a
//! session) with the values as typed and says what it answered; ArcGIS and
//! OAuth 2 ask for their token first.

use iced::widget::{Column, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{AuthKind, ConnectionSecret, ServiceConnection, connections_problem};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;

pub const TITLE: &str = "Bağlantılar";

/// A connection as the window edits it: its definition and its secrets as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The id it had (none: new).
    pub from: Option<String>,
    pub id: String,
    pub name: String,
    pub origin: String,
    pub auth: AuthKind,
    /// Query and header: each name with its value.
    pub pairs: Vec<(String, String)>,
    pub user: String,
    pub password: String,
    pub token: String,
    pub client_id: String,
    pub client_secret: String,
    pub token_url: String,
    pub scope: String,
}

impl Row {
    fn of(c: &ServiceConnection, secret: Option<ConnectionSecret>) -> Row {
        let s = secret.unwrap_or_default();
        Row {
            from: Some(c.id.clone()),
            id: c.id.clone(),
            name: c.name.clone(),
            origin: c.origin.clone(),
            auth: c.auth,
            pairs: c
                .names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.clone(), s.values.get(i).cloned().unwrap_or_default()))
                .collect(),
            user: s.user.unwrap_or_default(),
            password: s.password.unwrap_or_default(),
            token: s.token.unwrap_or_default(),
            client_id: s.client_id.unwrap_or_default(),
            client_secret: s.client_secret.unwrap_or_default(),
            token_url: c.token_url.clone().unwrap_or_default(),
            scope: c.scope.clone().unwrap_or_default(),
        }
    }

    /// The connection it defines (no secret).
    pub fn connection(&self) -> ServiceConnection {
        let named = matches!(self.auth, AuthKind::Query | AuthKind::Header);
        let some = |t: &str| (!t.trim().is_empty()).then(|| t.trim().to_owned());
        ServiceConnection {
            id: self.id.clone(),
            name: self.name.trim().to_owned(),
            origin: kentos_contracts::origin_of(&self.origin)
                .unwrap_or_else(|| self.origin.trim().to_owned()),
            auth: self.auth,
            names: if named {
                self.pairs
                    .iter()
                    .map(|(n, _)| n.trim().to_owned())
                    .collect()
            } else {
                Vec::new()
            },
            token_url: if matches!(self.auth, AuthKind::Arcgis | AuthKind::Oauth2) {
                some(&self.token_url)
            } else {
                None
            },
            scope: if self.auth == AuthKind::Oauth2 {
                some(&self.scope)
            } else {
                None
            },
        }
    }

    /// Its secrets as typed (none for a connection that proves nothing).
    pub fn secret(&self) -> Option<ConnectionSecret> {
        let some = |t: &str| (!t.is_empty()).then(|| t.to_owned());
        let s = match self.auth {
            AuthKind::None => return None,
            AuthKind::Query | AuthKind::Header => ConnectionSecret {
                id: self.id.clone(),
                values: self.pairs.iter().map(|(_, v)| v.clone()).collect(),
                ..ConnectionSecret::default()
            },
            AuthKind::Basic | AuthKind::Arcgis => ConnectionSecret {
                id: self.id.clone(),
                user: some(&self.user),
                password: some(&self.password),
                ..ConnectionSecret::default()
            },
            AuthKind::Bearer | AuthKind::Google => ConnectionSecret {
                id: self.id.clone(),
                token: some(self.token.trim()),
                ..ConnectionSecret::default()
            },
            AuthKind::Oauth2 => ConnectionSecret {
                id: self.id.clone(),
                client_id: some(self.client_id.trim()),
                client_secret: some(&self.client_secret),
                ..ConnectionSecret::default()
            },
        };
        Some(s)
    }
}

/// The window's state.
#[derive(Clone, Debug)]
pub struct Window {
    pub rows: Vec<Row>,
    /// The connections the window opened with, to know what changed.
    pub before: Vec<Row>,
    pub chosen: Option<usize>,
    /// Values shown as typed rather than hidden.
    pub show: bool,
    /// What Dene, Kaydet or a refusal said.
    pub said: Option<(Kind, String)>,
    pub trying: bool,
}

impl Window {
    fn changed(&self) -> bool {
        self.rows != self.before
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Add,
    Remove,
    Name(String),
    Origin(String),
    Auth(usize),
    PairName(usize, String),
    PairValue(usize, String),
    AddPair,
    RemovePair(usize),
    User(String),
    Password(String),
    Token(String),
    ClientId(String),
    ClientSecret(String),
    TokenUrl(String),
    Scope(String),
    Show(bool),
    Try,
    Tried(Result<String, String>),
    Save,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Services(super::app::Event::Connections(e))
}

/// A new connection's id: `baglanti`, then `baglanti-2` … as the others leave free.
fn free_id(rows: &[Row]) -> String {
    let taken = |id: &str| rows.iter().any(|r| r.id == id);
    (1..)
        .map(|n| {
            if n == 1 {
                "baglanti".to_owned()
            } else {
                format!("baglanti-{n}")
            }
        })
        .find(|id| !taken(id))
        .unwrap_or_else(|| "baglanti".into())
}

/// The services and data sources of the drawing that use connection `id`, by layer name.
fn users(doc: &crate::document::Document, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    for n in doc.model.layers().leaves() {
        let uses = n.service.as_ref().and_then(|s| s.connection.as_deref()) == Some(id)
            || n.feed.as_ref().and_then(|f| f.connection.as_deref()) == Some(id);
        if uses {
            out.push(n.name.clone());
        }
    }
    out
}

impl App {
    /// Opens Bağlantılar, the connection `focus` chosen when given.
    pub(crate) fn open_connections(&mut self, focus: Option<String>) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let secrets = super::secrets::secrets();
        let rows: Vec<Row> = doc
            .model
            .settings()
            .connections
            .iter()
            .map(|c| Row::of(c, secrets.get(&c.origin, &c.id)))
            .collect();
        let chosen = focus
            .and_then(|f| rows.iter().position(|r| r.id == f))
            .or((!rows.is_empty()).then_some(0));
        self.connections = Some(Window {
            before: rows.clone(),
            rows,
            chosen,
            show: false,
            said: None,
            trying: false,
        });
        self.dialog = Some(Dialog::Connections);
    }

    pub(crate) fn connections_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = &mut self.connections else {
            return Task::none();
        };
        let row = w.chosen.and_then(|i| w.rows.get_mut(i));
        match event {
            Event::Choose(i) => {
                w.chosen = (i < w.rows.len()).then_some(i);
                w.said = None;
            }
            Event::Add => {
                let id = free_id(&w.rows);
                w.rows.push(Row {
                    from: None,
                    id,
                    name: "Yeni bağlantı".into(),
                    origin: "https://".into(),
                    auth: AuthKind::Query,
                    pairs: vec![("apikey".into(), String::new())],
                    user: String::new(),
                    password: String::new(),
                    token: String::new(),
                    client_id: String::new(),
                    client_secret: String::new(),
                    token_url: String::new(),
                    scope: String::new(),
                });
                w.chosen = Some(w.rows.len() - 1);
                w.said = None;
            }
            Event::Remove => {
                if let Some(i) = w.chosen {
                    let id = w.rows[i].id.clone();
                    let used = self
                        .document
                        .as_ref()
                        .map(|d| users(d, &id))
                        .unwrap_or_default();
                    let Some(w) = &mut self.connections else {
                        return Task::none();
                    };
                    if used.is_empty() {
                        w.rows.remove(i);
                        w.chosen = if w.rows.is_empty() {
                            None
                        } else {
                            Some(i.min(w.rows.len() - 1))
                        };
                        w.said = None;
                    } else {
                        w.said = Some((
                            Kind::Error,
                            format!(
                                "Bu bağlantıyı şu katmanlar kullanıyor: {}. Önce onları silin ya da başka bağlantıya geçirin.",
                                used.join(", ")
                            ),
                        ));
                    }
                }
            }
            Event::Name(t) => {
                if let Some(r) = row {
                    r.name = t;
                }
            }
            Event::Origin(t) => {
                if let Some(r) = row {
                    r.origin = t;
                }
            }
            Event::Auth(k) => {
                if let (Some(r), Some(kind)) = (row, AuthKind::ALL.get(k)) {
                    r.auth = *kind;
                    if matches!(kind, AuthKind::Query | AuthKind::Header) && r.pairs.is_empty() {
                        r.pairs.push((
                            if *kind == AuthKind::Query {
                                "apikey".into()
                            } else {
                                "X-API-Key".into()
                            },
                            String::new(),
                        ));
                    }
                }
            }
            Event::PairName(i, t) => {
                if let Some(p) = row.and_then(|r| r.pairs.get_mut(i)) {
                    p.0 = t;
                }
            }
            Event::PairValue(i, t) => {
                if let Some(p) = row.and_then(|r| r.pairs.get_mut(i)) {
                    p.1 = t;
                }
            }
            Event::AddPair => {
                if let Some(r) = row {
                    r.pairs.push((String::new(), String::new()));
                }
            }
            Event::RemovePair(i) => {
                if let Some(r) = row
                    && i < r.pairs.len()
                    && r.pairs.len() > 1
                {
                    r.pairs.remove(i);
                }
            }
            Event::User(t) => {
                if let Some(r) = row {
                    r.user = t;
                }
            }
            Event::Password(t) => {
                if let Some(r) = row {
                    r.password = t;
                }
            }
            Event::Token(t) => {
                if let Some(r) = row {
                    r.token = t;
                }
            }
            Event::ClientId(t) => {
                if let Some(r) = row {
                    r.client_id = t;
                }
            }
            Event::ClientSecret(t) => {
                if let Some(r) = row {
                    r.client_secret = t;
                }
            }
            Event::TokenUrl(t) => {
                if let Some(r) = row {
                    r.token_url = t;
                }
            }
            Event::Scope(t) => {
                if let Some(r) = row {
                    r.scope = t;
                }
            }
            Event::Show(on) => w.show = on,
            Event::Try => return self.try_connection(),
            Event::Tried(answer) => {
                w.trying = false;
                w.said = Some(match answer {
                    Ok(t) => (Kind::Ok, t),
                    Err(t) => (Kind::Error, t),
                });
            }
            Event::Save => self.save_connections(),
            Event::Cancel => {
                self.connections = None;
                self.dialog = self.after_connections();
            }
        }
        Task::none()
    }

    /// Dene: a service of the drawing that uses the chosen connection asked
    /// with its values as typed, off the window's thread.
    fn try_connection(&mut self) -> Task<Message> {
        let Some(w) = &mut self.connections else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        let conn = r.connection();
        if let Some(p) = conn.problem() {
            w.said = Some((Kind::Error, p));
            return Task::none();
        }
        let secret = r.secret();
        if let Some(why) = kentos_services::auth::missing(&conn, secret.as_ref()) {
            w.said = Some((Kind::Error, why));
            return Task::none();
        }
        let from = r.from.clone().unwrap_or_else(|| r.id.clone());
        let service = self.document.as_ref().and_then(|d| {
            d.model.layers().leaves().into_iter().find_map(|n| {
                n.service
                    .clone()
                    .filter(|s| s.connection.as_deref() == Some(from.as_str()))
            })
        });
        let Some(service) = service else {
            w.said = Some((
                Kind::Info,
                "Bu bağlantıyı kullanan servis yok: Harita servisi penceresinde bu bağlantıyla Bağlan'a basarak deneyin.".into(),
            ));
            return Task::none();
        };
        let Some(req) = kentos_services::connect::probe(&service) else {
            w.said = Some((Kind::Error, "Servis için deneme isteği kurulamadı.".into()));
            return Task::none();
        };
        w.trying = true;
        w.said = Some((Kind::Info, "Servise soruluyor…".into()));
        let per_host = kentos_services::presets::per_host(&service) as usize;
        super::app::off_thread(
            move || {
                super::net::ask_with(req, Some((&conn, secret.as_ref())), &service.url, per_host)
                    .map(|(status, size)| {
                        format!(
                            "Bağlantı çalışıyor: servis {status} dedi ({}).",
                            bytes_text(size)
                        )
                    })
            },
            |answer| super::app::Event::Connections(Event::Tried(answer)),
        )
    }

    /// Kaydet: the project's connections and this device's secrets.
    fn save_connections(&mut self) {
        let Some(w) = &mut self.connections else {
            return;
        };
        let list: Vec<ServiceConnection> = w.rows.iter().map(Row::connection).collect();
        if let Some(p) = connections_problem(&list) {
            w.said = Some((Kind::Error, p));
            return;
        }
        let secrets = super::secrets::secrets();
        // The ones gone or moved: their secrets forgotten at their old place.
        for b in &w.before {
            let kept = w.rows.iter().any(|r| {
                r.from.as_deref() == Some(b.id.as_str()) && r.connection().origin == b.origin
            });
            if !kept && let Err(e) = secrets.remove(&b.origin, &b.id) {
                w.said = Some((Kind::Error, e));
                return;
            }
        }
        for r in &w.rows {
            let c = r.connection();
            let result = match r.secret() {
                Some(s) => secrets.put(&c.origin, s),
                None => secrets.remove(&c.origin, &c.id),
            };
            if let Err(e) = result {
                w.said = Some((Kind::Error, e));
                return;
            }
        }
        let n = list.len();
        if let Some(doc) = &mut self.document {
            let mut settings = doc.model.settings().clone();
            settings.connections = list;
            doc.model.set_settings(settings);
        }
        self.connections = None;
        self.dialog = self.after_connections();
        self.say(
            kentos_interaction::Level::Success,
            format!("Bağlantılar kaydedildi ({n}); gizli değerler bu cihazda saklandı."),
        );
    }

    /// The window Bağlantılar was opened over, shown again when it closes.
    fn after_connections(&self) -> Option<Dialog> {
        self.service_window.is_some().then_some(Dialog::ServiceAdd)
    }

    pub(crate) fn connections_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.connections else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(2);
        for (i, r) in w.rows.iter().enumerate() {
            let host = r
                .origin
                .split_once("://")
                .map_or(r.origin.as_str(), |(_, h)| h)
                .to_owned();
            let line = column![
                label::body(r.name.clone()),
                label::caption(host),
                label::caption(r.auth.label()),
            ]
            .spacing(1);
            list = list.push(
                button(line)
                    .width(Fill)
                    .padding([4, 8])
                    .on_press(msg(Event::Choose(i)))
                    .style(style::button::table_row(w.chosen == Some(i), false)),
            );
        }
        if w.rows.is_empty() {
            list = list
                .push(container(label::muted("Bağlantı yok. Yeni bağlantı ekleyin.")).padding(8));
        }
        let tool = |glyph: &str, words: &'static str, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press_maybe(on)
        };
        let left = column![
            container(scrollable(list).height(Length::Fixed(typography::scaled(300.0))))
                .style(style::container::field_box)
                .width(Fill),
            row![
                tool("plus", "Yeni bağlantı", Some(msg(Event::Add))),
                tool("erase", "Sil", w.chosen.map(|_| msg(Event::Remove))),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .width(Length::Fixed(typography::scaled(250.0)));
        let right: Element<'_, Message> = match w.chosen.and_then(|i| w.rows.get(i)) {
            None => container(label::muted("Soldan bir bağlantı seçin."))
                .padding(8)
                .into(),
            Some(r) => self.connection_form(w, r),
        };
        let line = match &w.said {
            Some((kind, t)) => words::text_line(*kind, t.clone()),
            None => words::text_line(
                Kind::Info,
                "Gizli değerler çizime yazılmaz; yalnız bu cihazda saklanır. Çizim başka cihazda açılınca orada yeniden girilir.",
            ),
        };
        overlay::modal(
            Frame::new(TITLE)
                .push(
                    column![row![left, right].spacing(16), words::summary(vec![line])].spacing(12),
                )
                .action(words::secondary("Vazgeç", Some(msg(Event::Cancel))))
                .action(words::primary(
                    "Kaydet",
                    w.changed().then(|| msg(Event::Save)),
                ))
                .width(860.0),
            msg(Event::Cancel),
        )
    }

    fn connection_form<'a>(&'a self, w: &'a Window, r: &'a Row) -> Element<'a, Message> {
        let field = |hint: &'a str,
                     value: &'a str,
                     on: fn(String) -> Event,
                     secret: bool|
         -> Element<'a, Message> {
            text_input(hint, value)
                .size(typography::body())
                .padding([4, 8])
                .secure(secret && !w.show)
                .on_input(move |t| msg(on(t)))
                .style(style::field::input)
                .into()
        };
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(130.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let kinds = Select::new(
            AuthKind::ALL.iter().map(|k| Choice::new(k.label())),
            AuthKind::ALL.iter().position(|k| *k == r.auth),
            |k| msg(Event::Auth(k)),
        );
        let mut form = Column::new()
            .spacing(8)
            .push(labelled(
                "Ad",
                field("Bağlantının adı", &r.name, Event::Name, false),
            ))
            .push(labelled(
                "Köken",
                field("https://makine[:kapı]", &r.origin, Event::Origin, false),
            ))
            .push(label::caption(
                "Kanıt yalnız bu kökene (şema, makine ve kapı) giden isteklere eklenir.",
            ))
            .push(labelled("Doğrulama", kinds.into()));
        match r.auth {
            AuthKind::None => {
                form = form.push(label::muted("Servis kimlik istemiyor."));
            }
            AuthKind::Query | AuthKind::Header => {
                let what = if r.auth == AuthKind::Query {
                    "Parametre"
                } else {
                    "Başlık"
                };
                for (i, (n, v)) in r.pairs.iter().enumerate() {
                    form = form.push(
                        row![
                            container(
                                text_input(what, n)
                                    .size(typography::body())
                                    .padding([4, 8])
                                    .on_input(move |t| msg(Event::PairName(i, t)))
                                    .style(style::field::input)
                            )
                            .width(Length::Fixed(typography::scaled(130.0))),
                            text_input("Değer", v)
                                .size(typography::body())
                                .padding([4, 8])
                                .secure(!w.show)
                                .on_input(move |t| msg(Event::PairValue(i, t)))
                                .style(style::field::input),
                            button(icon(kentos_ui::icon::Icon::Close).size(12.0))
                                .style(style::button::subtle)
                                .padding(4)
                                .on_press_maybe(
                                    (r.pairs.len() > 1).then(|| msg(Event::RemovePair(i)))
                                ),
                        ]
                        .spacing(8)
                        .align_y(Center),
                    );
                }
                form = form.push(
                    button(label::body(if r.auth == AuthKind::Query {
                        "Parametre ekle"
                    } else {
                        "Başlık ekle"
                    }))
                    .style(style::button::secondary)
                    .padding([3, 10])
                    .on_press(msg(Event::AddPair)),
                );
            }
            AuthKind::Basic | AuthKind::Arcgis => {
                form = form
                    .push(labelled(
                        "Kullanıcı adı",
                        field("", &r.user, Event::User, false),
                    ))
                    .push(labelled(
                        "Parola",
                        field("", &r.password, Event::Password, true),
                    ));
                if r.auth == AuthKind::Arcgis {
                    form = form.push(labelled(
                        "Belirteç adresi",
                        field(
                            "…/arcgis/tokens/generateToken (boşsa sunucunun)",
                            &r.token_url,
                            Event::TokenUrl,
                            false,
                        ),
                    ));
                }
            }
            AuthKind::Bearer => {
                form = form.push(labelled(
                    "Belirteç",
                    field("", &r.token, Event::Token, true),
                ));
            }
            AuthKind::Google => {
                form = form
                    .push(labelled("API anahtarı", field("AIza…", &r.token, Event::Token, true)))
                    .push(label::caption(
                        "Anahtar sizin Google Cloud hesabınızındır (Map Tiles API açık, faturalandırma tanımlı); KentOS anahtar vermez.",
                    ));
            }
            AuthKind::Oauth2 => {
                form = form
                    .push(labelled(
                        "İstemci kimliği",
                        field("", &r.client_id, Event::ClientId, false),
                    ))
                    .push(labelled(
                        "İstemci sırrı",
                        field("", &r.client_secret, Event::ClientSecret, true),
                    ))
                    .push(labelled(
                        "Belirteç adresi",
                        field("https://…/token", &r.token_url, Event::TokenUrl, false),
                    ))
                    .push(labelled(
                        "Kapsam",
                        field("isteğe bağlı", &r.scope, Event::Scope, false),
                    ));
            }
        }
        let used = self
            .document
            .as_ref()
            .map(|d| users(d, r.from.as_deref().unwrap_or(&r.id)))
            .unwrap_or_default();
        form = form
            .push(
                row![
                    check_box(
                        if w.show {
                            Check::Checked
                        } else {
                            Check::Unchecked
                        },
                        Some(msg(Event::Show(!w.show)))
                    ),
                    label::body("Değerleri göster"),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .push(label::caption(if used.is_empty() {
                "Bu bağlantıyı kullanan katman yok.".to_owned()
            } else {
                format!("Kullanan katmanlar: {}", used.join(", "))
            }))
            .push(
                button(
                    row![
                        icon(from_web(Some("serviceConnections"))).size(14.0),
                        label::body("Dene")
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .style(style::button::secondary)
                .padding([4, 12])
                .on_press_maybe((!w.trying).then(|| msg(Event::Try))),
            );
        container(form).width(Fill).into()
    }
}

/// A size in the window's words.
fn bytes_text(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{} KB", n / 1024)
    } else {
        format!("{n} bayt")
    }
}
