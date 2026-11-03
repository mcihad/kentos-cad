//! Ön güzergâh (plan/road.md Y12.1, docs/adr/0910 §1, `route.finder`): the least-cost route between two places over a
//! road project's terrain surface, as a corridor analysis gives it before an alignment is designed. The limits (the
//! steepest step, the cost of climbing, the grid) are the user's; KentOS gives none. The route is read as it is typed
//! (`road.routes.find`); Planda çiz draws it as a 3D polyline on a layer (one undo step “Ön güzergâh”), which Aksı
//! çizgiye oturt then makes an alignment.
//!
//! ```text
//! ┌ Ön güzergâh ────────────────────────────────────────────────────────────────────────────────┐
//! │ Yüzey [▾] Baş Y [ ] Baş X [ ] Son Y [ ] Son X [ ] Hücre [ ] Pay [ ]                           │
//! │ En dik eğim (%) [ ] Tırmanma ağırlığı [ ] Sadeleştirme (m) [ ]  Kaçınılacak: [x] Yasak alan … │
//! │ L … m · kot farkı … m · en dik %… · maliyet … · düğüm …                                        │
//! │ ┌ the route in plan ┐                                                                         │
//! │ └───────────────────┘                                                                         │
//! │                          Katman [▾] [Planda çiz]                                [Kapat]       │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{RouteAsk, RouteFound, RoutesDraw, RoutesFind, Vec2};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_routes_draw, road_routes_find};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::Check;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Ön güzergâh";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
/// The typed fields, in order. The places are written Y (east) then X (north), as surveyors write them.
pub(crate) const FIELDS: [&str; 9] = [
    "Baş Y",
    "Baş X",
    "Son Y",
    "Son X",
    "Hücre (m)",
    "Pay (m)",
    "En dik eğim (%)",
    "Tırmanma ağırlığı",
    "Sadeleştirme (m)",
];

/// The route as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) route: Option<RouteFound>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) surface: Option<String>,
    pub(crate) fields: [String; 9],
    pub(crate) avoid: Vec<String>,
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Surface(String),
    Field(usize, String),
    Avoid(String),
    Layer(String),
    Draw,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Route(event)
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The route asked as typed, or why it is not one.
    pub(crate) fn ask(&self) -> Result<RouteAsk, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        Ok(RouteAsk {
            surface: self.surface.clone().ok_or("Bir yüzey seçin.")?,
            from: Vec2 { x: need(0)?, y: need(1)? },
            to: Vec2 { x: need(2)?, y: need(3)? },
            cell: need(4)?,
            margin: number(&self.fields[5], FIELDS[5])?.unwrap_or(0.0),
            grade: need(6)?,
            climb: need(7)?,
            avoid: self.avoid.clone(),
            tolerance: number(&self.fields[8], FIELDS[8])?,
        })
    }
}

impl App {
    /// Ön güzergâh: the window on the road project's first surface, its places and limits to be written.
    pub(crate) fn open_route(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Ön güzergâh yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let surface = doc.model.settings().road.as_ref().and_then(|r| r.surfaces.first()).map(|s| s.id.clone());
        if surface.is_none() {
            self.warn("Ön güzergâh bir arazi yüzeyinde aranır: önce Arazi'de bir yüzey oluşturun.".to_owned());
            return;
        }
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.route_window =
            Some(Box::new(Window { surface, fields: Default::default(), avoid: Vec::new(), layer, preview: Preview::default() }));
        self.dialog = Some(Dialog::Route);
        self.refresh_route();
    }

    fn refresh_route(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.route_window.as_mut() else { return };
        w.preview = match w.ask().and_then(|ask| road_routes_find::preview(&doc.model, &RoutesFind { ask })) {
            Err(why) => Preview { route: None, why: Some(why) },
            Ok(route) => Preview { route: Some(route), why: None },
        };
    }

    pub(crate) fn route_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.route_window.as_mut() else { return Task::none() };
        match event {
            Event::Surface(s) => w.surface = Some(s),
            Event::Field(k, t) => w.fields[k.min(8)] = t,
            Event::Avoid(id) => {
                if let Some(at) = w.avoid.iter().position(|a| *a == id) {
                    w.avoid.remove(at);
                } else {
                    w.avoid.push(id);
                }
            }
            Event::Layer(l) => w.layer = Some(l),
            Event::Draw => return self.draw_route(),
            Event::Close => {
                self.route_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_route();
        Task::none()
    }

    fn draw_route(&mut self) -> Task<Message> {
        let Some(w) = self.route_window.as_ref() else { return Task::none() };
        let ask = match w.ask() {
            Ok(a) => a,
            Err(why) => {
                self.warn(format!("Ön güzergâh çizilemez: {why}"));
                return Task::none();
            }
        };
        let Some(layer) = w.layer.clone() else {
            self.warn("Çizmek için bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = RoutesDraw { ask, layer_id: layer, expected_revision: None };
        let result = road_routes_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!("Ön güzergâh planda, {:.2} m; aks için çizgiyi seçip Aksı çizgiye oturt'u kullanın.", out.route.length),
            );
            self.route_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    pub(crate) fn route_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.route_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let road = self.document.as_ref().and_then(|d| d.model.settings().road.clone()).unwrap_or_default();
        let sids: Vec<String> = road.surfaces.iter().map(|s| s.id.clone()).collect();
        let on_surface = Select::new(
            road.surfaces.iter().map(|s| Choice::new(s.name.clone())),
            w.surface.as_ref().and_then(|s| sids.iter().position(|i| i == s)),
            move |i| msg(Event::Surface(sids[i].clone())),
        );
        let field = |k: usize, hint: &'static str, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Yüzey", container(on_surface).width(160), None),
            field(0, "Y", 100.0),
            field(1, "X", 100.0),
            field(2, "Y", 100.0),
            field(3, "X", 100.0),
            field(4, "m", 80.0),
            field(5, "0", 80.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let layers = self.relation_layers();
        let mut avoid = row![label::caption("Kaçınılacak alanlar").style(muted)].spacing(10).align_y(iced::alignment::Vertical::Center);
        for (id, name) in &layers {
            let on = w.avoid.contains(id);
            avoid = avoid.push(crate::style::fields::check(
                if on { Check::Checked } else { Check::Unchecked },
                name.clone(),
                Some(msg(Event::Avoid(id.clone()))),
            ));
        }
        let limits = row![field(6, "%", 110.0), field(7, "0", 120.0), field(8, "yok", 120.0)]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(10).push(top).push(limits).push(container(avoid).width(Fill));
        match (&w.preview.why, &w.preview.route) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "{:.2} m · kot farkı {:.2} m · en dik %{:.2} · maliyet {:.2} · {} nokta · {} düğümün {}'i yerleşti",
                    r.length,
                    r.climb,
                    r.steepest,
                    r.cost,
                    r.points.len(),
                    r.nodes,
                    r.settled
                )));
                let areas: Vec<Vec<[f64; 2]>> = self.document.as_ref().map_or_else(Vec::new, |d| {
                    d.model
                        .entities()
                        .filter_map(|e| match e {
                            kentos_commands::Entity::Polygon(p) if w.avoid.contains(&p.base.layer_id) => {
                                Some(p.pts.iter().map(|q| [q.x, q.y]).collect())
                            }
                            _ => None,
                        })
                        .collect()
                });
                body = body.push(
                    container(iced::widget::canvas(Plan { route: r, areas }).width(Fill).height(typography::scaled(220.0)))
                        .style(style::container::field_box),
                );
            }
            _ => {}
        }
        let lids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let on_layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        body = body.push(
            row![
                space::horizontal().width(Fill),
                label::caption("Katman").style(muted),
                container(on_layer).width(150),
                words::primary(DRAW, w.preview.route.as_ref().map(|_| msg(Event::Draw))),
                space::horizontal().width(24),
                words::secondary(CLOSE, Some(msg(Event::Close))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        );
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn route_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.route_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The route in plan, north up, its start and end marked, the areas avoided behind it.
struct Plan<'a> {
    route: &'a RouteFound,
    areas: Vec<Vec<[f64; 2]>>,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let pts = &self.route.points;
        if pts.len() < 2 {
            return vec![frame.into_geometry()];
        }
        let (x0, x1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let (y0, y1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.y), b.max(p.y)));
        let pad = 24.0_f64;
        let scale = ((f64::from(bounds.width) - 2.0 * pad) / (x1 - x0).max(1e-6))
            .min((f64::from(bounds.height) - 2.0 * pad) / (y1 - y0).max(1e-6));
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let at = |x: f64, y: f64| {
            Point::new(
                (f64::from(bounds.width) / 2.0 + (x - cx) * scale) as f32,
                (f64::from(bounds.height) / 2.0 - (y - cy) * scale) as f32,
            )
        };
        for ring in self.areas.iter().filter(|r| r.len() > 2) {
            let area = Path::new(|p| {
                p.move_to(at(ring[0][0], ring[0][1]));
                for q in &ring[1..] {
                    p.line_to(at(q[0], q[1]));
                }
                p.close();
            });
            frame.fill(&area, iced::Color { a: 0.15, ..tk.danger });
            frame.stroke(&area, Stroke::default().with_width(1.0).with_color(tk.danger));
        }
        let line = Path::new(|p| {
            p.move_to(at(pts[0].x, pts[0].y));
            for q in &pts[1..] {
                p.line_to(at(q.x, q.y));
            }
        });
        frame.stroke(&line, Stroke::default().with_width(2.0).with_color(tk.accent));
        for q in pts {
            frame.fill(&Path::circle(at(q.x, q.y), 2.0), tk.accent);
        }
        for (p, word) in [(&pts[0], "baş"), (&pts[pts.len() - 1], "son")] {
            frame.fill(&Path::circle(at(p.x, p.y), 5.0), tk.success);
            frame.fill_text(Text {
                content: format!("{word} {:.2}", p.z),
                position: at(p.x, p.y) + iced::Vector::new(8.0, -6.0),
                color: tk.text,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}
