//! Taşıma planı (plan/road.md Y23 and Y24, docs/adr/0924 §3 and 0925, `haul.plan`): a mass haul diagram's earthwork moved
//! at the least haul moment or the least cost (the diagram's free haul and prices), its borrow and waste sites taking up
//! to their volume. The window reads `road.hauls.plan` (nothing written) whenever a value changes (docs/adr/0930 §4: no
//! Önizle): the flow along the road (above the line forward, below it back), the sites marked where they stand; under
//! it four figures and one list at a time: the sites against their capacity, or the costs. The layout is the road
//! windows' one (road_dialog.rs).
//!
//! ```text
//! ┌ Taşıma planı ─────────────────────────────────────────────────────────────────────── × ┐
//! │ Girdi ───────────────   ┌ the flow along the road, sites ────────────────────────────┐ │
//! │  Brükner [Yan ▾]        └────────────────────────────────────────────────────────────┘ │
//! │  Amaç [Moment|Maliyet]  [durum] [taşınan] [moment] [ortalama / toplam maliyet]        │
//! │                         (Alanlar | Maliyet)  table                                    │
//! │                                                                              [Kapat] │
//! └──────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{HaulObjective, HaulPlanned, HaulSiteKind, HaulsPlan};
use kentos_native_application::road_hauls_plan;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Taşıma planı";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
pub(crate) const FLEET: &str = "Kamyon filosu…";
pub(crate) const MOMENT: &str = "En küçük moment";
pub(crate) const COST: &str = "En düşük maliyet";
/// Planı yaz (ROAD-HAUL-12, docs/adr/0953): the plan as a workbook or a CSV.
pub(crate) const WRITE: &str = "Planı yaz…";

/// Which list the window shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Listed {
    #[default]
    Sites,
    Costs,
}

impl std::fmt::Display for Listed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if *self == Listed::Sites { "Alanlar" } else { "Maliyet" })
    }
}

/// What the plan makes least, as the window shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Aim {
    #[default]
    Moment,
    Cost,
}

impl std::fmt::Display for Aim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Aim::Moment => MOMENT,
            Aim::Cost => COST,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) haul: Option<String>,
    pub(crate) aim: Aim,
    pub(crate) preview: Option<Result<HaulPlanned, String>>,
    pub(crate) warnings: Vec<String>,
    pub(crate) list: Listed,
    /// Kamyon filosu over the window (haul_fleet_window.rs, docs/adr/0936 §3), and its values kept when closed.
    pub(crate) fleet: Option<Box<crate::haul_fleet_window::Fleet>>,
    pub(crate) fleet_kept: Option<crate::haul_fleet_window::Fleet>,
    /// The format Planı yaz writes (docs/adr/0953); none: the workbook.
    pub(crate) format: Option<kentos_commands::WorkbookFormat>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Kamyon filosu…: the fleet window on the diagram and objective chosen.
    OpenFleet,
    Fleet(crate::haul_fleet_window::Event),
    List(Listed),
    Haul(String),
    Aim(Aim),
    Preview,
    /// Planı yaz's format, and the writing.
    Format(kentos_commands::WorkbookFormat),
    Write,
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
        let _ = self.haul_plan_event(Event::Preview);
    }

    pub(crate) fn haul_plan_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenFleet => {
                self.open_haul_fleet();
                return Task::none();
            }
            Event::Fleet(e) => {
                self.haul_fleet_event(e);
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.haul_plan_window.as_mut() else { return Task::none() };
        match event {
            // Answered before the window is borrowed.
            Event::OpenFleet | Event::Fleet(_) => return Task::none(),
            Event::Haul(h) => w.haul = Some(h),
            Event::Aim(a) => w.aim = a,
            Event::List(l) => {
                w.list = l;
                return Task::none();
            }
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let read = match w.haul.clone() {
                    Some(haul) => {
                        let objective = Some(if w.aim == Aim::Cost { HaulObjective::Cost } else { HaulObjective::Moment });
                        road_hauls_plan::preview(&doc.model, &HaulsPlan { haul, objective })
                    }
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
            Event::Format(f) => {
                if let Some(w) = self.haul_plan_window.as_mut() {
                    w.format = Some(f);
                }
                return Task::none();
            }
            Event::Write => {
                let Some(w) = &self.haul_plan_window else { return Task::none() };
                let (Some(doc), Some(Ok(planned))) = (&self.document, &w.preview) else {
                    self.warn("Plan yazılamaz: önce bir Brükner seçin; plan kurulmadı.".to_owned());
                    return Task::none();
                };
                let name = doc
                    .model
                    .road_hauls()
                    .iter()
                    .find(|h| h.id == planned.haul)
                    .map_or_else(|| "taşıma planı".to_owned(), |h| format!("{} taşıma planı", h.name));
                let sheets = road_hauls_plan::sheets(&doc.model, planned);
                return crate::table_write::save(sheets, w.format.unwrap_or(kentos_commands::WorkbookFormat::Xlsx), name);
            }
            Event::Close => {
                self.haul_plan_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        // The window only reads: it reads again with the new value (docs/adr/0930 §4).
        self.haul_plan_event(Event::Preview)
    }

    pub(crate) fn haul_plan_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.haul_plan_window else {
            return iced::widget::text("").into();
        };
        if let Some(x) = &w.fleet {
            return self.haul_fleet_view(x);
        }
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_hauls().iter().map(|h| (h.id.clone(), h.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.haul.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Haul(ids[i].clone())));
        let aim = Segmented::new([Aim::Moment, Aim::Cost], w.aim, |a| msg(Event::Aim(a)));
        let form = Form::new()
            .label_width(90.0)
            .section("Girdi")
            .field("Brükner", pick)
            .field("Amaç", aim)
            .help("En küçük moment taşıma uzaklığını, en düşük maliyet Brüknerin serbest taşımasını ve fiyatlarını okur.");
        let feasible = matches!(&w.preview, Some(Ok(r)) if r.feasible);
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .ghost(FLEET, feasible.then(|| msg(Event::OpenFleet)))
            .form_width(380.0)
            .empty("Her yarma yerleşir, her dolgu karşılanır; ödünç ve döküm alanları hacimleri kadar alır.");
        match &w.preview {
            None => {}
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(r)) => {
                d = d
                    .picture(iced::widget::canvas(Flow { read: r }).width(Fill).height(Fill), 320.0)
                    .legend(if r.unsuitable.is_some() {
                        "Mavi ileri, turuncu geri taşınan hacim, mor uygunsuz kısmı; yeşil ödünç, kırmızı döküm alanı; gri çizgi engel"
                    } else {
                        "Mavi ileri, turuncu geri taşınan hacim; yeşil ödünç, kırmızı döküm alanı"
                    })
                    .figure("Durum", if r.feasible { "Kurulur".to_owned() } else { "Kurulamaz".to_owned() })
                    .figure("Yarma / dolgu fazlası", format!("{} / {} m³", grouped(r.cut, 0), grouped(r.fill, 0)));
                if r.feasible {
                    d = d.figure("Taşıma momenti", format!("{} m³·m", grouped(r.moment, 0)));
                    d = match &r.cost {
                        Some(c) => d.figure("Toplam maliyet", format!("{} {}", grouped(c.total, 2), c.currency)),
                        None => d.figure("Ortalama uzaklık", r.average.map_or_else(|| "—".into(), |a| format!("{} m", grouped(a, 1)))),
                    };
                } else {
                    d = d
                        .figure("Karşılanmayan dolgu", format!("{} m³", grouped(r.shortage, 1)))
                        .figure("Yerleşmeyen yarma", format!("{} m³", grouped(r.surplus, 1)));
                }
                // The unsuitable cut and the barriers (docs/adr/0944): each said only where there is one.
                if let Some(u) = r.unsuitable.filter(|u| *u > 0.05) {
                    let stranded = r.stranded.unwrap_or(0.0);
                    d = d.figure("Uygunsuz yarma", format!("{} m³", grouped(u, 0)));
                    if stranded > 0.05 {
                        d = d.figure("Dökülemeyen uygunsuz", format!("{} m³", grouped(stranded, 1)));
                    }
                    d = d.note(Banner::info(format!(
                        "Uygunsuz yarma {} m³ yalnız döküm alanlarına gider; momentin {} m³·m'si onun.",
                        grouped(u, 0),
                        grouped(r.unsuitable_moment.unwrap_or(0.0), 0),
                    )));
                }
                if !r.barriers.is_empty() {
                    let crossings: Vec<String> = r
                        .barriers
                        .iter()
                        .map(|b| match b.detour {
                            Some(dt) => format!("{} (km {}, {} m dolaşma): {} m³ geçiyor", b.name, grouped(b.station, 0), grouped(dt, 0), grouped(b.crossed, 0)),
                            None => format!("{} (km {}) kapalı: taşıma geçmiyor", b.name, grouped(b.station, 0)),
                        })
                        .collect();
                    d = d.note(Banner::info(format!("Engeller: {}.", crossings.join("; "))));
                }
                let sorted = r.unsuitable.is_some();
                let mut columns = vec![col("Alan", 180.0), col("Tür", 80.0), num("Km (m)", 100.0), num("Kullanılan (m³)", 130.0)];
                if sorted {
                    columns.push(num("Uygunsuz (m³)", 120.0));
                }
                columns.push(num("Kapasite (m³)", 130.0));
                let sites = road_dialog::table(
                    columns,
                    r.sites
                        .iter()
                        .map(|s| {
                            let mut row = vec![
                                s.name.clone(),
                                if s.kind == HaulSiteKind::Borrow { "ödünç" } else { "döküm" }.to_owned(),
                                grouped(s.station, 1),
                                grouped(s.used, 1),
                            ];
                            if sorted {
                                row.push(s.used_unsuitable.map_or_else(|| "—".into(), |u| grouped(u, 1)));
                            }
                            row.push(grouped(s.capacity, 1));
                            row
                        })
                        .collect(),
                    "Ödünç ya da döküm alanı yok",
                );
                d = match &r.cost {
                    Some(c) => {
                        let money = |v: f64| format!("{} {}", grouped(v, 2), c.currency);
                        let mut rows = vec![
                            vec!["Serbest taşıma".to_owned(), format!("{} m", grouped(r.free.unwrap_or(0.0), 1))],
                            vec!["Fazla taşıma momenti".to_owned(), format!("{} m³·m", grouped(c.over_moment, 0))],
                            vec!["Fazla taşıma".to_owned(), money(c.overhaul)],
                            vec!["Döküm".to_owned(), money(c.waste)],
                            vec!["Ödünç".to_owned(), money(c.borrow)],
                            vec!["Toplam".to_owned(), money(c.total)],
                        ];
                        if let Some(e) = c.economic {
                            rows.insert(1, vec!["Ekonomik taşıma sınırı".to_owned(), format!("{} m", grouped(e, 1))]);
                        }
                        let costs = road_dialog::table(vec![col("Kalem", 220.0), num("Değer", 200.0)], rows, "");
                        let shown = match w.list {
                            Listed::Sites => sites,
                            Listed::Costs => costs,
                        };
                        d.detail(road_dialog::detail_choice([Listed::Sites, Listed::Costs], w.list, |l| msg(Event::List(l)), shown))
                    }
                    None => d.detail(sites),
                };
                // Planı yaz (ROAD-HAUL-12): the summary with its units and the volume's revision and approval, the
                // flows, the sites and the barriers, as a workbook or a CSV.
                let formats: Vec<kentos_commands::WorkbookFormat> = crate::table_write::FORMATS.iter().map(|c| c.format).collect();
                let chosen = w.format.unwrap_or(kentos_commands::WorkbookFormat::Xlsx);
                let format = Select::new(
                    crate::table_write::FORMATS.iter().map(|c| Choice::new(c.label.to_owned())),
                    formats.iter().position(|f| *f == chosen),
                    move |i| msg(Event::Format(formats[i])),
                );
                d = d.tail(
                    iced::widget::row![
                        kentos_ui::label::caption("Taşıma planı")
                            .style(|t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) }),
                        iced::widget::container(format).width(240),
                        crate::exchange::words::secondary(WRITE, Some(msg(Event::Write))),
                    ]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Center),
                );
            }
        }
        d.view()
    }

    pub(crate) fn haul_plan_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.haul_plan_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        if w.fleet.is_some() {
            use crate::haul_fleet_window::{BACK, Event as X, FIELDS as FF, TITLE as FT};
            let x = |e: X| msg(Event::Fleet(e));
            return Ok(match control {
                Control::Press(BACK) => Some(x(X::Back)),
                Control::Fill(name, text) => match FF.iter().position(|f| *f == name) {
                    Some(k) => Some(x(X::Field(k, text.to_owned()))),
                    None => return Err(format!("“{FT}” penceresinde “{name}” alanı yok")),
                },
                other => return Err(format!("“{FT}” penceresinde {other} yok")),
            });
        }
        Ok(match control {
            Control::Press(FLEET) => Some(msg(Event::OpenFleet)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(MOMENT) => Some(msg(Event::Aim(Aim::Moment))),
            Control::Press(COST) => Some(msg(Event::Aim(Aim::Cost))),
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
            // The unsuitable cut's part, over the band from the axis (docs/adr/0944).
            if let Some(u) = s.unsuitable_flow.filter(|u| u.abs() > 1e-9) {
                let part = Path::rectangle(
                    Point::new(x(s.from), y(u.max(0.0))),
                    iced::Size::new(x(s.to) - x(s.from), (y(0.0) - y(u.abs())).abs()),
                );
                frame.fill(&part, Color { a: 0.75, ..Color::from_rgb8(0x93, 0x33, 0xea) });
            }
        }
        for b in &r.barriers {
            let at = x(b.station);
            let dash = Stroke { line_dash: canvas::LineDash { segments: &[5.0, 4.0], offset: 0 }, ..Stroke::default().with_width(1.5).with_color(tk.muted) };
            frame.stroke(&Path::line(Point::new(at, top as f32), Point::new(at, (h - bottom) as f32)), dash);
            let words = match b.detour {
                Some(dt) => format!("{} · +{} m", b.name, grouped(dt, 0)),
                None => format!("{} · kapalı", b.name),
            };
            caption(&mut frame, words, Point::new(at + 4.0, 6.0), tk.muted);
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
        vec![frame.into_geometry()]
    }
}
