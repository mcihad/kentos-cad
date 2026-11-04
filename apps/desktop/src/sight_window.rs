//! Görüş mesafesi (plan/road.md Y20, docs/adr/0921 §3, `profile.sight`): a profile's sight distance, an eye and an
//! object over the profile, ahead and behind. The heights, how far to look, the step and the required distance are the
//! user's (the design standard's); KentOS fills in none. Önizle runs `road.profiles.sight` (nothing written): above,
//! the profile; under it the distance seen each way at every station, the required distance dashed, the stations
//! seen shorter red; beside, the shortest each way and the short stretches.
//!
//! ```text
//! ┌ Görüş mesafesi ─────────────────────────────────────────────────────────────────────────────────┐
//! │ Boykesit [Proje kotu ▾] Yön [İki yön ▾] Göz [ ] Nesne [ ] Bakılan [ ] Adım [ ] Gereken [ ]        │
//! │ ┌ profile ───────────────────────────┐  En kısa ileri … · En kısa geri …                         │
//! │ ├ distance seen ─────────────────────┤  Kısa aralıklar …                                         │
//! │ └────────────────────────────────────┘                                                          │
//! │                                                                         [Önizle] [Kapat]         │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{ProfilesSight, ProfilesSightRead, SightReach, SightSide};
use kentos_native_application::road_profiles_sight;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Görüş mesafesi";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 5] = ["Göz (m)", "Nesne (m)", "Bakılan (m)", "Adım (m)", "Gereken (m)"];
const SIDES: [&str; 3] = ["İki yön", "İleri", "Geri"];

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) profile: Option<String>,
    pub(crate) side: usize,
    pub(crate) fields: [String; 5],
    pub(crate) preview: Option<Result<ProfilesSightRead, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Profile(String),
    Side(usize),
    Field(usize, String),
    Preview,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Sight(event)
}

impl Window {
    pub(crate) fn input(&self) -> Result<ProfilesSight, String> {
        let Some(profile) = self.profile.clone() else { return Err("Bir boykesit seçin.".into()) };
        let number = |k: usize| -> Result<Option<f64>, String> {
            let t = self.fields[k].trim();
            if t.is_empty() {
                return Ok(None);
            }
            t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{} bir sayı olmalı: “{t}”.", FIELDS[k]))
        };
        let need = |k: usize| -> Result<f64, String> {
            number(k)?.ok_or_else(|| {
                format!("{} yazılmadı: göz ve nesne yüksekliği, bakılan uzaklık ve adım tasarım standardınındır.", FIELDS[k])
            })
        };
        Ok(ProfilesSight {
            profile,
            eye: need(0)?,
            object: need(1)?,
            reach: need(2)?,
            step: need(3)?,
            from: None,
            to: None,
            side: [SightSide::Both, SightSide::Forward, SightSide::Backward][self.side.min(2)],
            required: number(4)?,
        })
    }
}

impl App {
    /// Görüş mesafesi: the window on the road project's first profile.
    pub(crate) fn open_sight(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Görüş mesafesi yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_profiles().first().map(|p| p.id.clone()) else {
            self.warn("Görüş mesafesi bir boykesitte okunur: önce Boykesit'te bir boykesit oluşturun.".to_owned());
            return;
        };
        self.sight_window = Some(Box::new(Window { profile: Some(first), ..Window::default() }));
        self.dialog = Some(Dialog::Sight);
    }

    pub(crate) fn sight_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.sight_window.as_mut() else { return Task::none() };
        match event {
            Event::Profile(p) => w.profile = Some(p),
            Event::Side(i) => w.side = i.min(2),
            Event::Field(k, t) => w.fields[k.min(FIELDS.len() - 1)] = t,
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                match w.input().and_then(|q| road_profiles_sight::preview(&doc.model, &q)) {
                    Ok((r, warnings)) => {
                        w.preview = Some(Ok(r));
                        w.warnings = warnings.into_iter().map(|x| x.message).collect();
                    }
                    Err(why) => {
                        w.preview = Some(Err(why));
                        w.warnings.clear();
                    }
                }
                return Task::none();
            }
            Event::Close => {
                self.sight_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.preview = None;
        w.warnings.clear();
        Task::none()
    }

    pub(crate) fn sight_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.sight_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let profiles: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone())).collect());
        let ids: Vec<String> = profiles.iter().map(|p| p.0.clone()).collect();
        let chosen = w.profile.as_ref().and_then(|p| ids.iter().position(|i| i == p));
        let pick = container(Select::new(profiles.iter().map(|p| Choice::new(p.1.clone())), chosen, move |i| msg(Event::Profile(ids[i].clone()))))
            .width(170);
        let side = container(Select::new(SIDES.iter().map(|s| Choice::new(*s)), Some(w.side), |i| msg(Event::Side(i)))).width(110);
        let field = |k: usize| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(if k == 4 { "yok" } else { "" }, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(95),
                None,
            )
        };
        let mut top = row![words::field("Boykesit", pick, None), words::field("Yön", side, None)].spacing(10).align_y(iced::alignment::Vertical::Bottom);
        for k in 0..FIELDS.len() {
            top = top.push(field(k));
        }
        let mut body = Column::new().spacing(12).push(top);
        match &w.preview {
            None => {
                body = body.push(
                    label::caption("Göz ve nesne yüksekliği, bakılan uzaklık, adım ve gereken mesafe tasarım standardınındır; KentOS değer vermez. Yalnız boykesit okunur: arazi, plan ve engeller değil.")
                        .style(muted),
                )
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(r)) => {
                let picture = container(iced::widget::canvas(Chart { read: r, required: w.input().ok().and_then(|q| q.required) }).width(Fill).height(typography::scaled(380.0)))
                    .style(style::container::field_box);
                let line = |k: String, v: String| row![label::caption(k).style(muted).width(130), label::body(v)].spacing(8);
                let mut side = Column::new().spacing(5).width(380);
                side = side.push(line("Okunan".into(), format!("{} km noktası · adım {} m", r.rows.len(), r.step)));
                if let Some(l) = &r.least_forward {
                    side = side.push(line("En kısa ileri".into(), format!("{:.1} m · km {:.1}", l.distance, l.station)));
                }
                if let Some(l) = &r.least_backward {
                    side = side.push(line("En kısa geri".into(), format!("{:.1} m · km {:.1}", l.distance, l.station)));
                }
                if !r.short.is_empty() {
                    side = side.push(label::caption("Kısa aralıklar").style(muted));
                    for s in &r.short {
                        let way = if s.side == SightSide::Forward { "İleri" } else { "Geri" };
                        side = side.push(line(way.into(), format!("{:.1} – {:.1} m · en kısa {:.1} m", s.from, s.to, s.least)));
                    }
                }
                side = side.push(
                    label::caption(format!(
                        "Uzaklık {} m adımlarla ilerler, ilk kapanan çizgiden sonra {} mm'ye ikiye bölünür; adımdan kısa bir kapanma görülmez.",
                        r.step,
                        r.resolution * 1000.0
                    ))
                    .style(muted),
                );
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, scrollable(side).height(typography::scaled(380.0))].spacing(12));
            }
        }
        body = body.push(
            row![
                space::horizontal().width(Fill),
                words::secondary(PREVIEW, Some(msg(Event::Preview))),
                words::secondary(CLOSE, Some(msg(Event::Close)))
            ]
            .spacing(8),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn sight_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.sight_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Fill(name, t) if !name.is_empty() && FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The profile above, the distance seen under it.
struct Chart<'a> {
    read: &'a ProfilesSightRead,
    required: Option<f64>,
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let rows = &self.read.rows;
        let text = |frame: &mut canvas::Frame, s: String, p: Point, color: Color| {
            frame.fill_text(Text { content: s, position: p, color, size: Pixels(11.0), ..Text::default() })
        };
        if rows.len() < 2 {
            text(&mut frame, "Çizilecek km yok.".into(), Point::new(10.0, 10.0), tk.muted);
            return vec![frame.into_geometry()];
        }
        let (left, right) = (54.0_f32, bounds.width - 14.0);
        let (s0, s1) = (rows[0].station, rows[rows.len() - 1].station);
        let x = |s: f64| left + ((s - s0) / (s1 - s0).max(1e-9)) as f32 * (right - left);
        // The profile, in the upper third.
        let (zlo, zhi) = rows.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| (a.min(r.elevation), b.max(r.elevation)));
        let (ptop, pbot) = (24.0_f32, bounds.height * 0.34);
        let pz = |z: f64| pbot - ((z - zlo) / (zhi - zlo).max(1e-6)) as f32 * (pbot - ptop);
        let profile = Path::new(|p| {
            p.move_to(Point::new(x(rows[0].station), pz(rows[0].elevation)));
            for r in &rows[1..] {
                p.line_to(Point::new(x(r.station), pz(r.elevation)));
            }
        });
        frame.stroke(&profile, Stroke::default().with_width(2.0).with_color(tk.danger));
        text(&mut frame, "Boykesit".into(), Point::new(10.0, 6.0), tk.muted);
        text(&mut frame, format!("{zhi:.2}"), Point::new(4.0, ptop - 6.0), tk.muted);
        text(&mut frame, format!("{zlo:.2}"), Point::new(4.0, pbot - 6.0), tk.muted);
        // The distance seen, below.
        let (dtop, dbot) = (bounds.height * 0.42, bounds.height - 26.0);
        let most = rows
            .iter()
            .flat_map(|r| [r.forward.as_ref(), r.backward.as_ref()])
            .flatten()
            .map(|x| x.distance)
            .chain(self.required)
            .fold(1.0_f64, f64::max);
        let dy = |d: f64| dbot - (d / most) as f32 * (dbot - dtop);
        frame.stroke(
            &Path::line(Point::new(left, dbot), Point::new(right, dbot)),
            Stroke::default().with_width(1.0).with_color(tk.border),
        );
        text(&mut frame, "Görülen uzaklık (m)".into(), Point::new(10.0, dtop - 18.0), tk.muted);
        text(&mut frame, format!("{most:.0}"), Point::new(4.0, dtop - 6.0), tk.muted);
        text(&mut frame, "0".into(), Point::new(4.0, dbot - 6.0), tk.muted);
        let series = |pick: fn(&kentos_commands::SightRow) -> Option<&SightReach>, color: Color, frame: &mut canvas::Frame| {
            let pts: Vec<Point> = rows.iter().filter_map(|r| pick(r).map(|v| Point::new(x(r.station), dy(v.distance)))).collect();
            if pts.len() > 1 {
                let path = Path::new(|p| {
                    p.move_to(pts[0]);
                    for q in &pts[1..] {
                        p.line_to(*q);
                    }
                });
                frame.stroke(&path, Stroke::default().with_width(1.8).with_color(color));
            }
            for r in rows {
                if let Some(v) = pick(r) {
                    let short = v.until == kentos_commands::SightUntil::Hidden && self.required.is_some_and(|q| v.distance < q);
                    let at = Point::new(x(r.station), dy(v.distance));
                    frame.fill(&Path::circle(at, if short { 3.5 } else { 2.2 }), if short { tk.danger } else { color });
                }
            }
        };
        series(|r| r.forward.as_ref(), tk.accent, &mut frame);
        series(|r| r.backward.as_ref(), tk.success, &mut frame);
        if let Some(q) = self.required {
            let y = dy(q);
            frame.stroke(
                &Path::line(Point::new(left, y), Point::new(right, y)),
                Stroke { line_dash: LineDash { segments: &[6.0, 4.0], offset: 0 }, ..Stroke::default().with_width(1.2).with_color(tk.danger) },
            );
            text(&mut frame, format!("Gereken {q} m"), Point::new(right - 110.0, y - 15.0), tk.danger);
        }
        text(&mut frame, format!("{s0:.0}"), Point::new(left - 4.0, bounds.height - 18.0), tk.muted);
        text(&mut frame, format!("{s1:.0} m"), Point::new(right - 40.0, bounds.height - 18.0), tk.muted);
        text(&mut frame, "— İleri".into(), Point::new(150.0, dtop - 18.0), tk.accent);
        text(&mut frame, "— Geri".into(), Point::new(205.0, dtop - 18.0), tk.success);
        vec![frame.into_geometry()]
    }
}
