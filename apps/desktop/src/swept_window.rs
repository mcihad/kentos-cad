//! Araç izi (plan/road.md Y17.3, docs/adr/0915 §3, `swept.editor`): a design vehicle's swept path. The vehicle's sizes
//! (and a trailer's) are the user's; KentOS fills in none. The path is a junction's quadrant (round its curb return at
//! an offset, with a straight approach on each leg) or the selected line, polyline or arc; the selected objects can be
//! taken as obstacles. Önizle runs `road.swept.query` (nothing written): in plan the front axle's path, the bodies'
//! corners' tracks, the bodies every few samples, the junction's curb returns and the place of the least clearance;
//! the path's length, the off-tracking and the clearance. Planda çiz draws them on a layer (one undo step “Araç izi
//! çiz”).
//!
//! ```text
//! ┌ Araç izi ──────────────────────────────────────────────────────────────────────────────────────┐
//! │ Dingil aralığı [ ] Ön çıkıntı [ ] Arka çıkıntı [ ] Genişlik [ ]  Römork [Yok ▾]                  │
//! │ Mafsal [ ] Römork dingili [ ] Römork önü [ ] Römork arkası [ ] Römork genişliği [ ]               │
//! │ Yol [Kavşağın çeyreği ▾] Kavşak [▾] Çeyrek [▾] Bordürden [ ] Kollarda [ ] | [Seçimden al]        │
//! │ Aralık [ ] Gövde her [ ] Yön [▾] Engeller: n [Seçimdekiler] [Temizle]              [Önizle]     │
//! │ ┌ the tracks in plan ┐  Yolun boyu … · İç kayma … · Römork … · En az açıklık …                   │
//! │ └────────────────────┘                                                                          │
//! │                        Katman [▾] [Planda çiz]                                       [Kapat]    │
//! └────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    Entity, JunctionReport, JunctionsQuery, SweptDraw, SweptJunctionPath, SweptQueried, SweptQuery, SweptTrailerDef,
    SweptVehicleDef,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_junctions_query, road_swept, road_swept_draw};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Araç izi";
const PREVIEW: &str = "Önizle";
const DRAW: &str = "Planda çiz";
const CLOSE: &str = "Kapat";
const FROM_SELECTION: &str = "Seçimden al";
const OBSTACLES: &str = "Seçimdekiler";
const CLEAR: &str = "Temizle";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 13] = [
    "Dingil aralığı (m)",
    "Ön çıkıntı (m)",
    "Arka çıkıntı (m)",
    "Genişlik (m)",
    "Mafsal (m)",
    "Römork dingili (m)",
    "Römork önü (m)",
    "Römork arkası (m)",
    "Römork genişliği (m)",
    "Bordürden (m)",
    "Kollarda düz (m)",
    "Aralık (m)",
    "Gövde her (örnek)",
];
/// The quadrants' names in `road.junctions.query`'s order: the first leg's side and the second's.
const QUADRANTS: [&str; 4] = ["Sağ–sağ", "Sağ–sol", "Sol–sol", "Sol–sağ"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Junction,
    Object,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) fields: [String; 13],
    pub(crate) trailer: bool,
    pub(crate) source: Source,
    pub(crate) junction: Option<String>,
    pub(crate) quadrant: u32,
    pub(crate) path: Option<String>,
    pub(crate) reverse: bool,
    pub(crate) obstacles: Vec<String>,
    pub(crate) layer: Option<String>,
    pub(crate) preview: Option<Result<SweptQueried, String>>,
    pub(crate) junction_read: Option<JunctionReport>,
    pub(crate) obstacle_lines: Vec<Vec<[f64; 2]>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Field(usize, String),
    Trailer(bool),
    Source(Source),
    Junction(String),
    Quadrant(u32),
    Reverse(bool),
    FromSelection,
    Obstacles,
    Clear,
    Layer(String),
    Preview,
    Draw,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Swept(event)
}

fn number(text: &str, what: &str) -> Result<f64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err(format!("{what} yazılmalı."));
    }
    t.parse::<f64>().map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    fn value(&self, k: usize) -> Result<f64, String> {
        number(&self.fields[k], FIELDS[k])
    }

    /// The query as typed, or why it is not one yet.
    pub(crate) fn query(&self) -> Result<SweptQuery, String> {
        let trailer = if self.trailer {
            Some(SweptTrailerDef {
                kingpin: self.value(4)?,
                wheelbase: self.value(5)?,
                front: self.value(6)?,
                rear: self.value(7)?,
                width: self.value(8)?,
            })
        } else {
            None
        };
        let vehicle =
            SweptVehicleDef { name: None, wheelbase: self.value(0)?, front: self.value(1)?, rear: self.value(2)?, width: self.value(3)?, trailer };
        let (uid, junction) = match self.source {
            Source::Junction => {
                let j = self.junction.clone().ok_or("Bir kavşak seçin.")?;
                (None, Some(SweptJunctionPath { junction: j, quadrant: self.quadrant, offset: self.value(9)?, approach: self.value(10)? }))
            }
            Source::Object => (Some(self.path.clone().ok_or("Yol için bir çizgi, çoklu çizgi ya da yay seçip Seçimden al'a basın.")?), None),
        };
        Ok(SweptQuery {
            vehicle,
            path: Vec::new(),
            uid,
            junction,
            reverse: self.reverse,
            step: self.value(11)?,
            obstacles: self.obstacles.clone(),
        })
    }

    fn every(&self) -> Option<u32> {
        self.fields[12].trim().parse::<u32>().ok().filter(|n| *n > 0)
    }
}

/// The selected objects' uids, and whether the one selected is a path.
fn chosen(app: &App) -> (Vec<String>, Option<String>) {
    let Some(doc) = app.document.as_ref() else { return (Vec::new(), None) };
    let ids = app.selection.ids();
    let uids: Vec<String> = ids.iter().filter_map(|s| doc.model.uid(*s).map(|u| u.to_string())).collect();
    let path = match ids {
        [slot] => match doc.model.get(*slot) {
            Some(Entity::Line(_) | Entity::Arc(_)) => uids.first().cloned(),
            Some(Entity::Polyline(p)) if !p.closed => uids.first().cloned(),
            _ => None,
        },
        _ => None,
    };
    (uids, path)
}

impl App {
    /// Araç izi: the window on the road project's first junction, or on the selected path.
    pub(crate) fn open_swept(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let junction = doc.model.road_junctions().first().map(|j| j.id.clone());
        let (_, path) = chosen(self);
        let source = if junction.is_some() && path.is_none() { Source::Junction } else { Source::Object };
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.swept_window = Some(Box::new(Window {
            fields: Default::default(),
            trailer: false,
            source,
            junction,
            quadrant: 0,
            path,
            reverse: false,
            obstacles: Vec::new(),
            layer,
            preview: None,
            junction_read: None,
            obstacle_lines: Vec::new(),
            warnings: Vec::new(),
        }));
        self.dialog = Some(Dialog::Swept);
    }

    fn preview_swept(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.swept_window.as_mut() else { return };
        let (preview, warnings) = match w.query().and_then(|q| road_swept::preview(&doc.model, &q)) {
            Err(why) => (Err(why), Vec::new()),
            Ok((q, warnings)) => (Ok(q), warnings.into_iter().map(|x| x.message).collect()),
        };
        w.junction_read = match (w.source, &w.junction) {
            (Source::Junction, Some(j)) => road_junctions_query::preview(&doc.model, &JunctionsQuery { junction: Some(j.clone()) })
                .ok()
                .and_then(|(q, _)| q.junctions.into_iter().next()),
            _ => None,
        };
        let step = w.fields[11].trim().parse::<f64>().ok().filter(|s| *s > 0.0).unwrap_or(1.0);
        w.obstacle_lines = w.obstacles.iter().filter_map(|u| road_swept::obstacle_lines(&doc.model, u, step)).flatten().collect();
        w.preview = Some(preview);
        w.warnings = warnings;
    }

    pub(crate) fn swept_event(&mut self, event: Event) -> Task<Message> {
        let (picked, path) = chosen(self);
        let Some(w) = self.swept_window.as_mut() else { return Task::none() };
        match event {
            Event::Field(k, t) => w.fields[k.min(12)] = t,
            Event::Trailer(on) => w.trailer = on,
            Event::Source(s) => w.source = s,
            Event::Junction(j) => w.junction = Some(j),
            Event::Quadrant(q) => w.quadrant = q.min(3),
            Event::Reverse(on) => w.reverse = on,
            Event::FromSelection => match path {
                Some(p) => {
                    w.path = Some(p);
                    w.source = Source::Object;
                }
                None => {
                    self.warn("Yol için bir çizgi, açık çoklu çizgi ya da yay seçin.".to_owned());
                    return Task::none();
                }
            },
            Event::Obstacles => {
                if picked.is_empty() {
                    self.warn("Engel olacak çizgileri, çoklu çizgileri, yayları ya da daireleri seçin.".to_owned());
                    return Task::none();
                }
                for u in picked {
                    if !w.obstacles.contains(&u) && w.path.as_ref() != Some(&u) {
                        w.obstacles.push(u);
                    }
                }
            }
            Event::Clear => w.obstacles.clear(),
            Event::Layer(l) => {
                w.layer = Some(l);
                return Task::none();
            }
            Event::Preview => {
                self.preview_swept();
                return Task::none();
            }
            Event::Draw => return self.draw_swept(),
            Event::Close => {
                self.swept_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        if let Some(w) = self.swept_window.as_mut() {
            w.preview = None;
            w.warnings.clear();
        }
        Task::none()
    }

    fn draw_swept(&mut self) -> Task<Message> {
        let Some(w) = self.swept_window.as_ref() else { return Task::none() };
        let (query, Some(layer), every) = (w.query(), w.layer.clone(), w.every()) else {
            self.warn("Çizmek için bir katman seçin.".to_owned());
            return Task::none();
        };
        let query = match query {
            Ok(q) => q,
            Err(why) => {
                self.warn(format!("Araç izi çizilemez: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = SweptDraw { query, layer_id: layer, every, labels: true, expected_revision: None };
        let result = road_swept_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("Araç izi planda, {} nesne.", out.uids.len()));
            self.swept_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    pub(crate) fn swept_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.swept_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let pick = |names: Vec<&'static str>, at: usize, on: fn(usize) -> Event, width: f32| {
            container(Select::new(names.into_iter().map(Choice::new), Some(at), move |i| msg(on(i)))).width(width)
        };
        let vehicle = row![
            field("m", 0, 130.0),
            field("m", 1, 110.0),
            field("m", 2, 110.0),
            field("m", 3, 100.0),
            words::field("Römork", pick(vec!["Yok", "Var"], usize::from(w.trailer), |i| Event::Trailer(i == 1), 90.0), None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(12).push(vehicle);
        if w.trailer {
            body = body.push(
                row![field("m", 4, 100.0), field("m", 5, 130.0), field("m", 6, 120.0), field("m", 7, 120.0), field("m", 8, 140.0)]
                    .spacing(10)
                    .align_y(iced::alignment::Vertical::Bottom),
            );
        }
        let source = words::field(
            "Yol",
            pick(
                vec!["Kavşağın çeyreği", "Seçili nesne"],
                usize::from(w.source == Source::Object),
                |i| Event::Source(if i == 0 { Source::Junction } else { Source::Object }),
                170.0,
            ),
            None,
        );
        let mut path_row = row![source].spacing(10).align_y(iced::alignment::Vertical::Bottom);
        match w.source {
            Source::Junction => {
                let junctions: Vec<(String, String)> =
                    model.map_or_else(Vec::new, |m| m.road_junctions().iter().map(|j| (j.id.clone(), j.name.clone())).collect());
                let jids: Vec<String> = junctions.iter().map(|j| j.0.clone()).collect();
                let on_junction = Select::new(
                    junctions.iter().map(|j| Choice::new(j.1.clone())),
                    w.junction.as_ref().and_then(|j| jids.iter().position(|i| i == j)),
                    move |i| msg(Event::Junction(jids[i].clone())),
                )
                .placeholder("Kavşak yok");
                path_row = path_row
                    .push(words::field("Kavşak", container(on_junction).width(160), None))
                    .push(words::field(
                        "Çeyrek",
                        pick(QUADRANTS.to_vec(), w.quadrant as usize, |i| Event::Quadrant(i as u32), 110.0),
                        None,
                    ))
                    .push(field("m", 9, 110.0))
                    .push(field("m", 10, 130.0));
            }
            Source::Object => {
                let said = w.path.as_ref().map_or("seçilmedi".to_owned(), |u| format!("…{}", &u[u.len().saturating_sub(6)..]));
                path_row = path_row
                    .push(label::caption(format!("Yol nesnesi: {said}")).style(muted))
                    .push(words::secondary(FROM_SELECTION, chosen(self).1.map(|_| msg(Event::FromSelection))));
            }
        }
        body = body.push(path_row);
        let second = row![
            field("m", 11, 90.0),
            field("yalnız iz", 12, 130.0),
            words::field("Yön", pick(vec!["Baştan", "Sondan"], usize::from(w.reverse), |i| Event::Reverse(i == 1), 100.0), None),
            label::caption(format!("Engeller: {}", w.obstacles.len())).style(muted),
            words::secondary(OBSTACLES, Some(msg(Event::Obstacles))),
            words::secondary(CLEAR, (!w.obstacles.is_empty()).then(|| msg(Event::Clear))),
            space::horizontal().width(Fill),
            words::secondary(PREVIEW, Some(msg(Event::Preview))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        body = body.push(second);
        match &w.preview {
            None => {
                body = body.push(
                    label::caption(
                        "Aracın ölçüleri kurumun tasarım aracınındır; KentOS değer vermez. Önizle izi yazmadan gösterir; değer değişince yeniden önizleyin.",
                    )
                    .style(muted),
                )
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(q)) => {
                let picture = container(
                    iced::widget::canvas(Plan {
                        q,
                        junction: w.junction_read.as_ref(),
                        obstacles: &w.obstacle_lines,
                        every: w.every().unwrap_or(10),
                    })
                        .width(Fill)
                        .height(typography::scaled(340.0)),
                )
                .style(style::container::field_box);
                let mut side = Column::new().spacing(6).width(380);
                let mut rows: Vec<(&str, String)> = vec![
                    ("Yolun boyu", format!("{:.2} m · {} örnek", q.length, q.samples)),
                    ("İç kayma", format!("{:.2} m", q.offtracking)),
                ];
                if let Some(t) = q.trailer_offtracking {
                    rows.push(("Römorkun iç kayması", format!("{t:.2} m")));
                }
                rows.push((
                    "En az açıklık",
                    q.clearance.as_ref().map_or("engel yok".to_owned(), |c| {
                        format!(
                            "{:.2} m · yolun {:.1} m'sinde · {} · {}",
                            c.distance,
                            c.along,
                            if c.body == 0 { "çekici" } else { "römork" },
                            if c.curb { "bordür".to_owned() } else { "engel".to_owned() }
                        )
                    }),
                ));
                for (k, v) in rows {
                    side = side.push(row![label::caption(k).style(muted).width(130), label::body(v)].spacing(8));
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, side].spacing(12));
            }
        }
        let lids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let lnames: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let on_layer = Select::new(
            lnames.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        let ready = matches!(&w.preview, Some(Ok(_)));
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                space::horizontal().width(Fill),
                label::caption("Katman").style(muted),
                container(on_layer).width(170),
                words::secondary(DRAW, ready.then(|| msg(Event::Draw))),
                space::horizontal().width(24),
                words::secondary(CLOSE, Some(msg(Event::Close))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn swept_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.swept_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(FROM_SELECTION) => Some(msg(Event::FromSelection)),
            Control::Press(OBSTACLES) => Some(msg(Event::Obstacles)),
            Control::Press(CLEAR) => Some(msg(Event::Clear)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The tracks in plan: the junction's curb returns, the front's path, the corners' tracks, the bodies every few samples
/// and the least clearance's place.
struct Plan<'a> {
    q: &'a SweptQueried,
    junction: Option<&'a JunctionReport>,
    obstacles: &'a [Vec<[f64; 2]>],
    every: u32,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let q = self.q;
        let pts: Vec<[f64; 2]> = q.corners.iter().flatten().chain(&q.front).copied().collect();
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
        let grow = 0.06 * (x1 - x0).max(y1 - y0).max(1.0);
        (x0, y0, x1, y1) = (x0 - grow, y0 - grow, x1 + grow, y1 + grow);
        let pad = 18.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let scale = (w / (x1 - x0)).min(h / (y1 - y0));
        let (ox, oy) = (pad + (w - (x1 - x0) * scale) / 2.0, pad + (h - (y1 - y0) * scale) / 2.0);
        let at = |p: [f64; 2]| Point::new((ox + (p[0] - x0) * scale) as f32, (oy + (y1 - p[1]) * scale) as f32);
        let line = |ps: &[[f64; 2]], closed: bool| {
            Path::new(|b| {
                if let Some(first) = ps.first() {
                    b.move_to(at(*first));
                    for p in &ps[1..] {
                        b.line_to(at(*p));
                    }
                    if closed {
                        b.close();
                    }
                }
            })
        };
        // The curb returns, each round its centre through its sweep, and the legs' edges a little beyond.
        if let Some(j) = self.junction {
            for r in &j.returns {
                let start = (r.on_a.y - r.cy).atan2(r.on_a.x - r.cx);
                let sweep = r.sweep.to_radians();
                let arc: Vec<[f64; 2]> = (0..=40)
                    .map(|k| {
                        let a = start + sweep * f64::from(k) / 40.0;
                        [r.cx + r.radius * a.cos(), r.cy + r.radius * a.sin()]
                    })
                    .collect();
                frame.stroke(&line(&arc, false), Stroke::default().with_width(2.0).with_color(tk.text));
            }
        }
        for o in self.obstacles {
            frame.stroke(&line(o, false), Stroke::default().with_width(2.0).with_color(tk.text));
        }
        let units = q.corners.len() / 4;
        let n = q.front.len();
        let colors = [tk.accent, tk.info];
        for k in (0..n).filter(|k| (*k as u32).is_multiple_of(self.every.max(1)) || *k + 1 == n) {
            for j in 0..units {
                let b: Vec<[f64; 2]> = (0..4).map(|i| q.corners[4 * j + i][k]).collect();
                let shape = line(&b, true);
                frame.fill(&shape, iced::Color { a: 0.08, ..colors[j % 2] });
                frame.stroke(&shape, Stroke::default().with_width(0.8).with_color(iced::Color { a: 0.55, ..colors[j % 2] }));
            }
        }
        for (i, c) in q.corners.iter().enumerate() {
            frame.stroke(&line(c, false), Stroke::default().with_width(1.4).with_color(colors[(i / 4) % 2]));
        }
        frame.stroke(&line(&q.front, false), Stroke::default().with_width(1.6).with_color(tk.muted));
        frame.stroke(&line(&q.rear, false), Stroke::default().with_width(1.2).with_color(tk.warning));
        if let Some(c) = &q.clearance {
            let p = at(c.at);
            let color = if c.distance == 0.0 { tk.danger } else { tk.warning };
            frame.stroke(&Path::circle(p, 7.0), Stroke::default().with_width(2.0).with_color(color));
            frame.fill_text(Text {
                content: format!("açıklık {:.2} m", c.distance),
                position: p + iced::Vector::new(9.0, -16.0),
                color: tk.text,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        let legend = [("gövde izi", tk.accent), ("arka dingil", tk.warning), ("ön dingil yolu", tk.muted), ("bordür ve engel", tk.text)];
        let mut x = 8.0_f32;
        for (word, color) in legend {
            frame.stroke(&Path::line(Point::new(x, 12.0), Point::new(x + 14.0, 12.0)), Stroke::default().with_width(2.0).with_color(color));
            frame.fill_text(Text { content: word.into(), position: Point::new(x + 18.0, 5.0), color: tk.muted, size: Pixels(11.0), ..Text::default() });
            x += 30.0 + 7.0 * word.chars().count() as f32;
        }
        vec![frame.into_geometry()]
    }
}
