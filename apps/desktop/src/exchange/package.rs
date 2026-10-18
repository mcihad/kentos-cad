//! Paketle (docs/adr/0506): the folder asked for, then `file.package` on a copy of the drawing off the interface's
//! thread (the files are read whole and copied): the project file and `veri/` with the files it uses; what was left
//! (a missing file, a database, an address) said one by one.

use std::path::PathBuf;

use iced::Task;
use kentos_contracts::{CommandWarning, PackageProject, ProjectPackaged};
use kentos_interaction::Level;

use super::{Event as Exchange, message, off_thread};
use crate::app::{App, Message, Picker};

#[derive(Debug, Clone)]
pub enum Event {
    /// The folder to make; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    /// Packed, or why not.
    Done(Result<(ProjectPackaged, Vec<CommandWarning>), String>),
}

fn event(e: Event) -> Message {
    message(Exchange::Package(e))
}

impl App {
    pub(super) fn package_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        let name = self
            .document
            .as_ref()
            .map_or_else(|| "proje".to_owned(), |d| format!("{}-paket", d.name()));
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Paketle: paketin klasörü")
                    .set_file_name(name)
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Picked(p)),
        )
    }

    pub(super) fn package_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None) => Task::none(),
            Event::Picked(Some(path)) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let input = PackageProject {
                    path: path.display().to_string(),
                    name: None,
                    from: doc
                        .path
                        .as_ref()
                        .and_then(|p| p.parent())
                        .map(|p| p.display().to_string()),
                };
                let model = doc.model.clone();
                self.output(format!("{} paketleniyor…", path.display()));
                off_thread(
                    move || {
                        let planned = kentos_headless::package::plan(&model, &input)
                            .map_err(|e| e.message)?;
                        kentos_headless::package::write(&planned).map_err(|e| e.message)?;
                        Ok((planned.output, planned.warnings))
                    },
                    |r| Exchange::Package(Event::Done(r)),
                )
            }
            Event::Done(Ok((out, warnings))) => {
                let mb = out.bytes as f64 / (1024.0 * 1024.0);
                self.announce(
                    if out.left.is_empty() { Level::Success } else { Level::Warn },
                    format!(
                        "Paket yapıldı: {} ve {} dosya ({mb:.1} MB). Klasör başka bilgisayarda açılınca bağlı katmanlar kaynaklarını bulur.",
                        out.project,
                        out.files.len()
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
                Task::none()
            }
            Event::Done(Err(why)) => {
                self.announce(Level::Error, format!("Paket yapılamadı. {why}"));
                Task::none()
            }
        }
    }
}
