//! Kaynak durumu's catalog (docs/adr/9192): every layer whose objects or rows come from a file or a database, under its
//! physical source, and how it holds them. A data layer reads its rows from the source as it is now; a bound layer keeps
//! the objects in the project and writes them back to the table; a copy an import took keeps them in the project, the
//! source neither bound nor written. A file source says whether it is where the layers say; a copy says whether its
//! file changed since it was taken (its SHA-256 then, and now as read off the interface's thread).

use std::collections::BTreeMap;
use std::path::Path;

use iced::widget::{Column, container, row};
use iced::{Element, Fill, Task};
use kentos_ui::theme::typography;
use kentos_ui::widget::Elided;

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};

/// How a layer holds what its source has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Way {
    Data,
    Bound,
    Copy,
}

impl Way {
    pub(crate) fn words(self) -> &'static str {
        match self {
            Way::Data => "Veri katmanı",
            Way::Bound => "Bağlı katman",
            Way::Copy => "Alınmış kopya",
        }
    }
}

/// A layer under its source: its id and name, the way, its table; a copy's time taken and the file's SHA-256 then.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub layer: String,
    pub name: String,
    pub way: Way,
    pub table: Option<String>,
    pub taken: Option<(String, Option<String>)>,
}

/// A physical source and the layers that come from it.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub address: String,
    /// A file (not a database).
    pub file: bool,
    pub entries: Vec<Entry>,
}

/// A file's SHA-256 now, read off the interface's thread, by its address; a file not there yet is “hesaplanıyor”.
pub(crate) type Digests = BTreeMap<String, Result<String, String>>;

/// A database's address (a URI or `key=value` words) as against a file's path.
fn is_database(address: &str) -> bool {
    address.contains("://") || address.contains("dbname=") || address.starts_with("PG:")
}

/// `2026-11-23T09:30:00Z` as the interface writes a moment: 23.11.2026 09:30 (UTC).
fn taken_words(at: &str) -> String {
    match (at.get(0..4), at.get(5..7), at.get(8..10), at.get(11..16)) {
        (Some(y), Some(m), Some(d), Some(hm)) => format!("{d}.{m}.{y} {hm} (UTC)"),
        _ => at.to_owned(),
    }
}

impl App {
    /// The project's physical sources with their layers, in the order the tree first names them: its data layers, its
    /// bound layers and the copies its imports took.
    pub(crate) fn source_catalog(&self) -> Vec<Source> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        let links: BTreeMap<&str, &kentos_commands::SourceLink> = model
            .settings()
            .sources
            .iter()
            .map(|l| (l.layer.as_str(), l))
            .collect();
        let mut out: Vec<Source> = Vec::new();
        let mut put =
            |address: &str, entry: Entry| match out.iter_mut().find(|s| s.address == address) {
                Some(s) => s.entries.push(entry),
                None => out.push(Source {
                    address: address.to_owned(),
                    file: !is_database(address),
                    entries: vec![entry],
                }),
            };
        for n in model.layers().leaves() {
            if let Some(d) = &n.data {
                put(
                    &d.address,
                    Entry {
                        layer: n.id.clone(),
                        name: n.name.clone(),
                        way: Way::Data,
                        table: Some(d.table.clone()),
                        taken: None,
                    },
                );
            }
            if let Some(l) = links.get(n.id.as_str()) {
                put(
                    &l.path,
                    Entry {
                        layer: n.id.clone(),
                        name: n.name.clone(),
                        way: Way::Bound,
                        table: Some(l.table.clone()),
                        taken: None,
                    },
                );
            }
            if let Some(o) = &n.origin {
                put(
                    &o.address,
                    Entry {
                        layer: n.id.clone(),
                        name: n.name.clone(),
                        way: Way::Copy,
                        table: o.table.clone(),
                        taken: Some((o.at.clone(), o.sha256.clone())),
                    },
                );
            }
        }
        out
    }

    /// The copies' files read again off the interface's thread, each once (`Hashed` says each answer).
    pub(crate) fn hash_copy_sources(&self) -> Task<Message> {
        let files: Vec<String> = self
            .source_catalog()
            .into_iter()
            .filter(|s| s.file && s.entries.iter().any(|e| e.way == Way::Copy))
            .map(|s| s.address)
            .collect();
        Task::batch(files.into_iter().map(|address| {
            let path = address.clone();
            crate::db_connections::off_thread(
                move || {
                    std::fs::read(&path)
                        .map(|bytes| kentos_commands::encoding::sha256_hex(&bytes))
                        .map_err(|e| e.to_string())
                },
                move |hashed| super::msg(super::Event::Hashed(address.clone(), hashed)),
            )
        }))
    }

    /// What a copy says of its file now: not there, the same as when it was taken, changed since, or not read yet.
    fn copy_state(source: &Source, entry: &Entry, digests: &Digests) -> (Kind, String) {
        let Some((at, then)) = &entry.taken else {
            return (Kind::Info, String::new());
        };
        let taken = format!("{} alındı", taken_words(at));
        if !source.file {
            return (Kind::Info, taken);
        }
        if !Path::new(&source.address).exists() {
            return (Kind::Warn, format!("{taken}; dosya yerinde yok"));
        }
        match (digests.get(&source.address), then) {
            (None, _) => (Kind::Info, format!("{taken}; dosya okunuyor…")),
            (Some(Err(why)), _) => (Kind::Warn, format!("{taken}; dosya okunamadı: {why}")),
            (Some(Ok(now)), Some(then)) if now == then => {
                (Kind::Ok, format!("{taken}; dosya değişmedi"))
            }
            (Some(Ok(_)), Some(_)) => (Kind::Warn, format!("{taken}; dosya sonra değişti")),
            (Some(Ok(_)), None) => (Kind::Info, taken),
        }
    }

    /// The catalog's field of the window; none when no layer comes from a source.
    pub(crate) fn source_catalog_view(&self, digests: &Digests) -> Option<Element<'_, Message>> {
        let sources = self.source_catalog();
        if sources.is_empty() {
            return None;
        }
        let cell = |t: String| {
            Elided::new(t)
                .size(typography::caption())
                .font(typography::ui())
                .width(Fill)
        };
        let head = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::widget::table::title("Katman"))
                    .width(typography::scaled(150.0)),
                container(kentos_ui::widget::table::title("Nasıl"))
                    .width(typography::scaled(120.0)),
                container(kentos_ui::widget::table::title("Tablo"))
                    .width(typography::scaled(120.0)),
                container(kentos_ui::widget::table::title("Durum")).width(Fill),
            ]
            .spacing(8)
            .padding([0, 8]),
        );
        let mut lines = Column::new().push(head);
        for s in &sources {
            // The source itself: its file (name and folder) or database, and whether it is where the layers say.
            let place = if s.file {
                let p = Path::new(&s.address);
                let name = p
                    .file_name()
                    .map_or_else(|| s.address.clone(), |n| n.to_string_lossy().into_owned());
                let folder = p
                    .parent()
                    .map(|f| f.display().to_string())
                    .unwrap_or_default();
                if p.exists() {
                    format!("{name} — {folder}")
                } else {
                    format!("{name} — {folder} (dosya yerinde yok)")
                }
            } else {
                format!("{} (veritabanı)", s.address)
            };
            lines = lines.push(
                container(words::text_line(
                    if s.file && !Path::new(&s.address).exists() {
                        Kind::Warn
                    } else {
                        Kind::Info
                    },
                    place,
                ))
                .padding([4, 8]),
            );
            for e in &s.entries {
                let (_, state) = match e.way {
                    Way::Copy => Self::copy_state(s, e, digests),
                    Way::Data => (
                        Kind::Info,
                        "satırları kaynakta; sayısı aşağıda, Veri katmanları'nda".to_owned(),
                    ),
                    Way::Bound => (
                        Kind::Info,
                        "kaynağa yazılır; durumu aşağıda, Bağlı katmanlar'da".to_owned(),
                    ),
                };
                let face = row![
                    container(cell(e.name.clone())).width(typography::scaled(150.0)),
                    container(cell(e.way.words().to_owned())).width(typography::scaled(120.0)),
                    container(cell(e.table.clone().unwrap_or_else(|| "—".to_owned())))
                        .width(typography::scaled(120.0)),
                    cell(state),
                ]
                .spacing(8)
                .padding([2, 8])
                .height(typography::scaled(26.0))
                .align_y(iced::Center);
                lines = lines.push(
                    iced::widget::button(face)
                        .on_press(super::msg(super::Event::PickLayer(e.layer.clone())))
                        .padding(0)
                        .width(Fill)
                        .style(kentos_ui::style::button::row(
                            self.selected_layer.as_deref() == Some(e.layer.as_str()),
                        )),
                );
            }
        }
        Some(words::field(
            "Kaynaklar",
            container(lines).style(kentos_ui::style::container::field_box).width(Fill),
            Some(
                "Her kaynak dosyası ya da veritabanı ve ondan gelen katmanlar. Veri katmanı satırlarını kaynaktan okur; bağlı \
                 katman nesnelerini projede tutar ve kaynağa yazar; alınmış kopya nesnelerini projede tutar, kaynağa bağlı \
                 değildir. Satıra basınca katman Katmanlar'da seçilir."
                    .to_owned(),
            ),
        ))
    }
}

#[cfg(test)]
#[path = "source_catalog_tests.rs"]
mod tests;
