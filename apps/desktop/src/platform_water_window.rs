//! Platform suyu (plan/road.md Y22, docs/adr/0923 §3, `corridor.water`): water on a corridor's platform. The least
//! slope is the user's; KentOS fills in none. Önizle runs `road.corridors.water` (nothing written): in plan the
//! platform's triangles, those flatter than the least red, the slopes beyond faint, the low points on the platform's
//! edges marked; beside, the flat stretches, the zero crossfall stations and the low points.
//!
//! ```text
//! ┌ Platform suyu ──────────────────────────────────────────────────────────────────────────────────┐
//! │ Koridor [Yan koridor ▾]  En küçük eğim (%) [ ]                                                   │
//! │ ┌ the platform in plan, flat red, low points ┐  Yatık yerler · Sıfır enine eğim · Alçak noktalar  │
//! │ └────────────────────────────────────────────┘                                                   │
//! │                                                                         [Önizle] [Kapat]         │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CorridorsWater, CorridorsWatered};
use kentos_native_application::road_corridors_water;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Platform suyu";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
pub(crate) const LEAST: &str = "En küçük eğim (%)";

/// A read: the answer and the platform's triangles (plan, paved, flat).
type Picture = Vec<road_corridors_water::PlatformTriangle>;

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) corridor: Option<String>,
    pub(crate) least: String,
    pub(crate) preview: Option<Result<(CorridorsWatered, Picture), String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Corridor(String),
    Least(String),
    Preview,
    Close,
}

fn msg(event: Event) -> Message {
    Message::PlatformWater(event)
}

impl Window {
    pub(crate) fn input(&self) -> Result<CorridorsWater, String> {
        let Some(corridor) = self.corridor.clone() else { return Err("Bir koridor seçin.".into()) };
        let t = self.least.trim();
        if t.is_empty() {
            return Err("En küçük eğimi yazın: suyun akması için gereken eğim tasarım standardınındır.".into());
        }
        let p = t.replace(',', ".").parse::<f64>().map_err(|_| format!("{LEAST} bir sayı olmalı: “{t}”."))?;
        Ok(CorridorsWater { corridor, least: p / 100.0 })
    }
}

impl App {
    /// Platform suyu: the window on the road project's first corridor.
    pub(crate) fn open_platform_water(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Platform suyu yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first().map(|c| c.id.clone()) else {
            self.warn("Platform suyu bir koridorda okunur: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        self.platform_water_window = Some(Box::new(Window { corridor: Some(first), ..Window::default() }));
        self.dialog = Some(Dialog::PlatformWater);
    }

    pub(crate) fn platform_water_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.platform_water_window.as_mut() else { return Task::none() };
        match event {
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Least(t) => w.least = t,
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let read = w.input().and_then(|q| {
                    let (r, x) = road_corridors_water::preview(&doc.model, &q)?;
                    Ok(((r, road_corridors_water::picture(&doc.model, &q)?), x))
                });
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
                self.platform_water_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.preview = None;
        w.warnings.clear();
        Task::none()
    }

    pub(crate) fn platform_water_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.platform_water_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.corridor.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = container(Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Corridor(ids[i].clone())))).width(190);
        let least = kentos_ui::widget::text_field("", &w.least).on_input(|t| msg(Event::Least(t))).width(110);
        let top = row![words::field("Koridor", pick, None), words::field(LEAST, least, None)].spacing(10).align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(12).push(top);
        match &w.preview {
            None => body = body.push(label::caption("En küçük eğim tasarım standardınındır; KentOS değer vermez. Yalnız şerit ve banket okunur.").style(muted)),
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok((r, tris))) => {
                let picture = container(iced::widget::canvas(Plan { tris, read: r }).width(Fill).height(typography::scaled(360.0))).style(style::container::field_box);
                let line = |k: String, v: String| row![label::caption(k).style(muted).width(150), label::body(v)].spacing(8);
                let mut side = Column::new().spacing(5).width(400);
                side = side.push(line("Yatık yer".into(), format!("{} üçgen · {:.2} m²", r.flat_triangles, r.flat_area)));
                for f in &r.flats {
                    side = side.push(line(
                        format!("{}. bölge", f.region + 1),
                        format!("{:.1} – {:.1} m · {:.2} m² · en yatık %{:.3}", f.from, f.to, f.area, 100.0 * f.least),
                    ));
                }
                if !r.zeros.is_empty() {
                    side = side.push(label::caption("Enine eğimin sıfır olduğu km'ler").style(muted));
                    for z in &r.zeros {
                        side = side.push(line(if z.right { "Sağ".into() } else { "Sol".into() }, format!("{:.2} m", z.station)));
                    }
                }
                if !r.lows.is_empty() {
                    side = side.push(label::caption("Kenarlardaki alçak noktalar").style(muted));
                    for l in &r.lows {
                        side = side.push(line(l.code.clone(), format!("{:.2} m · Z {:.3}", l.station, l.z)));
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

    pub(crate) fn platform_water_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.platform_water_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Fill(LEAST, t) => Some(msg(Event::Least(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

struct Plan<'a> {
    tris: &'a Picture,
    read: &'a CorridorsWatered,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (t, _, _) in self.tris {
            for p in t {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            }
        }
        if !x0.is_finite() {
            frame.fill_text(Text { content: "Üst yüzey yok.".into(), position: Point::new(10.0, 10.0), color: tk.muted, size: Pixels(11.0), ..Text::default() });
            return vec![frame.into_geometry()];
        }
        let pad = 20.0_f64;
        let scale = ((f64::from(bounds.width) - 2.0 * pad) / (x1 - x0).max(1e-6)).min((f64::from(bounds.height) - 2.0 * pad) / (y1 - y0).max(1e-6));
        let ox = pad + (f64::from(bounds.width) - 2.0 * pad - (x1 - x0) * scale) / 2.0;
        let oy = pad + (f64::from(bounds.height) - 2.0 * pad - (y1 - y0) * scale) / 2.0;
        let at = |p: [f64; 2]| Point::new((ox + (p[0] - x0) * scale) as f32, (oy + (y1 - p[1]) * scale) as f32);
        for (t, paved, flat) in self.tris {
            let shape = Path::new(|p| {
                p.move_to(at(t[0]));
                p.line_to(at(t[1]));
                p.line_to(at(t[2]));
                p.close();
            });
            let color = if *flat {
                Color { a: 0.75, ..tk.danger }
            } else if *paved {
                Color { a: 0.35, ..tk.muted }
            } else {
                Color { a: 0.12, ..tk.success }
            };
            frame.fill(&shape, color);
        }
        for l in &self.read.lows {
            frame.stroke(&Path::circle(at([l.x, l.y]), 5.0), Stroke::default().with_width(2.0).with_color(tk.accent));
        }
        frame.fill_text(Text {
            content: "Gri platform, kırmızı en küçük eğimden yatık, mavi halka alçak nokta".into(),
            position: Point::new(10.0, 6.0),
            color: tk.muted,
            size: Pixels(11.0),
            ..Text::default()
        });
        vec![frame.into_geometry()]
    }
}
