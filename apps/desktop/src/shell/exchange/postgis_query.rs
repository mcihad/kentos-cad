//! PostGIS'e bağlan's query layer (docs/adr/9113 §3): an SQL `SELECT` read as a live, read-only data layer, not a copy.
//! Its columns are read first (Sütunları oku, off the interface's thread; the query is not run past its first row): the
//! key is chosen among its integer columns (`fid`, `id`, `gid`, `ogc_fid` offered first), the geometry among its
//! geometry columns or none, and the layer named (after its first table unless named). Ekle checks the key tells the
//! rows apart and adds the layer; its rows stay in the database and the view reads them as it draws.

use iced::Task;
use iced::widget::{Column, row};
use kentos_data::{QueryColumn, QueryTable};
use kentos_ui::widget::{Choice, InputField, Select};

use super::postgis::{Event, State};
use super::words::{self, Kind as Line};
use super::{Window, message};
use crate::app::{App, Message};

fn event(e: Event) -> Message {
    message(super::Event::Postgis(e))
}

/// The query's words a column list or a key needs: a column's name can go on the layer's line only without spaces.
const NO_KEY: &str = "Sorguda tam sayı bir sütun yok: anahtar, satırları ayıran tam sayı bir sütundur (fid, id, gid ya da bir sayaç). Sorguya ekleyin, sonra Sütunları oku'ya basın.";

/// A query longer than a layer keeps: its text is the layer's table, at most `MAX_DATA_TEXT` bytes.
fn too_long(q: &QueryTable) -> bool {
    q.text().len() > kentos_commands::MAX_DATA_TEXT
}

/// What the window says of a query longer than a layer keeps, and what to do instead.
pub(super) fn too_long_words() -> String {
    format!(
        "Sorgu {} karakterden uzun; katman bu kadar uzun bir sorguyu saklayamaz. Sorguyu veritabanında bir view yapın: view tablolarla birlikte listelenir ve canlı bağlanır.",
        kentos_commands::MAX_DATA_TEXT
    )
}

impl State {
    /// The query as written, its key and geometry as chosen, when its columns are read and a key is chosen.
    fn query_chosen(&self) -> Option<QueryTable> {
        let key = self.query_key.clone()?;
        matches!(self.query_columns, Some(Ok(_))).then(|| QueryTable {
            sql: self.query.trim().trim_end_matches(';').trim().to_owned(),
            key,
            geometry: self.query_geometry.clone(),
        })
    }

    /// The query as the layer will read it: chosen, and short enough for the layer to keep.
    pub(crate) fn query_table(&self) -> Option<QueryTable> {
        self.query_chosen().filter(|q| !too_long(q))
    }

    /// The SQL changed: the columns read before are another query's. The connection's line (Bağlandı …) stays: it is
    /// the connection's, and taking it away moved every line under it up one, the SQL's input with them, which then
    /// lost the keyboard after its first letter.
    pub(crate) fn query_changed(&mut self, sql: String) {
        self.query = sql;
        self.query_columns = None;
        self.query_key = None;
        self.query_geometry = None;
    }

    /// The columns read: the key and the geometry offered, the layer's name after its first table.
    pub(crate) fn query_read(&mut self, sql: &str, read: Result<Vec<QueryColumn>, String>) {
        self.query_reading = false;
        if sql != self.query.trim() {
            return;
        }
        if let Ok(cols) = &read {
            self.query_key = kentos_data::query::likely_key(cols).map(str::to_owned);
            self.query_geometry = cols.iter().find(|c| c.geometry).map(|c| c.name.clone());
            if self.query_name.trim().is_empty() {
                self.query_name = QueryTable {
                    sql: sql.to_owned(),
                    key: String::new(),
                    geometry: None,
                }
                .shown();
            }
        }
        self.query_columns = Some(read);
    }
}

impl App {
    /// Sütunları oku: the query's columns asked of the database off the interface's thread.
    pub(super) fn postgis_query_columns(&mut self) -> Task<Message> {
        let Some(Window::Postgis(s)) = &mut self.exchange else {
            return Task::none();
        };
        let Some(c) = s.connection.as_deref().and_then(|n| self.db_store.get(n)) else {
            return Task::none();
        };
        let sql = s.query.trim().trim_end_matches(';').trim().to_owned();
        if let Err(why) = kentos_data::query::check(&sql) {
            s.query_columns = Some(Err(why));
            return Task::none();
        }
        s.query_reading = true;
        let address = c.address();
        crate::db_connections::off_thread(
            {
                let sql = sql.clone();
                move || {
                    kentos_data::registry()
                        .source(&address)
                        .and_then(|source| source.query_columns(&sql))
                        .map_err(|e| e.to_string())
                }
            },
            move |read| event(Event::QueryColumns(sql.clone(), read)),
        )
    }
}

/// The query layer's part of the window: the SQL, Sütunları oku, then the key, the geometry and the name.
pub(super) fn view<'a>(s: &'a State, mut body: Column<'a, Message>) -> Column<'a, Message> {
    let sql = InputField::new("SELECT fid, ada, geom FROM şema.tablo WHERE …", &s.query)
        .mono()
        .on_input(|t| event(Event::Query(t)));
    body = body.push(words::field(
        "Sorgu katmanı",
        sql,
        Some("Tek bir SELECT ya da WITH. Katman sorguyu veritabanında canlı okur: satırlar indirilmez, salt okunurdur.".to_owned()),
    ));
    let reading = s.query_reading;
    let can_read = !s.query.trim().is_empty() && !reading;
    body = body.push(row![words::secondary(
        if reading {
            "Okunuyor…"
        } else {
            "Sütunları oku"
        },
        can_read.then(|| event(Event::QueryRead))
    ),]);
    match &s.query_columns {
        None => {}
        Some(Err(why)) => body = body.push(words::text_line(Line::Warn, why.clone())),
        Some(Ok(cols)) => {
            let keys: Vec<&QueryColumn> = cols
                .iter()
                .filter(|c| c.integer && QueryTable::plain(&c.name))
                .collect();
            if keys.is_empty() {
                body = body.push(words::text_line(Line::Warn, NO_KEY));
                return body;
            }
            let key_names: Vec<String> = keys.iter().map(|c| c.name.clone()).collect();
            let at = key_names
                .iter()
                .position(|k| Some(k) == s.query_key.as_ref());
            let key = Select::new(key_names.iter().cloned().map(Choice::new), at, |i| {
                event(Event::QueryKey(i))
            })
            .searchable(false);
            body = body.push(words::field(
                "Anahtar sütunu",
                key,
                Some("Her satırda başka olan tam sayı sütun; eklerken denetlenir.".to_owned()),
            ));
            let mut geometries: Vec<String> = cols
                .iter()
                .filter(|c| c.geometry && QueryTable::plain(&c.name))
                .map(|c| c.name.clone())
                .collect();
            geometries.push("Geometrisiz (tablo)".to_owned());
            let at = s
                .query_geometry
                .as_ref()
                .and_then(|g| geometries.iter().position(|n| n == g))
                .or(Some(geometries.len() - 1));
            let geometry = Select::new(geometries.into_iter().map(Choice::new), at, |i| {
                event(Event::QueryGeometry(i))
            })
            .searchable(false);
            body = body.push(words::field("Geometri sütunu", geometry, None));
            let name = InputField::new("Katmanın adı", &s.query_name)
                .on_input(|t| event(Event::QueryName(t)));
            body = body.push(words::field("Katmanın adı", name, None));
            if s.query_chosen().is_some_and(|q| too_long(&q)) {
                body = body.push(words::text_line(Line::Warn, too_long_words()));
            }
        }
    }
    body
}

/// What a choice of the lists means: the key by its place among the integer columns, the geometry among the geometry
/// columns (the last place: none).
pub(super) fn choose(s: &mut State, e: &Event) {
    let Some(Ok(cols)) = &s.query_columns else {
        return;
    };
    match e {
        Event::QueryKey(i) => {
            s.query_key = cols
                .iter()
                .filter(|c| c.integer && QueryTable::plain(&c.name))
                .nth(*i)
                .map(|c| c.name.clone());
        }
        Event::QueryGeometry(i) => {
            s.query_geometry = cols
                .iter()
                .filter(|c| c.geometry && QueryTable::plain(&c.name))
                .nth(*i)
                .map(|c| c.name.clone());
        }
        _ => {}
    }
}
