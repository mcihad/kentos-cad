//! Parsel kayıtları (docs/adr/4220; `cad.parcels.records`, `cad.parcels.record`): the drawing's parcel register, opened
//! from Parsel alan denetimi. The summary counts the current and closed parcels and what the drawing draws otherwise; a
//! choice shows the parcels (identity, documents, the stated, recorded and drawn areas, the state, the neighbours), the
//! documents (what each made and closed) or the boundaries (corners, length, documents, parcels, whether drawn), one
//! table at a time; Kapananlar da adds the closed ones. Yeni belge writes one document: it makes parcels from the areas
//! selected when Parsel alan denetimi opened and closes the current parcels pressed in the table (marked in Kapat). Its
//! plan is said as the form is filled; Belgeyi kaydet writes one undo step and the window shows the register after it.

use iced::{Element, Length, Task};
use kentos_commands::{CommandResult, ParcelDocumentInput, ParcelToRecord, ParcelsRecord, ParcelsRecorded, ParcelsRecords, ParcelsRecordsListed};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, parcel_register};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet_chosen};
use crate::traces::Control;

pub const TITLE: &str = "Parsel kayıtları";
/// Parsel alan denetimi's button that opens it.
pub const OPEN: &str = "Parsel kayıtları…";
const CLOSE: &str = "Kapat";
pub const SAVE: &str = "Belgeyi kaydet";
pub const NAME: &str = "Belge adı";
pub const KIND: &str = "Türü";
pub const DATE: &str = "Tarihi";
pub const NOTE: &str = "Not";
pub const HISTORY: &str = "Kapananlar da";

/// Which rows the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Parcels,
    Documents,
    Boundaries,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Parcels => "Parseller",
            Show::Documents => "Belgeler",
            Show::Boundaries => "Sınırlar",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub show: Show,
    /// Kapananlar da: the closed parcels and boundaries in the tables too.
    pub history: bool,
    /// The register with its history, read against the drawing.
    pub answer: Option<ParcelsRecordsListed>,
    /// The areas the new document makes: those Parsel alan denetimi opened with.
    pub create: Vec<String>,
    /// The current parcels the new document closes, as pressed in the table.
    pub close: Vec<u32>,
    pub name: String,
    pub kind: String,
    pub date: String,
    pub note: String,
    /// What Belgeyi kaydet would write, or why it would not.
    pub plan: Option<Result<ParcelsRecorded, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Show(Show),
    History(bool),
    /// A row of the parcels' table pressed: its parcel goes into Kapat, or out of it.
    Toggle(usize),
    Name(String),
    Kind(String),
    Date(String),
    Note(String),
    Save,
    Close,
}

fn msg(e: Event) -> Message {
    Message::ParcelRegister(e)
}

/// A parcel's state in words.
pub fn state_word(state: &str) -> &'static str {
    match state {
        "current" => "Kayıttaki gibi",
        "changed" => "Çizimde değişti",
        "missing" => "Alanı yok",
        "closed" => "Kapandı",
        _ => "—",
    }
}

/// `Ada/Parsel` of a parcel number, else `#n`.
fn label(r: &ParcelsRecordsListed, n: u32) -> String {
    r.parcels.iter().find(|p| p.number == n).map_or_else(|| format!("#{n}"), |p| format!("{}/{}", p.ada, p.parsel))
}

fn labels(r: &ParcelsRecordsListed, list: &[u32]) -> String {
    if list.is_empty() { "—".into() } else { list.iter().map(|n| label(r, *n)).collect::<Vec<_>>().join(", ") }
}

fn document_name(r: &ParcelsRecordsListed, n: u32) -> String {
    r.documents.iter().find(|d| d.number == n).map_or_else(|| format!("#{n}"), |d| d.name.clone())
}

/// The rows of the parcels' table as shown (the closed ones only with `history`).
fn shown_parcels(r: &ParcelsRecordsListed, history: bool) -> Vec<&kentos_commands::RecordedParcelRow> {
    r.parcels.iter().filter(|p| history || p.closed_by.is_none()).collect()
}

/// The table's rows for `show`.
pub fn rows(r: &ParcelsRecordsListed, show: Show, history: bool, close: &[u32], f: &kentos_interaction::Format) -> Vec<Vec<String>> {
    match show {
        Show::Parcels => shown_parcels(r, history)
            .into_iter()
            .map(|p| {
                let state = match p.closed_by {
                    Some(c) => format!("Kapandı: {}", document_name(r, c)),
                    None => state_word(&p.state).to_owned(),
                };
                vec![
                    p.number.to_string(),
                    format!("{}/{}", p.ada, p.parsel),
                    p.mahalle.clone(),
                    document_name(r, p.created_by),
                    state,
                    p.stated_area.clone().unwrap_or_else(|| "—".into()),
                    f.area(p.recorded_area),
                    p.area.map_or_else(|| "—".into(), |a| f.area(a)),
                    labels(r, &p.neighbours),
                    if close.contains(&p.number) { "✓".into() } else { String::new() },
                ]
            })
            .collect(),
        Show::Documents => r
            .documents
            .iter()
            .map(|d| {
                vec![
                    d.number.to_string(),
                    d.name.clone(),
                    if d.kind.is_empty() { "—".into() } else { d.kind.clone() },
                    if d.date.is_empty() { "—".into() } else { d.date.clone() },
                    labels(r, &d.created),
                    labels(r, &d.closed),
                ]
            })
            .collect(),
        Show::Boundaries => r
            .boundaries
            .iter()
            .filter(|b| history || b.closed_by.is_none())
            .map(|b| {
                vec![
                    b.number.to_string(),
                    format!("{}, {}", f.coord(b.from[0]), f.coord(b.from[1])),
                    format!("{}, {}", f.coord(b.to[0]), f.coord(b.to[1])),
                    if b.bulge == 0.0 { f.length(b.length) } else { format!("{} yay", f.length(b.length)) },
                    document_name(r, b.created_by),
                    labels(r, &b.parcels),
                    match (b.closed_by, b.drawn) {
                        (Some(c), _) => format!("Kapandı: {}", document_name(r, c)),
                        (None, true) => "Kayıttaki yerinde".into(),
                        (None, false) => "Çizimde değişti".into(),
                    },
                ]
            })
            .collect(),
    }
}

/// What a document does, in words.
pub fn plan_words(r: Option<&ParcelsRecordsListed>, created: &[String], out: &ParcelsRecorded, name: &str) -> String {
    let mut parts = Vec::new();
    if !out.created.is_empty() {
        let names = if created.is_empty() { String::new() } else { format!(" ({})", created.join(", ")) };
        parts.push(format!("{} parsel oluşturur{names}", out.created.len()));
    }
    if !out.closed.is_empty() {
        let names = r.map(|r| format!(" ({})", labels(r, &out.closed))).unwrap_or_default();
        parts.push(format!("{} parsel kapatır{names}", out.closed.len()));
    }
    let mut lines = Vec::new();
    if out.boundaries_created > 0 {
        lines.push(format!("{} yeni sınır", out.boundaries_created));
    }
    if out.boundaries_shared > 0 {
        lines.push(format!("{} kayıtlı sınır paylaşılır", out.boundaries_shared));
    }
    if out.boundaries_closed > 0 {
        lines.push(format!("{} sınır kapanır", out.boundaries_closed));
    }
    let tail = if lines.is_empty() { String::new() } else { format!("; {}", lines.join(", ")) };
    format!("Belge {} “{name}”: {}{tail}.", out.document, parts.join(", "))
}

impl App {
    fn parcel_register_input(&self) -> ParcelsRecord {
        let s = &self.parcel_register;
        let text = |t: &str| (!t.trim().is_empty()).then(|| t.trim().to_owned());
        ParcelsRecord {
            document: ParcelDocumentInput { name: s.name.trim().to_owned(), kind: text(&s.kind), date: text(&s.date), note: text(&s.note) },
            create: s.create.iter().map(|u| ParcelToRecord { uid: u.clone(), stated_area: None }).collect(),
            close: s.close.clone(),
            expected_revision: None,
        }
    }

    /// The register read again and the new document's plan.
    fn parcel_register_run(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        self.parcel_register.answer = Some(parcel_register::list(&doc.model, &ParcelsRecords { history: true }));
        let s = &self.parcel_register;
        if s.name.trim().is_empty() || s.create.is_empty() && s.close.is_empty() {
            self.parcel_register.plan = None;
            return;
        }
        let input = self.parcel_register_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.parcel_register.plan = Some(match parcel_register::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Önizleme alınamadı.".to_owned()),
        });
    }

    /// The areas the new document makes, by their identity (`Ada/Parsel`) as their attributes write it.
    fn parcel_register_made(&self) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        self.parcel_register
            .create
            .iter()
            .map(|u| {
                kentos_domain::Uuid::parse_str(u)
                    .ok()
                    .and_then(|id| doc.model.slot_of(id))
                    .and_then(|slot| doc.model.get(slot))
                    .map(|e| {
                        let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
                        format!("{}/{}", attr("Ada"), attr("Parsel"))
                    })
                    .unwrap_or_else(|| "?".to_owned())
            })
            .collect()
    }

    /// Parsel kayıtları from Parsel alan denetimi: the new document makes its selected areas.
    pub(crate) fn open_parcel_register(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.parcel_register = State { create: self.parcels_check.uids.clone().unwrap_or_default(), ..State::default() };
        self.parcel_register_run();
        self.dialog = Some(Dialog::ParcelRegister);
        Task::none()
    }

    pub(crate) fn parcel_register_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_parcel_register(),
            Event::Show(v) => self.parcel_register.show = v,
            Event::History(on) => self.parcel_register.history = on,
            Event::Toggle(i) => {
                let s = &self.parcel_register;
                let Some(r) = &s.answer else { return Task::none() };
                if s.show != Show::Parcels {
                    return Task::none();
                }
                // Only a current parcel closes; a closed one's row is history.
                let Some(n) = shown_parcels(r, s.history).get(i).filter(|p| p.closed_by.is_none()).map(|p| p.number) else {
                    return Task::none();
                };
                let close = &mut self.parcel_register.close;
                if let Some(at) = close.iter().position(|c| *c == n) {
                    close.remove(at);
                } else {
                    close.push(n);
                }
                self.parcel_register_run();
            }
            Event::Name(v) => {
                self.parcel_register.name = v;
                self.parcel_register_run();
            }
            Event::Kind(v) => {
                self.parcel_register.kind = v;
                self.parcel_register_run();
            }
            Event::Date(v) => {
                self.parcel_register.date = v;
                self.parcel_register_run();
            }
            Event::Note(v) => {
                self.parcel_register.note = v;
                self.parcel_register_run();
            }
            Event::Save => {
                let input = self.parcel_register_input();
                let made = self.parcel_register_made();
                let before = self.parcel_register.answer.clone();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match parcel_register::execute(&mut ExecutionContext::new(&mut doc.model), input.clone()) {
                    CommandResult::Completed { output, .. } => {
                        let words = plan_words(before.as_ref(), &made, &output, &input.document.name);
                        self.say(Level::Success, format!("{TITLE}: {words} Tek adımda geri alınır."));
                        let keep = (self.parcel_register.show, self.parcel_register.history);
                        self.parcel_register = State { show: keep.0, history: keep.1, ..State::default() };
                        self.parcel_register_run();
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
            }
            Event::Close => {
                self.parcel_register = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn parcel_register_view(&self) -> Element<'_, Message> {
        let s = &self.parcel_register;
        let f = self.format();
        let mut lines = Vec::new();
        let mut table_rows = Vec::new();
        if let Some(r) = &s.answer {
            if r.documents.is_empty() {
                lines.push(words::text_line(
                    Kind::Info,
                    "Çizimde parsel kaydı yok. İlk belge seçili alanlardan parsel oluşturur.",
                ));
            } else {
                let shared = r.boundaries.iter().filter(|b| b.closed_by.is_none() && b.parcels.len() > 1).count();
                let current = r.boundaries.iter().filter(|b| b.closed_by.is_none()).count();
                lines.push(words::text_line(
                    Kind::Ok,
                    format!(
                        "{} belge; {} güncel, {} kapanan parsel; {current} güncel sınır, {shared} tanesi iki parselin.",
                        r.documents.len(),
                        r.current,
                        r.closed
                    ),
                ));
                if r.changed + r.missing + r.moved > 0 {
                    let mut parts = Vec::new();
                    if r.changed > 0 {
                        parts.push(format!("{} parsel çizimde değişti", r.changed));
                    }
                    if r.missing > 0 {
                        parts.push(format!("{} parselin alanı yok", r.missing));
                    }
                    if r.moved > 0 {
                        parts.push(format!("{} sınır kayıttaki yerinde çizilmiyor", r.moved));
                    }
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "Çizim kayıttan ayrılıyor: {}. Kayıt değişmedi; değişikliği yeni bir belgeyle kaydedin.",
                            parts.join(", ")
                        ),
                    ));
                }
            }
            table_rows = rows(r, s.show, s.history, &s.close, &f);
        }
        let cols = match s.show {
            Show::Parcels => vec![
                num("No", 32.0),
                col("Ada/Parsel", 70.0),
                col("Mahalle", 70.0),
                col("Belge", 104.0),
                col("Durum", 140.0),
                num("Belgedeki alan", 86.0),
                num("Kayıttaki alan", 86.0),
                num("Çizimdeki alan", 86.0),
                col("Komşular", 90.0),
                col("Kapat", 46.0),
            ],
            Show::Documents => vec![
                num("No", 36.0),
                col("Ad", 150.0),
                col("Türü", 110.0),
                col("Tarihi", 90.0),
                col("Oluşturduğu", 200.0),
                col("Kapattığı", 200.0),
            ],
            Show::Boundaries => vec![
                num("No", 32.0),
                col("Başı (Y, X)", 186.0),
                col("Sonu (Y, X)", 186.0),
                num("Boy", 96.0),
                col("Belge", 100.0),
                col("Parseller", 90.0),
                col("Çizimde", 140.0),
            ],
        };
        let table = sheet_chosen(cols, table_rows, "Gösterilecek satır yok.", 260.0, None, |i| msg(Event::Toggle(i)));
        let choose = iced::widget::row![
            Segmented::new([Show::Parcels, Show::Documents, Show::Boundaries], s.show, |v| msg(Event::Show(v))),
            iced::widget::Space::new().width(Length::Fill),
            Switch::new(s.history, |on| msg(Event::History(on))).label(HISTORY),
        ]
        .spacing(12)
        .align_y(iced::Center);
        let made = self.parcel_register_made();
        let closing = s.answer.as_ref().map(|r| labels(r, &s.close)).unwrap_or_else(|| "—".into());
        let text = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |v| msg(on(v))).width(Length::Fill)
        };
        let plan = match (&s.plan, s.name.trim().is_empty(), s.create.is_empty() && s.close.is_empty()) {
            (_, _, true) => words::text_line(
                Kind::Info,
                "Oluşturacağı alanları seçip Parsel alan denetimini açın ya da kapatacağı parselleri tabloda seçin.",
            ),
            (_, true, _) => words::text_line(Kind::Info, "Belgenin adını yazın."),
            (Some(Ok(out)), ..) => words::text_line(Kind::Ok, plan_words(s.answer.as_ref(), &made, out, s.name.trim())),
            (Some(Err(why)), ..) => words::text_line(Kind::Error, why.clone()),
            (None, ..) => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let ready = matches!(s.plan, Some(Ok(_)));
        let form = Form::new()
            .label_width(150.0)
            .section("Yeni belge")
            .field(NAME, text("2026/145 ifraz", &s.name, Event::Name))
            .field(KIND, text("Tapu planı, değişiklik …", &s.kind, Event::Kind))
            .field(DATE, text("Belgede yazdığı gibi", &s.date, Event::Date))
            .field(NOTE, text("", &s.note, Event::Note))
            .field("Oluşturduğu", words::text_line(Kind::Info, if made.is_empty() { "—".to_owned() } else { made.join(", ") }))
            .field("Kapattığı", words::text_line(Kind::Info, closing))
            .help("Kimlik ve belgedeki alan (Tapu alanı) alanın özniteliklerinden; kapatılacak güncel parseli tabloda seçin. Kayıt çizimden ayrıdır: alanlar değişmez.")
            .row(plan)
            .row(words::primary(SAVE, ready.then(|| msg(Event::Save))));
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(choose)
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0))
            .push(form);
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(1040.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn parcel_register_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => matches!(self.parcel_register.plan, Some(Ok(_))).then(|| msg(Event::Save)),
            Control::Fill(NAME, v) => Some(msg(Event::Name(v.to_owned()))),
            Control::Fill(KIND, v) => Some(msg(Event::Kind(v.to_owned()))),
            Control::Fill(DATE, v) => Some(msg(Event::Date(v.to_owned()))),
            Control::Fill(NOTE, v) => Some(msg(Event::Note(v.to_owned()))),
            Control::Check(HISTORY, on) => Some(msg(Event::History(on))),
            Control::Row(n) if n > 0 => Some(msg(Event::Toggle(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
