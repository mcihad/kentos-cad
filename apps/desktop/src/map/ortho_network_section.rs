//! Mozaik birleşme çizgisi's Ağ and Ayak izi (docs/adr/5260): the window's kind chooses between two images' seamline
//! (ortho_seamlines_window.rs), the mosaic elements of every image ticked (`cad.orthophoto.network`: each place goes
//! to the covering image whose footprint's centre is nearest; the junctions where three or more meet) and a raster's
//! footprint from its pixels that are not NoData (`cad.orthophoto.footprint`, its outline or its convex hull). Each
//! plan runs on every change and is drawn; its button writes it in one undo step and selects what it wrote.

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{column, container, row, space};
use iced::{Center, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{
    CommandResult, OrthoNetworkPart, OrthophotoFootprint, OrthophotoFootprintBuilt, OrthophotoNetwork,
    OrthophotoNetworkBuilt,
};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::orthophoto_network::{network, outline};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Form;

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;

/// The network's and the footprint's write buttons.
pub const WRITE_NETWORK: &str = "Ağın elemanlarını çiz";
pub const WRITE_FOOTPRINT: &str = "Ayak izini çiz";

/// The window's kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Pair,
    Network,
    Footprint,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Pair => "İki görüntü",
            Mode::Network => "Ağ",
            Mode::Footprint => "Ayak izi",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetState {
    pub mode: Mode,
    /// The rasters ticked for the network, in the drawing's order.
    pub ticked: Vec<String>,
    pub network: Option<Result<OrthophotoNetworkBuilt, String>>,
    /// The footprint's raster and whether its hull is drawn.
    pub raster: String,
    pub hull: bool,
    pub footprint: Option<Result<OrthophotoFootprintBuilt, String>>,
}

#[derive(Clone, Debug)]
pub enum NetEvent {
    Mode(Mode),
    Tick(String),
    Raster(String),
    Hull(bool),
    Write,
}

fn msg(e: NetEvent) -> Message {
    Message::OrthoSeamlines(crate::ortho_seamlines_window::Event::Net(e))
}

fn answer<T>(r: CommandResult<T>) -> Result<T, String> {
    match r {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Kurulamadı.".to_owned()),
    }
}

impl App {
    fn ortho_net_inputs(&self) -> (OrthophotoNetwork, OrthophotoFootprint) {
        let s = &self.ortho_seamlines;
        let layer = (!s.layer.is_empty()).then(|| s.layer.clone());
        let folder = self.document.as_ref().and_then(|d| d.path.as_ref()).and_then(|p| p.parent()).map(|p| p.display().to_string());
        (
            OrthophotoNetwork { images: s.net.ticked.clone(), footprints: Vec::new(), layer, expected_revision: None },
            OrthophotoFootprint { raster: s.net.raster.clone(), hull: s.net.hull, layer: None, folder, expected_revision: None },
        )
    }

    /// The plans of the kind shown.
    pub(crate) fn ortho_net_run(&mut self) {
        let (net, foot) = self.ortho_net_inputs();
        let mode = self.ortho_seamlines.net.mode;
        let Some(doc) = self.document.as_mut() else { return };
        match mode {
            Mode::Network => {
                self.ortho_seamlines.net.network = Some(answer(network::plan(&ExecutionContext::new(&mut doc.model), &net)));
            }
            Mode::Footprint => {
                self.ortho_seamlines.net.footprint = Some(answer(outline::plan(&ExecutionContext::new(&mut doc.model), &foot)));
            }
            Mode::Pair => {}
        }
    }

    pub(crate) fn ortho_net_event(&mut self, e: NetEvent) {
        let s = &mut self.ortho_seamlines.net;
        match e {
            NetEvent::Mode(m) => {
                s.mode = m;
                if s.ticked.is_empty() {
                    s.ticked = self.ortho_seamlines.rasters.iter().map(|(u, _)| u.clone()).collect();
                }
                if s.raster.is_empty() {
                    s.raster = self.ortho_seamlines.first.clone();
                }
            }
            NetEvent::Tick(u) => match s.ticked.iter().position(|t| *t == u) {
                Some(k) => {
                    s.ticked.remove(k);
                }
                None => {
                    // In the drawing's order.
                    let order: Vec<String> = self.ortho_seamlines.rasters.iter().map(|(u, _)| u.clone()).collect();
                    s.ticked.push(u);
                    s.ticked.sort_by_key(|t| order.iter().position(|o| o == t));
                }
            },
            NetEvent::Raster(u) => s.raster = u,
            NetEvent::Hull(on) => s.hull = on,
            NetEvent::Write => return self.ortho_net_write(),
        }
        self.ortho_net_run();
    }

    fn ortho_net_write(&mut self) {
        let (net, foot) = self.ortho_net_inputs();
        let mode = self.ortho_seamlines.net.mode;
        let Some(doc) = self.document.as_mut() else { return };
        let title = crate::ortho_seamlines_window::TITLE;
        let (uids, said) = match mode {
            Mode::Network => match answer(network::execute(&mut ExecutionContext::new(&mut doc.model), net)) {
                Ok(out) => {
                    let n = out.elements.len();
                    let j = out.junctions.len();
                    (out.elements.into_iter().flat_map(|e| e.uids).collect::<Vec<_>>(), format!("{n} mozaik elemanı yazıldı ve seçildi; {j} üçlü düğüm."))
                }
                Err(why) => return self.warn(format!("{title}: {why}")),
            },
            Mode::Footprint => match answer(outline::execute(&mut ExecutionContext::new(&mut doc.model), foot)) {
                Ok(out) => (out.uid.into_iter().collect(), format!("ayak izi ({:.2} m²) yazıldı ve seçildi.", out.area)),
                Err(why) => return self.warn(format!("{title}: {why}")),
            },
            Mode::Pair => return,
        };
        let slots: Vec<kentos_domain::Slot> = self.document.as_ref().map_or_else(Vec::new, |d| {
            uids.iter().filter_map(|u| kentos_domain::Uuid::parse_str(u).ok()).filter_map(|u| d.model.slot_of(u)).collect()
        });
        self.selection.set(slots);
        self.say(Level::Success, format!("{title}: {said} Tek adımda geri alınır."));
        self.dialog = None;
    }

    /// The kind's form rows, summary, button and plan (Ağ or Ayak izi).
    pub(crate) fn ortho_net_view(&self) -> Element<'_, Message> {
        let s = &self.ortho_seamlines;
        let mut lines = Vec::new();
        let mut form = Form::new().label_width(150.0);
        let (ready, button, rows, parts, junctions): (bool, &str, Vec<Vec<String>>, Vec<Vec<OrthoNetworkPart>>, Vec<[f64; 2]>) =
            match s.net.mode {
                Mode::Network => {
                    let ticks = s.rasters.iter().fold(column![].spacing(4), |c, (u, name)| {
                        c.push(words::check(s.net.ticked.contains(u), name, Some(msg(NetEvent::Tick(u.clone())))))
                    });
                    form = form
                        .field("Ortofotolar", ticks)
                        .help("Her yer, onu kaplayan görüntülerden ayak izinin (raster sınırının) merkezi en yakın olanın; üç ya da daha çok elemanın buluştuğu yer üçlü düğümdür.");
                    match &s.net.network {
                        Some(Ok(r)) => {
                            for (k, n) in r.notes.iter().enumerate() {
                                lines.push(words::text_line(if k == 0 { Kind::Ok } else { Kind::Info }, n.clone()));
                            }
                            let name = |u: &str| s.rasters.iter().find(|(x, _)| x == u).map_or(u.to_owned(), |(_, n)| n.clone());
                            let rows = r
                                .elements
                                .iter()
                                .map(|e| {
                                    vec![
                                        name(&e.image),
                                        e.parts.len().to_string(),
                                        e.parts.iter().map(|p| p.holes.len()).sum::<usize>().to_string(),
                                        format!("{:.2}", e.area).replace('.', ","),
                                    ]
                                })
                                .collect();
                            (true, WRITE_NETWORK, rows, r.elements.iter().map(|e| e.parts.clone()).collect(), r.junctions.iter().map(|j| j.at).collect())
                        }
                        Some(Err(why)) => {
                            lines.push(words::text_line(Kind::Error, why.clone()));
                            (false, WRITE_NETWORK, Vec::new(), Vec::new(), Vec::new())
                        }
                        None => (false, WRITE_NETWORK, Vec::new(), Vec::new(), Vec::new()),
                    }
                }
                _ => {
                    form = form
                        .field(
                            "Raster",
                            row![
                                container(fields::select_owned(s.rasters.clone(), &s.net.raster, |v| msg(NetEvent::Raster(v)))).width(300),
                                words::check(s.net.hull, "dışbükey zarf", Some(msg(NetEvent::Hull(!s.net.hull))))
                            ]
                            .spacing(10)
                            .align_y(Center),
                        )
                        .help("NoData olmayan piksellerin sınırı, piksellerin kenarlarından; zarf birleşme çizgisinin ve ağın ayak izi olarak alınır.");
                    match &s.net.footprint {
                        Some(Ok(r)) => {
                            for (k, n) in r.notes.iter().enumerate() {
                                lines.push(words::text_line(if k == 0 { Kind::Ok } else { Kind::Info }, n.clone()));
                            }
                            let rows = r
                                .parts
                                .iter()
                                .enumerate()
                                .map(|(k, p)| vec![(k + 1).to_string(), p.ring.len().to_string(), p.holes.len().to_string()])
                                .collect();
                            (true, WRITE_FOOTPRINT, rows, vec![r.parts.clone()], Vec::new())
                        }
                        Some(Err(why)) => {
                            lines.push(words::text_line(Kind::Error, why.clone()));
                            (false, WRITE_FOOTPRINT, Vec::new(), Vec::new(), Vec::new())
                        }
                        None => (false, WRITE_FOOTPRINT, Vec::new(), Vec::new(), Vec::new()),
                    }
                }
            };
        let table = if s.net.mode == Mode::Network {
            sheet(vec![col("Görüntü", 260.0), num("Parça", 70.0), num("Delik", 70.0), num("Alan (m²)", 120.0)], rows, "Eleman yok.", 120.0)
        } else {
            sheet(vec![num("Parça", 70.0), num("Köşe", 70.0), num("Delik", 70.0)], rows, "Ayak izi yok.", 120.0)
        };
        let write = row![space::horizontal().width(Fill), words::primary(button, ready.then(|| msg(NetEvent::Write)))].align_y(Center);
        let plan = crate::picture::picture(
            iced::widget::canvas(NetPlan { parts, junctions }).width(Fill).height(Fill),
            crate::ortho_seamlines_window::TITLE,
        );
        column![form, words::summary(lines), write, table, container(plan).height(typography::scaled(240.0))]
            .spacing(12)
            .into()
    }
}

/// The elements (each image its colour) or the footprint's parts, and the junctions.
struct NetPlan {
    parts: Vec<Vec<OrthoNetworkPart>>,
    junctions: Vec<[f64; 2]>,
}

impl canvas::Program<Message> for NetPlan {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let pts: Vec<[f64; 2]> = self.parts.iter().flatten().flat_map(|p| p.ring.iter().copied()).collect();
        if pts.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &pts {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        let pad = 20.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let scale = (w / (x1 - x0).max(1e-9)).min(h / (y1 - y0).max(1e-9));
        let (ox, oy) = (pad + (w - (x1 - x0) * scale) / 2.0, pad + (h - (y1 - y0) * scale) / 2.0);
        let at = |p: [f64; 2]| Point::new((ox + (p[0] - x0) * scale) as f32, (oy + (y1 - p[1]) * scale) as f32);
        let ring = |r: &[[f64; 2]]| {
            Path::new(|b| {
                b.move_to(at(r[0]));
                for q in &r[1..] {
                    b.line_to(at(*q));
                }
                b.close();
            })
        };
        let colours = [tk.accent, tk.success, tk.warning, tk.info, tk.danger];
        for (k, parts) in self.parts.iter().enumerate() {
            let color = colours[k % colours.len()];
            for p in parts.iter().filter(|p| p.ring.len() >= 3) {
                let shape = ring(&p.ring);
                frame.fill(&shape, iced::Color { a: 0.22, ..color });
                frame.stroke(&shape, Stroke::default().with_width(1.4).with_color(color));
                for hole in p.holes.iter().filter(|h| h.len() >= 3) {
                    frame.stroke(&ring(hole), Stroke::default().with_width(1.2).with_color(color));
                }
            }
            if let Some(p) = parts.first().filter(|_| self.parts.len() > 1) {
                let n = p.ring.len() as f64;
                let c = p.ring.iter().fold([0.0, 0.0], |s, q| [s[0] + q[0] / n, s[1] + q[1] / n]);
                frame.fill_text(Text {
                    content: (k + 1).to_string(),
                    position: at(c),
                    color,
                    size: Pixels(16.0),
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    ..Text::default()
                });
            }
        }
        for j in &self.junctions {
            frame.fill(&Path::circle(at(*j), 5.0), tk.text);
        }
        vec![frame.into_geometry()]
    }
}
