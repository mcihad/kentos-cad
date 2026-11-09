//! İş grafiği (docs/adr/1940; `road.project.graph`): a road or map project's chain from its sources to its sheets, read
//! only, opened from the project tree's panel. The summary counts the drawings not up to date and the sheets that
//! cannot be delivered, and says the project's system, epoch and heights; the table lists every drawing with its
//! sources (a source changed since it was drawn, or gone, said by name), its objects written and there now, its state
//! and the sheets whose maps show it (Güncel olmayanlar: only the drawings not up to date). Tarifi kaydet writes the
//! graph as JSON: every drawing's recipe (command, version, input), its sources' stamps then and now, the system.
//! Tariften üret (docs/adr/3240; `road.project.produce`) draws a saved recipe's drawings in this project, one undo
//! step: what it drew and what it left (a drawing the project has, one that does not draw here) are said above the
//! table, which is read again.

use std::path::PathBuf;

use iced::{Element, Task};
use kentos_commands::{
    RoadEdgeKind as E, RoadGraphKind as G, RoadNodeState as S, RoadProjectGraph, RoadProjectProduce, RoadProjectProduced,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_project_graph, road_project_produce};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "İş grafiği";
/// The project tree panel's button that opens it.
pub const OPEN: &str = "İş grafiği…";
const CLOSE: &str = "Kapat";
const SAVE: &str = "Tarifi kaydet…";
const PRODUCE: &str = "Tariften üret…";

/// Which drawings the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Outdated,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Outdated => "Güncel olmayanlar",
        })
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub show: Show,
    pub answer: Option<Result<RoadProjectGraph, String>>,
    /// The last Tariften üret's answer (docs/adr/3240).
    pub produced: Option<RoadProjectProduced>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Show(Show),
    Save,
    Saved(Option<PathBuf>),
    Produce,
    Picked(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::ProjectGraph(e)
}

/// Whether a drawing's state is one the print check stops: stale, missing, edited, gone.
fn outdated(s: S) -> bool {
    matches!(s, S::Stale | S::Missing | S::Edited | S::Gone)
}

/// What Tariften üret did (docs/adr/3240): the drawings drawn (each kept here, from its main source by name) and those
/// left with why.
pub fn produced_lines(out: &RoadProjectProduced) -> Vec<(Kind, String)> {
    let mut lines = Vec::new();
    if out.produced.is_empty() {
        lines.push((Kind::Info, "Tariften üretilecek yeni çizim yok.".to_owned()));
    } else {
        let list: Vec<String> = out.produced.iter().map(|p| format!("{} ({})", p.drawing, p.source.name)).collect();
        let objects: usize = out.produced.iter().map(|p| p.uids.len()).sum();
        lines.push((
            Kind::Ok,
            format!("Tariften üretildi: {} çizim, {objects} nesne: {}.", out.produced.len(), list.join(", ")),
        ));
    }
    for l in &out.left {
        lines.push((Kind::Info, format!("Tarifin {} çizimi bırakıldı: {}", l.drawing, l.message)));
    }
    lines
}

/// A drawing's table row: its name, its sources (each changed or gone source said), its objects, state and sheets.
pub fn drawing_rows(g: &RoadProjectGraph, only_outdated: bool) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for (k, n) in g.nodes.iter().enumerate().filter(|(_, n)| n.kind == G::Drawing) {
        if only_outdated && !outdated(n.state) {
            continue;
        }
        let into = |kind: E| g.edges.iter().filter(move |e| e.kind == kind && e.to as usize == k);
        let sources: Vec<String> = into(E::Source)
            .map(|e| {
                let name = g.nodes.get(e.from as usize).map_or("?", |s| s.name.as_str());
                match (&e.kept, &e.now) {
                    (_, None) => format!("{name} (yok)"),
                    (Some(then), Some(now)) if then != now => format!("{name} (değişti)"),
                    _ => name.to_owned(),
                }
            })
            .collect();
        let sheets: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.kind == E::Shows && e.from as usize == k)
            .map(|e| {
                let sheet = g.nodes.get(e.to as usize).map_or("?", |s| s.name.as_str());
                format!("{sheet} › {}", e.map.as_deref().unwrap_or(""))
            })
            .collect();
        // The version that draws it again, and (docs/adr/2840) the one that drew it when the record says another; the
        // revision its write made, as its record keeps it (an older file's record: none).
        let (command, objects, revision) = n.recipe.as_ref().map_or((String::new(), String::new(), String::new()), |r| {
            let short = r.command.rsplit('.').next().unwrap_or(&r.command).to_owned();
            let made = match r.made {
                Some(m) if m != r.version => format!(" (v{m} çizdi)"),
                _ => String::new(),
            };
            (
                format!("{short} v{}{made}", r.version),
                format!("{}/{}", r.objects, r.written),
                r.revision.clone().unwrap_or_else(|| "—".to_owned()),
            )
        });
        rows.push(vec![
            n.name.clone(),
            command,
            sources.join(", "),
            objects,
            revision,
            crate::road_project_panel::state_name(n.state).to_owned(),
            if sheets.is_empty() { "—".to_owned() } else { sheets.join("; ") },
        ]);
    }
    rows
}

impl App {
    fn project_graph_read(&mut self) {
        let deep = self.road_project.deep;
        let Some(doc) = self.document.as_ref() else { return };
        self.project_graph.answer = Some(road_project_graph::preview(&doc.model, deep));
    }

    /// İş grafiği from the project tree's panel: read at the panel's depth.
    pub(crate) fn open_project_graph(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.project_graph = State::default();
        self.project_graph_read();
        self.dialog = Some(Dialog::ProjectGraph);
        Task::none()
    }

    pub(crate) fn project_graph_save(&mut self, path: PathBuf) {
        let Some(Ok(g)) = &self.project_graph.answer else { return };
        let text = serde_json::to_string_pretty(g).unwrap_or_default();
        match std::fs::write(&path, text) {
            Ok(()) => self.say(
                Level::Success,
                format!("{TITLE}: {} çizimin tarifi {} dosyasına yazıldı.", g.drawings, path.display()),
            ),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    /// Tariften üret (docs/adr/3240): the recipe's drawings drawn in this project, one undo step; the graph read again.
    pub(crate) fn project_graph_produce(&mut self, path: PathBuf) {
        let Some(doc) = self.document.as_mut() else { return };
        let input = RoadProjectProduce { path: path.display().to_string(), drawings: None, expected_revision: None };
        let result = road_project_produce::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let left = if out.left.is_empty() { String::new() } else { format!("; {} çizim bırakıldı", out.left.len()) };
            self.say(
                if out.produced.is_empty() { Level::Info } else { Level::Success },
                format!("{TITLE}: {} çizim tariften üretildi{left} ({}).", out.produced.len(), path.display()),
            );
            self.project_graph.produced = Some(out);
            self.project_graph_read();
        }
    }

    pub(crate) fn project_graph_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_project_graph(),
            Event::Show(v) => self.project_graph.show = v,
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("is-grafigi.json")
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.project_graph_save(path),
            Event::Saved(None) => {}
            Event::Produce => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(PRODUCE.trim_end_matches('…'))
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(Some(path)) => self.project_graph_produce(path),
            Event::Picked(None) => {}
            Event::Close => {
                self.project_graph = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn project_graph_view(&self) -> Element<'_, Message> {
        let s = &self.project_graph;
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &s.answer {
            Some(Ok(g)) => {
                for (kind, said) in s.produced.iter().flat_map(produced_lines) {
                    lines.push(words::text_line(kind, said));
                }
                let fine = g.outdated == 0 && g.blocked == 0;
                lines.push(words::text_line(
                    if fine { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} çizim: {} güncel değil. {} pafta bir çizimi gösteriyor: {} teslim edilemez.",
                        g.drawings, g.outdated, g.sheets, g.blocked
                    ),
                ));
                for n in g.nodes.iter().filter(|n| n.kind == G::Sheet && n.state == S::Stale) {
                    lines.push(words::text_line(Kind::Error, format!("{}: {}", n.name, n.why.join(" "))));
                }
                let system = match (&g.system, &g.epsg) {
                    (Some(name), Some(code)) => format!("{name} ({code})"),
                    _ => "yerel (koordinat sistemi yok)".to_owned(),
                };
                let epoch = g.epoch.map_or("söylenmedi".to_owned(), |e| format!("{e}"));
                let heights = g.heights.clone().unwrap_or_else(|| "söylenmedi".to_owned());
                lines.push(words::text_line(Kind::Info, format!("Sistem {system}; epok {epoch}; kotlar {heights}.")));
                lines.push(words::text_line(
                    Kind::Info,
                    "Zincir: katman → yüzey → çizim → pafta. Güncel olmayan çizimi gösteren pafta baskı öncesi denetimde durur; çizimi Proje ağacından yeniden üretin. Çizime bir şey yazılmaz.",
                ));
                rows = drawing_rows(g, s.show == Show::Outdated);
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let table = sheet(
            vec![
                col("Çizim", 180.0),
                col("Komut", 100.0),
                col("Kaynaklar", 200.0),
                num("Nesne", 56.0),
                num("Revizyon", 64.0),
                col("Durum", 100.0),
                col("Paftalar", 190.0),
            ],
            rows,
            "Bu süzgeçte çizim yok.",
            300.0,
        );
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(Segmented::new([Show::All, Show::Outdated], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        let ready = matches!(&s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(PRODUCE, ready.then(|| msg(Event::Produce))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn project_graph_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(PRODUCE) => Some(msg(Event::Produce)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
