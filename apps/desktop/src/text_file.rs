//! Metin dosyası yerleştir's file (`kentos_interaction::text_file`,
//! docs/adr/0145 §6): the tool asks for it as it starts
//! (`ViewChange::OpenTextFile`); after the update the system's file dialog
//! opens (the trace player's and the pictures' `Picker::File` answer
//! without asking), and the file's name and bytes go back to the tool, or
//! none when the dialog is cancelled or the file cannot be read (said).

use iced::Task;

use crate::app::{App, Message, Picker};

impl App {
    /// The file dialog the tool asked for, once.
    pub(crate) fn text_file_tasks(&mut self) -> Task<Message> {
        if !std::mem::take(&mut self.text_file_wanted) {
            return Task::none();
        }
        match &self.picker {
            Picker::File(path) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                match std::fs::read(path) {
                    Ok(bytes) => Task::done(Message::TextFile(Some((name, bytes)))),
                    Err(e) => {
                        self.warn(format!("“{name}” okunamadı: {e}."));
                        Task::done(Message::TextFile(None))
                    }
                }
            }
            Picker::Dialog => Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Metin dosyası yerleştir")
                        .add_filter("Metin dosyası (UTF-8)", &["txt", "csv", "lst"])
                        .pick_file()
                        .await?;
                    let bytes = file.read().await;
                    Some((file.file_name(), bytes))
                },
                Message::TextFile,
            ),
        }
    }

    /// The file chosen, or none: to the tool that asked.
    pub(crate) fn text_file_given(&mut self, file: Option<(String, Vec<u8>)>) {
        self.with_tool(|s, cx| {
            s.file_given(file.as_ref().map(|(n, b)| (n.as_str(), b.as_slice())), cx);
        });
    }
}
