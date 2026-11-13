//! İnceleme notları (docs/adr/1000 §6): the drawing's review notes in the kit's data table, filtered by their state,
//! with what each is bound to and whether it went stale since it was written. Every change runs
//! `cad.review.notes.edit` (one undo step “İnceleme notu”); the table reads `cad.review.notes.list`'s answer, so the
//! window and Python and MCP see the same notes and the same staleness. A note is never drawn or plotted.

use std::fmt;

use iced::widget::{column, container, row, scrollable};
use iced::{Element, Fill, Task};
use kentos_commands::{
    ReviewAnchorState, ReviewNoteOp, ReviewNoteState, ReviewNotesEdit, ReviewNotesList, ReviewOrigin, ReviewStatus,
};
use kentos_domain::Uuid;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, review_notes};
use kentos_ui::widget::{
    Dialog as Frame,
    input_field::InputField,
    segmented::Segmented,
    table::{self, Table},
};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;

/// Which notes the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Open,
    Answered,
    Resolved,
    Stale,
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "Tümü",
            Self::Open => "Açık",
            Self::Answered => "Yanıtlandı",
            Self::Resolved => "Çözüldü",
            Self::Stale => "Eskimiş",
        })
    }
}

impl Filter {
    /// Whether the table shows the note under this filter.
    fn passes(self, n: &ReviewNoteState) -> bool {
        match self {
            Filter::All => true,
            Filter::Open => n.note.status == ReviewStatus::Open,
            Filter::Answered => n.note.status == ReviewStatus::Answered,
            Filter::Resolved => n.note.status == ReviewStatus::Resolved,
            Filter::Stale => n.stale,
        }
    }
}

/// A status as the window says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status(pub ReviewStatus);

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(status_words(self.0))
    }
}

fn status_words(s: ReviewStatus) -> &'static str {
    match s {
        ReviewStatus::Open => "Açık",
        ReviewStatus::Answered => "Yanıtlandı",
        ReviewStatus::Resolved => "Çözüldü",
    }
}

/// The window's state: the filter, the note chosen, the texts being written.
#[derive(Debug, Default)]
pub(crate) struct Window {
    pub filter: Filter,
    pub chosen: Option<String>,
    pub text: String,
    pub reply: String,
}

#[derive(Debug, Clone)]
pub enum Event {
    Filter(Filter),
    Pick(String),
    Text(String),
    Reply(String),
    Add,
    SendReply,
    Status(Status),
    Rebind,
    Remove,
    Select,
}

fn msg(event: Event) -> Message {
    Message::ReviewNotes(event)
}

/// Now as UTC ISO 8601 (`2026-11-05T18:30:00Z`), from the system clock: the host gives the note its time.
/// A recorded decision (docs/adr/4140) in a line: whose, the candidate's source, what it binds.
pub(crate) fn decision_words(d: &kentos_commands::ReviewDecision) -> String {
    let whose = if d.command == kentos_commands::MAP_FEATURES_RECONCILE { "Detay eşleştirme" } else { d.command.as_str() };
    let bound = if d.asset.is_some() { "aday ve yazılan varlık bağlı" } else { "aday bağlı" };
    format!("Kayıt: {whose} kararı · kaynak {} · {bound}", d.source)
}

pub(crate) fn iso_utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Days to a civil date (H. Hinnant's algorithm, proleptic Gregorian).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

fn author() -> Option<String> {
    std::env::var("USER").ok().map(|u| u.trim().to_owned()).filter(|u| !u.is_empty() && u.chars().count() <= 120)
}

fn anchor_words(n: &ReviewNoteState) -> String {
    if n.note.anchors.is_empty() {
        return match (&n.note.sheet, &n.note.view) {
            (Some(s), _) => format!("Pafta {s}"),
            (None, Some(v)) => format!("Görünüş {v}"),
            _ => "—".into(),
        };
    }
    format!("{} nesne", n.note.anchors.len())
}

fn stale_words(n: &ReviewNoteState) -> String {
    if !n.stale {
        return "Güncel".into();
    }
    let count = |s: ReviewAnchorState| n.anchor_states.iter().filter(|a| a.state == s).count();
    let mut parts = Vec::new();
    let (changed, block, gone) =
        (count(ReviewAnchorState::Changed), count(ReviewAnchorState::BlockChanged), count(ReviewAnchorState::Removed));
    if changed > 0 {
        parts.push(format!("{changed} nesne değişti"));
    }
    if block > 0 {
        parts.push(format!("{block} blok tanımı değişti"));
    }
    if gone > 0 {
        parts.push(format!("{gone} nesne silindi"));
    }
    if n.sheet_missing {
        parts.push("pafta yok".into());
    }
    if n.view_missing {
        parts.push("görünüş yok".into());
    }
    format!("Eskimiş: {}", parts.join(", "))
}

impl App {
    pub(crate) fn open_review_notes(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let first = self.review_listed().notes.first().map(|n| n.note.id.clone());
        self.review_window = Some(Window { chosen: first, ..Window::default() });
        self.dialog = Some(Dialog::ReviewNotes);
    }

    pub(crate) fn review_listed(&self) -> kentos_commands::ReviewNotesListed {
        self.document
            .as_ref()
            .map(|d| review_notes::list(&d.model, &ReviewNotesList::default()))
            .unwrap_or_default()
    }

    fn review_edit(&mut self, change: ReviewNoteOp) -> Option<kentos_commands::ReviewNotesEdited> {
        let doc = self.document.as_mut()?;
        let result = review_notes::execute(
            &mut ExecutionContext::new(&mut doc.model),
            ReviewNotesEdit { change, expected_revision: None },
        );
        self.answered(result)
    }

    /// The chosen note while the table shows it: a note the filter hides is neither detailed nor acted on, so a
    /// state change, reply or removal never reaches a row the user cannot see.
    pub(crate) fn review_visible_chosen(&self) -> Option<String> {
        let w = self.review_window.as_ref()?;
        let id = w.chosen.as_ref()?;
        self.review_listed()
            .notes
            .iter()
            .find(|n| &n.note.id == id && w.filter.passes(n))
            .map(|n| n.note.id.clone())
    }

    pub(crate) fn review_notes_event(&mut self, event: Event) -> Task<Message> {
        let visible = self.review_visible_chosen();
        let Some(w) = self.review_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Filter(f) => {
                w.filter = f;
                // The choice follows the filter: kept while shown, else the first note shown (none on an empty table).
                let listed = self.review_listed();
                if let Some(w) = self.review_window.as_mut() {
                    let keeps = w
                        .chosen
                        .as_ref()
                        .is_some_and(|id| listed.notes.iter().any(|n| &n.note.id == id && f.passes(n)));
                    if !keeps {
                        w.chosen = listed.notes.iter().find(|n| f.passes(n)).map(|n| n.note.id.clone());
                    }
                }
            }
            Event::Pick(id) => w.chosen = Some(id),
            Event::Text(t) => w.text = t,
            Event::Reply(t) => w.reply = t,
            Event::Add => {
                let text = w.text.trim().to_owned();
                let anchors: Vec<String> = match self.document.as_ref() {
                    Some(d) => self.selection.ids().iter().filter_map(|s| d.model.uid(*s)).map(|u| u.to_string()).collect(),
                    None => Vec::new(),
                };
                let change = ReviewNoteOp::Add {
                    id: None,
                    text,
                    anchors,
                    at: None,
                    sheet: None,
                    view: None,
                    revision: None,
                    origin: Some(ReviewOrigin::User),
                    author: author(),
                    created_at: Some(iso_utc_now()),
                };
                if let Some(done) = self.review_edit(change) {
                    self.say(Level::Success, format!("{}. inceleme notu eklendi.", done.number));
                    if let Some(w) = self.review_window.as_mut() {
                        w.text.clear();
                        w.chosen = Some(done.id);
                    }
                }
            }
            Event::SendReply => {
                let Some(id) = visible else {
                    return Task::none();
                };
                let text = w.reply.trim().to_owned();
                let change =
                    ReviewNoteOp::Reply { id, text, author: author(), created_at: Some(iso_utc_now()), status: None };
                if self.review_edit(change).is_some()
                    && let Some(w) = self.review_window.as_mut()
                {
                    w.reply.clear();
                }
            }
            Event::Status(Status(status)) => {
                if let Some(id) = visible {
                    let _ = self.review_edit(ReviewNoteOp::Update { id, text: None, status: Some(status), revision: None });
                }
            }
            Event::Rebind => {
                if let Some(id) = visible
                    && self.review_edit(ReviewNoteOp::Rebind { id, anchors: None }).is_some()
                {
                    self.say(Level::Success, "Not çizimin şimdiki hâline yeniden bağlandı.".to_owned());
                }
            }
            Event::Remove => {
                if let Some(id) = visible
                    && self.review_edit(ReviewNoteOp::Remove { id }).is_some()
                {
                    let filter = self.review_window.as_ref().map(|w| w.filter).unwrap_or_default();
                    let next = self.review_listed().notes.iter().find(|n| filter.passes(n)).map(|n| n.note.id.clone());
                    if let Some(w) = self.review_window.as_mut() {
                        w.chosen = next;
                    }
                }
            }
            Event::Select => {
                let Some(id) = visible else {
                    return Task::none();
                };
                let listed = self.review_listed();
                let Some(n) = listed.notes.iter().find(|n| n.note.id == id) else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let slots: Vec<_> = n
                    .note
                    .anchors
                    .iter()
                    .filter_map(|a| Uuid::parse_str(&a.uid).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                if slots.is_empty() {
                    self.warn("Notun bağlı olduğu nesne çizimde yok.");
                    return Task::none();
                }
                self.selection.set(slots);
                self.zoom_selection();
            }
        }
        Task::none()
    }

    pub(crate) fn review_notes_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.review_window else {
            return iced::widget::text("").into();
        };
        let listed = self.review_listed();
        let shown: Vec<&ReviewNoteState> = listed.notes.iter().filter(|n| w.filter.passes(n)).collect();
        let filters = Segmented::new(
            [Filter::All, Filter::Open, Filter::Answered, Filter::Resolved, Filter::Stale],
            w.filter,
            |f| msg(Event::Filter(f)),
        );
        let counts = label::caption(format!(
            "{} açık · {} yanıtlandı · {} çözüldü · {} eskimiş",
            listed.open, listed.answered, listed.resolved, listed.stale
        ));
        let columns = [
            table::Column::new("No").width(44.0).align_right(),
            table::Column::new("Durum").width(96.0),
            table::Column::new("Metin").width(250.0),
            table::Column::new("Bağ").width(80.0),
            table::Column::new("Eskime").width(190.0),
        ];
        let rows: Vec<table::Row<'_, Message>> = shown
            .iter()
            .map(|n| {
                let first_line = n.note.text.lines().next().unwrap_or("").to_owned();
                let cells: [Element<'_, Message>; 5] = [
                    label::caption(n.note.number.to_string()).into(),
                    label::caption(status_words(n.note.status)).into(),
                    label::caption(first_line).into(),
                    label::caption(anchor_words(n)).into(),
                    if n.stale {
                        label::caption(stale_words(n)).style(style::text::warning).into()
                    } else {
                        label::caption(stale_words(n)).into()
                    },
                ];
                table::Row::new(cells)
                    .selected(w.chosen.as_deref() == Some(n.note.id.as_str()))
                    .on_press(msg(Event::Pick(n.note.id.clone())))
            })
            .collect();
        let chosen = w
            .chosen
            .as_ref()
            .and_then(|id| listed.notes.iter().find(|n| &n.note.id == id && w.filter.passes(n)));
        // A note whose state just took it out of the filter: said, its details and actions gone with its row.
        let hidden = w
            .chosen
            .as_ref()
            .and_then(|id| listed.notes.iter().find(|n| &n.note.id == id && !w.filter.passes(n)));
        let selected = self.selection.len();
        let add_row = row![
            container(
                InputField::new("Yeni not metni", &w.text)
                    .on_input(|t| msg(Event::Text(t)))
                    .on_submit(msg(Event::Add))
                    .id("inceleme-yeni-not"),
            )
            .width(Fill),
            words::primary(
                "Seçimden not ekle",
                (!w.text.trim().is_empty()).then(|| msg(Event::Add)),
            ),
        ]
        .spacing(8);
        let mut body = column![
            label::body("Notlar tasarımdan ayrıdır: çizilmez, basılmaz, ölçülmez. Seçili nesnelere bağlanan not, nesne sonra değişirse eskimiş görünür."),
            row![filters, counts].spacing(16),
            container(
                Table::new(columns)
                    .extend(rows)
                    .empty("Bu süzgeçte not yok.")
                    .min_width(660.0)
                    .height(170.0),
            )
            .width(Fill)
            .style(style::container::field_box),
            add_row,
            label::caption(if selected == 0 {
                "Seçim yok: not bağsız eklenir.".to_owned()
            } else {
                format!("Not {selected} seçili nesneye bağlanacak.")
            }),
        ]
        .spacing(10);
        if let Some(n) = hidden {
            body = body.push(label::caption(format!(
                "{}. not bu süzgeçte görünmüyor ({}); ayrıntısı ve işlemleri için Tümü'ne geçin.",
                n.note.number,
                status_words(n.note.status).to_lowercase()
            )));
        }
        if let Some(n) = chosen {
            let mut detail = column![label::body(format!("{}. not", n.note.number)), label::caption(n.note.text.clone())]
                .spacing(4);
            let mut meta = Vec::new();
            if let Some(a) = &n.note.author {
                meta.push(a.clone());
            }
            if let Some(t) = &n.note.created_at {
                meta.push(t.clone());
            }
            if let Some(r) = &n.note.revision {
                meta.push(format!("Revizyon {r}"));
            }
            match n.note.origin {
                ReviewOrigin::Ai => meta.push("Kaynak: AI".into()),
                ReviewOrigin::Pdf => meta.push("Kaynak: PDF".into()),
                ReviewOrigin::User => {}
            }
            if !meta.is_empty() {
                detail = detail.push(label::caption(meta.join(" · ")));
            }
            if n.stale {
                detail = detail.push(label::caption(stale_words(n)).style(style::text::warning));
            }
            // Docs/adr/4140: the decision a command recorded with the note.
            if let Some(d) = &n.note.decision {
                detail = detail.push(label::caption(decision_words(d)));
            }
            for r in &n.note.replies {
                let who = r.author.clone().unwrap_or_else(|| "—".into());
                detail = detail.push(label::caption(format!("↳ {who}: {}", r.text)));
            }
            body = body.push(
                container(scrollable(detail).height(84.0)).padding([6, 10]).width(Fill).style(style::container::field_box),
            );
            body = body.push(
                row![
                    container(
                        InputField::new("Yanıt", &w.reply)
                            .on_input(|t| msg(Event::Reply(t)))
                            .on_submit(msg(Event::SendReply))
                            .id("inceleme-yanit"),
                    )
                    .width(Fill),
                    words::secondary("Yanıtla", (!w.reply.trim().is_empty()).then(|| msg(Event::SendReply))),
                ]
                .spacing(8),
            );
            body = body.push(
                row![
                    Segmented::new(
                        [Status(ReviewStatus::Open), Status(ReviewStatus::Answered), Status(ReviewStatus::Resolved)],
                        Status(n.note.status),
                        |s| msg(Event::Status(s)),
                    ),
                    words::secondary("Nesneleri seç", (!n.note.anchors.is_empty()).then(|| msg(Event::Select))),
                    words::secondary("Yeniden bağla", n.stale.then(|| msg(Event::Rebind))),
                    words::secondary("Sil", Some(msg(Event::Remove))),
                ]
                .spacing(8),
            );
        }
        Frame::new("İnceleme notları")
            .scroll(body)
            .action(words::primary("Kapat", Some(Message::DialogClosed)))
            .width(720.0)
            .modal(Message::DialogClosed)
    }
}

#[cfg(test)]
#[path = "../tests/review_window_tests.rs"]
mod tests;
