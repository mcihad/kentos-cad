//! Menfez (plan/road.md Y11.2, docs/adr/0909 §2, `culvert.editor`): a road project's culverts across a corridor. The
//! barrel's sizes are the project's; KentOS fills in none. The culvert is read as it is typed (`road.culverts.edit`'s
//! plan on a reading copy): its barrel along its line between the fill's toes, its flow line falling to the outlet, its
//! ends' elevations, the least cover and the quantities. Kaydet writes it (one undo step “Menfez”), Planda çiz draws its
//! barrel with its km on a layer (one step “Menfez çiz”).
//!
//! ```text
//! ┌ Menfez ───────────────────────────────────────────────────────────────────────────────────────┐
//! │ Menfez [▾] Ad [ ] Koridor [▾] Km (m) [ ] Açı (°) [ ] Akış kotu (m) [ ] Eğim (%) [ ] Akış [▾]     │
//! │ Kesit [Boru ▾]  Çap (m) [ ] Et (m) [ ] Sayı [ ] Aralık (m) [ ]  | Uzatma (m) [ ] En az örtü (m) [ ]│
//! │ L … m · genişlik … m · örtü en az … m · beton … m³ · boru … m · warnings                        │
//! │ ┌ the barrel along its line ┐  Uç  Öteleme  Y  X  Akış kotu                                      │
//! │ └───────────────────────────┘                                                                    │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]        │
//! └───────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, CulvertFlow, CulvertReport, CulvertShapeDef, CulvertsDraw, CulvertsEdit, RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_culverts_draw, road_culverts_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Menfez";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
/// The typed fields, in order: the culvert's, then the pipe's, then the box's.
pub(crate) const FIELDS: [&str; 16] = [
    "Ad",
    "Km (m)",
    "Açı (°)",
    "Akış kotu (m)",
    "Eğim (%)",
    "Uzatma (m)",
    "En az örtü (m)",
    "Malzeme",
    "Çap (m)",
    "Et (m)",
    "Boru sayısı",
    "Aralık (m)",
    "Göz genişliği (m)",
    "Göz yüksekliği (m)",
    "Duvar (m)",
    "Döşeme (m)",
];
const CELLS: &str = "Göz sayısı";

/// The culvert as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<CulvertReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The barrel's top above its flow line.
    pub(crate) top: f64,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) chosen: Option<String>,
    pub(crate) corridor: Option<String>,
    pub(crate) pipe: bool,
    pub(crate) flow: CulvertFlow,
    pub(crate) fields: [String; 16],
    pub(crate) cells: String,
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    Corridor(String),
    Pipe(bool),
    Flow(CulvertFlow),
    Field(usize, String),
    Cells(String),
    Layer(String),
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Culverts(event)
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The edit as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<CulvertsEdit, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        let count = |t: &str, what: &str| -> Result<u32, String> {
            t.trim().parse::<u32>().map_err(|_| format!("{what} bir tam sayı olmalı."))
        };
        let shape = if self.pipe {
            CulvertShapeDef::Pipe {
                diameter: need(8)?,
                thickness: need(9)?,
                count: count(&self.fields[10], FIELDS[10])?,
                spacing: number(&self.fields[11], FIELDS[11])?.unwrap_or(0.0),
            }
        } else {
            CulvertShapeDef::Box {
                width: need(12)?,
                height: need(13)?,
                thickness: need(14)?,
                slab: need(15)?,
                cells: count(&self.cells, CELLS)?,
            }
        };
        let material = self.fields[7].trim();
        Ok(CulvertsEdit {
            operation: RoadEditOperation::Set,
            culvert: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            corridor: Some(self.corridor.clone().ok_or("Bir koridor seçin.")?),
            station: Some(need(1)?),
            skew: Some(number(&self.fields[2], FIELDS[2])?.unwrap_or(0.0)),
            shape: Some(shape),
            invert: Some(need(3)?),
            slope: Some(need(4)?),
            flow: Some(self.flow),
            extend: Some(number(&self.fields[5], FIELDS[5])?.unwrap_or(0.0)),
            cover: Some(number(&self.fields[6], FIELDS[6])?),
            material: Some((!material.is_empty()).then(|| material.to_owned())),
            roughness: None,
            expected_revision: None,
        })
    }
}

impl App {
    /// Menfez: the window on the road project's first culvert, or a new one on its first corridor.
    pub(crate) fn open_culverts(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Menfez yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first() else {
            self.warn("Menfez bir koridorun altından geçer: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        let corridor = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_culverts().first().map(|c| c.id.clone());
        self.culvert_window = Some(Box::new(Window {
            chosen: None,
            corridor: Some(corridor),
            pipe: true,
            flow: CulvertFlow::Right,
            fields: Default::default(),
            cells: "1".into(),
            layer,
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Culverts);
        let _ = self.culvert_event(Event::Choose(chosen));
    }

    fn refresh_culvert(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.culvert_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| {
            let top = match q.shape.as_ref() {
                Some(CulvertShapeDef::Pipe { diameter, thickness, .. }) => diameter + thickness,
                Some(CulvertShapeDef::Box { height, slab, .. }) => height + slab,
                None => 0.0,
            };
            road_culverts_edit::preview(&doc.model, &q).map(|r| (r, top))
        }) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok(((report, warnings), top)) => Preview { report: Some(report), warnings, why: None, top },
        };
    }

    pub(crate) fn culvert_event(&mut self, event: Event) -> Task<Message> {
        let kept = self.document.as_ref().map(|d| d.model.road_culverts().to_vec()).unwrap_or_default();
        let Some(w) = self.culvert_window.as_mut() else { return Task::none() };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                let mut f: [String; 16] = Default::default();
                match id.as_ref().and_then(|id| kept.iter().find(|c| &c.id == id)) {
                    Some(c) => {
                        w.corridor = Some(c.corridor.clone());
                        w.flow = c.flow;
                        f[0] = c.name.clone();
                        f[1] = trimmed(c.station);
                        f[2] = if c.skew == 0.0 { String::new() } else { trimmed(c.skew) };
                        f[3] = trimmed(c.invert);
                        f[4] = trimmed(c.slope);
                        f[5] = if c.extend == 0.0 { String::new() } else { trimmed(c.extend) };
                        f[6] = c.cover.map(trimmed).unwrap_or_default();
                        f[7] = c.material.clone().unwrap_or_default();
                        match c.shape {
                            CulvertShapeDef::Pipe { diameter, thickness, count, spacing } => {
                                w.pipe = true;
                                f[8] = trimmed(diameter);
                                f[9] = trimmed(thickness);
                                f[10] = count.to_string();
                                f[11] = if spacing == 0.0 { String::new() } else { trimmed(spacing) };
                            }
                            CulvertShapeDef::Box { width, height, thickness, slab, cells } => {
                                w.pipe = false;
                                f[12] = trimmed(width);
                                f[13] = trimmed(height);
                                f[14] = trimmed(thickness);
                                f[15] = trimmed(slab);
                                w.cells = cells.to_string();
                            }
                        }
                    }
                    None => {
                        // A new one: its sizes are the user's; only the name and one barrel are filled in.
                        let n = kept.len() + 1;
                        f[0] = if n == 1 { "Menfez".into() } else { format!("Menfez {n}") };
                        f[10] = "1".into();
                        w.cells = "1".into();
                    }
                }
                w.fields = f;
            }
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Pipe(p) => w.pipe = p,
            Event::Flow(f) => w.flow = f,
            Event::Field(k, t) => w.fields[k.min(15)] = t,
            Event::Cells(t) => w.cells = t,
            Event::Layer(l) => w.layer = Some(l),
            Event::Save => return self.save_culvert(),
            Event::Draw => return self.draw_culvert(),
            Event::Remove => return self.remove_culvert(),
            Event::Close => {
                self.culvert_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_culvert();
        Task::none()
    }

    fn save_culvert(&mut self) -> Task<Message> {
        let Some(w) = self.culvert_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Menfez yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_culverts_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(c) = &out.culvert {
            self.say(Level::Success, format!("“{}”: {:.2} m gövde, en az örtü {:.2} m.", c.name, c.length, c.cover));
            let id = c.id.clone();
            return self.culvert_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    fn draw_culvert(&mut self) -> Task<Message> {
        let Some(w) = self.culvert_window.as_ref() else { return Task::none() };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce menfezi kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = CulvertsDraw { culvert: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_culverts_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("Menfez planda, {:.2} m.", out.length));
            self.culvert_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_culvert(&mut self) -> Task<Message> {
        let Some(id) = self.culvert_window.as_ref().and_then(|w| w.chosen.clone()) else { return Task::none() };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = CulvertsEdit {
            operation: RoadEditOperation::Remove,
            culvert: Some(id),
            name: None,
            corridor: None,
            station: None,
            skew: None,
            shape: None,
            invert: None,
            slope: None,
            flow: None,
            extend: None,
            cover: None,
            material: None,
            roughness: None,
            expected_revision: None,
        };
        let result = road_culverts_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_culverts().first().map(|c| c.id.clone()));
            return self.culvert_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn culvert_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.culvert_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni menfez".into())];
        listed.extend(
            model.map_or_else(Vec::new, |m| m.road_culverts().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()),
        );
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_culvert = Select::new(
            listed.iter().map(|x| Choice::new(x.1.clone())),
            ids.iter().position(|i| *i == w.chosen),
            move |i| msg(Event::Choose(ids[i].clone())),
        );
        let corridors: Vec<(String, String)> =
            model.map_or_else(Vec::new, |m| m.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect());
        let cids: Vec<String> = corridors.iter().map(|c| c.0.clone()).collect();
        let on_corridor = Select::new(
            corridors.iter().map(|c| Choice::new(c.1.clone())),
            w.corridor.as_ref().and_then(|c| cids.iter().position(|i| i == c)),
            move |i| msg(Event::Corridor(cids[i].clone())),
        );
        let on_shape = Select::new(["Boru", "Kutu"].map(Choice::new), Some(usize::from(!w.pipe)), |i| msg(Event::Pipe(i == 0)));
        let flows = [CulvertFlow::Right, CulvertFlow::Left];
        let on_flow = Select::new(["Sağa", "Sola"].map(Choice::new), flows.iter().position(|f| *f == w.flow), move |i| {
            msg(Event::Flow(flows[i]))
        });
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Menfez", container(on_culvert).width(170), None),
            field("Menfez", 0, 150.0),
            words::field("Koridor", container(on_corridor).width(150), None),
            field("m", 1, 80.0),
            field("0", 2, 60.0),
            field("m", 3, 90.0),
            field("%", 4, 60.0),
            words::field("Akış", container(on_flow).width(80), None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut sizes = row![words::field("Kesit", container(on_shape).width(90), None)].spacing(10);
        if w.pipe {
            sizes = sizes.push(field("m", 8, 70.0)).push(field("m", 9, 60.0)).push(field("1", 10, 60.0)).push(field("0", 11, 60.0));
        } else {
            sizes = sizes
                .push(field("m", 12, 80.0))
                .push(field("m", 13, 80.0))
                .push(field("m", 14, 60.0))
                .push(field("m", 15, 60.0))
                .push(words::field(
                    CELLS,
                    kentos_ui::widget::text_field("1", &w.cells).on_input(|t| msg(Event::Cells(t))).width(60),
                    None,
                ));
        }
        sizes = sizes.push(field("0", 5, 70.0)).push(field("yok", 6, 80.0)).push(field("", 7, 110.0));
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top).push(sizes.align_y(iced::alignment::Vertical::Bottom));
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "km {} · gövde {:.2} m, dış genişlik {:.2} m · en az örtü {:.2} m ({:.2} m ötelemede) · beton {:.2} m³{}",
                    r.km_text,
                    r.length,
                    r.width,
                    r.cover,
                    r.cover_offset,
                    r.concrete,
                    if r.pipes > 0.0 { format!(" · boru {:.2} m", r.pipes) } else { String::new() }
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let picture = crate::picture::picture(iced::widget::canvas(Barrel { report: r, top: w.preview.top }).width(Fill).height(Fill), TITLE);
                let mut list = Column::new().spacing(2).push(
                    row![
                        label::caption("Uç").style(muted).width(90),
                        label::caption("Öteleme (m)").style(muted).width(80),
                        label::caption("Y").style(muted).width(100),
                        label::caption("X").style(muted).width(100),
                        label::caption("Akış kotu").style(muted).width(80),
                    ]
                    .spacing(6),
                );
                for e in &r.ends {
                    let side = if e.side == "left" { "Sol" } else { "Sağ" };
                    let mark = if e.side == r.inlet { "giriş" } else { "çıkış" };
                    list = list.push(
                        row![
                            label::caption(format!("{side} ({mark})")).width(90),
                            label::caption(format!("{:.2}", e.offset)).width(80),
                            label::caption(format!("{:.3}", e.x)).width(100),
                            label::caption(format!("{:.3}", e.y)).width(100),
                            label::caption(format!("{:.3}", e.invert)).width(80),
                        ]
                        .spacing(6),
                    );
                }
                body = body.push_grow(
                    row![picture, container(scrollable(list).height(Fill)).width(typography::scaled(560.0)).padding([6, 8])]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(180.0),
                );
            }
            _ => {}
        }
        let lids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let lnames: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let on_layer = Select::new(
            lnames.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        let saved = w.chosen.is_some();
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                words::secondary(REMOVE, saved.then(|| msg(Event::Remove))),
                space::horizontal().width(Fill),
                label::caption("Katman").style(muted),
                container(on_layer).width(150),
                words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
                space::horizontal().width(24),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(SAVE, w.preview.report.as_ref().map(|_| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn culvert_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.culvert_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Fill(CELLS, t) => Some(msg(Event::Cells(t.to_owned()))),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The barrel along its line, left end to right: its flow line and top, the ends and the water's way.
struct Barrel<'a> {
    report: &'a CulvertReport,
    top: f64,
}

impl canvas::Program<Message> for Barrel<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let [a, b] = [&self.report.ends[0], &self.report.ends[1]];
        let (z0, z1) = (a.invert.min(b.invert), a.invert.max(b.invert) + self.top);
        let pad = 30.0_f32;
        let (w, h) = (f64::from(bounds.width - 2.0 * pad), f64::from(bounds.height - 2.0 * pad));
        let span = (b.distance - a.distance).max(1e-6);
        // Heights drawn at most five times the length's scale, so a fall and a top read without filling the picture.
        let sx = w / span;
        let sy = (h / (z1 - z0).max(1e-6)).min(5.0 * sx);
        let base = f64::from(pad) + 0.5 * (h + (z1 - z0) * sy);
        let at = |d: f64, z: f64| Point::new(pad + ((d - a.distance) * sx) as f32, (base - (z - z0) * sy) as f32);
        let shape = Path::new(|p| {
            p.move_to(at(a.distance, a.invert));
            p.line_to(at(b.distance, b.invert));
            p.line_to(at(b.distance, b.invert + self.top));
            p.line_to(at(a.distance, a.invert + self.top));
            p.close();
        });
        frame.fill(&shape, iced::Color { a: 0.2, ..tk.accent });
        frame.stroke(&shape, Stroke::default().with_width(1.4).with_color(tk.accent));
        for (e, align) in [(a, iced::alignment::Horizontal::Left), (b, iced::alignment::Horizontal::Right)] {
            let word = if e.side == self.report.inlet { "giriş" } else { "çıkış" };
            frame.fill_text(Text {
                content: format!("{word} {:.2}", e.invert),
                position: at(e.distance, e.invert) + iced::Vector::new(0.0, 12.0),
                color: tk.muted,
                size: Pixels(11.0),
                align_x: align.into(),
                ..Text::default()
            });
        }
        frame.fill_text(Text {
            content: format!("L = {:.2} m", self.report.length),
            position: Point::new(bounds.width / 2.0, 12.0),
            color: tk.text,
            size: Pixels(12.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            ..Text::default()
        });
        vec![frame.into_geometry()]
    }
}
