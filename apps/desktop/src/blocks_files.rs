//! The Bloklar panel's library operations (docs/adr/0314 §3), each through `cad.blocks.edit`: Kopyasını oluştur
//! (a copy named “… kopya”, the next free name), Dışa aktar… (a block or every block with those they insert, as a
//! `.kblok` file: the `kentos.blocks` text), Blokları al… (a `.kblok` file's definitions; a name the drawing has is
//! kept and said).

use std::path::PathBuf;

use iced::Task;
use kentos_contracts::{BlockEditOperation, BlockId, BlocksEdit, BlocksEdited, CommandResult};
use kentos_native_application::{ExecutionContext, blocks_edit};

use crate::app::{App, Message};
use crate::blocks_panel::Event;
use kentos_interaction::Level;

fn input(operation: BlockEditOperation) -> BlocksEdit {
    BlocksEdit {
        operation,
        block: None,
        name: None,
        uids: None,
        base: None,
        replace: None,
        layer_id: None,
        attributes: None,
        blocks: None,
        library: None,
        expected_revision: None,
    }
}

impl App {
    /// `cad.blocks.edit` on the drawing; a refusal said.
    fn blocks_command(&mut self, input: BlocksEdit) -> Option<BlocksEdited> {
        let doc = self.document.as_mut()?;
        match blocks_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Some(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    /// Kopyasını oluştur: “<ad> kopya”, then “<ad> kopya 2” … until the drawing has none of the name.
    pub(crate) fn blocks_duplicate(&mut self, id: BlockId) {
        let Some(doc) = &self.document else { return };
        let Some(name) = doc.model.block(id).map(|b| b.name.clone()) else {
            return;
        };
        let base = format!("{name} kopya");
        let free = (1..)
            .map(|n| {
                if n == 1 {
                    base.clone()
                } else {
                    format!("{base} {n}")
                }
            })
            .find(|n| doc.model.block_named(n).is_none())
            .unwrap_or(base);
        let edit = BlocksEdit {
            block: Some(id),
            name: Some(free.clone()),
            ..input(BlockEditOperation::Duplicate)
        };
        if self.blocks_command(edit).is_some() {
            self.say(
                Level::Info,
                format!("“{name}” bloğunun kopyası “{free}” oluşturuldu."),
            );
        }
    }

    /// Dışa aktar…: where the file goes, asked.
    pub(crate) fn blocks_export(&mut self, id: Option<BlockId>) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let name = id
            .and_then(|id| doc.model.block(id).map(|b| b.name.clone()))
            .unwrap_or_else(|| format!("{} blokları", doc.model.name()));
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Blokları dışa aktar")
                    .add_filter("KentOS blok dosyası (.kblok)", &["kblok"])
                    .set_file_name(format!("{name}.kblok"))
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| Message::BlocksPanel(Event::Exported(id, path)),
        )
    }

    /// The file written: the block's (or every block's) definitions with those they insert.
    pub(crate) fn blocks_exported(&mut self, id: Option<BlockId>, path: Option<PathBuf>) {
        let Some(path) = path else { return };
        let edit = BlocksEdit {
            blocks: id.map(|id| vec![id]),
            ..input(BlockEditOperation::Export)
        };
        let Some(text) = self.blocks_command(edit).and_then(|d| d.library) else {
            return;
        };
        match std::fs::write(&path, text) {
            Ok(()) => self.say(
                Level::Info,
                format!("Bloklar {} dosyasına yazıldı.", path.display()),
            ),
            Err(e) => self.warn(format!(
                "{} yazılamadı: {e}. Başka bir yer seçin.",
                path.display()
            )),
        }
    }

    /// Blokları al…: which file, asked.
    pub(crate) fn blocks_import(&mut self) -> Task<Message> {
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Blokları al")
                    .add_filter("KentOS blok dosyası (.kblok)", &["kblok"])
                    .pick_file()
                    .await?;
                let bytes = file.read().await;
                Some((
                    file.file_name(),
                    String::from_utf8_lossy(&bytes).into_owned(),
                ))
            },
            |read| Message::BlocksPanel(Event::Imported(read)),
        )
    }

    /// The file's definitions taken in, in one undo step; the names kept said.
    pub(crate) fn blocks_imported(&mut self, read: Option<(String, String)>) {
        let Some((file, text)) = read else { return };
        let edit = BlocksEdit {
            library: Some(text),
            ..input(BlockEditOperation::Import)
        };
        let Some(done) = self.blocks_command(edit) else {
            return;
        };
        let mut words = format!("{file}: {} blok alındı", done.added.len());
        if !done.skipped.is_empty() {
            words.push_str(&format!(
                "; çizimde adı olan {} blok olduğu gibi kaldı ({})",
                done.skipped.len(),
                done.skipped.join(", ")
            ));
        }
        words.push('.');
        self.say(Level::Info, words);
    }
}
