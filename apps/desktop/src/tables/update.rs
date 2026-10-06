//! Tabloyu güncelle (docs/adr/0184 §6; the web's `app/tables.ts`
//! `updateTables`): each table rewritten from its source in one step: a
//! schedule from its objects still in the drawing, a file's from the file
//! chosen again (one question for each file); its look and the sizes given by
//! hand kept (`ops::table_edit::refresh`). A table on a locked layer is left
//! and said, one already up to date is not written.

use std::collections::BTreeMap;

use iced::Task;
use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, TableEntity, TableFileRead,
    TableSource,
};
use kentos_domain::Slot;
use kentos_geometry_core::ops::table::{Cells, ScheduleKind};
use kentos_geometry_core::ops::table_edit::refresh;
use kentos_interaction::Level;
use kentos_native_application::geometry::{drawing_font, edit_geometry, entity_of, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::app::{App, Message};

/// A run of Tabloyu güncelle waiting for its files.
#[derive(Clone, Debug, Default)]
pub struct Run {
    /// The tables, in the drawing's order.
    tables: Vec<Slot>,
    /// The files still to ask for, by the name their tables keep.
    asking: Vec<String>,
    /// The files read (none: not chosen).
    books: BTreeMap<String, Option<TableFileRead>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A file chosen for `name` (none: the picker cancelled).
    FileRead(String, Option<Vec<u8>>),
}

fn msg(event: Event) -> Message {
    Message::TableUpdate(event)
}

/// Asks for the file a table keeps the name of.
fn ask(name: String) -> Task<Message> {
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title(format!("“{name}” dosyasını seçin"))
                .add_filter("Tablo (.xlsx, .csv, .txt)", &["xlsx", "csv", "txt", "tsv"])
                .pick_file()
                .await;
            let bytes = match file {
                Some(f) => Some(f.read().await),
                None => None,
            };
            (name, bytes)
        },
        |(name, bytes)| msg(Event::FileRead(name, bytes)),
    )
}

/// Same cells, alignments and sizes: nothing to write.
fn same(a: &TableEntity, b: &TableEntity) -> bool {
    a.cells == b.cells
        && a.rows == b.rows
        && a.columns == b.columns
        && a.aligns == b.aligns
        && a.merges == b.merges
}

impl App {
    /// The tables Tabloyu güncelle rewrites: the selected ones with a
    /// source; with none selected, every one with a source.
    pub(crate) fn sourced_tables(&self) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        let sourced =
            |s: &Slot| matches!(model.get(*s), Some(Entity::Table(t)) if t.source.is_some());
        if !self.selection.ids().is_empty() {
            return kentos_interaction::table::in_order(model, self.selection.ids())
                .into_iter()
                .filter(sourced)
                .collect();
        }
        model
            .entities()
            .map(|e| Slot(e.base().id))
            .filter(sourced)
            .collect()
    }

    /// Tabloyu güncelle (`table.update`).
    pub(crate) fn update_tables(&mut self) -> Task<Message> {
        let all = self.sourced_tables();
        if all.is_empty() {
            self.say(
                Level::Info,
                "Kaynağı olan tablo yok: Tabloyu güncelle çizelgeleri ve dosyadan gelen tabloları yeniden yazar.",
            );
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = &doc.model;
        let locked: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|s| {
                model
                    .get(*s)
                    .is_some_and(|e| model.layers().is_locked(&e.base().layer_id))
            })
            .collect();
        let tables: Vec<Slot> = all.into_iter().filter(|s| !locked.contains(s)).collect();
        let mut asking: Vec<String> = Vec::new();
        for s in &tables {
            if let Some(Entity::Table(TableEntity {
                source: Some(TableSource::File { name, .. }),
                ..
            })) = model.get(*s)
                && !asking.contains(name)
            {
                asking.push(name.clone());
            }
        }
        if !locked.is_empty() {
            self.warn(format!(
                "{} tablo kilitli katmanda: güncellenmedi.",
                locked.len()
            ));
        }
        let mut run = Run {
            tables,
            asking,
            books: BTreeMap::new(),
        };
        let next = run.asking.first().cloned();
        match next {
            Some(name) => {
                run.asking.remove(0);
                self.say(
                    Level::Info,
                    format!("“{name}” dosyasını seçin: tablo ondan yeniden yazılır."),
                );
                self.table_update = Some(run);
                ask(name)
            }
            None => {
                self.finish_update(run);
                Task::none()
            }
        }
    }

    pub(crate) fn table_update_event(&mut self, event: Event) -> Task<Message> {
        let Event::FileRead(name, bytes) = event;
        let Some(mut run) = self.table_update.take() else {
            return Task::none();
        };
        run.books
            .insert(name, bytes.map(|b| kentos_formats::table_file::read(&b)));
        if let Some(next) = run.asking.first().cloned() {
            run.asking.remove(0);
            self.say(
                Level::Info,
                format!("“{next}” dosyasını seçin: tablo ondan yeniden yazılır."),
            );
            self.table_update = Some(run);
            return ask(next);
        }
        self.finish_update(run);
        Task::none()
    }

    /// The tables' new cells, written in one step “Tabloyu güncelle”.
    fn finish_update(&mut self, run: Run) {
        let format = self.format();
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let font = drawing_font(model.settings().drawing_font);
        let mut said: Vec<(Level, String)> = Vec::new();
        let mut changes = Vec::new();
        for slot in &run.tables {
            let Some(Entity::Table(t)) = model.get(*slot) else {
                continue;
            };
            let Some(source) = &t.source else {
                continue;
            };
            let cells: Cells = match source {
                TableSource::File { name, sheet } => {
                    let Some(Some(book)) = run.books.get(name) else {
                        continue;
                    };
                    if let Some(p) = &book.problem {
                        said.push((Level::Warn, format!("“{name}”: {p}")));
                        continue;
                    }
                    let found = match sheet {
                        None => book.sheets.first(),
                        Some(want) => book.sheets.iter().find(|s| s.name.as_ref() == Some(want)),
                    };
                    let Some(found) = found else {
                        said.push((
                            Level::Warn,
                            format!(
                                "Dosyada “{}” sayfası yok: tablo güncellenmedi.",
                                sheet.clone().unwrap_or_default()
                            ),
                        ));
                        continue;
                    };
                    kentos_interaction::table::sheet_cells(found, t.header)
                }
                schedule => {
                    let kind = match schedule {
                        TableSource::Coordinates { .. } => ScheduleKind::Coordinates,
                        TableSource::Areas { .. } => ScheduleKind::Areas,
                        _ => ScheduleKind::Attributes,
                    };
                    let objects: Vec<Slot> = schedule
                        .objects()
                        .iter()
                        .filter_map(|id| model.slot_of(kentos_domain::Uuid::from_bytes(id.0)))
                        .collect();
                    let missing = schedule.objects().len() - objects.len();
                    if missing > 0 {
                        said.push((
                            Level::Warn,
                            format!("Çizelgenin {missing} nesnesi çizimde yok: tablo kalanlardan yazıldı."),
                        ));
                    }
                    kentos_interaction::table::schedule(model, kind, &objects, &format)
                }
            };
            if let Some(p) = &cells.problem {
                said.push((Level::Warn, p.clone()));
                continue;
            }
            let Some(next) = refresh(
                &shape(&Entity::Table(t.clone())),
                &cells.cells,
                &cells.aligns,
                font,
            )
            .and_then(edit_geometry) else {
                continue;
            };
            let Entity::Table(fresh) = entity_of(&next, t.base.clone()) else {
                continue;
            };
            if same(t, &fresh) {
                continue;
            }
            let Some(uid) = model.uid(*slot) else {
                continue;
            };
            changes.push(EntityEdit::Update {
                uid: uid.to_string(),
                geometry: next,
            });
        }
        for (level, text) in said {
            self.say(level, text);
        }
        if changes.is_empty() {
            if !run.tables.is_empty() {
                self.say(Level::Info, "Tablolar güncel.");
            }
            return;
        }
        let n = changes.len();
        let input = EntitiesEdit {
            operation: EditOperation::TableUpdate,
            changes,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                self.say(Level::Success, format!("{n} tablo güncellendi."));
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            _ => {}
        }
    }
}
