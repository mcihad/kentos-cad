//! Veri katmanı on the desktop (docs/adr/9101, plan/veri.md VY-02): the data drivers registered at start with the
//! device's passwords, and a table added as a data layer: its catalogue and shape read (no row), its columns made the
//! layer's fields, `cad.layers.data` run as one undo step. The rows stay in the source; the view reads them as it draws
//! (VY-03).

use kentos_commands::{CommandResult, LayerDataOperation, LayersData};
use kentos_native_application::{ExecutionContext, layers_data};

// A table described for a data layer: the native layer's, which opening a GeoPackage shares (docs/adr/9116).
pub use kentos_native_application::data_describe::{Described, describe};

use iced::Task;

use crate::app::{App, Message};

/// What the data layers' work says back.
#[derive(Debug, Clone)]
pub enum Event {
    /// Tables described for data layers (each one, or why not), then added to the drawing.
    Described(Vec<Result<Described, String>>),
}

fn message(event: Event) -> Message {
    Message::Gis(crate::gis::Event::DataLayers(event))
}

/// The device's passwords for the PostGIS driver: the saved connections' and the session's (docs/adr/0623).
struct Device;

impl kentos_data::Secrets for Device {
    fn password(&self, host: &str, port: u16, database: &str, user: &str) -> Option<String> {
        kentos_postgis::connections::password_for(&kentos_postgis::connections::Key {
            host: host.to_ascii_lowercase(),
            port,
            database: database.to_owned(),
            user: user.to_owned(),
        })
    }
}

/// Registers the data drivers and the device's password finder (once, at start).
pub fn register() {
    kentos_data_gpkg::register();
    kentos_data_postgis::register();
    kentos_data::set_secrets(Box::new(Device));
}

/// The most rows a query's key is checked to be unique over when it is added (more are taken as they are).
pub const KEY_CHECKED: u64 = 1_000_000;

/// A query read as a data layer (docs/adr/9113 §3): its shape as a table's, read-only, named `name` (none: after its
/// first table). Its key is checked to tell its rows apart (counted at the source, up to [`KEY_CHECKED`] rows): a key
/// two rows share would pick, label and show the wrong row.
pub fn describe_query(address: &str, q: &kentos_data::QueryTable, name: Option<&str>) -> Result<Described, String> {
    let text = q.text();
    let source = kentos_data::registry().source(address).map_err(|e| e.to_string())?;
    let mut described = describe(address, &text)?;
    let rows = source.count(&text, &kentos_data::Request::all(), false).map_err(|e| e.to_string())?;
    if rows <= KEY_CHECKED {
        let distinct = source.unique(&text, &q.key, None, rows as usize + 1).map_err(|e| e.to_string())?;
        if (distinct.len() as u64) < rows {
            return Err(format!(
                "“{}” sütunu benzersiz değil: sorgunun {rows} satırında {} farklı değeri var. Anahtar olarak her satırda başka olan bir tam sayı sütunu seçin.",
                q.key,
                distinct.len()
            ));
        }
    }
    described.name = name.map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| q.shown(), str::to_owned);
    described.data.read_only = true;
    Ok(described)
}

/// A data layer's box in the project's system: its table's box as last read, moved when the table is in another
/// system; none without one.
pub fn extent_in_project(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
) -> Option<[f64; 4]> {
    let data = node.data.as_ref()?;
    let b = data.extent?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    kentos_native_application::sources::extent_in_project(settings, srid, b).ok()
}

impl App {
    /// Adds tables of a source as data layers (Katman olarak ekle, Bağla): their shapes read off the interface's thread
    /// (a database's catalogue may be slow to answer), then each added as one undo step.
    pub(crate) fn add_tables(&mut self, address: String, tables: Vec<String>) -> Task<Message> {
        if tables.is_empty() {
            return Task::none();
        }
        self.say(
            kentos_interaction::Level::Info,
            format!("{} tablo okunuyor…", tables.len()),
        );
        crate::db_connections::off_thread(
            move || {
                tables
                    .iter()
                    .map(|t| describe(&address, t))
                    .collect::<Vec<_>>()
            },
            |r| message(Event::Described(r)),
        )
    }

    /// A query added as a data layer (docs/adr/9113 §3), read and checked off the interface's thread.
    pub(crate) fn add_query(&mut self, address: String, q: kentos_data::QueryTable, name: String) -> Task<Message> {
        self.say(kentos_interaction::Level::Info, "Sorgu okunuyor…");
        crate::db_connections::off_thread(
            move || vec![describe_query(&address, &q, Some(&name))],
            |r| message(Event::Described(r)),
        )
    }

    pub(crate) fn data_layers_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Described(read) => {
                let mut ok = Vec::new();
                for r in read {
                    match r {
                        Ok(d) => ok.push(d),
                        Err(why) => self.warn(why),
                    }
                }
                let left: Vec<String> = ok.iter().flat_map(|d| d.left.clone()).collect();
                let names: Vec<String> = ok.iter().map(|d| d.name.clone()).collect();
                let first = ok.first().cloned();
                match self.add_data_layers(ok) {
                    Ok(added) if !added.is_empty() => {
                        self.say(
                            kentos_interaction::Level::Success,
                            format!(
                                "{} veri katmanı eklendi: {}. Satırlar kaynağında kalır; görünüm gerektikçe okunur.",
                                added.len(),
                                names.join(", ")
                            ),
                        );
                        if let Some(b) = first.and_then(|d| d.extent) {
                            self.zoom_to_data_extent(b);
                        }
                    }
                    Ok(_) => {}
                    Err(why) => self.warn(why),
                }
                for l in left {
                    self.warn(format!("Alan olarak alınmadı: {l}"));
                }
            }
        }
        Task::none()
    }

    /// Frames a table's catalogue box (in the table's system; the project's when they are the same).
    fn zoom_to_data_extent(&mut self, b: [f64; 4]) {
        if !(b.iter().all(|v| v.is_finite()) && b[2] >= b[0] && b[3] >= b[1]) {
            return;
        }
        self.navigating(|app| {
            app.zoom_to_bounds(&kentos_commands::Bounds {
                min_x: b[0],
                min_y: b[1],
                max_x: b[2],
                max_y: b[3],
            })
        });
    }

    /// Adds described tables as data layers, each one undo step; their names in a sentence, or why one could not be.
    pub(crate) fn add_data_layers(&mut self, tables: Vec<Described>) -> Result<Vec<String>, String> {
        let Some(doc) = &mut self.document else {
            return Err("Açık çizim yok. Önce bir çizim açın ya da yeni bir proje başlatın.".into());
        };
        let mut added = Vec::new();
        for t in tables {
            let input = LayersData {
                operation: LayerDataOperation::Add,
                layer: None,
                name: Some(t.name.clone()),
                parent: None,
                index: Some(0),
                data: Some(t.data.clone()),
                fields: (!t.fields.is_empty()).then(|| t.fields.clone()),
                expected_revision: None,
            };
            match layers_data::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => added.push(output.layer),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    return Err(format!("“{}” eklenemedi: {}", t.name, error.message));
                }
                _ => return Err(format!("“{}” eklenemedi.", t.name)),
            }
        }
        Ok(added)
    }
}
