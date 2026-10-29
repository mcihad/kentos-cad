//! PostGIS'e bağlan (docs/adr/0505, 0582, 0623; QGIS's Veri kaynağı yöneticisi › PostgreSQL): a saved connection is
//! chosen (Yeni and Düzenle open the connection window, db_connections.rs), its tables, views and materialized views
//! are read from the database's catalogue off the interface's thread (`kentos_postgis::probe::browse`: no row is
//! counted) and searched, and the chosen ones and a query layer are bound to new layers with `cad.sources.edit`, by
//! the view's area when asked, a table in another system only with Projenin sistemine çevir ticked. A server asking
//! for a password KentOS does not know opens the password question. Nobody types an address: the one a bound layer
//! keeps is built from the connection's fields and carries no password.

use std::collections::BTreeSet;
use std::sync::Arc;

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, SourceTable, SourcesDescribe, SourcesDescribed, SourcesEdit, SourcesEditOp,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_postgis::connections;
#[cfg(test)]
use kentos_postgis::connections::DbConnection;
use kentos_postgis::probe::{self, Catalog, Diagnosis, Problem, Relation};
use kentos_ui::widget::{Choice, Dialog, InputField, Select, overlay};
use kentos_ui::label;

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use crate::db_connections::{Event as Db, Retry, Then};

#[derive(Debug, Default)]
pub struct State {
    /// The chosen connection's name.
    pub connection: Option<String>,
    /// Its catalogue, once listed.
    pub catalog: Option<Arc<Catalog>>,
    pub listing: bool,
    /// Tablo ara.
    pub filter: String,
    /// The chosen tables, `schema.name`.
    pub chosen: BTreeSet<String>,
    pub reproject: bool,
    /// A query layer's SQL (docs/adr/0582 §3); empty: none.
    pub query: String,
    /// Gelişmiş (the query layer) open.
    pub advanced: bool,
    /// Only the rows in the view's area (docs/adr/0582 §1).
    pub area: bool,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A connection chosen, by its place in the list.
    Choose(usize),
    New,
    Edit,
    List,
    /// A connection's catalogue, or why not; `ask`: a password the server wants is asked for.
    Listed(String, Result<Arc<Catalog>, Diagnosis>, bool),
    Filter(String),
    Toggle(String),
    /// Every table shown chosen, or none.
    All(bool),
    Reproject,
    Query(String),
    Advanced,
    Area,
    Run,
}

/// An empty drawing for the listing: describing a database reads no drawing.
pub(crate) fn listing_document() -> Result<kentos_domain::Document, String> {
    let empty = r#"{"format":"kentos.document","version":1,"name":"PostGIS","settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},
        "origin":{"x":0,"y":0},"activeLayer":"0","layers":[{"id":"0","name":"0","type":"layer","visible":true,
        "locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "entities":[],"styles":{"items":[],"categories":[]}}"#;
    kentos_commands::DocumentSnapshotV1::from_json(empty)
        .map_err(|e| e.to_string())
        .and_then(kentos_domain::Document::from_snapshot)
}

/// A database's tables through `cad.sources.describe`, or why not.
pub(crate) fn list_tables(address: String) -> Result<Vec<SourceTable>, String> {
    let mut doc = listing_document()?;
    let answer = kentos_native_application::sources::describe::execute(
        &mut ExecutionContext::new(&mut doc),
        SourcesDescribe {
            path: Some(address),
            ..SourcesDescribe::default()
        },
    );
    match answer {
        CommandResult::Completed {
            output: SourcesDescribed { tables, .. },
            ..
        } => Ok(tables),
        CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
        _ => Err("tablolar okunamadı".into()),
    }
}

fn event(e: Event) -> Message {
    message(Exchange::Postgis(e))
}

/// How a bound source is said: a table's or a view's name, a query as “Sorgu katmanı”.
fn shown_name(table: &str) -> String {
    let first = table.split_whitespace().next().unwrap_or_default();
    if first.eq_ignore_ascii_case("select") || first.eq_ignore_ascii_case("with") {
        "Sorgu katmanı".to_owned()
    } else {
        table.to_owned()
    }
}

/// What a bind did: the sources bound, their objects, the commands' warnings and the new objects.
pub(crate) struct Bound {
    pub(crate) names: Vec<String>,
    pub(crate) objects: u32,
    pub(crate) warnings: Vec<String>,
    pub(crate) slots: Vec<Slot>,
}

/// Whether a relation is shown for `filter` (its schema, name or comment, Turkish case aside).
fn shown(r: &Relation, filter: &str) -> bool {
    let fold = |t: &str| t.to_lowercase().replace('İ', "i").replace('I', "ı");
    let f = fold(filter.trim());
    f.is_empty()
        || fold(&r.table()).contains(&f)
        || r.comment.as_deref().is_some_and(|c| fold(c).contains(&f))
}

impl App {
    pub(super) fn postgis_open(&mut self) -> Task<Message> {
        let list = self.db_store.list();
        let chosen = self
            .postgis_last
            .clone()
            .filter(|n| list.iter().any(|c| c.name == *n))
            .or_else(|| (list.len() == 1).then(|| list[0].name.clone()));
        self.open_window(Window::Postgis(State {
            connection: chosen.clone(),
            ..State::default()
        }));
        // A connection whose password is known lists at once; one that would ask waits for Bağlan.
        match chosen.and_then(|n| self.db_store.get(&n)) {
            Some(c) if connections::knows_password(&c) => self.postgis_list(),
            _ => Task::none(),
        }
    }

    /// The connection named `name` chosen in the window (a connection just saved from it) and listed.
    pub(crate) fn postgis_choose(&mut self, name: &str) -> Task<Message> {
        let Some(Window::Postgis(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.connection = Some(name.to_owned());
        s.catalog = None;
        s.chosen.clear();
        s.status = None;
        self.postgis_list()
    }

    fn postgis_list(&mut self) -> Task<Message> {
        let Some(Window::Postgis(s)) = &mut self.exchange else {
            return Task::none();
        };
        let Some(c) = s.connection.as_deref().and_then(|n| self.db_store.get(n)) else {
            return Task::none();
        };
        s.listing = true;
        s.status = None;
        self.postgis_last = Some(c.name.clone());
        let name = c.name.clone();
        crate::db_connections::off_thread(
            move || probe::browse(&c, None).map(Arc::new),
            move |r| event(Event::Listed(name, r, true)),
        )
    }

    pub(super) fn postgis_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::Postgis(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Choose(i) => {
                let list = self.db_store.list();
                let Some(c) = list.get(i) else {
                    return Task::none();
                };
                if s.connection.as_deref() == Some(c.name.as_str()) {
                    return Task::none();
                }
                s.connection = Some(c.name.clone());
                s.catalog = None;
                s.chosen.clear();
                s.status = None;
                if connections::knows_password(c) {
                    return self.postgis_list();
                }
            }
            Event::New => return self.db_event(Db::New(Then::Postgis)),
            Event::Edit => {
                if let Some(name) = s.connection.clone() {
                    return self.db_event(Db::Edit(name, Then::Postgis));
                }
            }
            Event::List => return self.postgis_list(),
            Event::Listed(name, result, ask) => {
                if s.connection.as_deref() != Some(name.as_str()) {
                    return Task::none();
                }
                s.listing = false;
                match result {
                    Ok(catalog) => {
                        s.chosen.retain(|t| catalog.relations.iter().any(|r| r.table() == *t));
                        s.status = Some((false, format!("Bağlandı: {}", catalog.facts.line())));
                        s.catalog = Some(catalog);
                    }
                    Err(d) => {
                        s.catalog = None;
                        s.chosen.clear();
                        let password = matches!(d.problem, Problem::PasswordNeeded | Problem::Password);
                        s.status = Some((true, d.message.clone()));
                        if ask && password {
                            self.ask_db_password(&name, d.message, Retry::Postgis(name.clone()));
                        }
                    }
                }
            }
            Event::Filter(f) => s.filter = f,
            Event::Toggle(t) => {
                if !s.chosen.remove(&t) {
                    s.chosen.insert(t);
                }
            }
            Event::All(all) => {
                let Some(catalog) = &s.catalog else {
                    return Task::none();
                };
                let c = s.connection.as_deref().and_then(|n| self.db_store.get(n)).unwrap_or_default();
                for (_, relations) in catalog.schemas(&c) {
                    for r in relations.into_iter().filter(|r| shown(r, &s.filter) && r.why_not().is_none()) {
                        if all {
                            s.chosen.insert(r.table());
                        } else {
                            s.chosen.remove(&r.table());
                        }
                    }
                }
            }
            Event::Reproject => s.reproject = !s.reproject,
            Event::Query(q) => {
                s.query = q;
                s.status = None;
            }
            Event::Advanced => s.advanced = !s.advanced,
            Event::Area => s.area = !s.area,
            Event::Run => self.postgis_bind(),
        }
        Task::none()
    }

    /// Binds `tables` (each `schema.name` or a query, and whether it has geometry) of the database at `address`, one
    /// `cad.sources.edit` each; the first refusal stops, with what was bound before it said.
    pub(crate) fn bind_database(
        &mut self,
        address: &str,
        tables: &[(String, bool)],
        reproject: bool,
        area: Option<[f64; 4]>,
    ) -> Result<Bound, String> {
        let Some(doc) = &mut self.document else {
            return Err("Açık çizim yok. Önce bir çizim açın (Ctrl+O) ya da yeni proje oluşturun.".to_owned());
        };
        let first = doc.model.next_slot();
        let (mut names, mut objects, mut warnings) = (Vec::new(), 0u32, Vec::new());
        for (table, geometry) in tables {
            let input = SourcesEdit {
                edit: SourcesEditOp::Link {
                    extent: area.filter(|_| *geometry),
                    path: address.to_owned(),
                    table: table.clone(),
                    layer: None,
                    reproject,
                    key: None,
                },
            };
            match kentos_native_application::sources::edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                input,
            ) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    objects += output.objects;
                    names.push(shown_name(table));
                    warnings.extend(w.into_iter().map(|w| w.message));
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    return Err(if names.is_empty() {
                        error.message
                    } else {
                        format!(
                            "{} bağlandı; “{}” bağlanamadı: {}",
                            names.join(", "),
                            shown_name(table),
                            error.message
                        )
                    });
                }
                _ => return Err("Bağlanamadı.".to_owned()),
            }
        }
        let last = doc.model.next_slot();
        let slots: Vec<Slot> = (first..last)
            .filter_map(|n| u32::try_from(n).ok())
            .map(Slot)
            .filter(|s| doc.model.get(*s).is_some())
            .collect();
        Ok(Bound {
            names,
            objects,
            warnings,
            slots,
        })
    }

    fn postgis_bind(&mut self) {
        let Some(Window::Postgis(s)) = &self.exchange else {
            return;
        };
        let Some(c) = s.connection.as_deref().and_then(|n| self.db_store.get(n)) else {
            return;
        };
        let reproject = s.reproject;
        // The view's area, for what has geometry (a query: whatever it holds; docs/adr/0582 §1).
        let area = s.area.then(|| {
            let b = self.viewport.camera.visible_bounds();
            [b.min_x, b.min_y, b.max_x, b.max_y]
        });
        let mut tables: Vec<(String, bool)> = s
            .catalog
            .iter()
            .flat_map(|catalog| catalog.relations.iter())
            .filter(|r| s.chosen.contains(&r.table()))
            .map(|r| (r.table(), r.geometry.is_some()))
            .collect();
        if !s.query.trim().is_empty() {
            tables.push((s.query.trim().to_owned(), true));
        }
        match self.bind_database(&c.address(), &tables, reproject, area) {
            Ok(done) => {
                self.close_exchange();
                self.zoom_to(&done.slots);
                // A warning is the toast's body: what is not drawn and why is not left to the log (docs/adr/0745 §3).
                let body = if done.warnings.is_empty() {
                    "Tabloların değişikliklerini Kaynağa yaz ile veritabanına yazın; view ve sorgu salt okunurdur.".to_owned()
                } else {
                    done.warnings.join(" ")
                };
                self.announce(
                    if done.warnings.is_empty() { Level::Success } else { Level::Warn },
                    format!(
                        "{} ({}) bağlı katman oldu, {} nesne. {body}",
                        done.names.join(", "),
                        c.name,
                        done.objects
                    ),
                );
            }
            Err(why) => {
                if let Some(Window::Postgis(s)) = &mut self.exchange {
                    s.status = Some((true, why));
                }
            }
        }
    }

    pub(super) fn postgis_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let project = self.document.as_ref().map_or(0, |d| d.settings().srid);
        let list = self.db_store.list();
        let mut body = Column::new().spacing(12);
        if list.is_empty() {
            body = body
                .push(label::body(
                    "Henüz bir veritabanı bağlantısı yok. Bağlantı; sunucu, veritabanı, kullanıcı ve paroladan oluşur, bir kez tanımlanır ve bu bilgisayarda saklanır.",
                ))
                .push(
                    row![
                        words::primary("Yeni bağlantı…", Some(event(Event::New))),
                        words::secondary(
                            "Bağlantı dosyasını içe aktar…",
                            Some(message_db(Db::Import)),
                        ),
                    ]
                    .spacing(8),
                );
            return overlay::blocking(
                Dialog::new("PostGIS'e bağlan")
                    .push(body)
                    .action(words::secondary("Kapat", Some(message(Exchange::Close))))
                    .width(560.0),
            );
        }
        let chosen = s
            .connection
            .as_deref()
            .and_then(|n| list.iter().position(|c| c.name == n));
        let connection = chosen.map(|i| list[i].clone()).unwrap_or_default();
        let picker = Select::new(
            list.iter()
                .map(|c| Choice::new(c.name.clone()).detail(c.summary())),
            chosen,
            |i| event(Event::Choose(i)),
        )
        .placeholder("Bir bağlantı seçin");
        let bar = row![
            iced::widget::container(picker).width(iced::Fill),
            words::secondary(
                if s.catalog.is_some() { "Yenile" } else { "Bağlan" },
                (chosen.is_some() && !s.listing).then(|| event(Event::List)),
            ),
            words::secondary("Yeni…", Some(event(Event::New))),
            words::secondary("Düzenle…", chosen.is_some().then(|| event(Event::Edit))),
        ]
        .spacing(6)
        .align_y(Center);
        body = body.push(words::field("Bağlantı", bar, chosen.map(|_| connection.summary())));
        if s.listing {
            body = body.push(words::text_line(Line::Info, "Bağlanılıyor, tablolar okunuyor…"));
        }
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Ok },
                text.clone(),
            ));
        }
        let mut other = false;
        if let Some(catalog) = &s.catalog {
            let search = InputField::new("Tablo ara (şema, ad ya da açıklama)", &s.filter)
                .icon(kentos_ui::icon::Icon::Search)
                .on_input(|t| event(Event::Filter(t)));
            let tools = row![
                iced::widget::container(search).width(iced::Fill),
                words::ghost("Görünenleri seç", Some(event(Event::All(true)))),
                words::ghost("Hiçbiri", Some(event(Event::All(false)))),
            ]
            .spacing(6)
            .align_y(Center);
            let mut tables = Column::new().spacing(4).push(tools);
            let mut any = false;
            for (schema, relations) in catalog.schemas(&connection) {
                let relations: Vec<&Relation> = relations.into_iter().filter(|r| shown(r, &s.filter)).collect();
                if relations.is_empty() {
                    continue;
                }
                any = true;
                tables = tables.push(label::strong(schema.to_owned()));
                for r in relations {
                    let why = r.why_not();
                    let mut words_line = format!("{} — {}", r.name, describe(r));
                    if why.is_some() {
                        words_line.push_str(" — bağlanamaz");
                    }
                    let check = words::check(
                        s.chosen.contains(&r.table()),
                        words_line,
                        why.is_none().then(|| event(Event::Toggle(r.table()))),
                    );
                    tables = tables.push(match why {
                        Some(w) => kentos_ui::widget::tip(
                            check,
                            kentos_ui::widget::Tip::new(r.table()).body(w),
                            iced::widget::tooltip::Position::Top,
                        ),
                        None => check,
                    });
                }
            }
            if !any {
                tables = tables.push(label::caption(if s.filter.trim().is_empty() {
                    "Listelenecek tablo yok."
                } else {
                    "Aramaya uyan tablo yok."
                }));
            }
            let hidden = catalog.hidden(&connection);
            let hint = (hidden > 0).then(|| {
                format!("{hidden} geometrisiz tablo gizli; göstermek için Düzenle… ile “Geometrisiz tabloları da listele”yi açın.")
            });
            body = body.push(words::field("Tablolar", tables, hint));
            other = catalog
                .relations
                .iter()
                .any(|r| s.chosen.contains(&r.table()) && r.srid.is_some_and(|srid| srid != project));
            body = body.push(words::field(
                "Alan",
                words::check(
                    s.area,
                    "Yalnız görünen alanın satırları".to_owned(),
                    Some(event(Event::Area)),
                ),
                Some("Milyon satırlı tabloda ilk harita bütün tabloyu indirmeden gelir: yalnız kutusu görünen alana değen satırlar alınır, yazma alanın dışına dokunmaz. Geometrisiz tablo bütün bağlanır.".to_owned()),
            ));
            if other {
                body = body.push(words::field(
                    "Koordinat sistemi",
                    words::check(
                        s.reproject,
                        format!("Projenin sistemine çevir (EPSG:{project}); tablo kendi sisteminde kalır"),
                        Some(event(Event::Reproject)),
                    ),
                    None,
                ));
            }
            let advanced = iced::widget::button(
                row![
                    kentos_ui::icon::icon(if s.advanced {
                        kentos_ui::icon::Icon::ChevronDown
                    } else {
                        kentos_ui::icon::Icon::ChevronRight
                    })
                    .size(12.0),
                    label::body("Gelişmiş: sorgu katmanı (SQL)")
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(event(Event::Advanced))
            .padding([2, 4])
            .style(kentos_ui::style::button::ghost);
            body = body.push(advanced);
            if s.advanced {
                let query = InputField::new("SELECT fid, ad, geom FROM şema.tablo WHERE …", &s.query)
                    .mono()
                    .on_input(|t| event(Event::Query(t)));
                body = body.push(words::field(
                    "Sorgu katmanı",
                    query,
                    Some("Tek bir SELECT; salt okunur bağlanır. Tam sayı bir anahtar sütunu (fid, id, gid ya da ogc_fid) gerekir; geometri ilk geometry sütunudur.".to_owned()),
                ));
            }
        }
        let count = s.chosen.len() + usize::from(!s.query.trim().is_empty());
        let ready = count > 0 && (!other || s.reproject) && !s.listing;
        let add = if count > 1 {
            format!("{count} kaynağı ekle")
        } else {
            "Ekle".to_owned()
        };
        overlay::blocking(
            Dialog::new("PostGIS'e bağlan")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(
                    iced::widget::button(label::body(add))
                        .on_press_maybe(ready.then(|| event(Event::Run)))
                        .padding([5, 16])
                        .style(kentos_ui::style::button::primary),
                )
                .width(680.0)
                .max_height(760.0),
        )
    }
}

fn message_db(e: Db) -> Message {
    crate::db_connections::msg(e)
}

/// A relation as the list says it: `çok parçalı alan · ≈200 satır · EPSG:5256`, `view · nokta`.
pub(crate) fn describe(r: &Relation) -> String {
    let geometry = match r.geometry.as_deref().map(str::to_ascii_uppercase).as_deref() {
        None => "geometrisiz".to_owned(),
        Some("POINT") => "nokta".to_owned(),
        Some("MULTIPOINT") => "çok noktalı".to_owned(),
        Some("LINESTRING") => "çizgi".to_owned(),
        Some("MULTILINESTRING") => "çok parçalı çizgi".to_owned(),
        Some("POLYGON") => "alan".to_owned(),
        Some("MULTIPOLYGON") => "çok parçalı alan".to_owned(),
        Some("GEOMETRY") => "karışık geometri".to_owned(),
        Some(other) => other.to_lowercase(),
    };
    let mut words = match r.kind {
        probe::RelationKind::Table | probe::RelationKind::Partitioned => geometry,
        kind => format!("{} · {geometry}", kind.label()),
    };
    if let Some(n) = r.estimate {
        words.push_str(&format!(" · ≈{} satır", grouped(n)));
    }
    if let Some(srid) = r.srid {
        words.push_str(&format!(" · EPSG:{srid}"));
    }
    if !r.writable && r.why_not().is_none() {
        words.push_str(" · salt okunur");
    }
    words
}

/// `12400` as `12.400`.
pub(crate) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

/// The connection a test or a picture names, kept in its app's store.
#[cfg(test)]
pub(crate) fn keep_connection(app: &App, c: &DbConnection) {
    let _ = app.db_store.remove(&c.name);
    app.db_store.put(None, c.clone()).expect("kept");
}
