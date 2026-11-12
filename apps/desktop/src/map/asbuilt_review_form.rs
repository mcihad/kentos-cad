//! Uygulama kontrolü › İzleme's İnceleme (docs/adr/5220; `cad.asbuilt.review`): the reviewer's statement typed as the
//! regulation's papers have it (role, title, name, day, “Kontrol edilmiştir” or “Görülmüştür”, how it was signed, a
//! note) and the name the review is kept under. Projede sakla keeps the job as checked with its thresholds' source;
//! Kaldır removes the review of that name. The project's reviews are listed with whether the points their job reads
//! are still the ones reviewed. KentOS writes what the reviewer states; it checks no authority nor signature.

use iced::widget::{container, row};
use iced::{Center, Element};
use kentos_commands::{AsbuiltReviewRow, MonitoringReviewer, ReviewPhrase, ReviewerRole, SignatureKind};

use crate::app::Message;
use crate::road_dialog::{col, num};
use crate::style::fields;

/// The roles' words, in [`ReviewerRole`]'s order.
pub const ROLES: [&str; 4] = ["Sorumlu mühendis", "Kontrol mühendisi", "Kontrolü yapan teknik personel", "Onaylayan"];
/// The phrases over the signature.
pub const PHRASES: [&str; 2] = ["Kontrol edilmiştir", "Görülmüştür"];
/// How it was signed.
pub const SIGNATURES: [&str; 2] = ["Islak imza", "Güvenli elektronik imza"];

/// The form's values, as typed.
#[derive(Clone, Debug, Default)]
pub struct ReviewForm {
    pub name: String,
    pub role: usize,
    pub title: String,
    pub person: String,
    pub date: String,
    pub phrase: usize,
    pub signature: usize,
    pub note: String,
}

#[derive(Clone, Debug)]
pub enum Change {
    Name(String),
    Role(usize),
    Title(String),
    Person(String),
    Date(String),
    Phrase(usize),
    Signature(usize),
    Note(String),
}

impl ReviewForm {
    /// As the window opens: the control engineer, “Kontrol edilmiştir”, a wet signature; nothing typed.
    pub fn new() -> ReviewForm {
        ReviewForm { role: 1, ..ReviewForm::default() }
    }

    pub fn apply(&mut self, c: Change) {
        match c {
            Change::Name(v) => self.name = v,
            Change::Role(v) => self.role = v.min(ROLES.len() - 1),
            Change::Title(v) => self.title = v,
            Change::Person(v) => self.person = v,
            Change::Date(v) => self.date = v,
            Change::Phrase(v) => self.phrase = v.min(PHRASES.len() - 1),
            Change::Signature(v) => self.signature = v.min(SIGNATURES.len() - 1),
            Change::Note(v) => self.note = v,
        }
    }

    /// The reviewer's statement as the command takes it (its words checked there).
    pub fn reviewer(&self) -> MonitoringReviewer {
        let note = self.note.trim().to_owned();
        MonitoringReviewer {
            role: [ReviewerRole::Responsible, ReviewerRole::Control, ReviewerRole::Staff, ReviewerRole::Approver][self.role.min(3)],
            title: self.title.trim().to_owned(),
            name: self.person.trim().to_owned(),
            date: self.date.trim().to_owned(),
            phrase: if self.phrase == 1 { ReviewPhrase::Seen } else { ReviewPhrase::Checked },
            signature: if self.signature == 1 { SignatureKind::Electronic } else { SignatureKind::Wet },
            note: (!note.is_empty()).then_some(note),
        }
    }
}

fn choices(words: &[&str]) -> Vec<(String, String)> {
    words.iter().enumerate().map(|(i, w)| (i.to_string(), (*w).to_owned())).collect()
}

fn input<'a>(value: &'a str, hint: &'a str, width: f32, to: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    container(fields::input(None, hint, value, false, false).on_input(to)).width(width).into()
}

/// The section's rows: each starts with its box; two values of one thing share a row.
pub fn rows<'a>(f: &'a ReviewForm, on: fn(Change) -> Message) -> Vec<(&'static str, Element<'a, Message>)> {
    let pick = |words: &[&str], at: usize, to: fn(usize) -> Change, width: f32| -> Element<'a, Message> {
        container(fields::select_owned(choices(words), &at.to_string(), move |v| on(to(v.parse().unwrap_or(0))))).width(width).into()
    };
    vec![
        ("İncelemenin adı", input(&f.name, "ör. Ekim 2026 incelemesi", 260.0, move |v| on(Change::Name(v)))),
        ("İnceleyen", row![
            pick(&ROLES, f.role, Change::Role, 230.0),
            input(&f.title, "Unvan", 170.0, move |v| on(Change::Title(v))),
            input(&f.person, "Adı soyadı", 170.0, move |v| on(Change::Person(v)))
        ]
        .spacing(8)
        .align_y(Center)
        .into()),
        ("Gün, ibare, imza", row![
            input(&f.date, "YYYY-AA-GG", 110.0, move |v| on(Change::Date(v))),
            pick(&PHRASES, f.phrase, Change::Phrase, 170.0),
            pick(&SIGNATURES, f.signature, Change::Signature, 200.0)
        ]
        .spacing(8)
        .align_y(Center)
        .into()),
        ("Not", input(&f.note, "isteğe bağlı", 560.0, move |v| on(Change::Note(v)))),
    ]
}

/// A kept review's row: name, reviewer, day, phrase, marks in alarm and warning, whether its points are today's.
pub fn cells(r: &AsbuiltReviewRow) -> Vec<String> {
    vec![
        r.name.clone(),
        r.reviewer.clone(),
        r.date.clone(),
        PHRASES[usize::from(r.phrase == ReviewPhrase::Seen)].to_owned(),
        r.alarm.to_string(),
        r.warning.to_string(),
        if r.current { "Güncel".into() } else { "Eski sonuca ait".into() },
    ]
}

/// The project's reviews in the kit's data table.
pub fn table(list: &[AsbuiltReviewRow]) -> Element<'_, Message> {
    let columns = vec![
        col("İnceleme", 190.0),
        col("İnceleyen", 140.0),
        col("Gün", 90.0),
        col("İbare", 140.0),
        num("Alarm", 60.0),
        num("Uyarı", 60.0),
        col("Noktalar", 140.0),
    ];
    crate::road_dialog::sheet(columns, list.iter().map(cells).collect(), "Projede izleme incelemesi yok.", 120.0)
}

/// The table's lead.
pub fn lead(list: &[AsbuiltReviewRow]) -> String {
    let old = list.iter().filter(|r| !r.current).count();
    let mut said = format!("Projedeki izleme incelemeleri: {}.", list.len());
    if old > 0 {
        said.push_str(&format!(" {old} incelemenin okuduğu noktalar sonradan değişti; inceleme eski sonuca aittir."));
    }
    said.push_str(" KentOS inceleyenin beyanını yazar; yetkiyi ve imzayı doğrulamaz.");
    said
}

impl crate::app::App {
    /// The project's reviews with whether their points are today's (docs/adr/5220), read when the form changes.
    pub(crate) fn asbuilt_reviews(&mut self) {
        self.map.asbuilt.reviews = match self.document.as_ref() {
            Some(doc) => kentos_native_application::asbuilt_review::rows(
                &doc.model,
                doc.model.monitoring_reviews(),
            ),
            None => Vec::new(),
        };
    }

    /// Projede sakla (keeps the job checked now under the review's name) or İncelemeyi kaldır, as one undo step.
    pub(crate) fn asbuilt_review(&mut self, remove: bool) {
        use kentos_commands::{AsbuiltReview, CommandResult};
        use kentos_interaction::Level;
        use kentos_native_application::{ExecutionContext, asbuilt_review};
        const TITLE: &str = "İzleme incelemesi";
        let name = self.map.asbuilt.review.name.trim().to_owned();
        let input = if remove {
            AsbuiltReview {
                name,
                check: None,
                thresholds_source: None,
                reviewer: None,
                remove: true,
                expected_revision: None,
            }
        } else {
            let check = match self.asbuilt_input() {
                Ok(c) => c,
                Err(why) => return self.warn(format!("{TITLE}: {why}")),
            };
            let source = self.map.asbuilt.trend.source.trim();
            let thresholds_source =
                (check.thresholds.is_some() && !source.is_empty()).then(|| source.to_owned());
            AsbuiltReview {
                name,
                check: Some(check),
                thresholds_source,
                reviewer: Some(self.map.asbuilt.review.reviewer()),
                remove: false,
                expected_revision: None,
            }
        };
        let Some(doc) = self.document.as_mut() else { return };
        match asbuilt_review::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let said = match &output.kept {
                    Some(k) => {
                        let count = |s: kentos_commands::ReviewedState| k.marks.iter().filter(|m| m.state == Some(s)).count();
                        format!(
                            "“{}” projede {} ({} nokta; {} alarmda, {} uyarıda). Tek adımda geri alınır.",
                            output.name,
                            if output.replaced { "yenisiyle değişti" } else { "saklandı" },
                            k.marks.len(),
                            count(kentos_commands::ReviewedState::Alarm),
                            count(kentos_commands::ReviewedState::Warning)
                        )
                    }
                    None => format!("“{}” kaldırıldı. Tek adımda geri alınır.", output.name),
                };
                self.map.asbuilt.reviews = output.reviews;
                self.say(Level::Success, format!("{TITLE}: {said}"));
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
    }
}
