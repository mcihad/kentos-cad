//! Yol paftaları (plan/road.md Y10.5, docs/adr/0908 §10, `road.sheets`): an alignment laid out on plan–profile sheets
//! as Civil 3D's Create View Frames and Create Sheets do. The sheets are read as they are typed (`road.sheets.plan`'s
//! plan): each sheet's frame drawn over the alignment in plan, turned along it, and listed with its km; Paftaları
//! oluştur draws the profile views, keeps the set and lays the sheets as one undo step, then shows the sheets.
//!
//! ```text
//! ┌ Yol paftaları ───────────────────────────────────────────────────────────────────────────────────┐
//! │ Aks [▾] Boykesit [▾] Katman [▾] Yer (Y; X) [ ] Ölçek 1: [ ] Abartma [ ] Bindirme (m) [ ]           │
//! │ n pafta · bir paftanın boyu L m · warnings                                                         │
//! │ ┌ the alignment and the frames ┐  No  Km                    Dönüş                                 │
//! │ └──────────────────────────────┘  …                                                                │
//! │                                                            [Kapat] [Paftaları oluştur]             │
//! └────────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CommandWarning, RoadSheetsLaid, RoadSheetsPlan};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_alignments, road_sheets};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Yol paftaları";
const CLOSE: &str = "Kapat";
const MAKE: &str = "Paftaları oluştur";

/// The sheets as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) laid: Option<RoadSheetsLaid>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The alignment's points in plan, for the picture.
    pub(crate) line: Vec<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) alignment: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) layer: Option<String>,
    /// The place (“Y; X”), the scale, the exaggeration, the overlap, the start and the end km, as typed.
    pub(crate) fields: [String; 6],
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Alignment(String),
    /// None: plan only.
    Profile(Option<String>),
    Layer(String),
    /// 0 the place, 1 the scale, 2 the exaggeration, 3 the overlap, 4 the start, 5 the end.
    Field(usize, String),
    Make,
    Close,
}

fn msg(event: Event) -> Message {
    Message::RoadSheets(event)
}

fn number(text: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("“{t}” bir sayı değil."))
}

impl Window {
    /// The command's input as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<RoadSheetsPlan, String> {
        let alignment = self.alignment.clone().ok_or("Bir aks seçin.")?;
        let at = if self.profile.is_some() {
            let parts: Vec<Option<f64>> =
                self.fields[0].split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
            match parts[..] {
                [Some(x), Some(y)] => Some([x, y]),
                _ => return Err("Boykesit görünümlerinin yerini “Y; X” olarak yazın.".into()),
            }
        } else {
            None
        };
        let scale = match number(&self.fields[1])? {
            None => None,
            Some(v) if v >= 1.0 && v.fract() == 0.0 && v <= 1e6 => Some(v as u32),
            Some(_) => return Err("Ölçek 1 ile 1 000 000 arası bir tam sayı olmalı.".into()),
        };
        Ok(RoadSheetsPlan {
            alignment,
            name: None,
            from: number(&self.fields[4])?,
            to: number(&self.fields[5])?,
            profile: self.profile.clone(),
            layer_id: self.profile.as_ref().and(self.layer.clone()),
            at,
            exaggeration: number(&self.fields[2])?,
            scale,
            overlap: number(&self.fields[3])?,
            source: None,
            options: serde_json::json!({}),
            first: None,
            expected_revision: None,
        })
    }
}

impl App {
    /// Yol paftaları: the window on the road project's first alignment built from its PIs.
    pub(crate) fn open_road_sheets(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Yol paftaları yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_alignments().iter().find(|a| a.offset_of.is_none()) else {
            self.warn("Yol paftaları bir aksın paftalarıdır: önce Güzergâh'ta bir aks oluşturun.".to_owned());
            return;
        };
        let alignment = first.id.clone();
        let profile = doc.model.road_profiles().iter().find(|p| p.alignment == alignment).map(|p| p.id.clone());
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.road_sheets_window = Some(Box::new(Window {
            alignment: Some(alignment),
            profile,
            layer,
            fields: ["0; -500".into(), "1000".into(), "5".into(), String::new(), String::new(), String::new()],
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::RoadSheets);
        self.refresh_road_sheets();
    }

    fn refresh_road_sheets(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road_sheets_window.as_mut() else { return };
        let line = w
            .alignment
            .as_ref()
            .and_then(|id| road_alignments::made_alignments(&doc.model).into_iter().find(|(d, _)| &d.id == id))
            .and_then(|(_, b)| b.ok())
            .map(|b| {
                let n = 400;
                (0..=n).filter_map(|i| b.core.at(b.core.length * i as f64 / n as f64)).map(|p| [p.x, p.y]).collect()
            })
            .unwrap_or_default();
        w.preview = match w.input().and_then(|q| road_sheets::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), line, ..Preview::default() },
            Ok((laid, warnings)) => Preview { laid: Some(laid), warnings, why: None, line },
        };
    }

    pub(crate) fn road_sheets_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.road_sheets_window.as_mut() else { return Task::none() };
        match event {
            Event::Alignment(id) => {
                w.profile = None;
                w.alignment = Some(id);
            }
            Event::Profile(p) => w.profile = p,
            Event::Layer(l) => w.layer = Some(l),
            Event::Field(k, t) => w.fields[k.min(5)] = t,
            Event::Make => return self.make_road_sheets(),
            Event::Close => {
                self.road_sheets_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_road_sheets();
        Task::none()
    }

    /// Paftaları oluştur: `road.sheets.plan`, one undo step; the sheets shown.
    fn make_road_sheets(&mut self) -> Task<Message> {
        let Some(w) = self.road_sheets_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Paftalar oluşturulamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_sheets::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        let views = out.sheets.iter().filter(|s| s.view.is_some()).count();
        self.say(
            Level::Success,
            format!("“{}”: {} pafta, {views} boykesit görünümü ({}–{}).", out.name, out.sheets.len(),
                out.sheets.first().map_or("", |s| s.number.as_str()), out.sheets.last().map_or("", |s| s.number.as_str())),
        );
        self.road_sheets_window = None;
        self.dialog = None;
        self.follow_sheet_book();
        self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(1)))
    }

    pub(crate) fn road_sheets_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road_sheets_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let alignments: Vec<(String, String)> = model.map_or_else(Vec::new, |m| {
            m.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| (a.id.clone(), a.name.clone())).collect()
        });
        let ids: Vec<String> = alignments.iter().map(|a| a.0.clone()).collect();
        let on_alignment = Select::new(
            alignments.iter().map(|a| Choice::new(a.1.clone())),
            w.alignment.as_ref().and_then(|c| ids.iter().position(|i| i == c)),
            move |i| msg(Event::Alignment(ids[i].clone())),
        )
        .placeholder("Aks");
        let profiles: Vec<(Option<String>, String)> = std::iter::once((None, "Yok (yalnız plan)".to_owned()))
            .chain(model.map_or_else(Vec::new, |m| {
                m.road_profiles()
                    .iter()
                    .filter(|p| Some(&p.alignment) == w.alignment.as_ref())
                    .map(|p| (Some(p.id.clone()), p.name.clone()))
                    .collect()
            }))
            .collect();
        let pids: Vec<Option<String>> = profiles.iter().map(|p| p.0.clone()).collect();
        let on_profile = Select::new(
            profiles.iter().map(|p| Choice::new(p.1.clone())),
            pids.iter().position(|p| *p == w.profile),
            move |i| msg(Event::Profile(pids[i].clone())),
        );
        let lids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let lnames: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let on_layer = Select::new(
            lnames.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| lids.iter().position(|id| id == l)),
            move |i| msg(Event::Layer(lids[i].clone())),
        )
        .placeholder("Katman");
        let field = |hint: &'static str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width)
        };
        let mut top = row![
            words::field("Aks", container(on_alignment).width(170), None),
            words::field("Boykesit", container(on_profile).width(170), None),
        ]
        .spacing(10);
        if w.profile.is_some() {
            top = top
                .push(words::field("Katman", container(on_layer).width(150), None))
                .push(words::field("Yer (Y; X)", field("Y; X", 0, 130.0), None))
                .push(words::field("Abartma", field("10", 2, 70.0), None));
        }
        top = top
            .push(words::field("Ölçek 1:", field("şablonun", 1, 90.0), None))
            .push(words::field("Bindirme (m)", field("0", 3, 90.0), None))
            .push(words::field("Baş km (m)", field("baştan", 4, 90.0), None))
            .push(words::field("Son km (m)", field("sona", 5, 90.0), None));
        let mut body = Column::new().spacing(10).push(top.align_y(iced::alignment::Vertical::Bottom));
        match (&w.preview.why, &w.preview.laid) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(l)) => {
                let views = l.sheets.iter().filter(|s| s.view.is_some()).count();
                body = body.push(label::caption(format!(
                    "{} pafta · bir paftanın boyu {:.1} m, haritanın yüksekliği {:.1} m (1:{}){}",
                    l.sheets.len(),
                    l.length,
                    l.height,
                    l.scale,
                    if views > 0 { format!(" · {views} boykesit görünümü çizilecek") } else { String::new() }
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let picture = container(
                    iced::widget::canvas(Plan { laid: l, line: &w.preview.line }).width(460).height(typography::scaled(300.0)),
                )
                .style(style::container::field_box);
                let mut list = Column::new().spacing(2);
                list = list.push(
                    row![
                        label::caption("No").style(muted).width(60),
                        label::caption("Km").style(muted).width(220),
                        label::caption("Dönüş").style(muted).width(70),
                        label::caption("").width(80),
                    ]
                    .spacing(6),
                );
                for s in &l.sheets {
                    list = list.push(
                        row![
                            label::caption(s.number.clone()).width(60),
                            label::caption(s.title.clone()).width(220),
                            label::caption(format!("{:.1}°", s.rotation)).width(70),
                            if s.outside { label::caption("taşıyor").style(warn) } else { label::caption("") }.width(80),
                        ]
                        .spacing(6),
                    );
                }
                body = body.push(
                    row![picture, container(scrollable(list).height(typography::scaled(300.0))).width(Fill).padding([6, 8])]
                        .spacing(12),
                );
            }
            _ => {}
        }
        body = body.push(
            row![
                label::caption("Boykesit görünümleri, takım ve paftalar tek geri alma adımı.").style(muted),
                space::horizontal().width(Fill),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(MAKE, w.preview.laid.as_ref().map(|_| msg(Event::Make))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        );
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn road_sheets_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road_sheets_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(MAKE) => Some(msg(Event::Make)),
            Control::Fill("Yer (Y; X)", t) => Some(msg(Event::Field(0, t.to_owned()))),
            Control::Fill("Ölçek 1:", t) => Some(msg(Event::Field(1, t.to_owned()))),
            Control::Fill("Abartma", t) => Some(msg(Event::Field(2, t.to_owned()))),
            Control::Fill("Bindirme (m)", t) => Some(msg(Event::Field(3, t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The alignment and each sheet's frame in plan, north up, numbered.
struct Plan<'a> {
    laid: &'a RoadSheetsLaid,
    line: &'a [[f64; 2]],
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let (hw, hh) = (0.5 * self.laid.length, 0.5 * self.laid.height);
        let corners = |c: [f64; 2], deg: f64| {
            let t = deg.to_radians();
            let (co, si) = (t.cos(), t.sin());
            [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|[u, v]| [c[0] + u * co - v * si, c[1] + u * si + v * co])
        };
        let mut pts: Vec<[f64; 2]> = self.line.to_vec();
        for s in &self.laid.sheets {
            pts.extend(corners(s.center, s.rotation));
        }
        if pts.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &pts {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        let pad = 14.0;
        let scale = ((bounds.width - 2.0 * pad) as f64 / (x1 - x0).max(1e-6)).min((bounds.height - 2.0 * pad) as f64 / (y1 - y0).max(1e-6));
        let (cx, cy) = (0.5 * (x0 + x1), 0.5 * (y0 + y1));
        let at = |p: [f64; 2]| {
            Point::new(
                (bounds.width as f64 * 0.5 + (p[0] - cx) * scale) as f32,
                (bounds.height as f64 * 0.5 - (p[1] - cy) * scale) as f32,
            )
        };
        for (k, s) in self.laid.sheets.iter().enumerate() {
            let c = corners(s.center, s.rotation);
            let path = Path::new(|b| {
                b.move_to(at(c[0]));
                for q in &c[1..] {
                    b.line_to(at(*q));
                }
                b.close();
            });
            let color = if s.outside { tk.warning } else { tk.accent };
            frame.fill(&path, iced::Color { a: 0.08, ..color });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));
            frame.fill_text(Text {
                content: format!("{}", k + 1),
                position: at(s.center),
                color: tk.muted,
                size: Pixels(11.0),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
        if self.line.len() > 1 {
            let path = Path::new(|b| {
                b.move_to(at(self.line[0]));
                for p in &self.line[1..] {
                    b.line_to(at(*p));
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.6).with_color(tk.danger));
        }
        vec![frame.into_geometry()]
    }
}
