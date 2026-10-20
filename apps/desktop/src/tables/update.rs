//! Tabloyu güncelle and Dosyaya bağla (docs/adr/0184 §6, docs/adr/0327 §4–§5): `cad.tables.edit`'s `refresh` over the
//! selected tables with a source (every one with none selected): a schedule from its objects, a linked file from its
//! path (from the drawing's folder when relative), cells changed by hand kept as the table's rule says; a block
//! schedule is the document's at every step. A table naming a file it is not linked to, or whose file is not where its
//! path says, asks for the file, which links it (`link`). A table on a locked layer is left and said.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_commands::{
    CommandResult, Entity, TableRefresh, TableSource, TablesEdit, TablesEdited, TablesOperation,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, tables_edit};

use crate::app::{App, Message};

/// The tables still to ask a file for: each's persistent id and the file's name it keeps.
#[derive(Clone, Debug, Default)]
pub struct Run {
    asking: Vec<(Uuid, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A file chosen for the table (none: the picker cancelled).
    Picked(Uuid, Option<PathBuf>),
}

fn msg(event: Event) -> Message {
    Message::TableUpdate(event)
}

/// Asks for the file the table `uid` keeps the name of.
fn ask(uid: Uuid, name: String) -> Task<Message> {
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title(format!("“{name}” dosyasını seçin"))
                .add_filter(
                    "Tablo (.xlsx, .ods, .csv, .txt)",
                    &["xlsx", "ods", "csv", "txt", "tsv"],
                )
                .pick_file()
                .await;
            (uid, file.map(|f| f.path().to_path_buf()))
        },
        |(uid, path)| msg(Event::Picked(uid, path)),
    )
}

/// The path a linked table keeps: from the drawing's folder when the file is in it, else as chosen.
pub fn kept_path(file: &Path, base: Option<&Path>) -> String {
    match base.and_then(|b| file.strip_prefix(b).ok()) {
        Some(rel) => rel.to_string_lossy().into_owned(),
        None => file.to_string_lossy().into_owned(),
    }
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

    pub(crate) fn run_tables(&mut self, operation: TablesOperation) -> Option<TablesEdited> {
        let doc = self.document.as_mut()?;
        let input = TablesEdit {
            operation,
            expected_revision: None,
        };
        match tables_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    /// Tabloyu güncelle (`table.update`).
    pub(crate) fn update_tables(&mut self) -> Task<Message> {
        let all = self.sourced_tables();
        if all.is_empty() {
            self.say(
                Level::Info,
                "Kaynağı olan tablo yok: Tabloyu güncelle çizelgeleri ve dosyaya bağlı tabloları yeniden yazar.",
            );
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = &doc.model;
        let (locked, open): (Vec<Slot>, Vec<Slot>) = all.into_iter().partition(|s| {
            model
                .get(*s)
                .is_some_and(|e| model.layers().is_locked(&e.base().layer_id))
        });
        let uids: Vec<String> = open
            .iter()
            .filter_map(|s| model.uid(*s).map(|u| u.to_string()))
            .collect();
        let names: Vec<(Uuid, String)> = open
            .iter()
            .filter_map(|s| match (model.uid(*s), model.get(*s)) {
                (Some(u), Some(Entity::Table(t))) => match &t.source {
                    Some(TableSource::File { name, .. }) => Some((u, name.clone())),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if !locked.is_empty() {
            self.warn(format!(
                "{} tablo kilitli katmanda: güncellenmedi.",
                locked.len()
            ));
        }
        if uids.is_empty() {
            return Task::none();
        }
        let base = self
            .drawing_folder()
            .map(|p| p.to_string_lossy().into_owned());
        let Some(out) = self.run_tables(TablesOperation::Refresh { uids, base }) else {
            return Task::none();
        };
        let count = |s: TableRefresh| out.report.iter().filter(|r| r.status == s).count();
        let (updated, same, live) = (
            count(TableRefresh::Updated),
            count(TableRefresh::Unchanged),
            count(TableRefresh::Live),
        );
        let kept: u32 = out.report.iter().map(|r| r.kept).sum();
        let mut words = Vec::new();
        if updated > 0 {
            words.push(format!("{updated} tablo güncellendi"));
        }
        if kept > 0 {
            words.push(format!("elle değişen {kept} hücre korundu"));
        }
        if same > 0 {
            words.push(format!("{same} tablo güncel"));
        }
        if live > 0 {
            words.push(format!(
                "{live} blok çizelgesi çizimle kendiliğinden güncel"
            ));
        }
        if !words.is_empty() {
            self.say(Level::Success, format!("{}.", words.join(", ")));
        }
        // The tables whose file is to be chosen: not linked, or not where their path says.
        let asking: Vec<(Uuid, String)> = out
            .report
            .iter()
            .filter(|r| matches!(r.status, TableRefresh::Unlinked | TableRefresh::Missing))
            .filter_map(|r| {
                let u = Uuid::parse_str(&r.uid).ok()?;
                names.iter().find(|(v, _)| *v == u).cloned()
            })
            .collect();
        self.ask_next(Run { asking })
    }

    /// Dosyaya bağla (`table.link`): the selected table asks for its file.
    pub(crate) fn link_table(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let found = self
            .selection
            .ids()
            .iter()
            .find_map(|s| match doc.model.get(*s) {
                Some(Entity::Table(t)) => {
                    let name = match &t.source {
                        Some(TableSource::File { name, .. }) => name.clone(),
                        _ => "tablo".to_owned(),
                    };
                    doc.model.uid(*s).map(|u| (u, name))
                }
                _ => None,
            });
        match found {
            Some(one) => self.ask_next(Run { asking: vec![one] }),
            None => {
                self.say(Level::Info, "Dosyaya bağlanacak tabloyu seçin.");
                Task::none()
            }
        }
    }

    fn ask_next(&mut self, mut run: Run) -> Task<Message> {
        if run.asking.is_empty() {
            self.table_update = None;
            return Task::none();
        }
        let (uid, name) = run.asking.remove(0);
        self.say(
            Level::Info,
            format!("“{name}” dosyasını seçin: tablo ona bağlanır ve ondan yazılır."),
        );
        self.table_update = Some(run);
        ask(uid, name)
    }

    pub(crate) fn table_update_event(&mut self, event: Event) -> Task<Message> {
        let Event::Picked(uid, path) = event;
        let run = self.table_update.take().unwrap_or_default();
        if let Some(file) = path {
            let base = self.drawing_folder();
            let kept = kept_path(&file, base.as_deref());
            let operation = TablesOperation::Link {
                uid: uid.to_string(),
                path: kept.clone(),
                sheet: None,
                base: base.map(|b| b.to_string_lossy().into_owned()),
                overrides: None,
            };
            if self.run_tables(operation).is_some() {
                self.say(
                    Level::Success,
                    format!("Tablo “{kept}” dosyasına bağlandı ve ondan yazıldı."),
                );
            }
        }
        self.ask_next(run)
    }
}
