//! Kavşak (plan/road.md Y12.2, docs/adr/0910 §2, `junction.editor`): a road project's junctions of two alignments. The
//! legs' edges and the curb returns' radii are the project's; KentOS fills in none. The junction is read as it is typed
//! (`road.junctions.edit`'s plan on a reading copy): the crossing, the legs' km and the angle, and at grade the four
//! curb returns drawn around the crossing. Kaydet writes it (one undo step “Kavşak”), Planda çiz draws the returns on a
//! layer (one step “Kavşak çiz”).
//!
//! ```text
//! ┌ Kavşak ─────────────────────────────────────────────────────────────────────────────────────┐
//! │ Kavşak [▾] Ad [ ] Tür [Hemzemin ▾] Kesişim [ ] Yarıçap (m) [ ]  Çeyrekler [ ][ ][ ][ ]           │
//! │ 1. kol [aks ▾] Sol [ ] Sağ [ ]    2. kol [aks ▾] Sol [ ] Sağ [ ]                               │
//! │ km … × km … · açı …° · bağlı · warnings                                                       │
//! │ ┌ the crossing in plan ┐  Çeyrek  R  Uzunluk  1. kol km  2. kol km                              │
//! │ └──────────────────────┘                                                                       │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]     │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, JunctionKind, JunctionLegDef, JunctionReport, JunctionsDraw, JunctionsEdit, RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_junctions_draw, road_junctions_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kavşak";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
/// The typed fields, in order: the junction's, each leg's edges, the quadrants' radii.
pub(crate) const FIELDS: [&str; 11] = [
    "Ad",
    "Kesişim",
    "Yarıçap (m)",
    "1. kolun solu (m)",
    "1. kolun sağı (m)",
    "2. kolun solu (m)",
    "2. kolun sağı (m)",
    "Sağ–sağ (m)",
    "Sağ–sol (m)",
    "Sol–sol (m)",
    "Sol–sağ (m)",
];

/// The junction as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<JunctionReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) chosen: Option<String>,
    pub(crate) kind: JunctionKind,
    pub(crate) legs: [Option<String>; 2],
    pub(crate) fields: [String; 11],
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    Kind(JunctionKind),
    Leg(usize, String),
    Field(usize, String),
    Layer(String),
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Junctions(event)
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
    pub(crate) fn input(&self) -> Result<JunctionsEdit, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        let mut legs = Vec::new();
        for (i, a) in self.legs.iter().enumerate() {
            legs.push(JunctionLegDef {
                alignment: a.clone().ok_or_else(|| format!("{}. kolun aksını seçin.", i + 1))?,
                alignment_revision: 0,
                left: need(3 + 2 * i)?,
                right: need(4 + 2 * i)?,
            });
        }
        let at = match self.fields[1].trim() {
            "" => 0,
            t => t.parse::<u32>().map_err(|_| "Kesişim bir tam sayı olmalı (1 ilki).".to_owned())?.saturating_sub(1),
        };
        let at_grade = self.kind == JunctionKind::AtGrade;
        let quadrants: Vec<Option<f64>> = (7..11).map(|k| number(&self.fields[k], FIELDS[k])).collect::<Result<_, _>>()?;
        let radii = if quadrants.iter().all(Option::is_none) {
            None
        } else {
            Some(quadrants.iter().zip(7..).map(|(q, k)| q.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]))).collect::<Result<Vec<_>, _>>()?)
        };
        Ok(JunctionsEdit {
            operation: RoadEditOperation::Set,
            junction: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            kind: Some(self.kind),
            legs: Some(legs),
            at: Some(at),
            radius: if at_grade { Some(need(2)?) } else { None },
            radii: Some(if at_grade { radii } else { None }),
            expected_revision: None,
        })
    }
}

impl App {
    /// Kavşak: the window on the road project's first junction, or a new one on its first two alignments.
    pub(crate) fn open_junctions(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Kavşak yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let axes: Vec<String> = doc.model.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| a.id.clone()).collect();
        if axes.len() < 2 {
            self.warn("Kavşak iki aksın kesişiminde olur: önce Güzergâh'ta iki aks oluşturun.".to_owned());
            return;
        }
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_junctions().first().map(|j| j.id.clone());
        self.junction_window = Some(Box::new(Window {
            chosen: None,
            kind: JunctionKind::AtGrade,
            legs: [Some(axes[0].clone()), Some(axes[1].clone())],
            fields: Default::default(),
            layer,
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Junctions);
        let _ = self.junction_event(Event::Choose(chosen));
    }

    fn refresh_junction(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.junction_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| road_junctions_edit::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None },
        };
    }

    pub(crate) fn junction_event(&mut self, event: Event) -> Task<Message> {
        let kept = self.document.as_ref().map(|d| d.model.road_junctions().to_vec()).unwrap_or_default();
        let Some(w) = self.junction_window.as_mut() else { return Task::none() };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                let mut f: [String; 11] = Default::default();
                match id.as_ref().and_then(|id| kept.iter().find(|c| &c.id == id)) {
                    Some(j) => {
                        w.kind = j.kind;
                        w.legs = [Some(j.legs[0].alignment.clone()), Some(j.legs[1].alignment.clone())];
                        f[0] = j.name.clone();
                        f[1] = (j.at + 1).to_string();
                        f[2] = if j.radius == 0.0 { String::new() } else { trimmed(j.radius) };
                        for (i, leg) in j.legs.iter().enumerate() {
                            f[3 + 2 * i] = trimmed(leg.left);
                            f[4 + 2 * i] = trimmed(leg.right);
                        }
                        for (k, r) in j.radii.iter().flatten().enumerate().take(4) {
                            f[7 + k] = trimmed(*r);
                        }
                    }
                    None => {
                        // A new one: its edges and radius are the user's; only the name and the first crossing.
                        let n = kept.len() + 1;
                        f[0] = if n == 1 { "Kavşak".into() } else { format!("Kavşak {n}") };
                        f[1] = "1".into();
                    }
                }
                w.fields = f;
            }
            Event::Kind(k) => w.kind = k,
            Event::Leg(i, a) => w.legs[i.min(1)] = Some(a),
            Event::Field(k, t) => w.fields[k.min(10)] = t,
            Event::Layer(l) => w.layer = Some(l),
            Event::Save => return self.save_junction(),
            Event::Draw => return self.draw_junction(),
            Event::Remove => return self.remove_junction(),
            Event::Close => {
                self.junction_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_junction();
        Task::none()
    }

    fn save_junction(&mut self) -> Task<Message> {
        let Some(w) = self.junction_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Kavşak yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_junctions_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(r) = &out.junction {
            self.say(Level::Success, format!("“{}”: {:.1}° açıyla, {} dönüş yayı.", r.name, r.angle, r.returns.len()));
            let id = r.id.clone();
            return self.junction_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    fn draw_junction(&mut self) -> Task<Message> {
        let Some(w) = self.junction_window.as_ref() else { return Task::none() };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce kavşağı kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = JunctionsDraw { junction: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_junctions_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            self.say(Level::Success, "Kavşağın dönüş yayları planda.");
            self.junction_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_junction(&mut self) -> Task<Message> {
        let Some(id) = self.junction_window.as_ref().and_then(|w| w.chosen.clone()) else { return Task::none() };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = JunctionsEdit {
            operation: RoadEditOperation::Remove,
            junction: Some(id),
            name: None,
            kind: None,
            legs: None,
            at: None,
            radius: None,
            radii: None,
            expected_revision: None,
        };
        let result = road_junctions_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_junctions().first().map(|c| c.id.clone()));
            return self.junction_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn junction_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.junction_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni kavşak".into())];
        listed.extend(
            model.map_or_else(Vec::new, |m| m.road_junctions().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()),
        );
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_junction = Select::new(
            listed.iter().map(|x| Choice::new(x.1.clone())),
            ids.iter().position(|i| *i == w.chosen),
            move |i| msg(Event::Choose(ids[i].clone())),
        );
        let kinds = [JunctionKind::AtGrade, JunctionKind::Separated];
        let on_kind = Select::new(["Hemzemin", "Katlı"].map(Choice::new), kinds.iter().position(|k| *k == w.kind), move |i| {
            msg(Event::Kind(kinds[i]))
        });
        let axes: Vec<(String, String)> = model.map_or_else(Vec::new, |m| {
            m.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| (a.id.clone(), a.name.clone())).collect()
        });
        let leg = |i: usize| {
            let aids: Vec<String> = axes.iter().map(|a| a.0.clone()).collect();
            let at = w.legs[i].as_ref().and_then(|l| aids.iter().position(|a| a == l));
            Select::new(axes.iter().map(|a| Choice::new(a.1.clone())), at, move |k| msg(Event::Leg(i, aids[k].clone())))
        };
        let field = |k: usize, hint: &'static str, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let at_grade = w.kind == JunctionKind::AtGrade;
        let mut top = row![
            words::field("Kavşak", container(on_junction).width(170), None),
            field(0, "Kavşak", 150.0),
            words::field("Tür", container(on_kind).width(110), None),
            field(1, "1", 60.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        if at_grade {
            top = top.push(field(2, "m", 80.0));
            for (k, hint) in (7..11).zip(["aynı", "aynı", "aynı", "aynı"]) {
                top = top.push(field(k, hint, 80.0));
            }
        }
        let legs = row![
            words::field("1. kol", container(leg(0)).width(160), None),
            field(3, "m", 110.0),
            field(4, "m", 110.0),
            space::horizontal().width(24),
            words::field("2. kol", container(leg(1)).width(160), None),
            field(5, "m", 110.0),
            field(6, "m", 110.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(10).push(top).push(legs);
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "km {} × km {} · açı {:.2}° · {}",
                    r.legs[0].km_text,
                    r.legs[1].km_text,
                    r.angle,
                    if r.connected { "kollar bağlı" } else { "katlı: kollar bağlı değil" }
                )));
                if !w.preview.warnings.is_empty() {
                    let all = w.preview.warnings.iter().map(|x| x.message.as_str()).collect::<Vec<_>>().join(" ");
                    body = body.push(label::caption(all).style(warn));
                }
                let edges: [[f64; 2]; 2] = [
                    [w.legs_edge(3), w.legs_edge(4)],
                    [w.legs_edge(5), w.legs_edge(6)],
                ];
                let picture = container(
                    iced::widget::canvas(Crossing { report: r, edges }).width(420).height(typography::scaled(220.0)),
                )
                .style(style::container::field_box);
                let head = |t: &'static str, wd: f32| label::caption(t).style(muted).width(wd);
                let side = |s: &str| if s == "right" { "sağ" } else { "sol" };
                let mut list = Column::new()
                    .spacing(2)
                    .push(row![head("Çeyrek", 80.0), head("R (m)", 60.0), head("Uzunluk", 70.0), head("1. kol km", 90.0), head("2. kol km", 90.0)].spacing(6));
                for q in &r.returns {
                    list = list.push(
                        row![
                            label::caption(format!("{}–{}", side(&q.a), side(&q.b))).width(80),
                            label::caption(format!("{:.2}", q.radius)).width(60),
                            label::caption(format!("{:.2}", q.length)).width(70),
                            label::caption(format!("{:.2}", q.on_a.station)).width(90),
                            label::caption(format!("{:.2}", q.on_b.station)).width(90),
                        ]
                        .spacing(6),
                    );
                }
                if r.returns.is_empty() {
                    list = list.push(label::caption("Katlı kavşağın dönüş yayı yok.").style(muted));
                }
                body = body.push(
                    row![picture, container(scrollable(list).height(typography::scaled(220.0))).width(Fill).padding([6, 8])]
                        .spacing(12),
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
        body = body.push(
            row![
                words::secondary(REMOVE, saved.then(|| msg(Event::Remove))),
                space::horizontal().width(Fill),
                label::caption("Katman").style(muted),
                container(on_layer).width(150),
                words::secondary(DRAW, (saved && at_grade).then(|| msg(Event::Draw))),
                space::horizontal().width(24),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(SAVE, w.preview.report.as_ref().map(|_| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        );
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn junction_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.junction_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Fill(name, t) if FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

impl Window {
    fn legs_edge(&self, k: usize) -> f64 {
        number(&self.fields[k], "").ok().flatten().unwrap_or(0.0)
    }
}

/// The crossing in plan, north up: the legs' lines and edges along their tangents and the curb returns.
struct Crossing<'a> {
    report: &'a JunctionReport,
    edges: [[f64; 2]; 2],
}

impl canvas::Program<Message> for Crossing<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.report;
        let reach = r
            .returns
            .iter()
            .flat_map(|q| [(q.on_a.x - r.x).hypot(q.on_a.y - r.y), (q.on_b.x - r.x).hypot(q.on_b.y - r.y)])
            .fold(self.edges.iter().flatten().fold(10.0_f64, |a, b| a.max(*b)) * 3.0, f64::max)
            * 1.2;
        let scale = (f64::from(bounds.width.min(bounds.height)) / 2.0 - 10.0) / reach;
        let at = |x: f64, y: f64| {
            Point::new(
                (f64::from(bounds.width) / 2.0 + (x - r.x) * scale) as f32,
                (f64::from(bounds.height) / 2.0 - (y - r.y) * scale) as f32,
            )
        };
        for (leg, e) in r.legs.iter().zip(&self.edges) {
            let d = leg.direction.to_radians();
            let (t, n) = ([d.cos(), d.sin()], [d.sin(), -d.cos()]);
            for (off, color, width) in [(0.0, tk.muted, 1.0), (e[1], tk.text, 1.4), (-e[0], tk.text, 1.4)] {
                let line = Path::line(
                    at(r.x + n[0] * off - t[0] * reach, r.y + n[1] * off - t[1] * reach),
                    at(r.x + n[0] * off + t[0] * reach, r.y + n[1] * off + t[1] * reach),
                );
                frame.stroke(&line, Stroke::default().with_width(width).with_color(color));
            }
        }
        for q in &r.returns {
            let (a0, sweep) = ((q.on_a.y - q.cy).atan2(q.on_a.x - q.cx), q.sweep.to_radians());
            let arc = Path::new(|p| {
                let n = 24;
                for k in 0..=n {
                    let a = a0 + sweep * f64::from(k) / f64::from(n);
                    let pt = at(q.cx + q.radius * a.cos(), q.cy + q.radius * a.sin());
                    if k == 0 {
                        p.move_to(pt);
                    } else {
                        p.line_to(pt);
                    }
                }
            });
            frame.stroke(&arc, Stroke::default().with_width(2.2).with_color(tk.accent));
        }
        frame.fill(&Path::circle(at(r.x, r.y), 3.0), tk.success);
        vec![frame.into_geometry()]
    }
}
