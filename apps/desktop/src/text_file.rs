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

impl App {
    /// Resim ekle's file dialog (docs/adr/0192 §5), once: a PNG or JPEG, its
    /// path kept for Bağlı.
    pub(crate) fn image_file_tasks(&mut self) -> Task<Message> {
        if !std::mem::take(&mut self.image_file_wanted) {
            return Task::none();
        }
        match &self.picker {
            Picker::File(path) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let at = Some(path.to_string_lossy().into_owned());
                match std::fs::read(path) {
                    Ok(bytes) => Task::done(Message::ImageFile(Some((name, at, bytes)))),
                    Err(e) => {
                        self.warn(format!("“{name}” okunamadı: {e}."));
                        Task::done(Message::ImageFile(None))
                    }
                }
            }
            Picker::Dialog => Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Resim ekle")
                        .add_filter("Resim (PNG, JPEG)", &["png", "jpg", "jpeg"])
                        .pick_file()
                        .await?;
                    let path = file.path().to_string_lossy().into_owned();
                    let bytes = file.read().await;
                    Some((file.file_name(), Some(path), bytes))
                },
                Message::ImageFile,
            ),
        }
    }

    /// The picture chosen, read as the tool takes it (its library item and
    /// size), or none: to the tool that asked. A file that is no picture is said.
    pub(crate) fn image_file_given(&mut self, file: Option<(String, Option<String>, Vec<u8>)>) {
        let given = match file {
            None => None,
            Some((name, path, bytes)) => match crate::pictures::library_item(&name, &bytes) {
                Ok((id, item)) => {
                    let size =
                        |k: &str| item.get(k).and_then(serde_json::Value::as_u64).unwrap_or(1);
                    let (width, height) = (size("width") as u32, size("height") as u32);
                    Some(kentos_interaction::ImageFile {
                        name,
                        path,
                        id,
                        library: kentos_contracts::ProjectStyles {
                            items: vec![item],
                            categories: Vec::new(),
                        },
                        width,
                        height,
                    })
                }
                Err(why) => {
                    self.warn(why);
                    None
                }
            },
        };
        self.with_tool(|s, cx| s.image_given(given.clone(), cx));
    }
}
