//! Drenaj (plan/road.md Y16.3, docs/adr/0929 §3, `drainage.editor`): a terrain surface's ponds and a discharge point's
//! catchment. The discharge point is typed (Y,X), taken from the selected point, or a culvert's inlet; the grid's cell,
//! the radius a drop must pass within, the runoff coefficient and the rainfall intensity are the user's; KentOS fills in
//! none. Önizle runs `road.drainage.query` (nothing written): in plan the catchment's cells' outline, its flow path from
//! the most distant cell, the discharge point and the ponds' bottoms; the catchment's area, flow path, Rational peak
//! flow and a culvert's full-flow capacity against it; the ponds' levels, depths, areas and volumes. Planda çiz draws
//! them on a layer (one undo step “Drenaj çiz”).
//!
//! ```text
//! ┌ Drenaj ───────────────────────────────────────────────────────────────────────────────────────┐
//! │ Yüzey [▾]  Boşaltım noktası [Y,X] [Seçimden al]  Menfez [▾]  Izgara [ ] Erim [ ] C [ ] i [ ]   │
//! │ Gölet en az derinliği [ ]                                                         [Önizle]    │
//! │ ┌ the catchment and ponds in plan ┐  Havza … ha · … hücre · akış yolu … m · Q … · kapasite …   │
//! │ └─────────────────────────────────┘  Göletler: dip · seviye · derinlik · alan · hacim          │
//! │                        Katman [▾] [Planda çiz]                                     [Kapat]    │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    DrainageCatchmentAsk, DrainageDraw, DrainagePondsAsk, DrainageQueried, DrainageQuery, Entity,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_drainage, road_drainage_draw};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Drenaj";
const PREVIEW: &str = "Önizle";
const DRAW: &str = "Planda çiz";
const CLOSE: &str = "Kapat";
const FROM_SELECTION: &str = "Seçimden al";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 6] = [
    "Boşaltım noktası (Y,X)",
    "Izgara aralığı (m)",
    "Erim (m)",
    "Akış katsayısı C",
    "Yağış şiddeti i (mm/sa)",
    "Gölet en az derinliği (m)",
];

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) surface: Option<String>,
    pub(crate) culvert: Option<String>,
    pub(crate) fields: [String; 6],
    pub(crate) layer: Option<String>,
    pub(crate) preview: Option<Result<DrainageQueried, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Surface(String),
    Culvert(Option<String>),
    Field(usize, String),
    FromSelection,
    Layer(String),
    Preview,
    Draw,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Drainage(event)
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl Window {
    /// The query as typed, or why it is not one yet.
    pub(crate) fn query(&self) -> Result<DrainageQuery, String> {
        let surface = self.surface.clone().ok_or("Bir arazi yüzeyi seçin.")?;
        let at = match self.fields[0].trim() {
            "" => None,
            t => {
                let mut it = t.split(',').map(str::trim);
                match (it.next().map(str::parse::<f64>), it.next().map(str::parse::<f64>), it.next()) {
                    (Some(Ok(y)), Some(Ok(x)), None) => Some([y, x]),
                    _ => return Err(format!("{}: “{t}” Y,X biçiminde iki sayı değil.", FIELDS[0])),
                }
            }
        };
        let catchment = if at.is_some() || self.culvert.is_some() {
            Some(DrainageCatchmentAsk {
                at,
                culvert: self.culvert.clone(),
                cell: number(&self.fields[1], FIELDS[1])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[1]))?,
                radius: number(&self.fields[2], FIELDS[2])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[2]))?,
                runoff: number(&self.fields[3], FIELDS[3])?,
                intensity: number(&self.fields[4], FIELDS[4])?,
            })
        } else {
            None
        };
        let ponds = number(&self.fields[5], FIELDS[5])?.map(|min_depth| DrainagePondsAsk { min_depth });
        Ok(DrainageQuery { surface, ponds, drops: Vec::new(), catchment })
    }
}

/// The selected point's place.
fn chosen_point(app: &App) -> Option<[f64; 2]> {
    let doc = app.document.as_ref()?;
    let [slot] = app.selection.ids() else { return None };
    match doc.model.get(*slot)? {
        Entity::Point(p) => Some([p.p.x, p.p.y]),
        _ => None,
    }
}

impl App {
    /// Drenaj: the window on the road project's first terrain surface.
    pub(crate) fn open_drainage(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        // A map project's surface drains as a road project's (docs/adr/1580).
        if !kentos_native_application::road_terrains::holds_surfaces(doc.model.settings().cad_work) {
            self.warn("Drenaj Harita ve Yol projelerinde olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.settings().road.as_ref().and_then(|r| r.surfaces.first()) else {
            self.warn("Drenaj bir arazi yüzeyinde okunur: önce Arazi yüzeyleri'nde bir yüzey oluşturun.".to_owned());
            return;
        };
        let surface = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let mut fields: [String; 6] = Default::default();
        if let Some(p) = chosen_point(self) {
            fields[0] = format!("{},{}", trimmed(p[0]), trimmed(p[1]));
        }
        self.drainage_window = Some(Box::new(Window { surface: Some(surface), culvert: None, fields, layer, preview: None, warnings: Vec::new() }));
        self.dialog = Some(Dialog::Drainage);
    }

    fn preview_drainage(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.drainage_window.as_mut() else { return };
        let (preview, warnings) = match w.query().and_then(|q| road_drainage::preview(&doc.model, &q)) {
            Err(why) => (Err(why), Vec::new()),
            Ok((q, warnings)) => (Ok(q), warnings.into_iter().map(|x| x.message).collect()),
        };
        w.preview = Some(preview);
        w.warnings = warnings;
    }

    pub(crate) fn drainage_event(&mut self, event: Event) -> Task<Message> {
        let picked = chosen_point(self);
        let Some(w) = self.drainage_window.as_mut() else { return Task::none() };
        match event {
            Event::Surface(s) => w.surface = Some(s),
            Event::Culvert(c) => w.culvert = c,
            Event::Field(k, t) => w.fields[k.min(5)] = t,
            Event::FromSelection => match picked {
                Some(p) => w.fields[0] = format!("{},{}", trimmed(p[0]), trimmed(p[1])),
                None => {
                    self.warn("Boşaltım noktası için bir nokta seçin.".to_owned());
                    return Task::none();
                }
            },
            Event::Layer(l) => w.layer = Some(l),
            Event::Preview => {
                self.preview_drainage();
                return Task::none();
            }
            Event::Draw => return self.draw_drainage(),
            Event::Close => {
                self.drainage_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        if let Some(w) = self.drainage_window.as_mut() {
            w.preview = None;
            w.warnings.clear();
        }
        Task::none()
    }

    fn draw_drainage(&mut self) -> Task<Message> {
        let Some(w) = self.drainage_window.as_ref() else { return Task::none() };
        let (query, Some(layer)) = (w.query(), w.layer.clone()) else {
            self.warn("Çizmek için bir katman seçin.".to_owned());
            return Task::none();
        };
        let query = match query {
            Ok(q) => q,
            Err(why) => {
                self.warn(format!("Drenaj çizilemez: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = DrainageDraw { query, layer_id: layer, labels: true, expected_revision: None };
        let result = road_drainage_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("Drenaj planda, {} nesne.", out.uids.len()));
            self.drainage_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    pub(crate) fn drainage_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.drainage_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let surfaces: Vec<(String, String)> = model
            .and_then(|m| m.settings().road.as_ref())
            .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect());
        let sids: Vec<String> = surfaces.iter().map(|s| s.0.clone()).collect();
        let on_surface = Select::new(
            surfaces.iter().map(|s| Choice::new(s.1.clone())),
            w.surface.as_ref().and_then(|s| sids.iter().position(|i| i == s)),
            move |i| msg(Event::Surface(sids[i].clone())),
        );
        let culverts: Vec<(Option<String>, String)> = std::iter::once((None, "Menfez yok".to_owned()))
            .chain(model.map_or_else(Vec::new, |m| m.road_culverts().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()))
            .collect();
        let cids: Vec<Option<String>> = culverts.iter().map(|c| c.0.clone()).collect();
        let on_culvert = Select::new(
            culverts.iter().map(|c| Choice::new(c.1.clone())),
            cids.iter().position(|c| *c == w.culvert),
            move |i| msg(Event::Culvert(cids[i].clone())),
        );
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Arazi yüzeyi", container(on_surface).width(160), None),
            field("menfezin girişi", 0, 200.0),
            words::secondary(FROM_SELECTION, chosen_point(self).map(|_| msg(Event::FromSelection))),
            words::field("Menfez", container(on_culvert).width(170), None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let second = row![
            field("m", 1, 110.0),
            field("m", 2, 90.0),
            field("yok", 3, 120.0),
            field("yok", 4, 150.0),
            field("gölet yok", 5, 170.0),
            space::horizontal().width(Fill),
            words::secondary(PREVIEW, Some(msg(Event::Preview))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(12.0).push(top).push(second);
        match &w.preview {
            None => {
                body = body.push(
                    label::caption("Önizle yüzeyi okur ve havzayı, göletleri yazmadan gösterir; değer değişince yeniden önizleyin.")
                        .style(muted),
                )
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(q)) => {
                let picture = crate::picture::picture(iced::widget::canvas(Plan { q }).width(Fill).height(Fill), TITLE);
                let mut side = Column::new().spacing(6).width(420);
                if let Some(c) = &q.catchment {
                    let rows: Vec<(&str, String)> = vec![
                        ("Havza", format!("{:.3} ha · {} hücre ({} m)", c.area / 10_000.0, c.cells, trimmed(c.cell))),
                        ("Akış yolu", format!("{:.1} m · düşü {:.2} m · eğim %{:.2}", c.length, c.fall, c.slope * 100.0)),
                        ("En yüksek debi", c.peak.map_or("C ve i yazılmadı".to_owned(), |p| format!("{p:.3} m³/s"))),
                        (
                            "Menfezin kapasitesi",
                            c.capacity.as_ref().map_or("menfez seçilmedi".to_owned(), |k| {
                                format!(
                                    "{:.3} m³/s · {:.2} m/s{}",
                                    k.discharge,
                                    k.velocity,
                                    k.ratio.map_or(String::new(), |r| format!(" · oran {:.2}", r))
                                )
                            }),
                        ),
                    ];
                    for (k, v) in rows {
                        side = side.push(row![label::caption(k).style(muted).width(140), label::body(v)].spacing(8));
                    }
                }
                if !q.ponds.is_empty() {
                    side = side.push(label::caption(format!("Göletler ({})", q.ponds.len())).style(muted));
                    use crate::road_dialog::{grouped, num};
                    side = side.push(crate::road_dialog::sheet(
                        vec![num("Dip (m)", 56.0), num("Seviye (m)", 66.0), num("Derinlik (m)", 72.0), num("Alan (m²)", 62.0), num("Hacim (m³)", 64.0)],
                        q.ponds
                            .iter()
                            .map(|p| vec![grouped(p.bottom[2], 2), grouped(p.level, 2), grouped(p.depth, 2), grouped(p.area, 2), grouped(p.volume, 2)])
                            .collect(),
                        "Gölet yok",
                        180.0,
                    ));
                }
                if q.catchment.is_none() && q.ponds.is_empty() {
                    side = side.push(label::caption("Gösterilecek havza ya da gölet yok.").style(muted));
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push_grow(row![picture, side].spacing(12).height(Fill), typography::scaled(320.0));
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
        let ready = matches!(&w.preview, Some(Ok(q)) if q.catchment.is_some() || !q.ponds.is_empty());
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

    pub(crate) fn drainage_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.drainage_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(FROM_SELECTION) => Some(msg(Event::FromSelection)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The catchment and the ponds in plan: the cells' outline, the flow path, the discharge point and the ponds' bottoms.
struct Plan<'a> {
    q: &'a DrainageQueried,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let mut pts: Vec<[f64; 2]> = Vec::new();
        if let Some(c) = &self.q.catchment {
            pts.extend(c.outline.iter().flatten().copied());
            pts.extend(c.path.iter().map(|p| [p[0], p[1]]));
            pts.push(c.at);
        }
        pts.extend(self.q.ponds.iter().map(|p| [p.bottom[0], p.bottom[1]]));
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
        let pad = 22.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let span = (x1 - x0).max(y1 - y0).max(1.0);
        let scale = (w / (x1 - x0).max(span * 0.05)).min(h / (y1 - y0).max(span * 0.05));
        let (ox, oy) = (pad + (w - (x1 - x0) * scale) / 2.0, pad + (h - (y1 - y0) * scale) / 2.0);
        let at = |x: f64, y: f64| Point::new((ox + (x - x0) * scale) as f32, (oy + (y1 - y) * scale) as f32);
        if let Some(c) = &self.q.catchment {
            for ring in &c.outline {
                if ring.len() < 3 {
                    continue;
                }
                let shape = Path::new(|p| {
                    p.move_to(at(ring[0][0], ring[0][1]));
                    for q in &ring[1..] {
                        p.line_to(at(q[0], q[1]));
                    }
                    p.close();
                });
                frame.fill(&shape, iced::Color { a: 0.18, ..tk.accent });
                frame.stroke(&shape, Stroke::default().with_width(1.4).with_color(tk.accent));
            }
            if c.path.len() >= 2 {
                let flow = Path::new(|p| {
                    p.move_to(at(c.path[0][0], c.path[0][1]));
                    for q in &c.path[1..] {
                        p.line_to(at(q[0], q[1]));
                    }
                });
                frame.stroke(&flow, Stroke::default().with_width(2.2).with_color(tk.info));
                frame.fill(&Path::circle(at(c.path[0][0], c.path[0][1]), 3.5), tk.info);
            }
            let d = at(c.at[0], c.at[1]);
            frame.fill(&Path::circle(d, 5.0), tk.danger);
            frame.fill_text(Text {
                content: "boşaltım".into(),
                position: d + iced::Vector::new(7.0, -14.0),
                color: tk.text,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        for p in &self.q.ponds {
            let c = at(p.bottom[0], p.bottom[1]);
            let r = (p.area.sqrt() * scale / 2.0).clamp(4.0, 40.0) as f32;
            frame.fill(&Path::circle(c, r), iced::Color { a: 0.3, ..tk.info });
            frame.stroke(&Path::circle(c, r), Stroke::default().with_width(1.2).with_color(tk.info));
            frame.fill_text(Text {
                content: format!("{:.2} m", p.depth),
                position: c + iced::Vector::new(r + 3.0, -7.0),
                color: tk.text,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        let legend = [("havza", tk.accent), ("akış yolu", tk.info), ("boşaltım", tk.danger)];
        let mut x = 8.0_f32;
        for (word, color) in legend {
            frame.stroke(&Path::line(Point::new(x, 12.0), Point::new(x + 14.0, 12.0)), Stroke::default().with_width(2.0).with_color(color));
            frame.fill_text(Text { content: word.into(), position: Point::new(x + 18.0, 5.0), color: tk.muted, size: Pixels(11.0), ..Text::default() });
            x += 30.0 + 7.0 * word.chars().count() as f32;
        }
        vec![frame.into_geometry()]
    }
}
