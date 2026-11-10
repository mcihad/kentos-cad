//! Katalog geçiş raporu (docs/adr/4940; `map.catalogReport`): what moving the drawing's data from one object catalog
//! (a data profile) to another would do, by `cad.profiles.compare`, read only. The two catalogs are chosen from KentOS's
//! and the project's profiles; the summary is the report's notes and how many rows need a decision; the tables give
//! the classes (followed by their ids, else their field codes), the fields, the value lists' codes and the reconcile
//! decisions recorded as review notes. Karar isteyenleri seç selects the objects of the shown table's rows that need a
//! decision. Nothing is written: Profil göçü and Veri profilini uygula move the data.
//!
//! ```text
//! ┌ Katalog geçiş raporu ───────────────────────────────────────────────────────┐
//! │ Eski katalog     [Harita varlıkları ▾]                                       │
//! │ Yeni katalog     [Belediye harita kataloğu ▾]                                │
//! │ ┌ summary ─────────────────────────────────────────────────────────────────┐│
//! │ [Sınıflar | Alanlar | Kodlar | Kararlar]            [Karar isteyenleri seç] │
//! │ ┌ the chosen table ───────────────────────────────────────────────────────┐ │
//! │                                                                     [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{container, row, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CatalogClassChangeKind, CatalogCodeChangeKind, CatalogDecisionChangeKind, CatalogFieldChangeKind,
    CommandResult, ProfileFileKind, ProfilesCompare, ProfilesCompared, ProfilesList,
};
use kentos_native_application::{ExecutionContext, profile_files, profiles_compare};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Katalog geçiş raporu";
const CLOSE: &str = "Kapat";
pub const SELECT: &str = "Karar isteyenleri seç";
/// The catalog the reconcile decisions are made with: the old one at first.
const FIRST: &str = "harita-varliklari";

/// Which table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Classes,
    Fields,
    Codes,
    Decisions,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::Classes => "Sınıflar",
            Show::Fields => "Alanlar",
            Show::Codes => "Kodlar",
            Show::Decisions => "Kararlar",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The catalogs to choose from (name, title).
    pub catalogs: Vec<(String, String)>,
    pub from: String,
    pub to: String,
    pub show: Show,
    pub answer: Option<Result<ProfilesCompared, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    From(String),
    To(String),
    Show(Show),
    Select,
    Close,
}

fn msg(e: Event) -> Message {
    Message::CatalogReport(e)
}

fn class_word(k: CatalogClassChangeKind) -> &'static str {
    match k {
        CatalogClassChangeKind::Same => "Aynı",
        CatalogClassChangeKind::Changed => "Değişti",
        CatalogClassChangeKind::Renamed => "Yeniden adlandırıldı",
        CatalogClassChangeKind::Split => "Bölündü",
        CatalogClassChangeKind::Merged => "Birleşti",
        CatalogClassChangeKind::Retired => "Kalktı",
        CatalogClassChangeKind::Added => "Yeni",
    }
}

fn field_word(k: CatalogFieldChangeKind) -> &'static str {
    match k {
        CatalogFieldChangeKind::Removed => "Kalktı",
        CatalogFieldChangeKind::Added => "Yeni",
        CatalogFieldChangeKind::KindChanged => "Türü değişti",
        CatalogFieldChangeKind::ListChanged => "Listesi değişti",
        CatalogFieldChangeKind::NowRequired => "Zorunlu oldu",
    }
}

fn code_word(k: CatalogCodeChangeKind) -> &'static str {
    match k {
        CatalogCodeChangeKind::Removed => "Kalktı",
        CatalogCodeChangeKind::Retired => "Kullanımdan kalktı",
        CatalogCodeChangeKind::Added => "Yeni",
        CatalogCodeChangeKind::Relabelled => "Etiketi değişti",
    }
}

fn decision_word(k: CatalogDecisionChangeKind) -> &'static str {
    match k {
        CatalogDecisionChangeKind::Kept => "Korunur",
        CatalogDecisionChangeKind::Follows => "Sınıfıyla sürer",
        CatalogDecisionChangeKind::FieldDropped => "Alanı kalkar",
        CatalogDecisionChangeKind::ClassRetired => "Sınıfı kalkar",
        CatalogDecisionChangeKind::ClassSplit => "Sınıfı bölünür",
    }
}

fn yes(b: bool) -> String {
    if b { "Karar ister".to_owned() } else { String::new() }
}

impl App {
    fn catalog_report_drawing(&self) -> Option<String> {
        self.document.as_ref().and_then(|d| d.path.as_ref()).map(|p| p.display().to_string())
    }

    /// KentOS's and the project's profiles that hold, by name and title.
    fn catalog_report_catalogs(&self) -> Vec<(String, String)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let drawing = self.catalog_report_drawing();
        profile_files::list::run(&doc.model.settings().profile_files, &ProfilesList { drawing })
            .items
            .into_iter()
            .filter(|p| p.kind == ProfileFileKind::Profile && p.problem.is_none())
            .map(|p| (p.name, p.title))
            .collect()
    }

    fn catalog_report_run(&mut self) {
        let input = ProfilesCompare {
            from: Some(self.catalog_report.from.clone()),
            to: Some(self.catalog_report.to.clone()),
            drawing: self.catalog_report_drawing(),
            ..ProfilesCompare::default()
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.catalog_report.answer =
            Some(match profiles_compare::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Katalog geçiş raporu çıkarılamadı.".to_owned()),
            });
    }

    /// Katalog geçiş raporu (`map.catalogReport`): Harita varlıkları to the first other catalog, at first.
    pub(crate) fn open_catalog_report(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        let catalogs = self.catalog_report_catalogs();
        let from = if catalogs.iter().any(|(n, _)| n == FIRST) {
            FIRST.to_owned()
        } else {
            catalogs.first().map(|(n, _)| n.clone()).unwrap_or_default()
        };
        // A project's catalog first: the institution's new one; else another of KentOS's.
        let to = catalogs
            .iter()
            .rev()
            .find(|(n, _)| *n != from)
            .map_or_else(|| from.clone(), |(n, _)| n.clone());
        self.catalog_report = State { catalogs, from, to, ..Default::default() };
        self.catalog_report_run();
        self.dialog = Some(Dialog::CatalogReport);
        Task::none()
    }

    /// The persistent ids of the shown table's rows that need a decision.
    fn catalog_report_deciding(&self) -> Vec<String> {
        let Some(Ok(r)) = &self.catalog_report.answer else { return Vec::new() };
        match self.catalog_report.show {
            Show::Classes => r.classes.iter().filter(|c| c.decide).flat_map(|c| c.uids.clone()).collect(),
            Show::Fields => r.fields.iter().filter(|c| c.decide).flat_map(|c| c.uids.clone()).collect(),
            Show::Codes => r.codes.iter().filter(|c| c.decide).flat_map(|c| c.uids.clone()).collect(),
            Show::Decisions => r
                .decisions
                .iter()
                .filter(|d| d.decide)
                .flat_map(|d| d.asset.clone().into_iter().chain(std::iter::once(d.candidate.clone())))
                .collect(),
        }
    }

    pub(crate) fn catalog_report_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::From(v) => self.catalog_report.from = v,
            Event::To(v) => self.catalog_report.to = v,
            Event::Show(v) => {
                self.catalog_report.show = v;
                return Task::none();
            }
            Event::Select => {
                let uids = self.catalog_report_deciding();
                let slots: Vec<_> = self.document.as_ref().map_or_else(Vec::new, |d| {
                    uids.iter()
                        .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                        .filter_map(|u| d.model.slot_of(u))
                        .collect()
                });
                if slots.is_empty() {
                    self.output(format!("{TITLE}: bu tabloda karar isteyen nesne yok."));
                } else {
                    let n = slots.len();
                    self.selection.set(slots);
                    self.output(format!("{TITLE}: karar isteyen {n} nesne seçildi."));
                }
                return Task::none();
            }
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.catalog_report_run();
        Task::none()
    }

    /// The summary's lines and the chosen table's rows.
    pub(crate) fn catalog_report_lines(&self) -> (Vec<Element<'_, Message>>, Vec<Vec<String>>) {
        let s = &self.catalog_report;
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let r = match &s.answer {
            None => return (lines, rows),
            Some(Err(why)) => {
                lines.push(words::text_line(Kind::Error, why.clone()));
                return (lines, rows);
            }
            Some(Ok(r)) => r,
        };
        let moved = r.classes.iter().filter(|c| c.kind != CatalogClassChangeKind::Same).count();
        lines.push(words::text_line(
            if r.decide > 0 { Kind::Warn } else { Kind::Ok },
            format!(
                "{} ({}. sürüm) → {} ({}. sürüm): {} sınıf ({} değişiyor), {} alan, {} kod ve {} kayıtlı karar satırı; karar isteyen satır {}.",
                r.from.title,
                r.from.release,
                r.to.title,
                r.to.release,
                r.classes.len(),
                moved,
                r.fields.len(),
                r.codes.len(),
                r.decisions.len(),
                r.decide
            ),
        ));
        for n in &r.notes {
            lines.push(words::text_line(Kind::Info, n.clone()));
        }
        match s.show {
            Show::Classes => {
                for c in &r.classes {
                    rows.push(vec![
                        c.from.clone().unwrap_or_default(),
                        class_word(c.kind).to_owned(),
                        c.to.join(", "),
                        c.codes.join(", "),
                        c.objects.to_string(),
                        yes(c.decide),
                        c.note.clone(),
                    ]);
                }
            }
            Show::Fields => {
                for f in &r.fields {
                    rows.push(vec![
                        format!("{} › {}", f.class, f.field),
                        field_word(f.kind).to_owned(),
                        f.was.clone(),
                        f.becomes.clone(),
                        f.objects.to_string(),
                        yes(f.decide),
                        f.note.clone(),
                    ]);
                }
            }
            Show::Codes => {
                for c in &r.codes {
                    rows.push(vec![
                        format!("{} › {}", c.domain, c.code),
                        code_word(c.kind).to_owned(),
                        c.label.clone(),
                        c.objects.to_string(),
                        yes(c.decide),
                        c.note.clone(),
                    ]);
                }
            }
            Show::Decisions => {
                for d in &r.decisions {
                    rows.push(vec![
                        d.number.to_string(),
                        d.class.clone(),
                        decision_word(d.kind).to_owned(),
                        d.to.join(", "),
                        yes(d.decide),
                        d.note.clone(),
                    ]);
                }
            }
        }
        (lines, rows)
    }

    pub(crate) fn catalog_report_view(&self) -> Element<'_, Message> {
        let s = &self.catalog_report;
        let choices: Vec<(String, String)> = s.catalogs.iter().map(|(n, t)| (n.clone(), t.clone())).collect();
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Eski katalog",
                container(fields::select_owned(choices.clone(), &s.from, |v| msg(Event::From(v)))).width(320),
            )
            .field("Yeni katalog", container(fields::select_owned(choices, &s.to, |v| msg(Event::To(v)))).width(320))
            .help("Sınıflar kalıcı kimlikleriyle, yoksa saha kodlarıyla izlenir. Rapor çizime yazmaz; veriyi Profil göçü ve Veri profilini uygula taşır.");
        let (lines, rows) = self.catalog_report_lines();
        let table = match s.show {
            Show::Classes => sheet(
                vec![col("Sınıf", 120.0), col("Durum", 140.0), col("Yeni sınıf", 170.0), col("Kodlar", 70.0), num("Nesne", 55.0), col("Karar", 85.0), col("Açıklama", 540.0)],
                rows,
                "Sınıf yok.",
                300.0,
            ),
            Show::Fields => sheet(
                vec![col("Sınıf › alan", 180.0), col("Durum", 120.0), col("Önceki", 140.0), col("Yeni", 140.0), num("Nesne", 55.0), col("Karar", 85.0), col("Açıklama", 460.0)],
                rows,
                "Alan değişikliği yok.",
                300.0,
            ),
            Show::Codes => sheet(
                vec![col("Liste › kod", 200.0), col("Durum", 140.0), col("Etiket", 170.0), num("Nesne", 55.0), col("Karar", 85.0), col("Açıklama", 530.0)],
                rows,
                "Kod değişikliği yok.",
                300.0,
            ),
            Show::Decisions => sheet(
                vec![num("Not", 50.0), col("Sınıf", 130.0), col("Durum", 130.0), col("Yeni sınıf", 260.0), col("Karar", 85.0), col("Açıklama", 525.0)],
                rows,
                "Kayıtlı Detay eşleştirme kararı yok.",
                300.0,
            ),
        };
        let tabs = row![
            Segmented::new([Show::Classes, Show::Fields, Show::Codes, Show::Decisions], s.show, |v| msg(Event::Show(v))),
            space::horizontal().width(Fill),
            words::secondary(SELECT, matches!(&s.answer, Some(Ok(r)) if r.decide > 0).then(|| msg(Event::Select))),
        ]
        .spacing(10)
        .align_y(Center);
        let body = Grows::new()
            .spacing(12.0)
            .push(form)
            .push(words::summary(lines))
            .push(tabs)
            .push_grow(table, 300.0);
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(1200.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn catalog_report_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
