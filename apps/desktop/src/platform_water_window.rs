//! Platform suyu (plan/road.md Y22, docs/adr/0923 §3, `corridor.water`): water on a corridor's platform. The least
//! slope is the user's; KentOS fills in none. The window reads `road.corridors.water` (nothing written) whenever a value
//! changes (docs/adr/0930 §4: no Önizle): in plan the platform's triangles, those flatter than the least red, the slopes
//! beyond faint, the low points on the platform's edges marked; under it four figures and one list at a time: the flat
//! stretches, the zero crossfall stations or the low points. The layout is the road windows' one (road_dialog.rs).
//!
//! ```text
//! ┌ Platform suyu ─────────────────────────────────────────────────────────────────────── × ┐
//! │ Girdi ─────────────────   ┌ the platform in plan, flat red, low points ───────────────┐ │
//! │  Koridor [Yan ▾]          └───────────────────────────────────────────────────────────┘ │
//! │ Standart ──────────────   [yatık üçgen] [yatık alan] [sıfır eğim] [alçak nokta]         │
//! │  En küçük eğim (%) [ ]    (Yatık yerler | Sıfır enine eğim | Alçak noktalar)  table     │
//! │                                                                               [Kapat] │
//! └───────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CorridorsWater, CorridorsWatered};
use kentos_native_application::road_corridors_water;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};
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
    pub(crate) list: Listed,
}

/// Which list the window shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Listed {
    #[default]
    Flats,
    Zeros,
    Lows,
}

impl std::fmt::Display for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Listed::Flats => "Yatık yerler",
            Listed::Zeros => "Sıfır enine eğim",
            Listed::Lows => "Alçak noktalar",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Corridor(String),
    Least(String),
    List(Listed),
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
        let _ = self.platform_water_event(Event::Preview);
    }

    pub(crate) fn platform_water_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.platform_water_window.as_mut() else { return Task::none() };
        match event {
            Event::Corridor(c) => w.corridor = Some(c),
            Event::Least(t) => w.least = t,
            Event::List(l) => {
                w.list = l;
                return Task::none();
            }
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
        // The window only reads: it reads again with the new value (docs/adr/0930 §4).
        self.platform_water_event(Event::Preview)
    }

    pub(crate) fn platform_water_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.platform_water_window else {
            return iced::widget::text("").into();
        };
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.corridor.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Corridor(ids[i].clone())));
        let least = kentos_ui::widget::text_field("", &w.least).on_input(|t| msg(Event::Least(t)));
        let form = Form::new()
            .label_width(130.0)
            .section("Girdi")
            .field("Koridor", pick)
            .section("Standart")
            .field(LEAST, least)
            .help("Suyun akması için gereken eğim tasarım standardınındır; KentOS değer vermez. Yalnız şerit ve banket okunur.");
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close));
        match &w.preview {
            None => {}
            Some(Err(why)) if w.input().is_err() => d = d.empty(why.clone()),
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok((r, tris))) => {
                d = d
                    .picture(iced::widget::canvas(Plan { tris, read: r }).width(Fill).height(Fill), 330.0)
                    .legend("Gri platform, kırmızı en küçük eğimden yatık, mavi halka alçak nokta")
                    .figure("Yatık üçgen", r.flat_triangles.to_string())
                    .figure("Yatık alan", format!("{:.2} m²", r.flat_area))
                    .figure("Sıfır enine eğim", r.zeros.len().to_string())
                    .figure("Alçak nokta", r.lows.len().to_string());
                for wn in &w.warnings {
                    d = d.note(Banner::warning(wn.clone()));
                }
                let table = match w.list {
                    Listed::Flats => road_dialog::table(
                        vec![col("Bölge", 80.0), num("Baş (m)", 100.0), num("Son (m)", 100.0), num("Alan (m²)", 100.0), num("En yatık (%)", 110.0)],
                        r.flats
                            .iter()
                            .map(|f| {
                                vec![
                                    format!("{}.", f.region + 1),
                                    format!("{:.1}", f.from),
                                    format!("{:.1}", f.to),
                                    format!("{:.2}", f.area),
                                    format!("{:.3}", 100.0 * f.least),
                                ]
                            })
                            .collect(),
                        "Yatık yer yok",
                    ),
                    Listed::Zeros => road_dialog::table(
                        vec![col("Yan", 80.0), num("Km (m)", 110.0)],
                        r.zeros.iter().map(|z| vec![if z.right { "Sağ" } else { "Sol" }.to_owned(), format!("{:.2}", z.station)]).collect(),
                        "Enine eğimin sıfır olduğu km yok",
                    ),
                    Listed::Lows => road_dialog::table(
                        vec![col("Kenar", 110.0), num("Km (m)", 110.0), num("Kot (m)", 110.0)],
                        r.lows.iter().map(|l| vec![l.code.clone(), format!("{:.2}", l.station), format!("{:.3}", l.z)]).collect(),
                        "Kenarlarda alçak nokta yok",
                    ),
                };
                d = d.detail(road_dialog::detail_choice([Listed::Flats, Listed::Zeros, Listed::Lows], w.list, |l| msg(Event::List(l)), table));
            }
        }
        d.view()
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
        vec![frame.into_geometry()]
    }
}
