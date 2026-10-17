//! Bağlı katmanlar (docs/adr/0503, plan/gis.md CBS-01, CBS-02): a layer kept as the working copy of a GeoPackage table.
//! Katmanlar shows the binding as a badge and its menu's Kaynak ▸ (Kaynağa yaz, Kaynaktan oku, Kaynak durumu, Bağı
//! çöz); the same commands run from the ribbon on the layer selected in the tree, else the active one. Each writes
//! through `cad.sources.sync` and `cad.sources.edit`, so a script does the same. A reload over the layer's own edits
//! asks first (the command's `pending_changes`), and goes on with `discard`; a write over a file another program
//! changed is refused and said (`source_changed`).

use iced::Element;
use kentos_contracts::{
    CommandResult, SourceLink, SourcesDescribe, SourcesEdit, SourcesEditOp, SourcesSync,
    SyncDirection,
};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::sources::{describe, edit, sync};

use crate::app::{App, Dialog, Message};

#[derive(Debug, Clone)]
pub enum Event {
    /// Kaynağa yaz on a layer (none: the one at hand).
    Write(Option<String>),
    /// Kaynaktan oku.
    Read(Option<String>),
    /// Kaynak durumu: every binding's state.
    Status,
    /// Bağı çöz.
    Unlink(Option<String>),
}

const NO_BOUND: &str = "Seçili ya da etkin katman bir kaynağa bağlı değil. GeoPackage'ı “Bağlı katman olarak aç” ile alın.";

impl App {
    /// The binding of a layer, if it is bound.
    pub(crate) fn bound_link(&self, layer: &str) -> Option<SourceLink> {
        self.document
            .as_ref()?
            .model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == layer)
            .cloned()
    }

    /// The bound layer a command acts on: the one named, else the one selected in the tree, else the active one.
    fn bound_at_hand(&self, layer: Option<String>) -> Option<SourceLink> {
        let doc = self.document.as_ref()?;
        let id = layer
            .or_else(|| self.selected_layer.clone())
            .unwrap_or_else(|| doc.model.layers().active().to_owned());
        self.bound_link(&id)
    }

    fn bound_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    }

    pub(crate) fn bound_event(&mut self, e: Event) {
        match e {
            Event::Write(layer) => match self.bound_at_hand(layer) {
                Some(link) => self.bound_sync(&link, SyncDirection::Write, false),
                None => self.warn(NO_BOUND),
            },
            Event::Read(layer) => match self.bound_at_hand(layer) {
                Some(link) => self.bound_sync(&link, SyncDirection::Read, false),
                None => self.warn(NO_BOUND),
            },
            Event::Unlink(layer) => match self.bound_at_hand(layer) {
                Some(link) => self.bound_unlink(&link),
                None => self.warn(NO_BOUND),
            },
            Event::Status => self.bound_status(),
        }
    }

    fn bound_sync(&mut self, link: &SourceLink, direction: SyncDirection, discard: bool) {
        let name = self.bound_name(&link.layer);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = SourcesSync {
            layer: link.layer.clone(),
            direction,
            discard,
        };
        match sync::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                let what = |n: u32, word: &str| (n > 0).then(|| format!("{n} {word}"));
                let parts: Vec<String> = [
                    what(output.added, "eklendi"),
                    what(output.modified, "değişti"),
                    what(output.removed, "silindi"),
                ]
                .into_iter()
                .flatten()
                .collect();
                let file = file_name(&link.path);
                let said = match (direction, parts.is_empty()) {
                    (SyncDirection::Write, true) => {
                        format!(
                            "“{name}” zaten {file} › {} ile aynı; yazılacak bir şey yok.",
                            link.table
                        )
                    }
                    (SyncDirection::Write, false) => format!(
                        "“{name}” {file} › {} tablosuna yazıldı: {}{}.",
                        link.table,
                        parts.join(", "),
                        if output.columns.is_empty() {
                            String::new()
                        } else {
                            format!("; yeni sütunlar: {}", output.columns.join(", "))
                        }
                    ),
                    (SyncDirection::Read, true) => {
                        format!(
                            "“{name}” {file} › {} ile aynı; okunacak bir değişiklik yok.",
                            link.table
                        )
                    }
                    (SyncDirection::Read, false) => format!(
                        "“{name}” {file} › {} tablosundan okundu: {}. Ctrl+Z geri alır.",
                        link.table,
                        parts.join(", ")
                    ),
                };
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    said,
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Conflict { error } if error.code == "pending_changes" => {
                self.bound_pending = Some(link.layer.clone());
                self.dialog = Some(Dialog::SourceDiscard);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.announce(Level::Error, error.message);
            }
            _ => {}
        }
    }

    /// Kaynaktan oku over the layer's own edits, asked: “Değişiklikleri at” reads with `discard`.
    pub(crate) fn bound_discard(&mut self) {
        if let Some(layer) = self.bound_pending.take()
            && let Some(link) = self.bound_link(&layer)
        {
            self.bound_sync(&link, SyncDirection::Read, true);
        }
    }

    pub(crate) fn bound_discard_question(&self) -> Element<'_, Message> {
        let layer = self.bound_pending.clone().unwrap_or_default();
        let name = self.bound_name(&layer);
        let table = self
            .bound_link(&layer)
            .map(|l| format!("{} › {}", file_name(&l.path), l.table))
            .unwrap_or_default();
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(
                "Kaynaktan oku",
                Message::DialogConfirmed,
                Message::DialogClosed,
            )
            .message(format!(
                "“{name}” katmanı {table} ile son eşleştiğinden beri değişti. Okumak katmanı tablonun hâline getirir ve bu \
                 değişiklikleri götürür (Ctrl+Z geri alır). Önce Kaynağa yaz ile yazabilirsiniz."
            ))
            .confirm("Değişiklikleri at ve oku")
            .destructive(),
            Message::DialogClosed,
        )
    }

    fn bound_unlink(&mut self, link: &SourceLink) {
        let name = self.bound_name(&link.layer);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = SourcesEdit {
            edit: SourcesEditOp::Unlink {
                id: link.id.clone(),
            },
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { .. } => self.announce(
                Level::Success,
                format!(
                    "“{name}” katmanının {} › {} bağı çözüldü; nesneleri çizimde kaldı.",
                    file_name(&link.path),
                    link.table
                ),
            ),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.announce(Level::Error, error.message),
            _ => {}
        }
    }

    fn bound_status(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = describe::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SourcesDescribe::default(),
        );
        let CommandResult::Completed { output, .. } = answer else {
            return;
        };
        if output.links.is_empty() {
            self.announce(Level::Info, "Projede bağlı katman yok.".to_owned());
            return;
        }
        for s in output.links {
            let name = self.bound_name(&s.link.layer);
            let file = format!("{} › {}", file_name(&s.link.path), s.link.table);
            let (level, said) = if !s.found {
                (
                    Level::Warn,
                    format!("“{name}”: {file} yerinde yok ({}).", s.link.path),
                )
            } else if s.added + s.modified + s.removed == 0 {
                (Level::Success, format!("“{name}”: {file} ile aynı."))
            } else {
                let side = match (s.edited, s.changed) {
                    (true, false) => "katmanda yazılmamış değişiklikler",
                    (false, true) => "dosya başka bir programda değişmiş",
                    (true, true) => "hem katman hem dosya değişmiş",
                    (false, false) => "fark var",
                };
                (
                    Level::Warn,
                    format!(
                        "“{name}”: {file} — {side}: tabloda olmayan {}, farklı {}, katmanda olmayan {} nesne.",
                        s.added, s.modified, s.removed
                    ),
                )
            };
            self.announce(level, said);
        }
    }
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}
