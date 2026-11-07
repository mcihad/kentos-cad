//! Adayları uzlaştır (docs/adr/1760; `map.featuresReconcile`): candidates of map assets from two extractions (a point
//! cloud's, an orthophoto's) reconciled by `map.features.reconcile` through KentOS's `harita-varliklari` profile. The
//! form names the two sources' layers and names, the class and confidence fields, the radius and the reviewer; the
//! summary counts the groups by status; the table lists the groups (class, status, sources, candidates, spread, missing
//! fields, whether it is written). A single's Kabul accepts it, a conflict's Direk/Ağaç… corrects its other
//! candidates' class to it, Reddet rejects the group's candidates with a reason; the decisions stay in the window until
//! Uzlaştır writes them all in one undo step. Harita only.

use iced::widget::{container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CandidateDecision, CandidateSource, CommandResult, FeaturesReconcile, FeaturesReconciled};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_features_reconcile};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{self, col, num};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Adayları uzlaştır";
const CLOSE: &str = "Kapat";
const RUN: &str = "Uzlaştır";
const PROFILE: &str = "harita-varliklari";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's layers: id and name.
    pub layers: Vec<(String, String)>,
    pub sources: [(String, String); 2],
    pub field: String,
    pub confidence: String,
    pub radius: f64,
    pub reviewer: String,
    pub decisions: Vec<CandidateDecision>,
    pub answer: Option<Result<FeaturesReconciled, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Layer(usize, String),
    Name(usize, String),
    Field(String),
    Confidence(String),
    Radius(f64),
    Reviewer(String),
    /// A group's candidates: a single accepted, a conflict's corrected to a class, the group rejected.
    Accept(usize),
    Correct(usize, String),
    Reject(usize),
    Run,
    Close,
}

fn msg(e: Event) -> Message {
    Message::FeaturesReconcile(e)
}

/// A status in words.
fn status_word(s: &str) -> &'static str {
    match s {
        "agreed" => "Uyuşuyor",
        "single" => "Tek kaynak",
        "conflict" => "Sınıf çelişkisi",
        "existing" => "Zaten var",
        _ => "—",
    }
}

impl App {
    fn reconcile_input(&self) -> FeaturesReconcile {
        let s = &self.features_reconcile;
        let some = |t: &str| Some(t.trim().to_owned()).filter(|t| !t.is_empty());
        FeaturesReconcile {
            name: PROFILE.into(),
            drawing: None,
            sources: s
                .sources
                .iter()
                .filter(|(layer, _)| !layer.is_empty())
                .map(|(layer, name)| CandidateSource { layer: layer.clone(), name: name.clone() })
                .collect(),
            field: some(&s.field).filter(|f| f != kentos_commands::DEFAULT_CODE_FIELD),
            confidence: some(&s.confidence),
            radius: s.radius,
            decisions: s.decisions.clone(),
            reviewer: some(&s.reviewer),
            expected_revision: None,
        }
    }

    fn reconcile_run(&mut self) {
        let input = self.reconcile_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.features_reconcile.answer = Some(match map_features_reconcile::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Adaylar uzlaştırılamadı.".to_owned()),
        });
    }

    /// Adayları uzlaştır (`map.featuresReconcile`): the window, its sources guessed by their layers' names.
    pub(crate) fn open_features_reconcile(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers: Vec<(String, String)> = doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        // The candidates' layers by their names: one with LiDAR (or bulut), one with ortofoto.
        let guess = |words: &[&str]| -> String {
            layers
                .iter()
                .find(|(_, name)| {
                    let n = name.to_lowercase();
                    words.iter().any(|w| n.contains(w))
                })
                .map(|(id, _)| id.clone())
                .unwrap_or_default()
        };
        let first = guess(&["lidar", "bulut"]);
        let second = guess(&["ortofoto", "orto"]);
        self.features_reconcile = State {
            layers,
            sources: [(first, "LiDAR".into()), (second, "Ortofoto".into())],
            field: kentos_commands::DEFAULT_CODE_FIELD.into(),
            confidence: "Güven".into(),
            radius: 0.5,
            ..State::default()
        };
        self.reconcile_run();
        self.dialog = Some(Dialog::FeaturesReconcile);
        Task::none()
    }

    fn reconcile_decide(&mut self, group: usize, f: impl Fn(&str, Option<&str>) -> Option<CandidateDecision>) {
        let Some(Ok(answer)) = &self.features_reconcile.answer else { return };
        let Some(g) = answer.groups.get(group) else { return };
        let mut made = Vec::new();
        for uid in &g.candidates {
            let class = answer.candidates.iter().find(|c| c.uid == *uid).and_then(|c| c.class.as_deref());
            if let Some(d) = f(uid, class) {
                made.push(d);
            }
        }
        let s = &mut self.features_reconcile;
        for d in made {
            s.decisions.retain(|x| x.candidate != d.candidate);
            s.decisions.push(d);
        }
        self.reconcile_run();
    }

    pub(crate) fn features_reconcile_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Layer(i, id) => {
                self.features_reconcile.sources[i].0 = id;
                self.features_reconcile.decisions.clear();
                self.reconcile_run();
            }
            Event::Name(i, name) => {
                self.features_reconcile.sources[i].1 = name;
                self.reconcile_run();
            }
            Event::Field(t) => {
                self.features_reconcile.field = t;
                self.reconcile_run();
            }
            Event::Confidence(t) => {
                self.features_reconcile.confidence = t;
                self.reconcile_run();
            }
            Event::Radius(v) => {
                if v > 0.0 {
                    self.features_reconcile.radius = v;
                    self.reconcile_run();
                }
            }
            Event::Reviewer(t) => self.features_reconcile.reviewer = t,
            Event::Accept(g) => self.reconcile_decide(g, |uid, _| {
                Some(CandidateDecision { candidate: uid.to_owned(), accept: true, ..CandidateDecision::default() })
            }),
            Event::Correct(g, class) => self.reconcile_decide(g, |uid, now| {
                (now != Some(class.as_str()))
                    .then(|| CandidateDecision { candidate: uid.to_owned(), class: Some(class.clone()), ..CandidateDecision::default() })
            }),
            Event::Reject(g) => self.reconcile_decide(g, |uid, _| {
                Some(CandidateDecision {
                    candidate: uid.to_owned(),
                    reject: Some("Adayları uzlaştır penceresinde reddedildi.".into()),
                    ..CandidateDecision::default()
                })
            }),
            Event::Run => {
                let input = self.reconcile_input();
                if let Some(doc) = self.document.as_mut() {
                    let result = map_features_reconcile::execute(&mut ExecutionContext::new(&mut doc.model), input);
                    if let Some(out) = self.answered(result) {
                        self.say(
                            Level::Success,
                            format!("{} varlık yazıldı, {} aday işaretlendi.", out.written.len(), out.marked),
                        );
                    }
                }
                self.features_reconcile.decisions.clear();
                self.reconcile_run();
            }
            Event::Close => {
                self.features_reconcile = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn features_reconcile_view(&self) -> Element<'_, Message> {
        let s = &self.features_reconcile;
        let source = |i: usize| {
            row![
                container(fields::select_owned(s.layers.clone(), &s.sources[i].0, move |v| msg(Event::Layer(i, v)))).width(240),
                container(fields::input(None, "Adı", &s.sources[i].1, false, false).on_input(move |v| msg(Event::Name(i, v)))).width(160)
            ]
            .spacing(8)
            .align_y(Center)
        };
        let text = |value: &str, hint: &str, on: fn(String) -> Event| {
            container(fields::input(None, hint, value, false, false).on_input(move |v| msg(on(v)))).width(160)
        };
        let form = Form::new()
            .label_width(130.0)
            .field("Birinci kaynak", source(0))
            .field("İkinci kaynak", source(1))
            .field(
                "Sınıf ve güven",
                row![text(&s.field, "Kod", Event::Field), text(&s.confidence, "Güven alanı", Event::Confidence)].spacing(8).align_y(Center),
            )
            .field(
                "Yarıçap (m)",
                row![
                    NumberInput::new(s.radius, |v| msg(Event::Radius(v))).step(0.1).decimals(2).point().width(160),
                    text(&s.reviewer, "Gözden geçiren", Event::Reviewer)
                ]
                .spacing(8)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut runnable = false;
        let mut table: Element<'_, Message> = label::caption("").into();
        match &s.answer {
            Some(Ok(a)) => {
                let count = |st: &str| a.groups.iter().filter(|g| g.status == st).count();
                runnable = a.groups.iter().any(|g| g.writes || g.status == "existing")
                    || a.candidates.iter().any(|c| c.status == "rejected");
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "{} grup: {} uyuşuyor, {} tek kaynak, {} sınıf çelişkisi, {} zaten var; {} yazılacak.",
                        a.groups.len(),
                        count("agreed"),
                        count("single"),
                        count("conflict"),
                        count("existing"),
                        a.groups.iter().filter(|g| g.writes).count()
                    ),
                ));
                let duplicates = a.candidates.iter().filter(|c| c.status == "duplicate").count();
                let unknown = a.candidates.iter().filter(|c| matches!(c.status.as_str(), "unknown" | "wrongKind")).count();
                let reviewed = a.candidates.iter().filter(|c| c.status == "reviewed").count();
                if duplicates + unknown + reviewed > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!("{duplicates} yinelenen aday, {unknown} sınıfı okunmayan aday; {reviewed} aday daha önce uzlaştırıldı (okunmaz)."),
                    ));
                }
                let missing = a.groups.iter().filter(|g| !g.missing.is_empty()).count();
                if missing > 0 {
                    lines.push(words::text_line(Kind::Warn, format!("{missing} grubun zorunlu alanı eksik: alan verilmeden yazılmaz.")));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Tek kaynaklı aday kabul edilmeden, çelişen aday sınıfı düzeltilmeden kesin varlık olmaz; kararlar Uzlaştır'la yazılır.",
                ));
                // The classes by the profile's titles (the answer names them by id).
                let titles: std::collections::BTreeMap<String, String> = kentos_commands::builtin_profile(PROFILE)
                    .map(|p| p.classes.into_iter().map(|c| (c.id, c.title)).collect())
                    .unwrap_or_default();
                let title = move |id: &str| titles.get(id).cloned().unwrap_or_else(|| id.to_owned());
                let rows = a.groups.clone();
                table = road_dialog::grid_table(
                    vec![
                        col("Sınıf", 110.0),
                        col("Durum", 120.0),
                        col("Kaynaklar", 140.0),
                        num("Aday", 50.0),
                        num("Yayılım (mm)", 100.0),
                        col("Eksik", 90.0),
                        col("Yazılır", 60.0),
                        col("", 170.0),
                    ],
                    rows.len(),
                    move |i, k| {
                        let g = &rows[i];
                        let text = |t: String| -> Element<'_, Message> { kentos_ui::widget::table::text(t).into() };
                        let button = |words: String, event: Event| -> Element<'_, Message> {
                            iced::widget::button(label::caption(words)).style(kentos_ui::style::button::ghost).padding([0, 6]).on_press(msg(event)).into()
                        };
                        match k {
                            0 => text(match &g.class {
                                Some(c) => title(c),
                                None => g.classes.iter().map(|c| title(c)).collect::<Vec<_>>().join(" / "),
                            }),
                            1 => text(status_word(&g.status).to_owned()),
                            2 => text(g.sources.join(", ")),
                            3 => kentos_ui::widget::table::number(g.candidates.len().to_string()).into(),
                            4 => kentos_ui::widget::table::number(format!("{:.0}", g.spread * 1000.0)).into(),
                            5 => text(g.missing.join(", ")),
                            6 => text(if g.writes { "Evet".into() } else { "—".into() }),
                            _ => {
                                let mut actions = row![].spacing(2);
                                match g.status.as_str() {
                                    "single" if !g.writes => actions = actions.push(button("Kabul".into(), Event::Accept(i))),
                                    "conflict" => {
                                        for c in &g.classes {
                                            actions = actions.push(button(title(c), Event::Correct(i, c.clone())));
                                        }
                                    }
                                    _ => {}
                                }
                                if g.status != "existing" {
                                    actions = actions.push(button("Reddet".into(), Event::Reject(i)));
                                }
                                kentos_ui::widget::Cell::padded(container(actions).center_x(Fill)).into()
                            }
                        }
                    },
                    "Aday yok.",
                );
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let body = Grows::new()
            .spacing(12.0)
            .width(Fill)
            .push(form)
            .push(words::summary(lines))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(RUN, runnable.then(|| msg(Event::Run))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn features_reconcile_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(RUN) => Some(msg(Event::Run)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
