//! Taşınma raporu (docs/adr/1262; `topology.cleanReport`): Topolojik temizlik's moves before it writes, by
//! `cad.topology.clean`'s plan. The scope is the tool's (the selection, else every line, polyline, arc and area on a
//! visible, unlocked layer); the tolerance and the four works are typed here. The window says the counts, how many
//! vertices move, the farthest and the total, the areas' change and the quality profile's acceptance threshold; its
//! table lists each changed object; CSV kaydet writes every move (Y, X before and after, how far). Uygula writes one
//! undo step through the command, which refuses a move past the acceptance threshold.

use std::path::PathBuf;

use iced::widget::{Column, row};
use iced::{Element, Task};
use kentos_commands::{CommandResult, Entity, TopologyClean, TopologyCleaned};
use kentos_domain::Uuid;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, topology_clean};
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Taşınma raporu";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
const SAVE: &str = "CSV kaydet…";
/// The works' words, in Uçlar, Köşeler, Uzat, Buda's order (docs/adr/0148 §3).
const WORKS: [&str; 4] = ["Uçlar", "Köşeler", "Uzat", "Buda"];

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The objects to correct; none: the whole drawing.
    pub uids: Option<Vec<String>>,
    pub tolerance: f64,
    pub works: [bool; 4],
    pub plan: Option<Result<TopologyCleaned, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Tolerance(f64),
    Work(usize, bool),
    Save,
    Saved(Option<PathBuf>),
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::CleanReport(e)
}

/// A move's kind in words.
fn kind_word(kind: &str) -> &'static str {
    match kind {
        "end" => "uç birleşir",
        "vertex" => "köşe birleşir",
        "extended" => "uç uzar",
        "trimmed" => "uç kısalır",
        "edge" => "uç kenara taşınır",
        _ => "taşınır",
    }
}

/// The report's moves as CSV (`;`, Turkish surveying order Y then X; metres as computed, not rounded).
pub fn csv(r: &TopologyCleaned) -> String {
    let mut out = String::from("Nesne;İş;Y önce;X önce;Y sonra;X sonra;Uzaklık (m)\n");
    for m in &r.moves {
        out.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            m.uid,
            kind_word(&m.kind),
            m.from.x,
            m.from.y,
            m.to.x,
            m.to.y,
            m.distance
        ));
    }
    out
}

impl App {
    fn clean_report_input(&self) -> TopologyClean {
        let s = &self.clean_report;
        TopologyClean {
            uids: s.uids.clone(),
            tolerance: s.tolerance,
            ends: Some(s.works[0]),
            vertices: Some(s.works[1]),
            extend: Some(s.works[2]),
            trim: Some(s.works[3]),
            expected_revision: None,
        }
    }

    fn clean_report_plan(&mut self) {
        let input = self.clean_report_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.clean_report.plan = Some(match topology_clean::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Rapor alınamadı.".to_owned()),
        });
    }

    /// Taşınma raporu (`topology.cleanReport`): the selection or the whole drawing, with the tool's tolerance and works.
    pub(crate) fn open_clean_report(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let uids = (!self.selection.is_empty())
            .then(|| self.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect());
        let w = self.memory.topology_works;
        self.clean_report = State {
            uids,
            tolerance: self.memory.topology_tolerance,
            works: [w.ends, w.vertices, w.extend, w.trim],
            plan: None,
        };
        self.clean_report_plan();
        self.dialog = Some(Dialog::CleanReport);
        Task::none()
    }

    /// The moves written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn clean_report_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.clean_report.plan else { return };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} taşınma {} dosyasına yazıldı.", r.moves.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    pub(crate) fn clean_report_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tolerance(v) => self.clean_report.tolerance = v,
            Event::Work(i, on) => {
                if let Some(w) = self.clean_report.works.get_mut(i) {
                    *w = on;
                }
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Taşınma raporu")
                            .set_file_name("tasinma-raporu.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.clean_report_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Apply => {
                let input = self.clean_report_input();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match topology_clean::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        let f = self.format();
                        let text = if output.changes.is_empty() {
                            format!("{TITLE}: düzeltilecek bir şey yok; hiçbir şey değişmedi.")
                        } else {
                            format!(
                                "Topolojik temizlik: {} nesne değişti, {} köşe taşındı; en uzak {}, toplam {} (tek adımda geri alınır).",
                                output.changes.len(),
                                output.moved,
                                f.length(output.max_moved),
                                f.length(output.total_moved)
                            )
                        };
                        self.say(Level::Success, text);
                        self.clean_report = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
                return Task::none();
            }
            Event::Close => {
                self.clean_report = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.clean_report_plan();
        Task::none()
    }

    /// An object's name in the table: its kind and its number in the drawing.
    fn clean_report_name(&self, uid: &str) -> String {
        let e = self
            .document
            .as_ref()
            .and_then(|d| Uuid::parse_str(uid).ok().and_then(|u| d.model.slot_of(u)).and_then(|s| d.model.get(s)));
        match e {
            Some(e) => {
                let kind = match e {
                    Entity::Line(_) => "Çizgi",
                    Entity::Polyline(_) => "Çoklu çizgi",
                    Entity::Polygon(_) => "Alan",
                    Entity::Arc(_) => "Yay",
                    _ => "Nesne",
                };
                format!("{kind} {}", e.base().id)
            }
            None => uid.chars().take(8).collect(),
        }
    }

    pub(crate) fn clean_report_view(&self) -> Element<'_, Message> {
        let s = &self.clean_report;
        let f = self.format();
        // The tolerance on its own line, the four works side by side on the box column: short switches that belong
        // together share a row (the owner: forms neat, not always one under the other).
        let works = WORKS.iter().enumerate().fold(row![].spacing(18), |r, (i, w)| {
            r.push(Switch::new(s.works[i], move |on| msg(Event::Work(i, on))).label(*w))
        });
        let scope = match &s.uids {
            Some(u) => format!("seçili {} nesne", u.len()),
            None => "bütün çizim".to_owned(),
        };
        let form = Form::new()
            .label_width(110.0)
            .field(
                "Tolerans (m)",
                row![
                    NumberInput::new(s.tolerance, |v| msg(Event::Tolerance(v))).step(0.005).decimals(3).point().width(140),
                    kentos_ui::label::caption(format!("Kapsam: {scope}"))
                ]
                .spacing(14)
                .align_y(iced::Center),
            )
            .field("İşler", works);
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &s.plan {
            Some(Ok(r)) if r.moves.is_empty() => lines.push(words::text_line(
                Kind::Warn,
                format!("{} toleransla düzeltilecek bir şey yok; toleransı büyütün ya da işleri açın.", f.length(s.tolerance)),
            )),
            Some(Ok(r)) => {
                let c = &r.counts;
                let counted: Vec<String> = [
                    (c.ends, "uç birleşir"),
                    (c.vertices, "köşe birleşir"),
                    (c.extended, "uç uzar"),
                    (c.trimmed, "uç kısalır"),
                    (c.edges, "uç kenara taşınır"),
                ]
                .iter()
                .filter(|(n, _)| *n > 0)
                .map(|(n, w)| format!("{n} {w}"))
                .collect();
                lines.push(words::text_line(Kind::Info, format!("{}; {} nesne değişir.", counted.join(", "), r.changes.len())));
                lines.push(words::text_line(
                    Kind::Ok,
                    format!(
                        "{} köşe taşınır: en uzak {}, toplam {}; köşeler var olan köşelere gider, koordinat yuvarlanmaz.",
                        r.moved,
                        f.length(r.max_moved),
                        f.length(r.total_moved)
                    ),
                ));
                if r.area_change != 0.0 {
                    lines.push(words::text_line(Kind::Info, format!("Alanların toplamı {} değişir.", f.area(r.area_change))));
                }
                match r.acceptance {
                    Some(a) if r.beyond > 0 => lines.push(words::text_line(
                        Kind::Error,
                        format!(
                            "{} taşınma kalite profilinin kabul eşiğini ({}) aşıyor: Uygula reddeder. Toleransı indirin.",
                            r.beyond,
                            f.length(a)
                        ),
                    )),
                    Some(a) => lines.push(words::text_line(
                        Kind::Ok,
                        format!("Bütün taşınmalar kalite profilinin kabul eşiğinin ({}) içinde.", f.length(a)),
                    )),
                    None => lines.push(words::text_line(Kind::Info, "Kalite profili yok: kabul eşiği denetlenmez.")),
                }
                for c in &r.changes {
                    let change = match (c.area_before, c.area_after, c.length_before, c.length_after) {
                        (Some(a), Some(b), _, _) => f.area(b - a),
                        (_, _, Some(a), Some(b)) => f.length(b - a),
                        _ => "—".to_owned(),
                    };
                    rows.push(vec![
                        self.clean_report_name(&c.uid),
                        self.document.as_ref().map_or_else(|| c.layer.clone(), |d| d.model.layers().path(&c.layer)),
                        c.moves.to_string(),
                        f.length(c.max_moved),
                        f.length(c.total_moved),
                        change,
                    ]);
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => lines.push(words::text_line(Kind::Info, "Rapor yok.")),
        }
        let table = sheet(
            vec![
                col("Nesne", 130.0),
                col("Katman", 110.0),
                num("Taşınan", 70.0),
                num("En uzak", 90.0),
                num("Toplam", 90.0),
                num("Alan / boy farkı", 120.0),
            ],
            rows,
            "Değişen nesne yok.",
            260.0,
        );
        let body = Column::new().spacing(12).push(form).push(words::summary(lines)).push(table);
        let plan = s.plan.as_ref().and_then(|p| p.as_ref().ok());
        let ready = plan.is_some_and(|r| !r.changes.is_empty() && r.beyond == 0);
        let saveable = plan.is_some_and(|r| !r.moves.is_empty());
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, saveable.then(|| msg(Event::Save))))
            .action(words::ghost(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
            .width(720.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn clean_report_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
