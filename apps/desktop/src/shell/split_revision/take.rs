//! Farkları al (docs/adr/1000 §5–§6): the revision view's differences in a table the user ticks, and the ticked ones
//! taken into the open drawing by `cad.drawing.take`, one undo step. The state taken from is the revision exactly as
//! the view shows it: written to the app's temporary folder as a `.kcad` and named by its path, so later edits of the
//! open drawing (which move the history's “steps back”) cannot change what is taken.

use std::collections::BTreeSet;
use std::path::PathBuf;

use iced::widget::{column, container, row};
use iced::{Element, Fill, Task};
use kentos_commands::{CompareAgainst, DrawingTake};
use kentos_domain::Uuid;
use kentos_native_application::{ExecutionContext, drawing_take};
use kentos_ui::widget::{
    Dialog as Frame,
    table::{self, Table},
};
use kentos_ui::{label, style};

use super::{Kind, Revision};
use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::split_view;

#[derive(Debug, Clone)]
pub enum Event {
    Open(usize),
    Toggle(usize, Uuid),
    All(usize),
    None(usize),
    Take(usize),
}

fn msg(event: Event) -> Message {
    Message::Split(split_view::Event::Take(event))
}

/// One difference: the object, what it is in the revision and now.
pub(crate) struct Row {
    pub uid: Uuid,
    pub what: &'static str,
    pub label: String,
}

impl Revision {
    /// The differences, by their kind (changed, only in the revision, only now) and then by id.
    pub(crate) fn take_rows(&self, now: &kentos_domain::Document) -> Vec<Row> {
        let mut rows: Vec<Row> = self
            .kinds
            .iter()
            .map(|(uid, kind)| {
                let (what, label) = match kind {
                    Kind::Changed(_) => ("Değişti", super::candidates::object_label(now, &uid.to_string())),
                    Kind::Removed(_) => ("Yalnız sürümde", super::candidates::object_label(&self.doc.model, &uid.to_string())),
                    Kind::Added => ("Yalnız açık çizimde", super::candidates::object_label(now, &uid.to_string())),
                };
                Row { uid: *uid, what, label }
            })
            .collect();
        rows.sort_by(|a, b| a.what.cmp(b.what).then(a.uid.cmp(&b.uid)));
        rows
    }
}

/// Where the revision is written for the command: one file per view, replaced each time.
fn revision_file(session: u64, i: usize) -> PathBuf {
    std::env::temp_dir()
        .join("kentos-revision-take")
        .join(format!("revizyon-{session}-{i}.kcad"))
}

impl App {
    fn take_revision(&self, i: usize) -> Option<&Revision> {
        self.split.as_ref()?.others.get(i)?.revision.as_ref()
    }

    fn take_revision_mut(&mut self, i: usize) -> Option<&mut Revision> {
        self.split.as_mut()?.others.get_mut(i)?.revision.as_mut()
    }

    pub(crate) fn revision_take_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(i) => {
                if let Some(r) = self.take_revision_mut(i) {
                    r.take_chosen.clear();
                    self.dialog = Some(Dialog::RevisionTake(i));
                }
            }
            Event::Toggle(i, uid) => {
                if let Some(r) = self.take_revision_mut(i)
                    && !r.take_chosen.remove(&uid)
                {
                    r.take_chosen.insert(uid);
                }
            }
            Event::All(i) => {
                if let Some(r) = self.take_revision_mut(i) {
                    r.take_chosen = r.kinds.keys().copied().collect();
                }
            }
            Event::None(i) => {
                if let Some(r) = self.take_revision_mut(i) {
                    r.take_chosen.clear();
                }
            }
            Event::Take(i) => self.take_chosen_differences(i),
        }
        Task::none()
    }

    fn take_chosen_differences(&mut self, i: usize) {
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return;
        }
        let Some(session) = self.document.as_ref().map(|d| d.session) else {
            return;
        };
        let Some(r) = self.take_revision(i) else {
            return;
        };
        if r.take_chosen.is_empty() {
            self.warn("Alınacak farkı tablodan işaretleyin.");
            return;
        }
        let uids: Vec<String> = r.take_chosen.iter().map(Uuid::to_string).collect();
        let path = revision_file(session, i);
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .map_err(|e| e.to_string())
            .and_then(|()| kentos_kcad::encode_verified(&r.doc.model.to_snapshot_v2()).map_err(|e| e.to_string()))
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        if let Err(why) = written {
            self.warn(format!(
                "Sürüm geçici klasöre yazılamadı ({why}); farklar alınmadı. Diskte yer olduğunu denetleyip yeniden deneyin."
            ));
            return;
        }
        let input = DrawingTake {
            against: CompareAgainst::File { path: path.to_string_lossy().into_owned() },
            uids,
            base: None,
            overwrite: false,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = drawing_take::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return;
        };
        let n = done.added.len() + done.replaced.len() + done.removed.len();
        self.say(
            kentos_interaction::Level::Success,
            format!(
                "{n} fark alındı: {} geri eklendi, {} değiştirildi, {} silindi. Geri almak için Geri al.",
                done.added.len(),
                done.replaced.len(),
                done.removed.len()
            ),
        );
        if let Some(r) = self.take_revision_mut(i) {
            r.take_chosen.clear();
        }
        self.dialog = None;
    }

    pub(crate) fn revision_take_view(&self, i: usize) -> Element<'_, Message> {
        let (Some(r), Some(now)) = (self.take_revision(i), self.document.as_ref()) else {
            return Frame::new("Farkları al")
                .push(label::body("Revizyon görünümü kapandı."))
                .action(words::primary("Kapat", Some(Message::DialogClosed)))
                .modal(Message::DialogClosed);
        };
        let rows = r.take_rows(&now.model);
        let columns = [
            table::Column::new("").width(28.0),
            table::Column::new("Nesne").width(220.0),
            table::Column::new("Fark").width(170.0),
        ];
        let body_rows: Vec<table::Row<'_, Message>> = rows
            .iter()
            .map(|row| {
                let on = r.take_chosen.contains(&row.uid);
                table::Row::new(
                    [if on { "✓" } else { "" }.to_owned(), row.label.clone(), row.what.to_owned()]
                        .into_iter()
                        .map(|s| label::caption(s).into()),
                )
                .selected(on)
                .on_press(msg(Event::Toggle(i, row.uid)))
            })
            .collect();
        let chosen = r.take_chosen.len();
        let body = column![
            label::body("İşaretlenen nesneler sürümdeki hâliyle açık çizime alınır: yalnız sürümde olan aynı kimlikle geri eklenir, yalnız açık çizimde olan silinir, değişen sürümdeki hâliyle değiştirilir. Tek adımda geri alınır."),
            label::caption("Blok tanımları ve katman görünüşü alınmaz; farklıysa uyarı verilir. Kilitli katmandaki bir nesne bütün işlemi durdurur."),
            row![
                words::secondary("Tümünü işaretle", Some(msg(Event::All(i)))),
                words::secondary("İşaretleri kaldır", Some(msg(Event::None(i)))),
                label::caption(format!("{chosen} / {} fark işaretli", rows.len())),
            ]
            .spacing(12),
            container(
                Table::new(columns)
                    .extend(body_rows)
                    .empty("Sürümle açık çizim arasında model farkı yok.")
                    .min_width(430.0)
                    .height(260.0),
            )
            .width(Fill)
            .style(style::container::field_box),
        ]
        .spacing(12);
        Frame::new("Farkları al")
            .scroll(body)
            .action(words::secondary("Kapat", Some(Message::DialogClosed)))
            .action(words::primary("Seçilenleri al", (chosen > 0).then(|| msg(Event::Take(i)))))
            .width(560.0)
            .modal(Message::DialogClosed)
    }
}

/// The chosen ids of a revision view: set by the table, cleared when it opens and after a take.
pub(crate) type Chosen = BTreeSet<Uuid>;
