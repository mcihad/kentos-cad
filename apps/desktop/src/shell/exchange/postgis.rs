//! PostGIS'e bağlan (docs/adr/0505, 0582, 0623, 9101): a saved connection is chosen (Yeni and Düzenle open the
//! connection window, db_connections.rs), its tables, views and materialized views are read from the database's
//! catalogue off the interface's thread (`kentos_postgis::probe::browse`: no row is counted) and searched. The chosen
//! tables are added as data layers (`cad.layers.data`): their rows stay in the database and the view reads them as it
//! draws. A query layer is a data layer too, read live and read-only (postgis_query.rs, docs/adr/9113 §3). A server asking for a password KentOS does not know opens
//! the password question. Nobody types an address: the one a layer keeps is built from the connection's fields and
//! carries no password. The list is a tree of schemas, each opened and closed, with the connection's starred tables
//! first under Sık kullanılanlar (docs/adr/9172). A chosen table says its geometry column and its extent (asked of the
//! database off the interface's thread) and offers the tables a foreign key ties to it (docs/adr/9183).

use std::collections::BTreeSet;
use std::sync::Arc;

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, SourceTable, SourcesDescribe, SourcesDescribed};
use kentos_native_application::ExecutionContext;
use kentos_postgis::connections;
#[cfg(test)]
use kentos_postgis::connections::DbConnection;
use kentos_postgis::probe::{self, Catalog, Diagnosis, Problem, Relation};
use kentos_ui::label;
use kentos_ui::widget::{Choice, Dialog, InputField, Select, overlay};

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
    /// A query layer's SQL (docs/adr/9113 §3); empty: none.
    pub query: String,
    /// Its columns as last read, or why not; the key and the geometry chosen (none: without geometry); the layer's name.
    pub query_columns: Option<Result<Vec<kentos_data::QueryColumn>, String>>,
    pub query_reading: bool,
    pub query_key: Option<String>,
    pub query_geometry: Option<String>,
    pub query_name: String,
    /// Gelişmiş (the query layer) open.
    pub advanced: bool,
    /// Only the rows in the view's area (docs/adr/0582 §1).
    pub area: bool,
    pub status: Option<(bool, String)>,
    /// The schemas closed in the list (docs/adr/9172); a search shows every schema with a match open.
    pub closed: BTreeSet<String>,
    /// The chosen tables' extents as the database gave them (docs/adr/9183), by `schema.name`: none for a table without
    /// geometry or an empty one; why, when it could not.
    pub extents: std::collections::HashMap<String, Result<Option<[f64; 4]>, String>>,
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
    /// Sütunları oku.
    QueryRead,
    /// The query's columns (its SQL), or why not.
    QueryColumns(String, Result<Vec<kentos_data::QueryColumn>, String>),
    QueryKey(usize),
    QueryGeometry(usize),
    QueryName(String),
    Advanced,
    Area,
    Run,
    /// A schema of the list opened or closed (docs/adr/9172).
    Schema(String),
    /// A table starred as the connection's favourite, or its star taken away (docs/adr/9172).
    Favorite(String),
    /// A chosen table's extent as the database gave it, or why not (docs/adr/9183).
    Extent(String, Result<Option<[f64; 4]>, String>),
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
pub(crate) fn shown_name(table: &str) -> String {
    let first = table.split_whitespace().next().unwrap_or_default();
    if first.eq_ignore_ascii_case("select") || first.eq_ignore_ascii_case("with") {
        "Sorgu katmanı".to_owned()
    } else {
        table.to_owned()
    }
}

/// Whether a relation is shown for `filter` (its schema, name or comment, Turkish case aside).
/// A line of the catalogue's list (docs/adr/9172).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ListLine {
    /// Sık kullanılanlar's head: how many starred tables the search shows.
    Starred(usize),
    /// A starred table, `schema.table`.
    Favorite(String),
    /// A schema's head: its name, how many of its tables the search shows and whether it is open.
    Schema(String, usize, bool),
    /// A table of an open schema, `schema.table`.
    Table(String),
}

/// The list's lines (docs/adr/9172): the starred tables first under Sık kullanılanlar, in the order starred; then each
/// schema with its count and, when it is open, its tables. A search keeps what it matches and shows every schema with a
/// match open.
pub(crate) fn list_lines(
    catalog: &Catalog,
    connection: &connections::DbConnection,
    favorites: &[String],
    closed: &BTreeSet<String>,
    filter: &str,
) -> Vec<ListLine> {
    let schemas = catalog.schemas(connection);
    let mut out = Vec::new();
    let starred: Vec<String> = favorites
        .iter()
        .filter(|t| {
            schemas
                .iter()
                .flat_map(|(_, rs)| rs.iter())
                .any(|r| r.table() == **t && shown(r, filter))
        })
        .cloned()
        .collect();
    if !starred.is_empty() {
        out.push(ListLine::Starred(starred.len()));
        out.extend(starred.into_iter().map(ListLine::Favorite));
    }
    let searching = !filter.trim().is_empty();
    for (schema, relations) in &schemas {
        let kept: Vec<&&Relation> = relations.iter().filter(|r| shown(r, filter)).collect();
        if kept.is_empty() {
            continue;
        }
        let open = searching || !closed.contains(*schema);
        out.push(ListLine::Schema((*schema).to_owned(), kept.len(), open));
        if open {
            out.extend(kept.into_iter().map(|r| ListLine::Table(r.table())));
        }
    }
    out
}

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
            Event::Schema(schema) => {
                if !s.closed.remove(&schema) {
                    s.closed.insert(schema);
                }
                return Task::none();
            }
            Event::Favorite(table) => {
                if let Some(name) = s.connection.clone()
                    && let Err(why) = self.db_store.toggle_favorite(&name, &table)
                {
                    self.warn(why);
                }
                return Task::none();
            }
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
                        s.chosen
                            .retain(|t| catalog.relations.iter().any(|r| r.table() == *t));
                        // Listed again: the extents are asked again.
                        s.extents.clear();
                        s.status = Some((false, format!("Bağlandı: {}", catalog.facts.line())));
                        s.catalog = Some(catalog);
                    }
                    Err(d) => {
                        s.catalog = None;
                        s.chosen.clear();
                        let password =
                            matches!(d.problem, Problem::PasswordNeeded | Problem::Password);
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
                    s.chosen.insert(t.clone());
                    return self.postgis_extent(t);
                }
            }
            Event::Extent(t, extent) => {
                s.extents.insert(t, extent);
            }
            Event::All(all) => {
                let Some(catalog) = &s.catalog else {
                    return Task::none();
                };
                let c = s
                    .connection
                    .as_deref()
                    .and_then(|n| self.db_store.get(n))
                    .unwrap_or_default();
                for (_, relations) in catalog.schemas(&c) {
                    for r in relations
                        .into_iter()
                        .filter(|r| shown(r, &s.filter) && r.why_not().is_none())
                    {
                        if all {
                            s.chosen.insert(r.table());
                        } else {
                            s.chosen.remove(&r.table());
                        }
                    }
                }
            }
            Event::Reproject => s.reproject = !s.reproject,
            Event::Query(q) => s.query_changed(q),
            Event::QueryRead => return self.postgis_query_columns(),
            Event::QueryColumns(sql, read) => s.query_read(&sql, read),
            Event::QueryKey(_) | Event::QueryGeometry(_) => super::postgis_query::choose(s, &e),
            Event::QueryName(n) => s.query_name = n,
            Event::Advanced => s.advanced = !s.advanced,
            Event::Area => s.area = !s.area,
            Event::Run => return self.postgis_bind(),
        }
        Task::none()
    }

    /// Bağla: the chosen tables, views and materialized views added as data layers (docs/adr/9101): their rows stay in
    /// the database and the view reads them as it draws; the window closes at once and the catalogue is read on a thread
    /// of its own. A query is a data layer too, read live and read-only (docs/adr/9113 §3).
    /// A chosen table's extent asked of the database off the interface's thread (docs/adr/9183): the catalogue's estimate,
    /// measured when there is none; nothing for a table without geometry or one already asked.
    fn postgis_extent(&mut self, table: String) -> Task<Message> {
        let Some(Window::Postgis(s)) = &self.exchange else {
            return Task::none();
        };
        let spatial = s
            .catalog
            .as_ref()
            .and_then(|c| c.relations.iter().find(|r| r.table() == table))
            .is_some_and(|r| r.geometry.is_some());
        if !spatial || s.extents.contains_key(&table) {
            return Task::none();
        }
        let Some(address) = s
            .connection
            .as_deref()
            .and_then(|n| self.db_store.get(n))
            .map(|c| c.address())
        else {
            return Task::none();
        };
        let asked = table.clone();
        crate::db_connections::off_thread(
            move || {
                kentos_data::registry()
                    .source(&address)
                    .and_then(|source| source.extent(&asked, true))
                    .map_err(|e| e.to_string())
            },
            move |extent| event(Event::Extent(table, extent)),
        )
    }

    fn postgis_bind(&mut self) -> Task<Message> {
        let Some(Window::Postgis(s)) = &self.exchange else {
            return Task::none();
        };
        let Some(c) = s.connection.as_deref().and_then(|n| self.db_store.get(n)) else {
            return Task::none();
        };
        let tables: Vec<String> = s
            .catalog
            .iter()
            .flat_map(|catalog| catalog.relations.iter())
            .filter(|r| s.chosen.contains(&r.table()))
            .map(|r| r.table())
            .collect();
        let query = s.query_table().map(|q| (q, s.query_name.clone()));
        let address = c.address();
        self.close_exchange();
        let mut tasks = vec![self.add_tables(address.clone(), tables)];
        // The query a live data layer too (docs/adr/9113 §3).
        if let Some((q, name)) = query {
            tasks.push(self.add_query(address, q, name));
        }
        Task::batch(tasks)
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
                    .on_close(message(Exchange::Close))
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
                if s.catalog.is_some() {
                    "Yenile"
                } else {
                    "Bağlan"
                },
                (chosen.is_some() && !s.listing).then(|| event(Event::List)),
            ),
            words::secondary("Yeni…", Some(event(Event::New))),
            words::secondary("Düzenle…", chosen.is_some().then(|| event(Event::Edit))),
        ]
        .spacing(6)
        .align_y(Center);
        body = body.push(words::field(
            "Bağlantı",
            bar,
            chosen.map(|_| connection.summary()),
        ));
        if s.listing {
            body = body.push(words::text_line(
                Line::Info,
                "Bağlanılıyor, tablolar okunuyor…",
            ));
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
            // The list scrolls in a height of its own: the window and its search stay where they are as the search
            // narrows it (docs/adr/9140).
            let mut tables = Column::new().spacing(4);
            let mut any = false;
            // The list's lines (docs/adr/9172): the starred tables first, then the tree of schemas.
            let favorites = s
                .connection
                .as_ref()
                .map(|c| self.db_store.favorites(c))
                .unwrap_or_default();
            let lines = list_lines(catalog, &connection, &favorites, &s.closed, &s.filter);
            let by_table: std::collections::HashMap<String, &Relation> = catalog
                .schemas(&connection)
                .into_iter()
                .flat_map(|(_, rs)| rs)
                .map(|r| (r.table(), r))
                .collect();
            let line = |r: &Relation, prefix: bool| -> Element<'a, Message> {
                let why = r.why_not();
                let name = if prefix { r.table() } else { r.name.clone() };
                let mut words_line = format!("{name} — {}", describe(r));
                if why.is_some() {
                    words_line.push_str(" — bağlanamaz");
                }
                let check = words::check(
                    s.chosen.contains(&r.table()),
                    words_line,
                    why.is_none().then(|| event(Event::Toggle(r.table()))),
                );
                let check: Element<'a, Message> = match why {
                    Some(w) => kentos_ui::widget::tip(
                        check,
                        kentos_ui::widget::Tip::new(r.table()).body(w),
                        iced::widget::tooltip::Position::Top,
                    ),
                    None => check,
                };
                let on = favorites.contains(&r.table());
                let star = kentos_ui::widget::tip(
                    iced::widget::button(
                        kentos_ui::icon::icon(crate::icons::from_web(Some(if on {
                            "starOn"
                        } else {
                            "star"
                        })))
                        .size(13.0)
                        .tone(if on {
                            kentos_ui::icon::Tone::Accent
                        } else {
                            kentos_ui::icon::Tone::Muted
                        }),
                    )
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(kentos_ui::style::button::ghost)
                    .on_press(event(Event::Favorite(r.table()))),
                    kentos_ui::widget::Tip::new(if on {
                        "Sık kullanılanlardan çıkar"
                    } else {
                        "Sık kullanılanlara ekle"
                    }),
                    iced::widget::tooltip::Position::Top,
                );
                row![star, check].spacing(4).align_y(Center).into()
            };
            for l in &lines {
                any = true;
                let element: Element<'a, Message> = match l {
                    ListLine::Starred(n) => {
                        label::strong(format!("Sık kullanılanlar ({n})")).into()
                    }
                    ListLine::Favorite(t) | ListLine::Table(t) => match by_table.get(t) {
                        Some(r) => line(r, matches!(l, ListLine::Favorite(_))),
                        None => continue,
                    },
                    ListLine::Schema(schema, n, open) => iced::widget::button(
                        row![
                            kentos_ui::icon::icon(if *open {
                                kentos_ui::icon::Icon::ChevronDown
                            } else {
                                kentos_ui::icon::Icon::ChevronRight
                            })
                            .size(12.0),
                            label::strong(schema.clone()),
                            label::caption(format!("({n})")),
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .padding([1, 2])
                    .style(kentos_ui::style::button::ghost)
                    .on_press(event(Event::Schema(schema.clone())))
                    .into(),
                };
                tables = tables.push(element);
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
            let list = iced::widget::container(
                iced::widget::scrollable(
                    iced::widget::container(tables)
                        .padding([4, 8])
                        .width(iced::Fill),
                )
                .width(iced::Fill)
                .height(kentos_ui::theme::typography::scaled(LIST_HEIGHT)),
            )
            .style(kentos_ui::style::container::field_box)
            .width(iced::Fill);
            body = body.push(words::field(
                "Tablolar",
                iced::widget::column![tools, list].spacing(6),
                hint,
            ));
            // Each chosen table as it will be bound: where it is, its key, its geometry and system, whether it may be
            // written.
            let chosen: Vec<&Relation> = catalog
                .relations
                .iter()
                .filter(|r| s.chosen.contains(&r.table()))
                .collect();
            if !chosen.is_empty() {
                // Each with its extent (docs/adr/9183) and the tables a foreign key ties to it, one press to choose.
                let mut lines: Vec<Element<'a, Message>> = Vec::new();
                for r in chosen {
                    let extent = extent_words(r, s.extents.get(&r.table()));
                    let text = match extent {
                        Some(e) => format!("{} · {e}", identity(r)),
                        None => identity(r),
                    };
                    lines.push(words::text_line(Line::Info, text));
                    let ties: Vec<&String> = r
                        .related
                        .iter()
                        .filter(|t| catalog.relations.iter().any(|x| x.table() == **t))
                        .collect();
                    if !ties.is_empty() {
                        let mut tie_row = row![label::caption(RELATED)].spacing(6).align_y(Center);
                        for t in ties {
                            let on = s.chosen.contains(t);
                            tie_row = tie_row.push(words::ghost(
                                t.clone(),
                                (!on).then(|| event(Event::Toggle(t.clone()))),
                            ));
                        }
                        lines.push(tie_row.wrap().into());
                    }
                }
                body = body.push(words::field(
                    "Seçilen tablolar",
                    words::summary(lines),
                    None,
                ));
            }
            other = catalog.relations.iter().any(|r| {
                s.chosen.contains(&r.table()) && r.srid.is_some_and(|srid| srid != project)
            });
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
                body = super::postgis_query::view(s, body);
            }
        }
        let count = s.chosen.len() + usize::from(s.query_table().is_some());
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
                        .padding(kentos_ui::theme::metrics::action())
                        .style(kentos_ui::style::button::primary),
                )
                .width(680.0)
                .max_height(760.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}

fn message_db(e: Db) -> Message {
    crate::db_connections::msg(e)
}

/// A relation as the list says it: `çok parçalı alan · ≈200 satır · EPSG:5256`, `view · nokta`.
/// The table list's height (logical pixels at the default type size).
const LIST_HEIGHT: f32 = 280.0;

/// A chosen table as it will be bound (docs/adr/9140): its schema and name, the column its rows are keyed by, its
/// geometry and system, and whether this connection may write it.
pub(crate) fn identity(r: &Relation) -> String {
    let key = r
        .key
        .as_deref()
        .map_or_else(|| "anahtar yok".to_owned(), |k| format!("anahtar {k}"));
    let right = if r.writable {
        "yazılabilir"
    } else {
        "salt okunur (yazma yetkisi yok ya da görünüm)"
    };
    // The geometry column the layer draws (docs/adr/9183): the first of several.
    let column = match (&r.geometry_column, r.geometry_columns) {
        (Some(c), n) if n > 1 => format!(" · geometri sütunu {c} ({n} sütundan ilki)"),
        (Some(c), _) => format!(" · geometri sütunu {c}"),
        (None, _) => String::new(),
    };
    format!("{}{column} · {key} · {} · {right}", r.table(), describe(r))
}

/// The words before a chosen table's related tables (docs/adr/9183).
const RELATED: &str = "İlişkili tablolar:";

/// A chosen table's extent as the database gave it (docs/adr/9183), in the table's own system, east (Y) before north
/// (X); while it is asked, so said; none for a table without geometry.
pub(crate) fn extent_words(
    r: &Relation,
    extent: Option<&Result<Option<[f64; 4]>, String>>,
) -> Option<String> {
    r.geometry.as_ref()?;
    Some(match extent {
        None => "kapsam okunuyor…".to_owned(),
        Some(Ok(None)) => "kapsam yok (tabloda geometri yok)".to_owned(),
        Some(Err(why)) => format!("kapsam okunamadı: {why}"),
        Some(Ok(Some([a, b, c, d]))) => {
            // Degrees for a geographic system's values, else metres (shown only, the extent is not used further).
            let n = if [a, b, c, d].iter().all(|v| v.abs() <= 360.0) {
                6
            } else {
                2
            };
            format!("kapsam Y {a:.n$}–{c:.n$}, X {b:.n$}–{d:.n$}")
        }
    })
}

pub(crate) fn describe(r: &Relation) -> String {
    let geometry = match r
        .geometry
        .as_deref()
        .map(str::to_ascii_uppercase)
        .as_deref()
    {
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
