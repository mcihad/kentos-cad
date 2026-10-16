//! The Bloklar panel's library operations (docs/adr/0314 §3), each through `cad.blocks.edit`: Kopyasını oluştur
//! (a copy named “… kopya”, the next free name), Dışa aktar… (a block or every block with those they insert, as a
//! `.kblok` file: the `kentos.blocks` text), Blokları al… (a `.kblok` file's definitions; a name the drawing has is
//! kept and said). Bloğu dosyaya yaz… (WBLOCK) and Başka çizimden yeniden tanımla… write and read drawing files
//! through `file.export.block` and `file.import.block` (docs/adr/0316 §3).

use std::path::PathBuf;

use iced::Task;
use kentos_contracts::{
    BlockEditOperation, BlockFileMode, BlockId, BlocksEdit, BlocksEdited, CommandResult,
    ExportBlock, ImportBlock,
};
use kentos_native_application::{ExecutionContext, blocks_edit, file_import_block};

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
        units: None,
        rescale: None,
        entities: None,
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

    /// Birim ▸ (docs/adr/0315 §4): the unit the block is drawn in, its size kept, or with `rescale` its numbers.
    pub(crate) fn blocks_units(
        &mut self,
        id: BlockId,
        units: kentos_contracts::BlockUnits,
        rescale: bool,
    ) {
        let Some(name) = self
            .document
            .as_ref()
            .and_then(|d| d.model.block(id))
            .map(|b| b.name.clone())
        else {
            return;
        };
        let edit = BlocksEdit {
            block: Some(id),
            units: Some(units),
            rescale: rescale.then_some(true),
            ..input(BlockEditOperation::Units)
        };
        if let Some(done) = self.blocks_command(edit)
            && !done.changed.is_empty()
        {
            let how = if rescale {
                "sayıları korunarak, boyu birimlerin oranıyla değişti"
            } else {
                "boyu korundu"
            };
            self.say(
                Level::Info,
                format!("“{name}” bloğunun birimi {}: {how}.", units.word()),
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

    /// The block a ribbon command works on: the selected insert's, else the Bloklar panel's chosen row.
    pub(crate) fn block_at_hand(&mut self) -> Option<BlockId> {
        let doc = self.document.as_ref()?;
        let mut ids = self.selection.ids().iter();
        let selected = match (ids.next(), ids.next()) {
            (Some(slot), None) => match doc.model.get(*slot) {
                Some(kentos_contracts::Entity::Insert(i)) => Some(i.block),
                _ => None,
            },
            _ => None,
        };
        let id = selected.or(self
            .blocks_panel
            .focused
            .filter(|b| doc.model.block(*b).is_some()));
        if id.is_none() {
            self.warn("Bir bloğun yerleştirmesini seçin ya da Bloklar panelinde bir blok seçin.");
        }
        id
    }

    /// Bloğu dosyaya yaz…: where the drawing goes, asked.
    pub(crate) fn blocks_write_file(&mut self, id: BlockId) -> Task<Message> {
        let Some(name) = self
            .document
            .as_ref()
            .and_then(|d| d.model.block(id).map(|b| b.name.clone()))
        else {
            return Task::none();
        };
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Bloğu dosyaya yaz")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .set_file_name(format!("{name}.kcad"))
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| Message::BlocksPanel(Event::FileChosen(id, path)),
        )
    }

    /// The block's objects written as a drawing of their own, its base at the origin (`file.export.block`).
    pub(crate) fn blocks_file_chosen(&mut self, id: BlockId, path: Option<PathBuf>) {
        let Some(path) = path else { return };
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let name = doc
            .model
            .block(id)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        // The dialog asked before a file there is written over.
        let input = ExportBlock {
            path: path.display().to_string(),
            block: Some(id),
            uids: None,
            base: None,
            overwrite: true,
        };
        match kentos_headless::file_commands::export_block(&doc.model, &input) {
            Ok(done) => self.say(
                Level::Success,
                format!(
                    "“{name}” bloğu {} dosyasına yazıldı: {} nesne{}, taban noktası başlangıçta.",
                    path.display(),
                    done.objects,
                    if done.blocks > 0 {
                        format!(", {} iç blok", done.blocks)
                    } else {
                        String::new()
                    }
                ),
            ),
            Err(e) => self.warn(format!("Blok yazılamadı: {}", e.message)),
        }
    }

    /// Başka çizimden yeniden tanımla…: which drawing, asked.
    pub(crate) fn blocks_redefine_from(&mut self, id: BlockId) -> Task<Message> {
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Başka çizimden yeniden tanımla")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| Message::BlocksPanel(Event::RedefineChosen(id, path)),
        )
    }

    /// The other drawing's block of this name in its place, one undo step; the inserts show it.
    pub(crate) fn blocks_redefine_chosen(&mut self, id: BlockId, path: Option<PathBuf>) {
        let Some(path) = path else { return };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let Some(name) = doc.model.block(id).map(|b| b.name.clone()) else {
            return;
        };
        let input = ImportBlock {
            path: path.display().to_string(),
            mode: BlockFileMode::Blocks,
            name: None,
            names: Some(vec![name.clone()]),
            replace: Some(true),
            expected_revision: None,
        };
        match file_import_block::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let inserts = doc
                    .model
                    .entities()
                    .filter(|e| matches!(e, kentos_contracts::Entity::Insert(i) if i.block == id))
                    .count();
                if output.changed.contains(&id) {
                    self.say(
                        Level::Success,
                        format!(
                            "“{name}” bloğu {} çizimindekiyle yeniden tanımlandı; {inserts} yerleştirmesi yeni hâlini gösteriyor. Tek adımda geri alınır.",
                            file_name_of(&path)
                        ),
                    );
                } else {
                    self.say(
                        Level::Info,
                        format!(
                            "“{name}” bloğu {} çizimindekiyle aynıydı.",
                            file_name_of(&path)
                        ),
                    );
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            _ => {}
        }
    }
}

fn file_name_of(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
