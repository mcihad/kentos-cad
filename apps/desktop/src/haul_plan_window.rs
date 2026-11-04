//! Taşıma planı (plan/road.md Y23, docs/adr/0924 §3, `haul.plan`): a mass haul diagram's earthwork moved at the least
//! haul moment, its borrow and waste sites taking up to their volume. Önizle runs `road.hauls.plan` (nothing written):
//! the flow along the road (above the line forward, below it back), the sites marked where they stand; beside, the
//! totals, the moment and its average, each site's use against its capacity, and why a plan does not close.
//!
//! ```text
//! ┌ Taşıma planı ───────────────────────────────────────────────────────────────────────────────────┐
//! │ Brükner [Yan Brükner ▾]                                                                          │
//! │ ┌ the flow along the road, sites ┐  Yarma fazlası · Dolgu fazlası · Moment · Alanlar             │
//! │ └────────────────────────────────┘                                                               │
//! │                                                                         [Önizle] [Kapat]         │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{HaulPlanned, HaulSiteKind, HaulsPlan};
use kentos_native_application::road_hauls_plan;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Taşıma planı";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) haul: Option<String>,
    pub(crate) preview: Option<Result<HaulPlanned, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Haul(String),
    Preview,
    Close,
}

fn msg(event: Event) -> Message {
    Message::HaulPlan(event)
}

/// Thousands parted by a thin space, for the large volumes and moments.
fn grouped(v: f64, places: usize) -> String {
    let text = format!("{:.places$}", v.abs());
    let (whole, part) = text.split_once('.').map_or((text.as_str(), None), |(a, b)| (a, Some(b)));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if let Some(p) = part {
        out.push(',');
        out.push_str(p);
    }
    if v < 0.0 && out.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.insert(0, '−');
    }
    out
}

impl App {
    /// Taşıma planı: the window on the road project's first mass haul diagram.
    pub(crate) fn open_haul_plan(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Taşıma planı yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_hauls().first().map(|h| h.id.clone()) else {
            self.warn("Taşıma planı bir Brüknerde kurulur: önce Brükner oluştur ile bir Brükner yazın.".to_owned());
            return;
        };
        self.haul_plan_window = Some(Box::new(Window { haul: Some(first), ..Window::default() }));
        self.dialog = Some(Dialog::HaulPlan);
    }

    pub(crate) fn haul_plan_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.haul_plan_window.as_mut() else { return Task::none() };
        match event {
            Event::Haul(h) => w.haul = Some(h),
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let read = match w.haul.clone() {
                    Some(haul) => road_hauls_plan::preview(&doc.model, &HaulsPlan { haul }),
                    None => Err("Bir Brükner seçin.".to_owned()),
                };
                match read {
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
                self.haul_plan_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.preview = None;
        w.warnings.clear();
        Task::none()
    }

    pub(crate) fn haul_plan_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.haul_plan_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_hauls().iter().map(|h| (h.id.clone(), h.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.haul.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = container(Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Haul(ids[i].clone())))).width(220);
        let mut body = Column::new().spacing(12).push(row![words::field("Brükner", pick, None)]);
        match &w.preview {
            None => {
                body = body.push(
                    label::caption(
                        "Her yarma yerleşir, her dolgu karşılanır; ödünç ve döküm alanları hacimleri kadar alır. Plan en küçük taşıma momentidir.",
                    )
                    .style(muted),
                );
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(r)) => {
                let picture = container(iced::widget::canvas(Flow { read: r }).width(Fill).height(typography::scaled(360.0))).style(style::container::field_box);
                let line = |k: String, v: String| row![label::caption(k).style(muted).width(150), label::body(v)].spacing(8);
                let mut side = Column::new().spacing(5).width(400);
                side = side.push(line("Durum".into(), if r.feasible { "Plan kurulur".into() } else { "Plan kurulamaz".into() }));
                side = side.push(line("Yarma fazlası".into(), format!("{} m³", grouped(r.cut, 1))));
                side = side.push(line("Dolgu fazlası".into(), format!("{} m³", grouped(r.fill, 1))));
                if r.feasible {
                    side = side.push(line("Taşınan".into(), format!("{} m³", grouped(r.moved, 1))));
                    side = side.push(line("Taşıma momenti".into(), format!("{} m³·m", grouped(r.moment, 0))));
                    if let Some(a) = r.average {
                        side = side.push(line("Ortalama uzaklık".into(), format!("{} m", grouped(a, 1))));
                    }
                } else {
                    if r.shortage > 0.0 {
                        side = side.push(line("Karşılanmayan dolgu".into(), format!("{} m³", grouped(r.shortage, 1))));
                    }
                    if r.surplus > 0.0 {
                        side = side.push(line("Yerleşmeyen yarma".into(), format!("{} m³", grouped(r.surplus, 1))));
                    }
                }
                if !r.sites.is_empty() {
                    side = side.push(label::caption("Alanlar").style(muted));
                    for s in &r.sites {
                        let kind = if s.kind == HaulSiteKind::Borrow { "ödünç" } else { "döküm" };
                        side = side.push(line(
                            s.name.clone(),
                            format!("{kind} · {} m · {} / {} m³", grouped(s.station, 1), grouped(s.used, 1), grouped(s.capacity, 1)),
                        ));
                    }
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, scrollable(side).height(typography::scaled(360.0))].spacing(12));
            }
        }
        body = body.push(
            row![space::horizontal().width(Fill), words::secondary(PREVIEW, Some(msg(Event::Preview))), words::secondary(CLOSE, Some(msg(Event::Close)))]
                .spacing(8),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn haul_plan_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.haul_plan_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

struct Flow<'a> {
    read: &'a HaulPlanned,
}

impl canvas::Program<Message> for Flow<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.read;
        let caption = |frame: &mut canvas::Frame, content: String, at: Point, color: Color| {
            frame.fill_text(Text { content, position: at, color, size: Pixels(11.0), ..Text::default() });
        };
        if r.segments.is_empty() {
            let why = if r.feasible { "Taşınacak bir şey yok." } else { "Plan kurulamadı: alanlar yetmiyor." };
            caption(&mut frame, why.into(), Point::new(10.0, 10.0), tk.muted);
            return vec![frame.into_geometry()];
        }
        let x0 = r.segments.first().map_or(0.0, |s| s.from).min(r.sites.iter().map(|s| s.station).fold(f64::INFINITY, f64::min));
        let x1 = r.segments.last().map_or(1.0, |s| s.to).max(r.sites.iter().map(|s| s.station).fold(f64::NEG_INFINITY, f64::max));
        let most = r.segments.iter().map(|s| s.flow.abs()).fold(1e-9, f64::max);
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let (left, right, top, bottom) = (56.0, 16.0, 26.0, 40.0);
        let mid = top + (h - top - bottom) / 2.0;
        let half = (h - top - bottom) / 2.0 - 6.0;
        let x = |s: f64| (left + (s - x0) / (x1 - x0).max(1e-9) * (w - left - right)) as f32;
        let y = |f: f64| (mid - f / most * half) as f32;
        let ahead = Color { a: 0.55, ..tk.accent };
        let back = Color { a: 0.55, ..tk.warning };
        for s in &r.segments {
            if s.flow == 0.0 {
                continue;
            }
            let band = Path::rectangle(
                Point::new(x(s.from), y(s.flow.max(0.0))),
                iced::Size::new(x(s.to) - x(s.from), (y(0.0) - y(s.flow.abs())).abs()),
            );
            frame.fill(&band, if s.flow > 0.0 { ahead } else { back });
        }
        frame.stroke(&Path::line(Point::new(x(x0), y(0.0)), Point::new(x(x1), y(0.0))), Stroke::default().with_width(1.0).with_color(tk.muted));
        caption(&mut frame, format!("{} m³", grouped(most, 0)), Point::new(4.0, y(most) - 6.0), tk.muted);
        caption(&mut frame, format!("{} m³", grouped(-most, 0)), Point::new(4.0, y(-most) - 6.0), tk.muted);
        for s in &r.sites {
            let at = x(s.station);
            let (color, up) = if s.kind == HaulSiteKind::Borrow { (tk.success, true) } else { (tk.danger, false) };
            let base = y(0.0);
            let tip = if up { base - 12.0 } else { base + 12.0 };
            let mark = Path::new(|p| {
                p.move_to(Point::new(at, tip));
                p.line_to(Point::new(at - 6.0, if up { base - 2.0 } else { base + 2.0 }));
                p.line_to(Point::new(at + 6.0, if up { base - 2.0 } else { base + 2.0 }));
                p.close();
            });
            frame.fill(&mark, color);
            let room = (w - right) as f32 - 7.0 * s.name.chars().count() as f32;
            caption(&mut frame, s.name.clone(), Point::new((at - 18.0).clamp(4.0, room.max(4.0)), if up { tip - 14.0 } else { tip + 2.0 }), color);
        }
        caption(&mut frame, format!("{} m", grouped(x0, 0)), Point::new(x(x0) - 8.0, (h - bottom + 8.0) as f32), tk.muted);
        caption(&mut frame, format!("{} m", grouped(x1, 0)), Point::new(x(x1) - 30.0, (h - bottom + 8.0) as f32), tk.muted);
        caption(
            &mut frame,
            "Mavi ileri, turuncu geri taşınan hacim; yeşil ödünç, kırmızı döküm alanı".into(),
            Point::new(10.0, 6.0),
            tk.muted,
        );
        vec![frame.into_geometry()]
    }
}
