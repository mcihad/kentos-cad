//! İstinat duvarı (plan/road.md Y11.3, docs/adr/0909 §3, `retainingWall.editor`): a road project's retaining walls
//! along a corridor's side. The wall's cross is the project's; KentOS fills in none. The wall is read as it is typed
//! (`road.walls.edit`'s plan on a reading copy): along its stations the road's edge, the ground under the face, its
//! top and footing, its stem's height, and its length, face area, concrete and highest stem. Kaydet writes it (one undo
//! step “İstinat duvarı”), Planda çiz draws its face and footing on a layer (one step “İstinat duvarı çiz”).
//!
//! ```text
//! ┌ İstinat duvarı ──────────────────────────────────────────────────────────────────────────────┐
//! │ Duvar [▾] Ad [ ] Koridor [▾] Yan [▾] Baş km [ ] Son km [ ] Aralık [ ] En yüksek [ ] Malzeme [ ]  │
//! │ Gövde üstü [ ] Eğim [ ] Temel eni [ ] Temel kalınlığı [ ] Temel önü [ ] Örtü [ ] Korkuluk payı [ ]│
//! │ L … m · yüz … m² · beton … m³ · en yüksek … m (km …) · warnings                                 │
//! │ ┌ the wall along its stations ┐  Km  Yol  Zemin  Üst  Temel  H  Alan                            │
//! │ └─────────────────────────────┘                                                                  │
//! │ [Sil]                         Katman [▾] [Planda çiz]                     [Kapat] [Kaydet]        │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::road;
use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{container, row, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, RetainingWallReport, RetainingWallsDraw, RetainingWallsEdit, RoadEditOperation, WallShapeDef, WallSide,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_walls_draw, road_walls_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "İstinat duvarı";
const SAVE: &str = "Kaydet";
const CLOSE: &str = "Kapat";
const DRAW: &str = "Planda çiz";
const REMOVE: &str = "Sil";
/// The typed fields, in order: the wall's, then its cross's.
pub(crate) const FIELDS: [&str; 13] = [
    "Ad",
    "Baş km (m)",
    "Son km (m)",
    "Aralık (m)",
    "En yüksek (m)",
    "Malzeme",
    "Gövde üstü (m)",
    "Eğim (m/m)",
    "Temel eni (m)",
    "Temel kalınlığı (m)",
    "Temel önü (m)",
    "Örtü (m)",
    "Korkuluk payı (m)",
];
const SHAPE_AT: usize = 6;

/// The wall as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<RetainingWallReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) chosen: Option<String>,
    pub(crate) corridor: Option<String>,
    pub(crate) side: WallSide,
    pub(crate) fields: [String; 13],
    pub(crate) layer: Option<String>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    Corridor(String),
    Side(WallSide),
    Field(usize, String),
    Layer(String),
    Save,
    Draw,
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Road(road::Event::RetainingWalls(event))
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn optional(v: f64) -> String {
    if v == 0.0 { String::new() } else { trimmed(v) }
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
    pub(crate) fn input(&self) -> Result<RetainingWallsEdit, String> {
        let need = |k: usize| number(&self.fields[k], FIELDS[k])?.ok_or_else(|| format!("{} yazılmalı.", FIELDS[k]));
        let may = |k: usize| number(&self.fields[k], FIELDS[k]).map(|v| v.unwrap_or(0.0));
        let shape = WallShapeDef {
            top: need(SHAPE_AT)?,
            batter: may(SHAPE_AT + 1)?,
            footing: need(SHAPE_AT + 2)?,
            base: need(SHAPE_AT + 3)?,
            toe: may(SHAPE_AT + 4)?,
            cover: may(SHAPE_AT + 5)?,
            above: may(SHAPE_AT + 6)?,
        };
        let material = self.fields[5].trim();
        Ok(RetainingWallsEdit {
            operation: RoadEditOperation::Set,
            wall: self.chosen.clone(),
            name: Some(self.fields[0].trim().to_owned()),
            corridor: Some(self.corridor.clone().ok_or("Bir koridor seçin.")?),
            side: Some(self.side),
            from: Some(need(1)?),
            to: Some(need(2)?),
            step: Some(need(3)?),
            shape: Some(shape),
            highest: Some(number(&self.fields[4], FIELDS[4])?),
            material: Some((!material.is_empty()).then(|| material.to_owned())),
            expected_revision: None,
        })
    }
}

impl App {
    /// İstinat duvarı: the window on the road project's first wall, or a new one on its first corridor.
    pub(crate) fn open_retaining_walls(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("İstinat duvarı yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first() else {
            self.warn("İstinat duvarı bir koridorun yanında durur: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        let corridor = first.id.clone();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = doc.model.road_walls().first().map(|w| w.id.clone());
        self.road.retaining_wall_window = Some(Box::new(Window {
            chosen: None,
            corridor: Some(corridor),
            side: WallSide::Right,
            fields: Default::default(),
            layer,
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::RetainingWalls));
        let _ = self.retaining_wall_event(Event::Choose(chosen));
    }

    fn refresh_retaining_wall(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road.retaining_wall_window.as_mut() else {
            return;
        };
        w.preview = match w
            .input()
            .and_then(|q| road_walls_edit::preview(&doc.model, &q))
        {
            Err(why) => Preview {
                why: Some(why),
                ..Preview::default()
            },
            Ok((report, warnings)) => Preview {
                report: Some(report),
                warnings,
                why: None,
            },
        };
    }

    pub(crate) fn retaining_wall_event(&mut self, event: Event) -> Task<Message> {
        let kept = self
            .document
            .as_ref()
            .map(|d| d.model.road_walls().to_vec())
            .unwrap_or_default();
        let Some(w) = self.road.retaining_wall_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Choose(id) => {
                w.chosen = id.clone();
                let mut f: [String; 13] = Default::default();
                match id.as_ref().and_then(|id| kept.iter().find(|c| &c.id == id)) {
                    Some(c) => {
                        w.corridor = Some(c.corridor.clone());
                        w.side = c.side;
                        f[0] = c.name.clone();
                        f[1] = trimmed(c.from);
                        f[2] = trimmed(c.to);
                        f[3] = trimmed(c.step);
                        f[4] = c.highest.map(trimmed).unwrap_or_default();
                        f[5] = c.material.clone().unwrap_or_default();
                        let s = &c.shape;
                        f[SHAPE_AT] = trimmed(s.top);
                        f[SHAPE_AT + 1] = optional(s.batter);
                        f[SHAPE_AT + 2] = trimmed(s.footing);
                        f[SHAPE_AT + 3] = trimmed(s.base);
                        f[SHAPE_AT + 4] = optional(s.toe);
                        f[SHAPE_AT + 5] = optional(s.cover);
                        f[SHAPE_AT + 6] = optional(s.above);
                    }
                    None => {
                        // A new one: its sizes and stations are the user's; only the name is filled in.
                        let n = kept.len() + 1;
                        f[0] = if n == 1 { "İstinat duvarı".into() } else { format!("İstinat duvarı {n}") };
                    }
                }
                w.fields = f;
            }
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Side(s) => w.side = s,
            Event::Field(k, t) => w.fields[k.min(12)] = t,
            Event::Layer(l) => w.layer = Some(l),
            Event::Save => return self.save_retaining_wall(),
            Event::Draw => return self.draw_retaining_wall(),
            Event::Remove => return self.remove_retaining_wall(),
            Event::Close => {
                self.road.retaining_wall_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_retaining_wall();
        Task::none()
    }

    fn save_retaining_wall(&mut self) -> Task<Message> {
        let Some(w) = self.road.retaining_wall_window.as_ref() else {
            return Task::none();
        };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("İstinat duvarı yazılamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_walls_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        if let Some(r) = &out.wall {
            self.say(Level::Success, format!("“{}”: {:.2} m, en yüksek gövde {:.2} m.", r.name, r.length, r.highest));
            let id = r.id.clone();
            return self.retaining_wall_event(Event::Choose(Some(id)));
        }
        Task::none()
    }

    fn draw_retaining_wall(&mut self) -> Task<Message> {
        let Some(w) = self.road.retaining_wall_window.as_ref() else {
            return Task::none();
        };
        let (Some(id), Some(layer)) = (w.chosen.clone(), w.layer.clone()) else {
            self.warn("Çizmek için önce duvarı kaydedin ve bir katman seçin.".to_owned());
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = RetainingWallsDraw { wall: id, layer_id: layer, labels: true, expected_revision: None };
        let result = road_walls_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("İstinat duvarı planda, {:.2} m.", out.length));
            self.road.retaining_wall_window = None;
            self.dialog = None;
        }
        Task::none()
    }

    fn remove_retaining_wall(&mut self) -> Task<Message> {
        let Some(id) = self
            .road
            .retaining_wall_window
            .as_ref()
            .and_then(|w| w.chosen.clone())
        else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = RetainingWallsEdit {
            operation: RoadEditOperation::Remove,
            wall: Some(id),
            name: None,
            corridor: None,
            side: None,
            from: None,
            to: None,
            step: None,
            shape: None,
            highest: None,
            material: None,
            expected_revision: None,
        };
        let result = road_walls_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let next = self.document.as_ref().and_then(|d| d.model.road_walls().first().map(|c| c.id.clone()));
            return self.retaining_wall_event(Event::Choose(next));
        }
        Task::none()
    }

    pub(crate) fn retaining_wall_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road.retaining_wall_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mut listed: Vec<(Option<String>, String)> = vec![(None, "Yeni duvar".into())];
        listed.extend(
            model.map_or_else(Vec::new, |m| m.road_walls().iter().map(|c| (Some(c.id.clone()), c.name.clone())).collect()),
        );
        let ids: Vec<Option<String>> = listed.iter().map(|x| x.0.clone()).collect();
        let on_wall = Select::new(
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
        let sides = [WallSide::Left, WallSide::Right];
        let on_side = Select::new(["Sol", "Sağ"].map(Choice::new), sides.iter().position(|s| *s == w.side), move |i| {
            msg(Event::Side(sides[i]))
        });
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Duvar", container(on_wall).width(170), None),
            field("İstinat duvarı", 0, 150.0),
            words::field("Koridor", container(on_corridor).width(150), None),
            words::field("Yan", container(on_side).width(80), None),
            field("m", 1, 80.0),
            field("m", 2, 80.0),
            field("m", 3, 70.0),
            field("yok", 4, 80.0),
            field("", 5, 110.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let shape = row![
            field("m", SHAPE_AT, 90.0),
            field("0", SHAPE_AT + 1, 80.0),
            field("m", SHAPE_AT + 2, 90.0),
            field("m", SHAPE_AT + 3, 110.0),
            field("0", SHAPE_AT + 4, 90.0),
            field("0", SHAPE_AT + 5, 70.0),
            field("0", SHAPE_AT + 6, 110.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top).push(shape);
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(r)) => {
                body = body.push(label::caption(format!(
                    "{:.2} m · gövde yüzü {:.2} m² · beton {:.2} m³ · en yüksek gövde {:.2} m (km {})",
                    r.length,
                    r.face,
                    r.concrete,
                    r.highest,
                    r.rows.iter().find(|x| x.station == r.highest_at).map_or("", |x| x.km_text.as_str())
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let picture = crate::picture::picture(iced::widget::canvas(Along { report: r }).width(Fill).height(Fill), TITLE);
                use crate::road_dialog::{col, grouped, num};
                let list = crate::road_dialog::sheet_fill(
                    vec![
                        col("Km", 88.0),
                        col("Tür", 56.0),
                        num("Yol (m)", 72.0),
                        num("Zemin (m)", 80.0),
                        num("Üst (m)", 72.0),
                        num("Temel (m)", 80.0),
                        num("H (m)", 60.0),
                        num("Alan (m²)", 80.0),
                    ],
                    r.rows
                        .iter()
                        .map(|x| {
                            vec![
                                x.km_text.clone(),
                                if x.fill { "dolgu" } else { "yarma" }.to_owned(),
                                grouped(x.road, 2),
                                grouped(x.ground, 2),
                                grouped(x.top, 2),
                                grouped(x.footing, 2),
                                grouped(x.height, 2),
                                grouped(x.area, 2),
                            ]
                        })
                        .collect(),
                    "Duvar satırı yok",
                );
                body = body.push_grow(
                    row![picture, list]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(200.0),
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

    pub(crate) fn retaining_wall_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road.retaining_wall_window.is_none() {
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

/// The wall along its stations: the road's edge, the ground, the wall's top and its footing's top.
struct Along<'a> {
    report: &'a RetainingWallReport,
}

impl canvas::Program<Message> for Along<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let rows = &self.report.rows;
        if rows.len() < 2 {
            return vec![frame.into_geometry()];
        }
        let (s0, s1) = (rows[0].station, rows[rows.len() - 1].station);
        let z0 = rows.iter().map(|r| r.footing).fold(f64::INFINITY, f64::min);
        let z1 = rows.iter().map(|r| r.top).fold(f64::NEG_INFINITY, f64::max);
        let pad = 28.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let at = |s: f64, z: f64| {
            Point::new(
                (pad + (s - s0) / (s1 - s0).max(1e-6) * w) as f32,
                (pad + h - (z - z0) / (z1 - z0).max(1e-6) * h) as f32,
            )
        };
        let body = Path::new(|p| {
            p.move_to(at(rows[0].station, rows[0].top));
            for r in &rows[1..] {
                p.line_to(at(r.station, r.top));
            }
            for r in rows.iter().rev() {
                p.line_to(at(r.station, r.footing));
            }
            p.close();
        });
        frame.fill(&body, iced::Color { a: 0.2, ..tk.accent });
        frame.stroke(&body, Stroke::default().with_width(1.4).with_color(tk.accent));
        let line = |pick: fn(&kentos_commands::RetainingWallRow) -> f64| {
            Path::new(|p| {
                p.move_to(at(rows[0].station, pick(&rows[0])));
                for r in &rows[1..] {
                    p.line_to(at(r.station, pick(r)));
                }
            })
        };
        frame.stroke(&line(|r| r.ground), Stroke::default().with_width(1.4).with_color(tk.success));
        frame.stroke(&line(|r| r.road), Stroke::default().with_width(1.4).with_color(tk.text));
        for (word, z, color) in [("yol", rows[0].road, tk.text), ("zemin", rows[0].ground, tk.success)] {
            frame.fill_text(Text {
                content: word.into(),
                position: at(s0, z) + iced::Vector::new(4.0, -12.0),
                color,
                size: Pixels(11.0),
                ..Text::default()
            });
        }
        for r in [&rows[0], &rows[rows.len() - 1]] {
            let right = r.station == s1;
            frame.fill_text(Text {
                content: r.km_text.clone(),
                position: Point::new(at(r.station, z0).x, bounds.height - 14.0),
                color: tk.muted,
                size: Pixels(11.0),
                align_x: if right { iced::alignment::Horizontal::Right } else { iced::alignment::Horizontal::Left }.into(),
                ..Text::default()
            });
        }
        frame.fill_text(Text {
            content: format!("en yüksek {:.2} m", self.report.highest),
            position: Point::new(bounds.width / 2.0, 10.0),
            color: tk.text,
            size: Pixels(12.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            ..Text::default()
        });
        vec![frame.into_geometry()]
    }
}
