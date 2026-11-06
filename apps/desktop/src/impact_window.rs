//! Alternatif etkisi (docs/adr/1440; `scenario.impact`): a scenario (a design alternative) beside the field state it
//! replaces, by `cad.scenarios.impact`, read only. The scenario chosen on one line; the summary says the base's
//! stamp (stale when it changed since the report last saved this session), the data's gaps and the assumptions; a
//! choice shows the measures, the areas by use or the decisions that changed. Raporu kaydet writes them all as CSV
//! with the stamp and keeps the stamp for the next comparison. No score picks a plan.

use std::collections::BTreeMap;
use std::path::PathBuf;

use iced::widget::{Column, container};
use iced::{Element, Task};
use kentos_commands::{CommandResult, ScenariosImpact, ScenariosImpacted};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, scenarios_impact};
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Alternatif etkisi";
const CLOSE: &str = "Kapat";
const SAVE: &str = "Raporu kaydet…";

/// What the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Measures,
    Uses,
    Changes,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Measures => "Ölçüler",
            View::Uses => "Kullanımlar",
            View::Changes => "Değişen kararlar",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The scenarios: id and name.
    pub scenarios: Vec<(String, String)>,
    pub scenario: String,
    pub view: View,
    pub answer: Option<Result<ScenariosImpacted, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Scenario(String),
    View(View),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Impact(e)
}

/// A change in words.
fn change_word(c: &str) -> &'static str {
    match c {
        "added" => "Eklendi",
        "removed" => "Kaldırıldı",
        "geometry" => "Sınırı değişti",
        "decision" => "Kararı değişti",
        "both" => "Sınırı ve kararı değişti",
        _ => "—",
    }
}

fn unit_word(u: &str) -> &'static str {
    match u {
        "m2" => "m²",
        "m" => "m",
        _ => "adet",
    }
}

/// The report as CSV: the stamp, the measures, the uses, the changes, the gaps and the assumptions (numbers as
/// computed, not rounded).
pub fn csv(r: &ScenariosImpacted) -> String {
    let mut out = format!("Alternatif;{};Taban damgası;{}\n\nÖlçü;Birim;Mevcut durum;Alternatif;Fark;Yöntem\n", r.name, r.stamp);
    for m in &r.measures {
        out.push_str(&format!("{};{};{};{};{};{}\n", m.label, m.unit, m.base, m.alternative, m.difference, m.method));
    }
    out.push_str("\nKullanım;Mevcut durum (m²);Alternatif (m²);Fark (m²)\n");
    for u in &r.uses {
        out.push_str(&format!("{};{};{};{}\n", u.code, u.base, u.alternative, u.difference));
    }
    out.push_str("\nKarar;Değişim;Önce kullanım;Sonra kullanım;Önce alan;Sonra alan;Önce inşaat alanı;Sonra inşaat alanı\n");
    let o = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or_default();
    for c in &r.changes {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{}\n",
            c.key,
            change_word(&c.change),
            c.base_use.as_deref().unwrap_or(""),
            c.alternative_use.as_deref().unwrap_or(""),
            o(c.base_area),
            o(c.alternative_area),
            o(c.base_capacity),
            o(c.alternative_capacity)
        ));
    }
    out.push_str("\nVeri eksikleri\n");
    for g in &r.gaps {
        out.push_str(&format!("{g}\n"));
    }
    out.push_str("\nVarsayımlar\n");
    for a in &r.assumptions {
        out.push_str(&format!("{a}\n"));
    }
    out
}

impl App {
    fn impact_run(&mut self) {
        let since = self.impact_stamps.get(&self.impact.scenario).cloned();
        let input = ScenariosImpact { scenario: self.impact.scenario.clone(), field: None, key: None, since };
        let Some(doc) = self.document.as_mut() else { return };
        self.impact.answer = Some(match scenarios_impact::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Karşılaştırma yapılamadı.".to_owned()),
        });
    }

    /// Alternatif etkisi (`scenario.impact`): the scenario at hand, else the first.
    pub(crate) fn open_impact(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let scenarios: Vec<(String, String)> = crate::temporal::scenario_groups(doc.model.layers().nodes())
            .into_iter()
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();
        let Some(first) = self.scenario_at_hand().or_else(|| scenarios.first().map(|(id, _)| id.clone())) else {
            self.warn("Projede alternatif (senaryo) yok; önce Senaryo oluştur ile tasarım katmanlarının bir alternatifini yapın.");
            return Task::none();
        };
        self.impact = State { scenarios, scenario: first, ..State::default() };
        self.impact_run();
        self.dialog = Some(Dialog::Impact);
        Task::none()
    }

    /// The report written to `path` as CSV, its stamp kept for the next comparison of this scenario (a test or a trace
    /// gives the path here, the picker in the app).
    pub(crate) fn impact_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.impact.answer else { return };
        let (text, scenario, stamp) = (csv(r), r.scenario.clone(), r.stamp.clone());
        match std::fs::write(&path, text) {
            Ok(()) => {
                self.impact_stamps.insert(scenario, stamp);
                self.say(Level::Success, format!("{TITLE}: rapor {} dosyasına yazıldı; taban damgası saklandı.", path.display()));
                self.impact_run();
            }
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    pub(crate) fn impact_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Scenario(id) => {
                self.impact.scenario = id;
                self.impact_run();
            }
            Event::View(v) => self.impact.view = v,
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("alternatif-etkisi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.impact_save(path),
            Event::Saved(None) => {}
            Event::Close => {
                self.impact = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn impact_view(&self) -> Element<'_, Message> {
        let s = &self.impact;
        let f = self.format();
        let pick = crate::style::fields::select_owned(s.scenarios.clone(), &s.scenario, |id| msg(Event::Scenario(id)));
        let form = Form::new().label_width(110.0).field("Alternatif", container(pick).width(300));
        let mut lines = Vec::new();
        let mut table: Element<'_, Message> = words::empty("Karşılaştırma yok.");
        match &s.answer {
            Some(Ok(r)) => {
                let layers: Vec<&str> = r.layers.iter().map(|l| l.name.as_str()).collect();
                let (kind, stamp) = match r.stale {
                    Some(true) => (Kind::Error, format!("Taban, kaydedilen rapordan sonra değişti: alternatif eski tabana göre (damga {}). Alternatifi gözden geçirin; kendiliğinden yeni veriye uymuş sayılmaz.", r.stamp)),
                    Some(false) => (Kind::Ok, format!("Taban kaydedilen rapordan beri aynı (damga {}).", r.stamp)),
                    None => (Kind::Info, format!("Taban damgası {}; raporu kaydedince sonraki karşılaştırma tabanın değişip değişmediğini söyler.", r.stamp)),
                };
                lines.push(words::text_line(Kind::Info, format!("Mevcut durumla karşılaştırılan katmanlar: {}.", layers.join(", "))));
                lines.push(words::text_line(kind, stamp));
                for g in &r.gaps {
                    lines.push(words::text_line(Kind::Warn, g.clone()));
                }
                lines.push(words::text_line(Kind::Info, r.assumptions.join(" ")));
                let fmt = |unit: &str, v: f64| match unit {
                    "m2" => f.area(v),
                    "m" => f.length(v),
                    _ => format!("{v}"),
                };
                let signed = |unit: &str, v: f64| if v > 0.0 { format!("+{}", fmt(unit, v)) } else { fmt(unit, v) };
                table = match s.view {
                    View::Measures => sheet(
                        vec![col("Ölçü", 190.0), num("Mevcut durum", 120.0), num("Alternatif", 120.0), num("Fark", 120.0), col("Birim", 50.0)],
                        r.measures
                            .iter()
                            .map(|m| vec![m.label.clone(), fmt(&m.unit, m.base), fmt(&m.unit, m.alternative), signed(&m.unit, m.difference), unit_word(&m.unit).to_owned()])
                            .collect(),
                        "Ölçü yok.",
                        260.0,
                    ),
                    View::Uses => sheet(
                        vec![col("Kullanım", 190.0), num("Mevcut durum", 120.0), num("Alternatif", 120.0), num("Fark", 120.0)],
                        r.uses
                            .iter()
                            .map(|u| vec![if u.code.is_empty() { "(yok)".to_owned() } else { u.code.clone() }, f.area(u.base), f.area(u.alternative), signed("m2", u.difference)])
                            .collect(),
                        "Kullanım yok.",
                        260.0,
                    ),
                    View::Changes => sheet(
                        vec![col("Karar", 50.0), col("Değişim", 115.0), col("Kullanım", 130.0), num("Alan", 185.0), num("İnşaat alanı", 185.0)],
                        r.changes
                            .iter()
                            .map(|c| {
                                let pair = |a: Option<String>, b: Option<String>| match (a, b) {
                                    (Some(a), Some(b)) if a == b => a,
                                    (a, b) => format!("{} → {}", a.unwrap_or_else(|| "—".into()), b.unwrap_or_else(|| "—".into())),
                                };
                                let area = |v: Option<f64>| v.map(|x| f.area(x));
                                vec![
                                    c.key.clone(),
                                    change_word(&c.change).to_owned(),
                                    pair(c.base_use.clone(), c.alternative_use.clone()),
                                    pair(area(c.base_area), area(c.alternative_area)),
                                    pair(area(c.base_capacity), area(c.alternative_capacity)),
                                ]
                            })
                            .collect(),
                        "Değişen karar yok.",
                        260.0,
                    ),
                };
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let views = [View::Measures, View::Uses, View::Changes];
        let body = Column::new()
            .spacing(12)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new(views, s.view, |v| msg(Event::View(v))))
            .push(table);
        let ready = matches!(s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(820.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn impact_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The stamps kept this session, by scenario id (docs/adr/1440 §3: kept in the drawing is later work).
pub type Stamps = BTreeMap<String, String>;
