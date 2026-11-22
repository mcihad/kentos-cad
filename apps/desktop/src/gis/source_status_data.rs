//! Kaynak durumu's data layers (docs/adr/9129): the project's data layers (docs/adr/9101) beside its bound layers, each
//! with its physical source (a file › table, a database › schema.table), whether it may be written, its edit session
//! and the rows waiting there, and its table's real row count: asked of the source off the interface's thread when the
//! window opens or is refreshed, “hesaplanmadı” until the source answers. The bound layers' list never says the
//! project has no source while data layers draw from one.

use std::collections::BTreeMap;

use iced::widget::{column, container, row};
use iced::{Element, Fill, Task};
use kentos_ui::theme::typography;
use kentos_ui::widget::Elided;

use crate::app::{App, Message};
use crate::exchange::words;

/// A data layer's row: its id, name, source, access and session in words.
pub(crate) struct DataRow {
    pub layer: String,
    pub name: String,
    pub place: String,
    pub access: &'static str,
    pub session: String,
    /// Its rows are in a file (not a database): Yeniden bağla… points it at a new place.
    pub file: bool,
}

/// The data layer's way to a moved file (docs/adr/9155), and its column's width.
const RELINK: &str = "Yeniden bağla…";
const ACT: f32 = 130.0;

/// A table's row count as the source answered it, by layer; a layer not there yet is “hesaplanmadı”.
pub(crate) type Counts = BTreeMap<String, Result<u64, String>>;

impl App {
    /// The project's data layers, in the tree's order.
    pub(crate) fn source_data_rows(&self) -> Vec<DataRow> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        model
            .layers()
            .leaves()
            .iter()
            .filter_map(|n| {
                let data = n.data.as_ref()?;
                let session = if model.editing(&n.id) {
                    let c = model.data_changes(&n.id);
                    if c.is_empty() {
                        "düzenleniyor".to_owned()
                    } else {
                        format!(
                            "düzenleniyor; kaynağa yazılmadı: {}",
                            crate::data_edits::counts_words(c.added.len(), c.changed.len(), c.deleted.len())
                        )
                    }
                } else {
                    "kapalı".to_owned()
                };
                Some(DataRow {
                    layer: n.id.clone(),
                    name: n.name.clone(),
                    place: data.place_words(),
                    access: if data.read_only { "salt okunur" } else { "yazılabilir" },
                    session,
                    file: data.driver != "postgis",
                })
            })
            .collect()
    }

    /// Every data layer's table counted by its source, off the interface's thread (`Counted` says each answer).
    pub(crate) fn count_data_layers(&self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let tasks: Vec<Task<Message>> = doc
            .model
            .layers()
            .leaves()
            .iter()
            .filter_map(|n| Some((n.id.clone(), n.data.clone()?)))
            .map(|(layer, data)| {
                crate::db_connections::off_thread(
                    move || {
                        kentos_data::registry()
                            .source(&data.address)
                            .and_then(|s| s.count(&data.table, &kentos_data::Request::all(), false))
                            .map_err(|e| e.to_string())
                    },
                    move |counted| super::msg(super::Event::Counted(layer, counted)),
                )
            })
            .collect();
        Task::batch(tasks)
    }

    /// The data layers' field of the window; none when the project has none.
    pub(crate) fn source_data_view(&self, counts: &Counts) -> Option<Element<'_, Message>> {
        let rows = self.source_data_rows();
        if rows.is_empty() {
            return None;
        }
        let cell = |t: String| Elided::new(t).size(typography::caption()).font(typography::ui()).width(Fill);
        let head = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::widget::table::title("Katman")).width(typography::scaled(120.0)),
                // The cells say “file › table” or “database › schema.table”.
                container(kentos_ui::widget::table::title("Kaynak")).width(Fill),
                container(kentos_ui::widget::table::title("Erişim")).width(typography::scaled(90.0)),
                container(kentos_ui::widget::table::title("Düzenleme")).width(typography::scaled(200.0)),
                container(kentos_ui::widget::table::title("Satır")).width(typography::scaled(100.0)),
                // The way out's column: Yeniden bağla… on a file not read (docs/adr/9155).
                container(iced::widget::text("")).width(typography::scaled(ACT)),
            ]
            .spacing(8)
            .padding([0, 8]),
        );
        let lines = rows.into_iter().fold(column![head], |c, r| {
            let unread = matches!(counts.get(&r.layer), Some(Err(_)));
            let count = match counts.get(&r.layer) {
                Some(Ok(n)) => crate::view::thousands(*n as f64),
                Some(Err(_)) => "okunamadı".to_owned(),
                None => "hesaplanmadı".to_owned(),
            };
            let line = row![
                container(cell(r.name)).width(typography::scaled(120.0)),
                cell(r.place),
                container(cell(r.access.to_owned())).width(typography::scaled(90.0)),
                container(cell(r.session)).width(typography::scaled(200.0)),
                container(cell(count)).width(typography::scaled(100.0)),
            ]
            .spacing(8)
            .padding([4, 8]);
            // A file not read: pointed at its new place, every layer of the file with it (docs/adr/9155).
            let act: Element<'_, Message> = if unread && r.file {
                words::secondary(RELINK, Some(super::msg(super::Event::RelinkData(r.layer.clone())))).into()
            } else {
                iced::widget::text("").into()
            };
            c.push(line.push(container(act).width(typography::scaled(ACT))))
        });
        let failed: Vec<String> = counts.values().filter_map(|r| r.as_ref().err().cloned()).collect();
        let hint = if failed.is_empty() {
            "Satır sayısı tablonun kaynaktaki gerçek sayısıdır; kaynak yanıtlayana kadar “hesaplanmadı” yazar. Haritada ya da \
             tabloda o an yüklenen satırlar bu sayı değildir."
                .to_owned()
        } else {
            format!("Sayılamayan: {}", failed.join("; "))
        };
        Some(words::field(
            "Veri katmanları",
            container(lines).style(kentos_ui::style::container::field_box).width(Fill),
            Some(hint),
        ))
    }
}
