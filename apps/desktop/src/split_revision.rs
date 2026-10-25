//! Two revisions side by side (docs/adr/0576 §3): the other view of a split area shows another state of the open
//! drawing, read only: the drawing some undo steps back (the session's history), a drawing file, or a revision or
//! checkpoint downloaded from the server (Geçmiş's “İkinci görünümde”). The state is a drawing of its own with a
//! geometry store of its own, read and never written; the active view stays the open drawing and the edit target,
//! and a press on the revision does not make it active.
//!
//! What the open drawing changed since is marked in the revision: the objects it has changed or removed (paired by
//! their persistent ids, the data compare core's rules: `kentos_native_application::drawing_compare::diff`), in the
//! warning colour; the chip says how many were added, removed and changed. The marks follow the open drawing's edits.
//! While something is selected in the open drawing, the revision shows those records instead, in the accent
//! (docs/adr/0637 §3): the same objects, by their lasting ids.
//!
//! The first comparison is whole; after it the marks follow the open drawing's journal and compare only the objects
//! an edit touched (docs/adr/0637 §4): a whole comparison of 100 000 parcels takes some 300 ms, an edit's a moment.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::Task;
use kentos_domain::{ChangeMark, Changes, Slot, Uuid};
use kentos_interaction::{Level, Selection, Spatial};
use kentos_native_application::drawing_compare;

use crate::app::{App, Message};
use crate::document::Document;
use crate::split_view::Source;

/// How far points may lie apart and be one (the command's default), metres.
pub(crate) const TOLERANCE: f64 = kentos_commands::COMPARE_TOLERANCE;

/// More touched slots than this and the marks are made again whole: the whole comparison is then the cheaper.
const MOST_FOLLOWED: usize = 20_000;

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
    /// The open drawing's selection as this state holds it, by their lasting ids (docs/adr/0637 §3): while something
    /// is selected the view shows these records instead of the marks.
    pub chosen: Selection,
    /// The open drawing's session, generation and selection version `chosen` was made for.
    chosen_for: Option<(u64, u64, u64)>,
    /// The counts of the last comparison.
    pub counts: Counts,
    /// The open drawing (its session and generation) the marks were made for.
    for_doc: Option<(u64, u64)>,
    /// What differs, by lasting id: objects not here are the same (or in neither state).
    kinds: HashMap<Uuid, Kind>,
    /// The open drawing's ids by slot, as the marks last saw them: a removed object's id is gone from the drawing.
    now_uids: HashMap<Slot, Uuid>,
    /// The open drawing's journal where the marks last read it.
    at: Option<ChangeMark>,
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

/// How an object differs: changed or removed (by its slot in the revision), or added in the open drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Changed(Slot),
    Removed(Slot),
    Added,
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
            chosen: Selection::new(),
            chosen_for: None,
            counts: Counts::default(),
            for_doc: None,
            kinds: HashMap::new(),
            now_uids: HashMap::new(),
            at: None,
            source,
        }
    }

    /// The marks for the open drawing `now`, made again only when it changed: whole the first time and for another
    /// drawing, else by the objects the journal says were touched since.
    pub(crate) fn mark(&mut self, now: &Document) {
        let key = (now.session, now.model.generation());
        if self.for_doc == Some(key) {
            return;
        }
        let same_drawing = self
            .for_doc
            .is_some_and(|(session, _)| session == now.session);
        self.for_doc = Some(key);
        let touched = match (same_drawing, self.at) {
            (true, Some(at)) => match now.model.changes_since(at) {
                Changes::Slots(slots) if slots.len() <= MOST_FOLLOWED => Some(slots.to_vec()),
                _ => None,
            },
            _ => None,
        };
        self.at = Some(now.model.change_mark());
        match touched {
            Some(slots) => self.follow(now, slots),
            None => self.mark_whole(now),
        }
        let mut marked: Vec<Slot> = self
            .kinds
            .values()
            .filter_map(|k| match k {
                Kind::Changed(s) | Kind::Removed(s) => Some(*s),
                Kind::Added => None,
            })
            .collect();
        marked.sort_by_key(|s| s.0);
        self.marked.set(marked);
    }

    fn mark_whole(&mut self, now: &Document) {
        let diff = drawing_compare::diff(&self.doc.model, &now.model, TOLERANCE);
        let them = &self.doc.model;
        self.kinds.clear();
        for c in &diff.changed {
            if let Some(u) = them.uid(c.old) {
                self.kinds.insert(u, Kind::Changed(c.old));
            }
        }
        for s in &diff.removed {
            if let Some(u) = them.uid(*s) {
                self.kinds.insert(u, Kind::Removed(*s));
            }
        }
        for s in &diff.added {
            if let Some(u) = now.model.uid(*s) {
                self.kinds.insert(u, Kind::Added);
            }
        }
        self.now_uids = now
            .model
            .entities_with_uids()
            .map(|(u, e)| (Slot(e.base().id), u))
            .collect();
        self.counts = Counts {
            same: diff.same,
            changed: diff.changed.len(),
            added: diff.added.len(),
            removed: diff.removed.len(),
        };
    }

    /// The objects at `slots` (touched since the last marks) compared again; the rest stays as it was.
    fn follow(&mut self, now: &Document, mut slots: Vec<Slot>) {
        slots.sort_by_key(|s| s.0);
        slots.dedup();
        let touched: std::collections::HashSet<Slot> = slots.iter().copied().collect();
        // The ids an edit may have changed: those the slots held and those they hold now.
        let before: std::collections::HashSet<Uuid> = slots
            .iter()
            .filter_map(|s| self.now_uids.get(s).copied())
            .collect();
        let mut ids: Vec<Uuid> = before.iter().copied().collect();
        ids.extend(slots.iter().filter_map(|s| now.model.uid(*s)));
        ids.sort();
        ids.dedup();
        for u in &ids {
            // Was it in the open drawing before? Held by a touched slot then, or by one the edit did not touch.
            let was_now =
                before.contains(u) || now.model.slot_of(*u).is_some_and(|s| !touched.contains(&s));
            if let Some(k) = self.kinds.get(u).copied() {
                self.count(k, -1);
            } else if was_now && self.doc.model.slot_of(*u).is_some() {
                self.counts.same = self.counts.same.saturating_sub(1);
            }
        }
        for s in &slots {
            match now.model.uid(*s) {
                Some(u) => self.now_uids.insert(*s, u),
                None => self.now_uids.remove(s),
            };
        }
        for u in ids {
            let theirs = self.doc.model.slot_of(u);
            let ours = now.model.slot_of(u);
            let kind = match (theirs, ours) {
                (Some(t), Some(o)) => {
                    match drawing_compare::differs(&self.doc.model, t, &now.model, o, TOLERANCE) {
                        Some(true) => Some(Kind::Changed(t)),
                        _ => None,
                    }
                }
                (Some(t), None) => Some(Kind::Removed(t)),
                (None, Some(_)) => Some(Kind::Added),
                (None, None) => {
                    self.kinds.remove(&u);
                    continue;
                }
            };
            match kind {
                Some(k) => {
                    self.kinds.insert(u, k);
                    self.count(k, 1);
                }
                None => {
                    self.kinds.remove(&u);
                    self.counts.same += 1;
                }
            }
        }
    }

    /// The counts and marks a whole comparison makes now, for the tests of the follower.
    #[cfg(test)]
    pub(crate) fn whole(&self, now: &Document) -> (Counts, Vec<Slot>) {
        let diff = drawing_compare::diff(&self.doc.model, &now.model, TOLERANCE);
        let mut marked: Vec<Slot> = diff
            .changed
            .iter()
            .map(|c| c.old)
            .chain(diff.removed.iter().copied())
            .collect();
        marked.sort_by_key(|s| s.0);
        let counts = Counts {
            same: diff.same,
            changed: diff.changed.len(),
            added: diff.added.len(),
            removed: diff.removed.len(),
        };
        (counts, marked)
    }

    fn count(&mut self, kind: Kind, by: isize) {
        let n = match kind {
            Kind::Changed(_) => &mut self.counts.changed,
            Kind::Removed(_) => &mut self.counts.removed,
            Kind::Added => &mut self.counts.added,
        };
        *n = n.saturating_add_signed(by);
    }
}

impl Revision {
    /// The open drawing `now`'s selection in this state, by the objects' lasting ids, made again only when it changed.
    pub(crate) fn choose(&mut self, now: &Document, selection: &Selection) {
        let key = (now.session, now.model.generation(), selection.version());
        if self.chosen_for == Some(key) {
            return;
        }
        self.chosen_for = Some(key);
        let slots: Vec<kentos_domain::Slot> = selection
            .ids()
            .iter()
            .filter_map(|s| now.model.uid(*s))
            .filter_map(|u| self.doc.model.slot_of(u))
            .collect();
        self.chosen.set(slots);
    }
}

impl App {
    /// The view `i` that is not active shows `doc` as a revision of the open drawing (splitting first); read only.
    pub(crate) fn show_revision(&mut self, i: usize, name: String, doc: Document, source: Source) {
        if self.document.is_none() {
            return;
        }
        if self.split.is_none() {
            self.toggle_split();
        }
        let mut revision = Revision::new(name.clone(), doc, source);
        if let Some(now) = &self.document {
            revision.mark(now);
        }
        let c = revision.counts;
        let Some(other) = self.split.as_mut().and_then(|s| s.others.get_mut(i)) else {
            return;
        };
        other.revision = Some(revision);
        // Its scene and its text are the revision's now; a theme belongs to the open drawing's states.
        other.viewport.rebuild();
        other.viewport.theme = None;
        other.reset_text();
        self.say(
            Level::Info,
            format!(
                "Görünüm {name}: salt okunur. Açık çizimde {} nesne eklendi, {} silindi, {} değişti; silinenler ve değişenler işaretli.",
                c.added, c.removed, c.changed
            ),
        );
    }

    /// The view `i` shows the open drawing again.
    pub(crate) fn hide_revision(&mut self, i: usize) {
        let Some(other) = self.split.as_mut().and_then(|s| s.others.get_mut(i)) else {
            return;
        };
        if other.revision.take().is_some() {
            other.viewport.rebuild();
            other.reset_text();
            self.say(Level::Info, "Görünüm yine açık çizimi gösteriyor.");
        }
    }

    /// The open drawing `steps` undo steps back in the view `i`.
    pub(crate) fn revision_from_history(&mut self, i: usize, steps: usize) {
        let Some(doc) = &self.document else {
            return;
        };
        match doc.model.past(steps) {
            Some((model, label)) => {
                let name = format!("{steps} adım geri, “{label}” öncesi");
                self.show_revision(i, name, Document::of_model(model), Source::History(steps));
            }
            None => self.warn("Geri alma geçmişi bu kadar adım tutmuyor."),
        }
    }

    /// A drawing file in the view `i`.
    pub(crate) fn revision_from_file(&mut self, i: usize, path: &Path) {
        match Document::read(path) {
            Ok(doc) => {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.show_revision(i, name, doc, Source::File);
            }
            Err(why) => self.warn(why),
        }
    }

    /// A revision or checkpoint downloaded to `path`: read, removed, and shown in the first view that is not active
    /// (the catalog goes).
    pub(crate) fn revision_from_download(&mut self, name: &str, path: &Path) {
        let read = Document::read(path);
        let _ = std::fs::remove_file(path);
        match read {
            Ok(doc) => {
                self.cloud.catalog = None;
                self.dialog = None;
                self.show_revision(0, name.to_owned(), doc, Source::File);
            }
            Err(why) => self.warn(why),
        }
    }

    /// “Dosyadan…”: the file chosen asks the window, for the view `i`.
    pub(crate) fn pick_revision_file(&self, i: usize) -> Task<Message> {
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Görünümde gösterilecek çizim")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            move |picked: Option<PathBuf>| {
                Message::Split(crate::split_view::Event::RevisionPicked(i, picked))
            },
        )
    }

    /// The marks follow the open drawing (after every message).
    pub(crate) fn follow_revision(&mut self) {
        let (Some(split), Some(doc)) = (self.split.as_mut(), self.document.as_ref()) else {
            return;
        };
        for o in &mut split.others {
            if let Some(revision) = o.revision.as_mut() {
                revision.mark(doc);
                revision.choose(doc, &self.selection);
            }
        }
    }
}
