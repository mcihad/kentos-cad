//! Detay eşleştirme (docs/adr/1760; `map.featuresReconcile`; named “Adayları uzlaştır” until 8 November 2026): candidates of map assets from two extractions (a point
//! cloud's, an orthophoto's) reconciled by `map.features.reconcile` through KentOS's `harita-varliklari` profile. The
//! form names the two sources' layers and names, the class and confidence fields, the radius and the reviewer; the
//! summary counts the groups by status; the table lists the groups (class, status, sources, candidates, spread, missing
//! fields, whether it is written). A single's Kabul accepts it, a conflict's Direk/Ağaç… corrects its other
//! candidates' class to it, Reddet rejects the group's candidates with a reason; the decisions stay in the window until
//! Eşleştir writes them all in one undo step. Harita only. Areas (building footprints, green areas; docs/adr/2140) are
//! grouped by their overlap ratio, the table says an area group's smallest one.
//!
//! Alan gir (docs/adr/2540): a group whose required field is missing or whose value its class layer's field rules refuse
//! opens a form of those fields under the table (a field with a list chooses from it, the others are typed); Alanları
//! ekle puts the values in the group's candidates' decisions and the plan reads them again, the rules' reasons in the
//! table and the summary.
//!
//! Önceki kararlar (docs/adr/4540): with the switch on, a candidate of a model run again takes the decision recorded on
//! an earlier candidate near it (a resolved review note of Detay eşleştirme); the summary says which notes were taken.

use iced::widget::{container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CandidateDecision, CandidateSource, CommandResult, FeaturesReconcile, FeaturesReconciled};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_features_reconcile};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{self, col, num};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Detay eşleştirme";
const CLOSE: &str = "Kapat";
const RUN: &str = "Eşleştir";
pub(crate) const PROFILE: &str = "harita-varliklari";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The drawing's layers: id and name.
    pub layers: Vec<(String, String)>,
    pub sources: [(String, String); 2],
    pub field: String,
    pub confidence: String,
    pub radius: f64,
    /// The overlap ratio two areas must exceed (docs/adr/2140).
    pub overlap: f64,
    pub reviewer: String,
    /// Docs/adr/4540: the decisions recorded on earlier candidates taken again by the new ones near them.
    pub reapply: bool,
    pub decisions: Vec<CandidateDecision>,
    pub answer: Option<Result<FeaturesReconciled, String>>,
    /// Alan gir: the group whose fields are typed, and the values typed by field (docs/adr/2540).
    pub fill: Option<usize>,
    pub typed: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Layer(usize, String),
    Name(usize, String),
    Field(String),
    Confidence(String),
    Radius(f64),
    Overlap(f64),
    Reviewer(String),
    /// Önceki kararlar: the recorded decisions taken again or not (docs/adr/4540).
    Reapply(bool),
    /// A group's candidates: a single accepted, a conflict's corrected to a class, the group rejected.
    Accept(usize),
    Correct(usize, String),
    Reject(usize),
    /// Alan gir: the group's missing and refused fields in a form; a field's value; the values into the decisions.
    Fill(usize),
    Typed(String, String),
    FillApply,
    FillCancel,
    Run,
    Close,
}

pub(crate) fn msg(e: Event) -> Message {
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
            overlap: (s.overlap != map_features_reconcile::DEFAULT_OVERLAP).then_some(s.overlap),
            decisions: s.decisions.clone(),
            reviewer: some(&s.reviewer),
            reapply: s.reapply,
            reviewed_at: None,
            expected_revision: None,
        }
    }

    pub(crate) fn reconcile_run(&mut self) {
        let input = self.reconcile_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.features_reconcile.answer = Some(match map_features_reconcile::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Adaylar eşleştirilemedi.".to_owned()),
        });
    }

    /// Detay eşleştirme (`map.featuresReconcile`): the window, its sources guessed by their layers' names.
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
            overlap: map_features_reconcile::DEFAULT_OVERLAP,
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
            Event::Overlap(v) => {
                if v > 0.0 && v < 1.0 {
                    self.features_reconcile.overlap = v;
                    self.reconcile_run();
                }
            }
            Event::Reviewer(t) => self.features_reconcile.reviewer = t,
            Event::Reapply(on) => {
                self.features_reconcile.reapply = on;
                self.reconcile_run();
            }
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
                    reject: Some("Detay eşleştirme penceresinde reddedildi.".into()),
                    ..CandidateDecision::default()
                })
            }),
            Event::Fill(g) => {
                self.features_reconcile.fill = Some(g);
                self.features_reconcile.typed.clear();
            }
            Event::Typed(field, text) => {
                self.features_reconcile.typed.insert(field, text);
            }
            Event::FillApply => self.reconcile_fill_apply(),
            Event::FillCancel => {
                self.features_reconcile.fill = None;
                self.features_reconcile.typed.clear();
            }
            Event::Run => {
                // Docs/adr/4140: the decisions are recorded as review notes at this time.
                let input = FeaturesReconcile { reviewed_at: Some(crate::review_window::iso_utc_now()), ..self.reconcile_input() };
                if let Some(doc) = self.document.as_mut() {
                    let result = map_features_reconcile::execute(&mut ExecutionContext::new(&mut doc.model), input);
                    if let Some(out) = self.answered(result) {
                        let notes = match out.notes.as_slice() {
                            [] => String::new(),
                            [n] => format!("; karar {n} numaralı inceleme notu olarak kaydedildi"),
                            [first, .., last] => {
                                format!("; {} karar {first}–{last} numaralı inceleme notları olarak kaydedildi", out.notes.len())
                            }
                        };
                        self.say(
                            Level::Success,
                            format!("{} varlık yazıldı, {} aday işaretlendi{notes}.", out.written.len(), out.marked),
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
                "Yarıçap, örtüşme",
                row![
                    NumberInput::new(s.radius, |v| msg(Event::Radius(v))).step(0.1).decimals(2).point().width(160),
                    NumberInput::new(s.overlap, |v| msg(Event::Overlap(v))).step(0.05).decimals(2).point().width(160),
                    label::caption("Nokta ve çizgi yarıçapla (m), alan örtüşmeyle (kesişim / birleşim) eşlenir.")
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field("Gözden geçiren", text(&s.reviewer, "Ad", Event::Reviewer))
            .field(
                "Önceki kararlar",
                Switch::new(s.reapply, |on| msg(Event::Reapply(on))).label("Yakındaki yeni adaylara yeniden uygula"),
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
                        format!("{duplicates} yinelenen aday, {unknown} sınıfı okunmayan aday; {reviewed} aday daha önce eşleştirildi (okunmaz)."),
                    ));
                }
                // Docs/adr/4540: the recorded decisions taken again, by their notes' numbers.
                if !a.reapplied.is_empty() {
                    let mut numbers: Vec<u32> = a.reapplied.iter().map(|r| r.note).collect();
                    numbers.sort_unstable();
                    numbers.dedup();
                    let numbers = numbers.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("{} adaya önceki karar yeniden uygulandı (inceleme notu {numbers}).", a.reapplied.len()),
                    ));
                }
                let missing = a.groups.iter().filter(|g| !g.missing.is_empty()).count();
                if missing > 0 {
                    lines.push(words::text_line(Kind::Warn, format!("{missing} grubun zorunlu alanı eksik: alan verilmeden yazılmaz.")));
                }
                // docs/adr/2540: the values the class layers' field rules refuse, each with the rule's sentence.
                for g in a.groups.iter().filter(|g| !g.invalid.is_empty()) {
                    for t in &g.invalid {
                        let class = crate::features_reconcile_fill::class_title(g.class.as_deref().unwrap_or_default());
                        lines.push(words::text_line(Kind::Warn, format!("{class} yazılmaz: {}", t.message)));
                    }
                }
                if missing > 0 || a.groups.iter().any(|g| !g.invalid.is_empty()) {
                    lines.push(words::text_line(Kind::Info, "Alan gir eksik ya da kurala uymayan alanları sorar; listesi olan alan listeden seçilir."));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Tek kaynaklı aday kabul edilmeden, çelişen aday sınıfı düzeltilmeden kesin varlık olmaz; kararlar Eşleştir'le yazılır.",
                ));
                // The classes by the profile's titles (the answer names them by id).
                let titles: std::collections::BTreeMap<String, String> = kentos_commands::builtin_profile(PROFILE)
                    .map(|p| p.classes.into_iter().map(|c| (c.id, c.title)).collect())
                    .unwrap_or_default();
                let title = move |id: &str| titles.get(id).cloned().unwrap_or_else(|| id.to_owned());
                let rows = a.groups.clone();
                table = road_dialog::grid_table(
                    vec![
                        col("Sınıf", 90.0),
                        col("Durum", 95.0),
                        col("Kaynaklar", 110.0),
                        num("Aday", 45.0),
                        num("Yayılım (mm)", 85.0),
                        num("Örtüşme", 60.0),
                        col("Alan", 140.0),
                        col("Yazılır", 50.0),
                        col("", 215.0),
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
                            5 => kentos_ui::widget::table::number(g.overlap.map_or(String::new(), |o| format!("{o:.2}"))).into(),
                            6 => {
                                // The fields by their aliases (docs/adr/2540).
                                let class = g.class.clone().unwrap_or_default();
                                let name = |f: &str| crate::features_reconcile_fill::field_title(&class, f);
                                text(
                                    g.missing
                                        .iter()
                                        .map(|f| format!("{} eksik", name(f)))
                                        .chain(g.invalid.iter().map(|t| format!("{} kurala uymaz", name(&t.field))))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                )
                            }
                            7 => text(if g.writes { "Evet".into() } else { "—".into() }),
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
                                if !g.missing.is_empty() || !g.invalid.is_empty() {
                                    actions = actions.push(button("Alan gir".into(), Event::Fill(i)));
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
        let mut body = Grows::new()
            .spacing(12.0)
            .width(Fill)
            .push(form)
            .push(words::summary(lines))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        if let Some((head, fill)) = self.reconcile_fill_view() {
            body = body.push(head).push(fill);
        }
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
            Control::Press(crate::features_reconcile_fill::FILL_APPLY) => Some(msg(Event::FillApply)),
            Control::Press(crate::features_reconcile_fill::FILL_CANCEL) => Some(msg(Event::FillCancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
