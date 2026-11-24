//! Teslim kaydı (docs/adr/8099; plan/bim-plan.md BIM-15; `arch.deliveries`): which package of the project went to which
//! authority, when and how, and what came back. The deliveries on the left (DESIGN.md §7.9 Liste ve ayrıntı); on the
//! right the chosen one: its package and record, the authority's answer with the review notes that bind its reasons to
//! objects, and its package beside the previous delivery's to the same authority. Every change of the record is
//! `cad.delivery.edit` (one undo step “Teslim kaydı”); a note bound to the selection is `cad.review.notes.edit`. The
//! comparison reads the two publication folders off the window's thread and writes nothing
//! (`kentos_native_application::delivery_compare`).
//!
//! ```text
//! ┌ Teslim kaydı ─────────────────────────────────────────────────────────────────────────────────┐
//! │ Yayımlanan paketin idareye teslimini ve idarenin cevabını tutar; iadenin gerekçeleri …         │
//! │ ┌ No Rev. Kurum              Gün        Cevap ┐│ 2. teslim · Rev. 001                [Kaydı sil] │
//! │ │ 1  000  Sivas Belediyesi   2.11.2026  İade  ││ Paket    3f9a1c2b7d10… · 12 dosya · çizim 37    │
//! │ │ 2  001  Sivas Belediyesi   16.11.2026 —     ││ Kurum    Sivas Belediyesi                       │
//! │ ├─────────────────────────────────────────────┤│ İdarenin cevabı ──────────────────────────────  │
//! │ │ [Teslim ekle]                               ││ Sonuç    (Kabul | İade)  …        [Cevabı yaz]  │
//! │ └─────────────────────────────────────────────┘│ Öncekiyle karşılaştır ────────────────────────  │
//! │ 2. teslimin cevabı yazıldı.                                                           [Kapat] │
//! └────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use crate::arch::ArchState;

use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    CommandResult, Delivery, DeliveryChange, DeliveryChannel, DeliveryEdit, DeliveryEdited, DeliveryOutcome,
    ReviewNoteOp, ReviewNotesEdit, ReviewOrigin,
};
use kentos_native_application::delivery_compare::{self, Compared, FileChange, Publication};
use kentos_native_application::{ExecutionContext, delivery_edit, review_notes};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Choice, Dialog as Frame, Form, ListDetails, Segmented, Select, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Teslim kaydı";
const LEAD: &str = "Yayımlanan paketin idareye teslimini ve idarenin cevabını tutar; iadenin gerekçeleri inceleme \
                    notlarıyla nesnelere bağlanır, düzeltilmiş paket öncekiyle karşılaştırılır.";
pub const ADD: &str = "Teslim ekle";
pub const PICK: &str = "Klasör seç…";
pub const SAVE: &str = "Teslimi kaydet";
pub const CANCEL: &str = "Vazgeç";
pub const REMOVE: &str = "Kaydı sil";
pub const RESPOND: &str = "Cevabı yaz";
pub const BIND: &str = "Seçilenlere bağla";
pub const COMPARE: &str = "Öncekiyle karşılaştır";
const CLOSE: &str = "Kapat";
/// The body's height (12 px text).
const BODY: f32 = 470.0;
/// The list's width.
const LIST: f32 = 360.0;
/// The label column of the forms (the kit's own).
const LABELS: f32 = 110.0;
/// The most notes the answer's form offers besides the ones it holds; the rest are in İnceleme notları.
const NOTES_SHOWN: usize = 12;

/// A day as the window writes it: `2026-11-02` as 2.11.2026; another text as it is.
pub fn day(iso: &str) -> String {
    let mut parts = iso.split('-');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d), None) if y.len() == 4 => {
            format!("{}.{}.{y}", d.trim_start_matches('0'), m.trim_start_matches('0'))
        }
        _ => iso.to_owned(),
    }
}

/// A day typed as GG.AA.YYYY or YYYY-AA-GG, as the record keeps it; none when it is not a day.
fn typed(text: &str) -> Option<String> {
    kentos_extensions::ui_panel::typed_date(text)
}

fn today() -> String {
    day(&kentos_plot::book::today())
}

/// Who publishes, as the machine names its user; empty when it does not.
fn user() -> String {
    std::env::var("USER").map(|u| u.trim().to_owned()).unwrap_or_default()
}

/// The answer as the segmented choice names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome(pub DeliveryOutcome);

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.label())
    }
}

/// A new delivery as it is written: the publication folder read, the record's fields.
#[derive(Clone, Debug, Default)]
pub struct Draft {
    pub read: Option<Result<Publication, String>>,
    pub revision: String,
    pub to: String,
    pub channel: DeliveryChannel,
    pub at: String,
    pub by: String,
    pub receipt: String,
}

/// The chosen delivery's answer as it is written, and a new note's text to bind to the selection.
#[derive(Clone, Debug)]
pub struct Answer {
    pub outcome: DeliveryOutcome,
    pub at: String,
    pub reason: String,
    pub notes: Vec<String>,
    pub receipt: String,
    pub note: String,
}

impl Answer {
    /// The answer `d` has, or a blank one dated today.
    fn of(d: Option<&Delivery>) -> Self {
        match d.and_then(|d| d.response.as_ref()) {
            Some(r) => Self {
                outcome: r.outcome,
                at: day(&r.at),
                reason: r.reason.clone().unwrap_or_default(),
                notes: r.notes.clone(),
                receipt: r.receipt.clone().unwrap_or_default(),
                note: String::new(),
            },
            None => Self {
                outcome: DeliveryOutcome::Accepted,
                at: today(),
                reason: String::new(),
                notes: Vec::new(),
                receipt: String::new(),
                note: String::new(),
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct Window {
    /// The chosen delivery, by id.
    pub chosen: Option<String>,
    /// A new delivery while Teslim ekle writes it.
    pub draft: Option<Draft>,
    pub answer: Answer,
    /// The last comparison: the delivery it is for and what it said; none while it runs.
    pub compared: Option<(String, Arc<Result<Compared, String>>)>,
    /// Whether a comparison runs.
    pub comparing: bool,
    /// What the last action said.
    pub said: Option<(Kind, String)>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    Add,
    Pick,
    Picked(Option<PathBuf>),
    Revision(String),
    To(String),
    Channel(usize),
    At(String),
    By(String),
    Receipt(String),
    Save,
    Cancel,
    Remove,
    Outcome(Outcome),
    AnswerAt(String),
    Reason(String),
    Toggle(String),
    AnswerReceipt(String),
    Note(String),
    Bind,
    Respond,
    Compare,
    Compared(String, Arc<Result<Compared, String>>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::Deliveries(e))
}

/// A revision number one more than `r`'s, three digits; `r` itself when it is not one.
fn next(r: &str) -> String {
    r.parse::<u32>().ok().filter(|_| r.len() == 3).map_or_else(|| r.to_owned(), |n| format!("{:03}", n + 1))
}

/// A new delivery's revision: the last delivery's, one more when it came back returned; 000 for the first.
fn next_revision(last: Option<&Delivery>) -> String {
    match last {
        None => "000".to_owned(),
        Some(d) if d.response.as_ref().is_some_and(|r| r.outcome == DeliveryOutcome::Returned) => next(&d.revision),
        Some(d) => d.revision.clone(),
    }
}

/// A removal: the red-outlined button (DESIGN.md §7.9).
fn remove<'a>(words: &'a str, on: Message) -> Element<'a, Message> {
    iced::widget::button(label::body(words))
        .on_press(on)
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::danger_outline)
        .into()
}

/// A text cut to `most` characters, “…” after.
fn cut(text: &str, most: usize) -> String {
    let line = text.lines().next().unwrap_or("");
    if line.chars().count() <= most && !text.contains('\n') {
        return line.to_owned();
    }
    format!("{}…", line.chars().take(most.saturating_sub(1)).collect::<String>())
}

impl App {
    /// The drawing's delivery record.
    pub(crate) fn deliveries(&self) -> &[Delivery] {
        self.document.as_ref().map_or(&[][..], |d| d.model.deliveries())
    }

    /// Teslim kaydı (`arch.deliveries`): the record, its last delivery chosen.
    pub(crate) fn open_deliveries(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let last = self.deliveries().last().cloned();
        self.modules.arch_mut().deliveries = Some(Window {
            chosen: last.as_ref().map(|d| d.id.clone()),
            draft: None,
            answer: Answer::of(last.as_ref()),
            compared: None,
            comparing: false,
            said: None,
        });
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::Deliveries));
    }

    fn deliveries_said(&mut self, kind: Kind, words: impl Into<String>) {
        if let Some(w) = self.modules.arch_mut().deliveries.as_mut() {
            w.said = Some((kind, words.into()));
        }
    }

    /// `cad.delivery.edit` on the open drawing; its refusal's words.
    fn delivery_edit(&mut self, change: DeliveryChange) -> Result<DeliveryEdited, String> {
        let Some(doc) = self.document.as_mut() else {
            return Err("Açık çizim yok.".into());
        };
        let input = DeliveryEdit { change, expected_revision: None };
        match delivery_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => Err("Teslim kaydı yazılmadı.".into()),
        }
    }

    pub(crate) fn deliveries_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Close => {
                self.modules.arch_mut().deliveries = None;
                self.dialog = None;
            }
            Event::Pick => return self.deliveries_pick(),
            Event::Save => self.deliveries_save(),
            Event::Remove => self.deliveries_remove(),
            Event::Respond => self.deliveries_respond(),
            Event::Bind => self.deliveries_bind(),
            Event::Compare => return self.deliveries_compare(),
            other => self.deliveries_form(other),
        }
        Task::none()
    }

    /// A choice or a box of the window.
    fn deliveries_form(&mut self, event: Event) {
        let list = self.deliveries().to_vec();
        let Some(w) = self.modules.arch_mut().deliveries.as_mut() else {
            return;
        };
        match event {
            Event::Choose(id) => {
                w.answer = Answer::of(list.iter().find(|d| d.id == id));
                w.chosen = Some(id);
                w.draft = None;
                w.said = None;
            }
            Event::Add => {
                let last = list.iter().max_by_key(|d| d.number);
                w.draft = Some(Draft {
                    read: None,
                    revision: next_revision(last),
                    to: last.map(|d| d.to.clone()).unwrap_or_default(),
                    channel: last.map_or(DeliveryChannel::Epys, |d| d.channel),
                    at: today(),
                    by: user(),
                    receipt: String::new(),
                });
                w.said = None;
            }
            Event::Cancel => {
                w.draft = None;
                w.said = None;
            }
            Event::Picked(Some(path)) => {
                if let Some(d) = w.draft.as_mut() {
                    d.read = Some(delivery_compare::read(&path));
                }
            }
            Event::Revision(t) => w.draft.iter_mut().for_each(|d| d.revision = t.clone()),
            Event::To(t) => w.draft.iter_mut().for_each(|d| d.to = t.clone()),
            Event::Channel(k) => {
                if let (Some(d), Some(c)) = (w.draft.as_mut(), DeliveryChannel::ALL.get(k)) {
                    d.channel = *c;
                }
            }
            Event::At(t) => w.draft.iter_mut().for_each(|d| d.at = t.clone()),
            Event::By(t) => w.draft.iter_mut().for_each(|d| d.by = t.clone()),
            Event::Receipt(t) => w.draft.iter_mut().for_each(|d| d.receipt = t.clone()),
            Event::Outcome(o) => w.answer.outcome = o.0,
            Event::AnswerAt(t) => w.answer.at = t,
            Event::Reason(t) => w.answer.reason = t,
            Event::AnswerReceipt(t) => w.answer.receipt = t,
            Event::Note(t) => w.answer.note = t,
            Event::Toggle(id) => match w.answer.notes.iter().position(|n| *n == id) {
                Some(i) => {
                    w.answer.notes.remove(i);
                }
                None => w.answer.notes.push(id),
            },
            Event::Compared(id, result) => {
                if w.chosen.as_ref() == Some(&id) {
                    w.comparing = false;
                    w.said = match &*result {
                        Ok(_) => None,
                        Err(_) => Some((Kind::Warn, "Karşılaştırılamadı; nedeni karşılaştırmanın yerinde.".into())),
                    };
                    w.compared = Some((id, result));
                }
            }
            Event::Picked(None)
            | Event::Close
            | Event::Pick
            | Event::Save
            | Event::Remove
            | Event::Respond
            | Event::Bind
            | Event::Compare => {}
        }
    }

    /// Klasör seç…: the publication folder of the package that went.
    fn deliveries_pick(&mut self) -> Task<Message> {
        if let crate::app::Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(Some(path.clone()))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Teslim edilen paketin yayın klasörünü seçin")
                    .pick_folder()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |p| msg(Event::Picked(p)),
        )
    }

    /// Teslimi kaydet: the new delivery written.
    fn deliveries_save(&mut self) {
        let Some(draft) = self.modules.arch().deliveries.as_ref().and_then(|w| w.draft.clone()) else {
            return;
        };
        let Some(Ok(publication)) = &draft.read else {
            self.deliveries_said(Kind::Warn, "Önce teslim edilen paketin yayın klasörünü seçin.");
            return;
        };
        let Some(at) = typed(&draft.at) else {
            self.deliveries_said(Kind::Warn, format!("“{}” bir gün değil: GG.AA.YYYY yazın.", draft.at.trim()));
            return;
        };
        let change = DeliveryChange::Add {
            id: None,
            revision: draft.revision.trim().to_owned(),
            package: publication.package.clone(),
            to: draft.to.trim().to_owned(),
            channel: draft.channel,
            at,
            by: Some(draft.by.clone()),
            receipt: Some(draft.receipt.clone()),
        };
        match self.delivery_edit(change) {
            Ok(done) => {
                if let Some(w) = self.modules.arch_mut().deliveries.as_mut() {
                    w.draft = None;
                    w.chosen = Some(done.id);
                    w.answer = Answer::of(None);
                    w.compared = None;
                    w.said = Some((Kind::Ok, format!("{}. teslim kaydedildi.", done.number)));
                }
            }
            Err(why) => self.deliveries_said(Kind::Warn, why),
        }
    }

    /// Kaydı sil: the chosen record taken away (one undo step); its notes stay.
    fn deliveries_remove(&mut self) {
        let Some(id) = self.modules.arch().deliveries.as_ref().and_then(|w| w.chosen.clone()) else {
            return;
        };
        match self.delivery_edit(DeliveryChange::Remove { id }) {
            Ok(done) => {
                let last = self.deliveries().last().cloned();
                if let Some(w) = self.modules.arch_mut().deliveries.as_mut() {
                    w.chosen = last.as_ref().map(|d| d.id.clone());
                    w.answer = Answer::of(last.as_ref());
                    w.compared = None;
                    w.said = Some((Kind::Ok, format!("{}. teslim kaydı silindi; inceleme notları duruyor.", done.number)));
                }
            }
            Err(why) => self.deliveries_said(Kind::Warn, why),
        }
    }

    /// Cevabı yaz: the authority's answer to the chosen delivery, in place of the one it has.
    fn deliveries_respond(&mut self) {
        let Some(w) = self.modules.arch().deliveries.as_ref() else {
            return;
        };
        let Some(id) = w.chosen.clone() else {
            return;
        };
        let a = w.answer.clone();
        let Some(at) = typed(&a.at) else {
            self.deliveries_said(Kind::Warn, format!("“{}” bir gün değil: GG.AA.YYYY yazın.", a.at.trim()));
            return;
        };
        let change = DeliveryChange::Respond {
            id,
            outcome: a.outcome,
            at,
            reason: Some(a.reason),
            notes: a.notes,
            receipt: Some(a.receipt),
        };
        match self.delivery_edit(change) {
            Ok(done) => self.deliveries_said(Kind::Ok, format!("{}. teslimin cevabı yazıldı.", done.number)),
            Err(why) => self.deliveries_said(Kind::Warn, why),
        }
    }

    /// Seçilenlere bağla: a review note of the typed text bound to the objects selected, held in the answer.
    fn deliveries_bind(&mut self) {
        let Some(w) = self.modules.arch().deliveries.as_ref() else {
            return;
        };
        let text = w.answer.note.trim().to_owned();
        let revision = w.chosen.as_ref().and_then(|id| self.deliveries().iter().find(|d| &d.id == id)).map(|d| d.revision.clone());
        if text.is_empty() {
            self.deliveries_said(Kind::Warn, "Notun metnini yazın: gerekçenin seçili nesnelerdeki karşılığı.");
            return;
        }
        let anchors: Vec<String> = match self.document.as_ref() {
            Some(d) => self.selection.ids().iter().filter_map(|s| d.model.uid(*s)).map(|u| u.to_string()).collect(),
            None => Vec::new(),
        };
        if anchors.is_empty() {
            self.deliveries_said(Kind::Warn, "Önce çizimde gerekçenin ilgili olduğu nesneleri seçin.");
            return;
        }
        let objects = anchors.len();
        let author = Some(user()).filter(|u| !u.is_empty() && u.chars().count() <= 120);
        let change = ReviewNoteOp::Add {
            id: None,
            text,
            anchors,
            at: None,
            sheet: None,
            view: None,
            revision,
            origin: Some(ReviewOrigin::User),
            author,
            created_at: Some(crate::review_window::iso_utc_now()),
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = review_notes::execute(
            &mut ExecutionContext::new(&mut doc.model),
            ReviewNotesEdit { change, expected_revision: None },
        );
        match result {
            CommandResult::Completed { output, .. } => {
                if let Some(w) = self.modules.arch_mut().deliveries.as_mut() {
                    w.answer.notes.push(output.id);
                    w.answer.note.clear();
                    w.said = Some((
                        Kind::Ok,
                        format!("{}. inceleme notu {objects} nesneye bağlandı; cevapla birlikte yazın.", output.number),
                    ));
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.deliveries_said(Kind::Warn, error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Öncekiyle karşılaştır: the chosen delivery's package beside the previous one's to the same authority, read off
    /// the window's thread.
    fn deliveries_compare(&mut self) -> Task<Message> {
        let list = self.deliveries();
        let Some(id) = self.modules.arch().deliveries.as_ref().and_then(|w| w.chosen.clone()) else {
            return Task::none();
        };
        let Some(chosen) = list.iter().find(|d| d.id == id).cloned() else {
            return Task::none();
        };
        let Some(previous) = chosen.previous(list).cloned() else {
            self.deliveries_said(Kind::Info, format!("“{}” kurumuna bundan önce teslim yok.", chosen.to));
            return Task::none();
        };
        let wanted: Vec<String> = previous.response.as_ref().map(|r| r.notes.clone()).unwrap_or_default();
        let notes: Vec<_> = self
            .document
            .as_ref()
            .map(|d| d.model.settings().review_notes.iter().filter(|n| wanted.contains(&n.id)).cloned().collect())
            .unwrap_or_default();
        if let Some(w) = self.modules.arch_mut().deliveries.as_mut() {
            w.compared = None;
            w.comparing = true;
            w.said = Some((Kind::Info, "Paketler karşılaştırılıyor…".into()));
        }
        Task::perform(async move { delivery_compare::compare(&chosen, &previous, &notes) }, move |r| {
            msg(Event::Compared(id.clone(), Arc::new(r)))
        })
    }

    pub(crate) fn deliveries_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.modules.arch().deliveries else {
            return space::horizontal().into();
        };
        let list = self.deliveries();
        let body: Element<'_, Message> =
            ListDetails::new(deliveries_table(list, w), self.delivery_details(w, list))
                .list_foot(words::secondary(ADD, Some(msg(Event::Add))))
                .list_width(LIST)
                .height(BODY)
                .into();
        let mut foot = row![].spacing(8).align_y(Center);
        if let Some((kind, said)) = &w.said {
            foot = foot.push(container(words::text_line(*kind, said.clone())).width(Length::Shrink));
        }
        let foot = foot.push(space::horizontal().width(Fill)).push(words::secondary(CLOSE, Some(msg(Event::Close))));
        Frame::new(TITLE)
            .push(label::caption(LEAD))
            .scroll(body)
            .footer(foot)
            .width(1120.0)
            .modal(msg(Event::Close))
    }

    /// The right side: the new delivery's form, the chosen delivery, or what to do on an empty record.
    fn delivery_details<'a>(&'a self, w: &'a Window, list: &'a [Delivery]) -> Element<'a, Message> {
        let inner: Element<'a, Message> = if let Some(draft) = &w.draft {
            draft_view(draft)
        } else if let Some(d) = w.chosen.as_ref().and_then(|id| list.iter().find(|d| &d.id == id)) {
            self.chosen_view(w, d, list)
        } else {
            steps()
        };
        scrollable(container(inner).padding(iced::Padding::ZERO.right(14))).height(Fill).into()
    }

    /// The chosen delivery: its record, the answer's form and the comparison with the previous delivery.
    fn chosen_view<'a>(&'a self, w: &'a Window, d: &'a Delivery, list: &'a [Delivery]) -> Element<'a, Message> {
        let head = row![
            label::strong(format!("{}. teslim · Rev. {}", d.number, d.revision)),
            space::horizontal().width(Fill),
            remove(REMOVE, msg(Event::Remove))
        ]
        .spacing(8)
        .align_y(Center);
        let package = &d.package;
        // The record, read only, on the forms' label column.
        let mut facts = Form::new()
            .label_width(scaled(LABELS))
            .field("Kurum", label::body(d.to.clone()))
            .field("Kanal", label::body(d.channel.label()))
            .field("Gün", label::body(day(&d.at)))
            .field(
                "Paket",
                label::body(format!(
                    "SHA-256 {}… · {} dosya{}",
                    &package.manifest[..package.manifest.len().min(12)],
                    package.files,
                    package.source.as_ref().map(|s| format!(" · çizim revizyonu {s}")).unwrap_or_default()
                )),
            );
        if let Some(p) = &package.path {
            facts = facts.field("Klasör", label::body(p.clone()));
        }
        if let Some(by) = &d.by {
            facts = facts.field("Yayımlayan", label::body(by.clone()));
        }
        if let Some(r) = &d.receipt {
            facts = facts.field("Makbuz", label::body(r.clone()));
        }
        let answer = self.answer_form(w);
        let compare = self.compare_view(w, d, list);
        column![head, facts, answer, compare].spacing(14).into()
    }

    /// İdarenin cevabı: Kabul or İade, its day, its reason, the notes that bind the reasons to objects, its receipt.
    fn answer_form<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let a = &w.answer;
        let notes = self.document.as_ref().map(|doc| doc.model.settings().review_notes.as_slice()).unwrap_or_default();
        let mut checks = Column::new().spacing(4);
        let held: Vec<_> = notes.iter().filter(|n| a.notes.contains(&n.id)).collect();
        let others: Vec<_> = notes.iter().rev().filter(|n| !a.notes.contains(&n.id)).take(NOTES_SHOWN).collect();
        for n in held.iter().chain(&others) {
            let on = a.notes.contains(&n.id);
            let words = format!("{}. {} · {} nesne", n.number, cut(&n.text, 54), n.anchors.len());
            checks = checks.push(words::check(on, words, Some(msg(Event::Toggle(n.id.clone())))));
        }
        let hidden = notes.len().saturating_sub(held.len() + others.len());
        if notes.is_empty() {
            checks = checks.push(label::caption("Çizimde inceleme notu yok; aşağıdan seçili nesnelere bağlayın."));
        } else if hidden > 0 {
            checks = checks.push(label::caption(format!("{hidden} not daha İnceleme notları'nda.")));
        }
        let returned = a.outcome == DeliveryOutcome::Returned;
        let form = Form::new()
            .label_width(scaled(LABELS))
            .section("İdarenin cevabı")
            .field(
                "Sonuç",
                Segmented::new(
                    [Outcome(DeliveryOutcome::Accepted), Outcome(DeliveryOutcome::Returned)],
                    Outcome(a.outcome),
                    |o| msg(Event::Outcome(o)),
                ),
            )
            .field(
                "Gün",
                text_field("GG.AA.YYYY", &a.at).on_input(|t| msg(Event::AnswerAt(t))).width(scaled(120.0)),
            )
            .field(
                "Gerekçe",
                text_field(if returned { "İdarenin iade gerekçesi" } else { "—" }, &a.reason)
                    .on_input(|t| msg(Event::Reason(t)))
                    .width(scaled(428.0)),
            )
            .help(if returned { "İadede gerekçe zorunlu; nesnelerdeki karşılığı inceleme notudur." } else { "İsteğe bağlı." })
            .field("Notlar", checks)
            .field(
                "Yeni not",
                row![
                    text_field("Gerekçenin seçili nesnelerdeki karşılığı", &a.note)
                        .on_input(|t| msg(Event::Note(t)))
                        .width(scaled(300.0)),
                    words::secondary(BIND, Some(msg(Event::Bind)))
                ]
                .spacing(8)
                .align_y(Center),
            )
            .help("Çizimde nesneleri seçin; not onlara bağlanır, cevap yazılınca gerekçenin parçası olur.")
            .field(
                "Makbuz",
                text_field("Sistemin evrak ya da başvuru numarası", &a.receipt)
                    .on_input(|t| msg(Event::AnswerReceipt(t)))
                    .width(scaled(300.0)),
            )
            .row(words::primary(RESPOND, Some(msg(Event::Respond))));
        form.into()
    }

    /// Öncekiyle karşılaştır: the previous delivery to the same authority and the two packages side by side.
    fn compare_view<'a>(&'a self, w: &'a Window, d: &'a Delivery, list: &'a [Delivery]) -> Element<'a, Message> {
        let mut out = Column::new().spacing(8);
        let Some(previous) = d.previous(list) else {
            return Form::new()
                .label_width(scaled(LABELS))
                .section(COMPARE)
                .wide(label::caption(format!("“{}” kurumuna bundan önce teslim yok.", d.to)))
                .into();
        };
        let before = format!(
            "{}. teslim · Rev. {} · {} · {}",
            previous.number,
            previous.revision,
            day(&previous.at),
            previous.response.as_ref().map_or("cevap bekleniyor", |r| r.outcome.label())
        );
        let action = if w.comparing {
            words::secondary(COMPARE, None)
        } else {
            words::secondary(COMPARE, Some(msg(Event::Compare)))
        };
        let form = Form::new()
            .label_width(scaled(LABELS))
            .section(COMPARE)
            .field("Önceki teslim", label::body(before))
            .row(action);
        out = out.push(form);
        let Some((_, result)) = w.compared.as_ref().filter(|(id, _)| id == &d.id) else {
            return out.into();
        };
        match &**result {
            Err(why) => out = out.push(words::text_line(Kind::Warn, why.clone())),
            Ok(c) => {
                out = out.push(label::body(files_words(&c.files)));
                let mut files = Table::new([
                    Col::new("Dosya").width(Fill),
                    Col::new("Önceki adı").width(scaled(170.0)),
                    Col::new("Durum").width(scaled(86.0)),
                ])
                .empty("Dosyalar aynı.");
                let differ: Vec<_> = c.files.iter().filter(|f| f.change != FileChange::Same).collect();
                for f in &differ {
                    let (name, before) = match (&f.new, &f.old) {
                        (Some(n), Some(o)) if n == o => (n.clone(), "—".to_owned()),
                        (Some(n), Some(o)) => (n.clone(), renamed(o, n)),
                        (Some(n), None) => (n.clone(), "—".to_owned()),
                        (None, Some(o)) => (o.clone(), "—".to_owned()),
                        (None, None) => continue,
                    };
                    files = files.push(Row::new([
                        table::text(name).into(),
                        table::text(before).into(),
                        label::body(f.change.label()).into(),
                    ]));
                }
                // As tall as its rows, six at most; it scrolls past them.
                let rows = differ.len().clamp(1, 6) as f32 + 1.0;
                out = out.push(files.height(rows * table::row_height() + 2.0));
                match &c.drawing {
                    Ok(g) => {
                        let mut line = format!(
                            "Çizim: {} nesne aynı, {} değişti, {} eklendi, {} silindi",
                            g.same, g.changed, g.added, g.removed
                        );
                        if g.project > 0 {
                            line.push_str(&format!("; paftalarda ve görünüşlerde {} fark", g.project));
                        }
                        line.push('.');
                        out = out.push(label::body(line));
                    }
                    Err(why) => out = out.push(words::text_line(Kind::Info, why.clone())),
                }
                if !c.notes.is_empty() {
                    let mut notes = Table::new([
                        Col::new("Not").width(scaled(36.0)).align_right(),
                        Col::new("Gerekçe").width(Fill),
                        Col::new("Nesne").width(scaled(52.0)).align_right(),
                        Col::new("Değişen").width(scaled(64.0)).align_right(),
                        Col::new("Silinen").width(scaled(56.0)).align_right(),
                    ]);
                    for n in &c.notes {
                        notes = notes.push(Row::new([
                            label::body(n.number.to_string()).into(),
                            table::text(n.text.lines().next().unwrap_or("").to_owned()).into(),
                            label::body(n.objects.to_string()).into(),
                            label::body(n.changed.to_string()).into(),
                            label::body(n.removed.to_string()).into(),
                        ]));
                    }
                    out = out.push(label::caption(format!(
                        "{}. teslimin iade gerekçelerinin nesneleri, iki paketin çizimleri arasında:",
                        c.previous.1
                    )));
                    out = out.push(notes);
                }
            }
        }
        out.into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn deliveries_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.modules.arch().deliveries.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(PICK) => Some(msg(Event::Pick)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(RESPOND) => Some(msg(Event::Respond)),
            Control::Press(BIND) => Some(msg(Event::Bind)),
            Control::Press(COMPARE) => Some(msg(Event::Compare)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The files' changes in words, the kinds there are: “Dosyalar: 1 aynı, 2 değişti.”
fn files_words(files: &[delivery_compare::FileCompared]) -> String {
    let parts: Vec<String> = [FileChange::Same, FileChange::Changed, FileChange::Renamed, FileChange::Added, FileChange::Removed]
        .into_iter()
        .map(|k| (k, files.iter().filter(|f| f.change == k).count()))
        .filter(|(_, n)| *n > 0)
        .map(|(k, n)| format!("{n} {}", k.label().to_lowercase()))
        .collect();
    if parts.is_empty() {
        return "Paketlerde dosya yok.".to_owned();
    }
    format!("Dosyalar: {}.", parts.join(", "))
}

/// A file's previous name beside its new one: from the separator before the first difference when the two share a
/// long beginning (the model's EK-2 name differs in its revision: “…_000.ifc”), whole otherwise.
pub(crate) fn renamed(old: &str, new: &str) -> String {
    let same = old.chars().zip(new.chars()).take_while(|(a, b)| a == b).count();
    if same < 8 {
        return old.to_owned();
    }
    let head: String = old.chars().take(same).collect();
    let from = head.rfind(['_', '-', ' ', '.']).unwrap_or(head.len());
    format!("…{}", &old[from..])
}

/// The record's deliveries: number, revision, authority, day, answer.
fn deliveries_table<'a>(list: &'a [Delivery], w: &'a Window) -> Element<'a, Message> {
    let mut t = Table::new([
        Col::new("No").width(scaled(30.0)).align_right(),
        Col::new("Rev.").width(scaled(40.0)),
        Col::new("Kurum").width(Fill),
        Col::new("Gün").width(scaled(82.0)),
        Col::new("Cevap").width(scaled(50.0)),
    ])
    .empty("Teslim yok.");
    for d in list {
        t = t.push(
            Row::new([
                label::body(d.number.to_string()).into(),
                label::body(d.revision.clone()).into(),
                table::text(d.to.clone()).into(),
                label::body(day(&d.at)).into(),
                label::body(d.response.as_ref().map_or("—", |r| r.outcome.label())).into(),
            ])
            .selected(w.draft.is_none() && w.chosen.as_deref() == Some(d.id.as_str()))
            .on_press(msg(Event::Choose(d.id.clone()))),
        );
    }
    t.into()
}

/// Teslim ekle's form: the publication folder, then the record's fields.
fn draft_view(draft: &Draft) -> Element<'_, Message> {
    let folder: Element<'_, Message> = row![
        words::secondary(PICK, Some(msg(Event::Pick))),
        label::body(match &draft.read {
            Some(Ok(p)) => p.package.path.clone().unwrap_or_default(),
            _ => "—".to_owned(),
        })
    ]
    .spacing(8)
    .align_y(Center)
    .into();
    let mut form = Form::new()
        .label_width(scaled(LABELS))
        .section("Paket")
        .field("Yayın klasörü", folder)
        .help("Paftaları toplu yayınla'nın yazdığı klasör; paket yayin.json'un SHA-256'sıyla tanınır.");
    match &draft.read {
        Some(Ok(p)) => {
            form = form.field(
                "Paket",
                label::body(format!(
                    "SHA-256 {}… · {} dosya{}{}",
                    &p.package.manifest[..12],
                    p.package.files,
                    p.package.source.as_ref().map(|s| format!(" · çizim revizyonu {s}")).unwrap_or_default(),
                    if p.drawing.is_some() { " · çizimi var" } else { "" }
                )),
            );
            if p.drawing.is_none() {
                form = form.help("Paket çizimi taşımıyor; öncekiyle nesne nesne karşılaştırılamaz, dosyaları karşılaştırılır.");
            }
        }
        Some(Err(why)) => form = form.warning(Some(why.as_str())),
        None => {}
    }
    let channel_at = DeliveryChannel::ALL.iter().position(|c| *c == draft.channel);
    let form = form
        .section("Teslim")
        .field(
            "Revizyon",
            text_field("000", &draft.revision).on_input(|t| msg(Event::Revision(t))).width(scaled(80.0)),
        )
        .help("EK-2'nin revizyon numarası: üç rakam (000, 001…).")
        .field(
            "Kurum",
            text_field("Paketin gittiği idare", &draft.to).on_input(|t| msg(Event::To(t))).width(scaled(340.0)),
        )
        .field(
            "Kanal",
            container(
                Select::new(DeliveryChannel::ALL.iter().map(|c| Choice::new(c.label())), channel_at, |k| {
                    msg(Event::Channel(k))
                })
                .searchable(false),
            )
            .width(scaled(220.0)),
        )
        .field("Gün", text_field("GG.AA.YYYY", &draft.at).on_input(|t| msg(Event::At(t))).width(scaled(120.0)))
        .field("Yayımlayan", text_field("—", &draft.by).on_input(|t| msg(Event::By(t))).width(scaled(220.0)))
        .field(
            "Makbuz",
            text_field("Sistemin evrak ya da başvuru numarası", &draft.receipt)
                .on_input(|t| msg(Event::Receipt(t)))
                .width(scaled(300.0)),
        )
        .row(
            row![words::secondary(CANCEL, Some(msg(Event::Cancel))), words::primary(SAVE, Some(msg(Event::Save)))]
                .spacing(8),
        );
    column![label::strong("Yeni teslim"), form].spacing(12).into()
}

/// An empty record's details: what to do, step by step (DESIGN.md §7.5).
fn steps<'a>() -> Element<'a, Message> {
    let list = [
        "Paketi Paftaları toplu yayınla ile yayımlayın.",
        "Soldaki Teslim ekle'ye basın ve yayın klasörünü seçin.",
        "İdarenin cevabını gelince yazın; iadenin gerekçesini nesnelere bağlayın.",
        "Düzeltilmiş paketi yayımlayıp kaydedin ve öncekiyle karşılaştırın.",
    ];
    let mut out = Column::new()
        .spacing(10)
        .push(label::strong("Teslim yok"))
        .push(label::caption("Projenin paketi idareye gidince buraya kaydedilir."));
    for (k, s) in list.iter().enumerate() {
        out = out.push(
            row![container(label::strong((k + 1).to_string())).width(scaled(18.0)), container(label::body(*s)).width(Fill)]
                .spacing(6),
        );
    }
    out.into()
}
