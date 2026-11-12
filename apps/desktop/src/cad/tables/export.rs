//! Export the selected CAD table through the same file command as scripts.

use std::path::PathBuf;

use iced::Task;
use kentos_commands::{Entity, ExportTable, TableNumbers};
use kentos_interaction::Level;

use crate::app::{App, Message, Picker};

#[derive(Debug, Clone)]
pub enum Event {
    Chosen(String, Option<PathBuf>),
}

fn msg(event: Event) -> Message {
    Message::TableExport(event)
}

impl App {
    pub(crate) fn export_selected_table(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let [slot] = self.selection.ids() else {
            self.say(Level::Info, "Dışa aktarılacak bir tablo seçin.");
            return Task::none();
        };
        if !matches!(doc.model.get(*slot), Some(Entity::Table(_))) {
            self.say(Level::Info, "Dışa aktarılacak bir tablo seçin.");
            return Task::none();
        }
        let Some(uid) = doc.model.uid(*slot).map(|u| u.to_string()) else {
            return Task::none();
        };
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Chosen(uid, Some(path.clone()))));
        }
        Task::perform(
            async move {
                let path = rfd::AsyncFileDialog::new()
                    .set_title("Tabloyu dışa aktar")
                    .add_filter("Tablo (.xlsx, .ods, .csv)", &["xlsx", "ods", "csv"])
                    .set_file_name("tablo.xlsx")
                    .save_file()
                    .await
                    .map(|file| file.path().to_path_buf());
                (uid, path)
            },
            |(uid, path)| msg(Event::Chosen(uid, path)),
        )
    }

    pub(crate) fn table_export_event(&mut self, event: Event) -> Task<Message> {
        let Event::Chosen(uid, Some(path)) = event else {
            return Task::none();
        };
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let input = ExportTable {
            path: path.display().to_string(),
            uid,
            groups: Some(true),
            numbers: TableNumbers::Exact,
            overwrite: true,
        };
        match kentos_headless::file_commands::export_table(&doc.model, &input) {
            Ok((out, _)) => {
                self.say(
                    Level::Success,
                    format!(
                        "{} yazıldı: {} satır, {} sütun.",
                        out.path, out.rows, out.columns
                    ),
                );
                for note in out.notes {
                    self.say(Level::Info, note);
                }
            }
            Err(error) => self.say(
                Level::Error,
                format!("Tablo dışa aktarılamadı. {}", error.message),
            ),
        }
        Task::none()
    }
}
