//! Parsel kayıtları (docs/adr/4220, 5020; `cad.parcels.records`, `cad.parcels.record`, `cad.parcels.approve`,
//! `cad.parcels.withdraw`): the drawing's parcel register, opened from Parsel alan denetimi. The summary counts the
//! current and closed parcels, what the drawing draws otherwise, the drafts waiting and the boundaries whose stated
//! length is past the frontage check's limit; a choice shows the parcels (identity, documents, the stated, recorded and
//! drawn areas, the state, the neighbours), the documents and drafts (state, what each made and closed) or the
//! boundaries (corners, length, documents, parcels, whether drawn, the lengths the documents state against it), one
//! table at a time; Kapananlar da adds the closed ones. Yeni belge writes one document, or its draft with Taslak olarak
//! yaz: it makes parcels from the areas selected when Parsel alan denetimi opened (their edge lengths from Kenar
//! ölçüleri (m)) and closes the current parcels pressed in the table (marked in Kapat). A document or draft pressed in
//! Belgeler is chosen: Onayı yaz writes its approval as given outside (a draft then becomes a document), Taslağı geri
//! çek removes a draft. Each plan is said as the form is filled; each write is one undo step.

use iced::{Element, Length, Task};
use kentos_commands::{
    CommandResult, ParcelApprovalInput, ParcelDocumentInput, ParcelToRecord, ParcelsApprove, ParcelsApproved,
    ParcelsRecord, ParcelsRecorded, ParcelsRecords, ParcelsRecordsListed, ParcelsWithdraw,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, parcel_drafts, parcel_register};
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
pub const DRAFT: &str = "Taslak olarak yaz";
pub const REFERENCE: &str = "Onay sayısı";
pub const APPROVAL_DATE: &str = "Onay tarihi";
pub const APPROVER: &str = "Onaylayan";
pub const APPROVE: &str = "Onayı yaz";
pub const WITHDRAW: &str = "Taslağı geri çek";

/// The document or draft pressed in Belgeler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    Document(u32),
    Draft(u32),
}

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
    /// Taslak olarak yaz: the document kept as a draft.
    pub draft: bool,
    /// The document or draft pressed in Belgeler, and its approval as given.
    pub chosen: Option<Chosen>,
    pub reference: String,
    pub approval_date: String,
    pub approver: String,
    /// What Onayı yaz would write, or why it would not.
    pub approve_plan: Option<Result<ParcelsApproved, String>>,
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
    Draft(bool),
    Reference(String),
    ApprovalDate(String),
    Approver(String),
    Save,
    Approve,
    Withdraw,
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
        Show::Documents => {
            let dash = |t: &str| if t.is_empty() { "—".to_owned() } else { t.to_owned() };
            let mut out: Vec<Vec<String>> = r
                .documents
                .iter()
                .map(|d| {
                    vec![
                        d.number.to_string(),
                        d.name.clone(),
                        dash(&d.kind),
                        dash(&d.date),
                        d.approval.as_ref().map_or_else(|| "Kayıtlı".to_owned(), |a| format!("Onaylı: {}", a.reference)),
                        labels(r, &d.created),
                        labels(r, &d.closed),
                    ]
                })
                .collect();
            out.extend(r.drafts.iter().map(|d| {
                vec![
                    format!("T{}", d.number),
                    d.name.clone(),
                    dash(&d.kind),
                    dash(&d.date),
                    "Taslak".to_owned(),
                    if d.create.is_empty() { "—".to_owned() } else { d.create.join(", ") },
                    labels(r, &d.close),
                ]
            }));
            out
        }
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
                    if b.lengths.is_empty() {
                        "—".into()
                    } else {
                        b.lengths.iter().map(|l| l.length.clone()).collect::<Vec<_>>().join("; ")
                    },
                    if b.lengths.is_empty() {
                        "—".into()
                    } else {
                        b.lengths
                            .iter()
                            .map(|l| {
                                let past = if l.within { "" } else { " aşıyor" };
                                format!("{} ({}){past}", f.coord(l.difference), f.coord(l.limit))
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
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
    // Docs/adr/4620: the neighbours' boundaries split at the new corners, and who took the pieces.
    if out.boundaries_split > 0 {
        let names = r.map(|r| format!(" ({} parçaları alır)", labels(r, &out.neighbours))).unwrap_or_default();
        lines.push(format!("{} komşu sınırı yeni köşede bölünür{names}", out.boundaries_split));
    }
    if out.lengths > 0 {
        lines.push(format!("{} kenar ölçüsü", out.lengths));
    }
    let tail = if lines.is_empty() { String::new() } else { format!("; {}", lines.join(", ")) };
    if out.draft {
        return format!(
            "Taslak {} “{name}”: onaylanınca {}{tail}. Güncel kayıt değişmez.",
            out.document,
            parts.join(", ")
        );
    }
    format!("Belge {} “{name}”: {}{tail}.", out.document, parts.join(", "))
}

/// What approving the chosen document or draft does, in words.
pub fn approve_words(r: &ParcelsRecordsListed, chosen: Chosen, out: &ParcelsApproved, reference: &str) -> String {
    match (chosen, &out.recorded) {
        (Chosen::Draft(n), Some(recorded)) => {
            let draft = r.drafts.iter().find(|d| d.number == n);
            let made = draft.map(|d| d.create.clone()).unwrap_or_default();
            let name = draft.map(|d| d.name.clone()).unwrap_or_default();
            format!("Onay {reference} yazılır; taslak belge olur. {}", plan_words(Some(r), &made, recorded, &name))
        }
        _ => format!("“{}” belgesinin onayı yazılır: {reference}. Belge değişmez.", document_name(r, out.document)),
    }
}

impl App {
    fn parcel_register_input(&self) -> ParcelsRecord {
        let s = &self.parcel_register;
        let text = |t: &str| (!t.trim().is_empty()).then(|| t.trim().to_owned());
        ParcelsRecord {
            document: ParcelDocumentInput { name: s.name.trim().to_owned(), kind: text(&s.kind), date: text(&s.date), note: text(&s.note) },
            create: s.create.iter().map(|u| ParcelToRecord { uid: u.clone(), stated_area: None, lengths: None }).collect(),
            close: s.close.clone(),
            draft: s.draft,
            expected_revision: None,
        }
    }

    fn parcel_approve_input(&self) -> Option<ParcelsApprove> {
        let s = &self.parcel_register;
        let text = |t: &str| (!t.trim().is_empty()).then(|| t.trim().to_owned());
        let (draft, document) = match s.chosen? {
            Chosen::Draft(n) => (Some(n), None),
            Chosen::Document(n) => (None, Some(n)),
        };
        Some(ParcelsApprove {
            draft,
            document,
            approval: ParcelApprovalInput {
                reference: s.reference.trim().to_owned(),
                date: text(&s.approval_date),
                by: text(&s.approver),
            },
            expected_revision: None,
        })
    }

    /// The chosen document's or draft's approval plan.
    fn parcel_approve_run(&mut self) {
        let input = self.parcel_approve_input();
        let Some(input) = input.filter(|i| !i.approval.reference.is_empty()) else {
            self.parcel_register.approve_plan = None;
            return;
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.parcel_register.approve_plan =
            Some(match parcel_drafts::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    Err(error.message)
                }
                _ => Err("Önizleme alınamadı.".to_owned()),
            });
    }

    /// The register read again and the new document's plan.
    fn parcel_register_run(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        self.parcel_register.answer = Some(parcel_register::list(&doc.model, &ParcelsRecords { history: true }));
        self.parcel_approve_run();
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

    /// The window after a write: the same table, the forms empty, the register read again.
    fn parcel_register_reset(&mut self) {
        let keep = (self.parcel_register.show, self.parcel_register.history);
        self.parcel_register = State { show: keep.0, history: keep.1, ..State::default() };
        self.parcel_register_run();
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
                if s.show == Show::Documents {
                    // A document or draft pressed is chosen for its approval; pressed again, no longer.
                    let picked = match r.documents.get(i) {
                        Some(d) => Some(Chosen::Document(d.number)),
                        None => r.drafts.get(i - r.documents.len()).map(|d| Chosen::Draft(d.number)),
                    };
                    let state = &mut self.parcel_register;
                    state.chosen = if state.chosen == picked { None } else { picked };
                    self.parcel_approve_run();
                    return Task::none();
                }
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
            Event::Draft(on) => {
                self.parcel_register.draft = on;
                self.parcel_register_run();
            }
            Event::Reference(v) => {
                self.parcel_register.reference = v;
                self.parcel_approve_run();
            }
            Event::ApprovalDate(v) => {
                self.parcel_register.approval_date = v;
                self.parcel_approve_run();
            }
            Event::Approver(v) => {
                self.parcel_register.approver = v;
                self.parcel_approve_run();
            }
            Event::Approve => {
                let (Some(input), Some(chosen)) = (self.parcel_approve_input(), self.parcel_register.chosen) else {
                    return Task::none();
                };
                let before = self.parcel_register.answer.clone().unwrap_or_default();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match parcel_drafts::execute(&mut ExecutionContext::new(&mut doc.model), input.clone()) {
                    CommandResult::Completed { output, .. } => {
                        let words = approve_words(&before, chosen, &output, &input.approval.reference);
                        self.say(Level::Success, format!("{TITLE}: {words} Tek adımda geri alınır."));
                        self.parcel_register_reset();
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
            }
            Event::Withdraw => {
                let Some(Chosen::Draft(n)) = self.parcel_register.chosen else { return Task::none() };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let input = ParcelsWithdraw { draft: n, expected_revision: None };
                match parcel_drafts::withdraw::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        self.say(
                            Level::Success,
                            format!("{TITLE}: “{}” taslağı geri çekildi; güncel kayıt değişmedi. Tek adımda geri alınır.", output.name),
                        );
                        self.parcel_register_reset();
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
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
                        self.parcel_register_reset();
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
            if r.documents.is_empty() && r.drafts.is_empty() {
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
                if r.over_limit > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "{} sınırın belgedeki ölçüsü kayıttaki boyundan cephe denetiminin sınırından (d = 0,05 + \
                             0,001·S m) çok ayrılıyor; Sınırlar tablosunda “aşıyor”.",
                            r.over_limit
                        ),
                    ));
                }
                if !r.drafts.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!(
                            "{} belge taslağı onay bekliyor; taslak güncel parselleri değiştirmez. Belgeler tablosunda \
                             seçip onayını yazın.",
                            r.drafts.len()
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
                col("Ad", 140.0),
                col("Türü", 100.0),
                col("Tarihi", 86.0),
                col("Durum", 130.0),
                col("Oluşturduğu", 180.0),
                col("Kapattığı", 160.0),
            ],
            Show::Boundaries => vec![
                num("No", 30.0),
                col("Başı (Y, X)", 178.0),
                col("Sonu (Y, X)", 178.0),
                num("Boy", 80.0),
                col("Belge", 92.0),
                col("Parseller", 68.0),
                col("Çizimde", 112.0),
                num("Belgede", 62.0),
                col("Fark (sınır)", 136.0),
            ],
        };
        // The chosen document's or draft's row, in Belgeler.
        let chosen_row = match (s.show, s.chosen, &s.answer) {
            (Show::Documents, Some(Chosen::Document(n)), Some(r)) => r.documents.iter().position(|d| d.number == n),
            (Show::Documents, Some(Chosen::Draft(n)), Some(r)) => {
                r.drafts.iter().position(|d| d.number == n).map(|k| r.documents.len() + k)
            }
            _ => None,
        };
        let table = sheet_chosen(cols, table_rows, "Gösterilecek satır yok.", 260.0, chosen_row, |i| msg(Event::Toggle(i)));
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
            .field("", Switch::new(s.draft, |on| msg(Event::Draft(on))).label(DRAFT))
            .help("Kimlik, belgedeki alan (Tapu alanı) ve kenar ölçüleri (Kenar ölçüleri (m): dış halkanın kenarları sırayla, ; ile) alanın özniteliklerinden; kapatılacak güncel parseli tabloda seçin. Taslak güncel kaydı değiştirmez, onayı yazılınca belge olur. Kayıt çizimden ayrıdır: alanlar değişmez.")
            .row(plan)
            .row(words::primary(SAVE, ready.then(|| msg(Event::Save))));
        // The document or draft chosen in Belgeler: its approval as given outside; a draft can be withdrawn. Shown above
        // Yeni belge, beside the table it was chosen in.
        let mut approval = None;
        if let (Some(chosen), Some(r)) = (s.chosen, &s.answer) {
            let (title, approved) = match chosen {
                Chosen::Document(n) => {
                    let d = r.documents.iter().find(|d| d.number == n);
                    (
                        format!("Belge {n} “{}”", d.map(|d| d.name.clone()).unwrap_or_default()),
                        d.and_then(|d| d.approval.as_ref().map(|a| a.reference.clone())),
                    )
                }
                Chosen::Draft(n) => (
                    format!("Taslak T{n} “{}”", r.drafts.iter().find(|d| d.number == n).map(|d| d.name.clone()).unwrap_or_default()),
                    None,
                ),
            };
            let mut form = Form::new().label_width(150.0).section("Seçili belge").field("Belge", words::text_line(Kind::Info, title));
            if let Some(reference) = approved {
                form = form.row(words::text_line(Kind::Ok, format!("Onayı yazılmış: {reference}. Onay bir kez yazılır.")));
            } else {
                let approve_plan = match (&s.approve_plan, s.reference.trim().is_empty()) {
                    (_, true) => words::text_line(Kind::Info, "Onayı veren yerin yazdığı onay sayısını yazın."),
                    (Some(Ok(out)), _) => words::text_line(Kind::Ok, approve_words(r, chosen, out, s.reference.trim())),
                    (Some(Err(why)), _) => words::text_line(Kind::Error, why.clone()),
                    (None, _) => words::text_line(Kind::Info, "Önizleme yok."),
                };
                let can = matches!(s.approve_plan, Some(Ok(_)));
                let mut buttons = iced::widget::row![words::primary(APPROVE, can.then(|| msg(Event::Approve)))].spacing(8);
                if matches!(chosen, Chosen::Draft(_)) {
                    buttons = buttons.push(words::secondary(WITHDRAW, Some(msg(Event::Withdraw))));
                }
                form = form
                    .field(REFERENCE, text("2026/145-K", &s.reference, Event::Reference))
                    .field(APPROVAL_DATE, text("Onayda yazdığı gibi", &s.approval_date, Event::ApprovalDate))
                    .field(APPROVER, text("Kadastro Müdürlüğü", &s.approver, Event::Approver))
                    .help("KentOS onay vermez: onay kontrolü yapan ve onaylayan yerin verdiği onayı yazıldığı gibi kaydeder.")
                    .row(approve_plan)
                    .row(buttons);
            }
            approval = Some(form);
        }
        let mut body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(choose)
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        if let Some(chosen) = approval {
            body = body.push(chosen);
        }
        let body = body.push(form);
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(1120.0)
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
            Control::Check(DRAFT, on) => Some(msg(Event::Draft(on))),
            Control::Fill(REFERENCE, v) => Some(msg(Event::Reference(v.to_owned()))),
            Control::Fill(APPROVAL_DATE, v) => Some(msg(Event::ApprovalDate(v.to_owned()))),
            Control::Fill(APPROVER, v) => Some(msg(Event::Approver(v.to_owned()))),
            Control::Press(APPROVE) => matches!(self.parcel_register.approve_plan, Some(Ok(_))).then(|| msg(Event::Approve)),
            Control::Press(WITHDRAW) => {
                matches!(self.parcel_register.chosen, Some(Chosen::Draft(_))).then(|| msg(Event::Withdraw))
            }
            Control::Row(n) if n > 0 => Some(msg(Event::Toggle(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
