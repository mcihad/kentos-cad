//! Two revisions side by side (docs/adr/0576 §3): the other view of a split area shows another state of the open
//! drawing, read only: the drawing some undo steps back (the session's history), a drawing file, or a revision or
//! checkpoint downloaded from the server (Geçmiş's “İkinci görünümde”). The state is a drawing of its own with a
//! geometry store of its own, read and never written; the active view stays the open drawing and the edit target,
//! and a press on the revision does not make it active.
//!
//! What the open drawing changed since is marked in the revision: the objects it has changed or removed (paired by
//! their persistent ids, the data compare core's rules: `kentos_native_application::drawing_compare::diff`), in the
//! warning colour; the chip says how many were added, removed and changed. The marks follow the open drawing's edits.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_interaction::{Level, Selection, Spatial};
use kentos_native_application::drawing_compare::{self, Diff};

use crate::app::{App, Message};
use crate::document::Document;
use crate::split_view::Source;

/// How far points may lie apart and be one (the command's default), metres.
pub(crate) const TOLERANCE: f64 = kentos_commands::COMPARE_TOLERANCE;

/// How many steps back the source list offers.
pub(crate) const MOST_STEPS: usize = 12;

/// The other state and what is marked in it.
pub(crate) struct Revision {
    /// What it is: “3 adım geri, “Taşı” öncesi”, a file's name, “Revizyon 12”.
    pub name: String,
    pub doc: Document,
    pub spatial: Spatial,
    /// The objects the open drawing changed or removed since, in this state's slots.
    pub marked: Selection,
    /// The counts of the last comparison.
    pub counts: Counts,
    /// The open drawing (its session and generation) the marks were made for.
    for_doc: Option<(u64, u64)>,
    /// Where it came from, for the source list.
    pub source: Source,
}

/// What differs between the state and the open drawing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub same: usize,
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
}

impl Revision {
    pub(crate) fn new(name: String, doc: Document, source: Source) -> Self {
        let mut spatial = Spatial::new();
        // A whole drawing, as opening one does: `sync` would take in only what changed since a mark.
        spatial.reload(&doc.model);
        Self {
            name,
            doc,
            spatial,
            marked: Selection::new(),
            counts: Counts::default(),
            for_doc: None,
            source,
        }
    }

    /// The marks for the open drawing `now`, made again only when it changed.
    pub(crate) fn mark(&mut self, now: &Document) {
        let key = (now.session, now.model.generation());
        if self.for_doc == Some(key) {
            return;
        }
        self.for_doc = Some(key);
        let diff = drawing_compare::diff(&self.doc.model, &now.model, TOLERANCE);
        self.marked.set(marked_of(&diff));
        self.counts = Counts {
            same: diff.same,
            changed: diff.changed.len(),
            added: diff.added.len(),
            removed: diff.removed.len(),
        };
    }
}

/// The objects of the state to mark: the changed ones and the removed ones, in its slots.
pub(crate) fn marked_of(diff: &Diff) -> Vec<kentos_domain::Slot> {
    diff.changed
        .iter()
        .map(|c| c.old)
        .chain(diff.removed.iter().copied())
        .collect()
}

impl App {
    /// The other view shows `doc` as a revision of the open drawing (splitting first); read only.
    pub(crate) fn show_revision(&mut self, name: String, doc: Document, source: Source) {
        if self.document.is_none() {
            return;
        }
        if self.split.is_none() {
            self.toggle_split();
        }
        let Some(split) = self.split.as_mut() else {
            return;
        };
        let mut revision = Revision::new(name.clone(), doc, source);
        if let Some(now) = &self.document {
            revision.mark(now);
        }
        let c = revision.counts;
        split.revision = Some(revision);
        // Its scene and its text are the revision's now; a theme belongs to the open drawing's states.
        split.viewport.rebuild();
        split.viewport.theme = None;
        split.reset_text();
        self.say(
            Level::Info,
            format!(
                "İkinci görünüm {name}: salt okunur. Açık çizimde {} nesne eklendi, {} silindi, {} değişti; silinenler ve değişenler işaretli.",
                c.added, c.removed, c.changed
            ),
        );
    }

    /// The other view shows the open drawing again.
    pub(crate) fn hide_revision(&mut self) {
        let Some(split) = self.split.as_mut() else {
            return;
        };
        if split.revision.take().is_some() {
            split.viewport.rebuild();
            split.reset_text();
            self.say(Level::Info, "İkinci görünüm yine açık çizimi gösteriyor.");
        }
    }

    /// The open drawing `steps` undo steps back in the other view.
    pub(crate) fn revision_from_history(&mut self, steps: usize) {
        let Some(doc) = &self.document else {
            return;
        };
        match doc.model.past(steps) {
            Some((model, label)) => {
                let name = format!("{steps} adım geri, “{label}” öncesi");
                self.show_revision(name, Document::of_model(model), Source::History(steps));
            }
            None => self.warn("Geri alma geçmişi bu kadar adım tutmuyor."),
        }
    }

    /// A drawing file in the other view.
    pub(crate) fn revision_from_file(&mut self, path: &Path) {
        match Document::read(path) {
            Ok(doc) => {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.show_revision(name, doc, Source::File);
            }
            Err(why) => self.warn(why),
        }
    }

    /// A revision or checkpoint downloaded to `path`: read, removed, and shown in the other view (the catalog goes).
    pub(crate) fn revision_from_download(&mut self, name: &str, path: &Path) {
        let read = Document::read(path);
        let _ = std::fs::remove_file(path);
        match read {
            Ok(doc) => {
                self.cloud.catalog = None;
                self.dialog = None;
                self.show_revision(name.to_owned(), doc, Source::File);
            }
            Err(why) => self.warn(why),
        }
    }

    /// “Dosyadan…”: the file chosen asks the window.
    pub(crate) fn pick_revision_file(&self) -> Task<Message> {
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("İkinci görünümde gösterilecek çizim")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |picked: Option<PathBuf>| {
                Message::Split(crate::split_view::Event::RevisionPicked(picked))
            },
        )
    }

    /// The marks follow the open drawing (after every message).
    pub(crate) fn follow_revision(&mut self) {
        let (Some(split), Some(doc)) = (self.split.as_mut(), self.document.as_ref()) else {
            return;
        };
        if let Some(revision) = split.revision.as_mut() {
            revision.mark(doc);
        }
    }
}
