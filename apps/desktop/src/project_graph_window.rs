//! İş grafiği (docs/adr/1940; `road.project.graph`): a road or map project's chain from its sources to its sheets, read
//! only, opened from the project tree's panel. The summary counts the drawings not up to date and the sheets that
//! cannot be delivered, and says the project's system, epoch and heights; the table lists every drawing with its
//! sources (a source changed since it was drawn, or gone, said by name), its objects written and there now, its state
//! and the sheets whose maps show it (Güncel olmayanlar: only the drawings not up to date). Tarifi kaydet writes the
//! graph as JSON: every drawing's recipe (command, version, input), its sources' stamps then and now, the system.

use std::path::PathBuf;

use iced::{Element, Task};
use kentos_commands::{RoadEdgeKind as E, RoadGraphKind as G, RoadNodeState as S, RoadProjectGraph};
use kentos_interaction::Level;
use kentos_native_application::road_project_graph;
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
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Show(Show),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::ProjectGraph(e)
}

/// Whether a drawing's state is one the print check stops: stale, missing, edited, gone.
fn outdated(s: S) -> bool {
    matches!(s, S::Stale | S::Missing | S::Edited | S::Gone)
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
        let (command, objects) = n.recipe.as_ref().map_or((String::new(), String::new()), |r| {
            let short = r.command.rsplit('.').next().unwrap_or(&r.command).to_owned();
            (format!("{short} v{}", r.version), format!("{}/{}", r.objects, r.written))
        });
        rows.push(vec![
            n.name.clone(),
            command,
            sources.join(", "),
            objects,
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
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn project_graph_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
