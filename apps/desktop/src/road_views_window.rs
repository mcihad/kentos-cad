//! Görünüşler (plan/road.md Y13.4, docs/adr/0911 §5, `road.views`; ROAD-MOD-05): an alignment's plan, its profile and
//! the section at a km side by side, one km driving the three, as Civil 3D's Section Editor (plan, profile and section
//! viewports, the station slider line in plan and profile, previous, next, first, last and a typed station) and
//! OpenRoads' Dynamic Sections with Synchronize View (the section closest to the cursor's station, Locate Station Via
//! Datapoint) show them. Nothing is written: the plan is the alignment built now, the profile the Boykesit window's
//! chart with the km's line, the section the nearest of a sample line set's sections or a corridor's.
//!
//! ```text
//! ┌ Görünüşler ───────────────────────────────────────────────────────────────────────────────────┐
//! │ Aks [Ana aks ▾]  Boykesit [Proje kotu ▾]  Enkesit [Ana koridor (koridor) ▾]  Abartma [10]      │
//! │ ┌ plan: the axis, the km's sample line ┐ ┌ profile: terrain, design, the km's dashed line ────┐ │
//! │ └──────────────────────────────────────┘ └──────────────────────────────────────────────────┘ │
//! │ ┌ section at the km: terrain, platform, slopes ─────────────────────────────────────────────┐ │
//! │ └────────────────────────────────────────────────────────────────────────────────────────────┘ │
//! │ [⏮][◀] ━━━━━━━━━━━━○━━━━━━━━━━ [▶][⏭]  Km [1+150]   Kesit km 1+150.000 · 3 / 8 · Yarma … Dolgu … │
//! │                                                                                      [Kapat]  │
//! └────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! A press in the plan or the profile, or a drag along them, moves the km there; the arrows step between the sections'
//! km's; the slider and the typed km (with the alignment's equations) move it anywhere, the section shown the nearest.

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, container, row, slider, space};
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Size, Task, Theme, mouse};
use kentos_commands::{ProfileDef, SectionRow};
use kentos_native_application::road_corridors::CorridorSections;
use kentos_native_application::{road_alignments, road_sections};
use kentos_road::alignment::km_text;
use kentos_road::alignment::stationing::Stationing;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::profile_window::{self, Ground};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Görünüşler";
const CLOSE: &str = "Kapat";
const FIRST: &str = "İlk kesit";
const PREVIOUS: &str = "Önceki kesit";
const NEXT: &str = "Sonraki kesit";
const LAST: &str = "Son kesit";

/// Where the sections come from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    None,
    /// A sample line set, by id.
    Set(String),
    /// A corridor, by id.
    Corridor(String),
}

/// What the window draws, read when the drawing or a choice changes.
#[derive(Debug, Clone, Default)]
pub(crate) struct Read {
    /// The alignment's points in plan, with their stations.
    pub(crate) line: Vec<(f64, [f64; 2])>,
    pub(crate) length: f64,
    /// The km regions, for typed km's and the slider's words.
    regions: Vec<kentos_road::alignment::stationing::Region>,
    pub(crate) ground: Ground,
    pub(crate) profile: Option<ProfileDef>,
    /// The sections, in km order.
    pub(crate) rows: Vec<SectionRow>,
    /// Why there is nothing to show (the alignment, the sections).
    pub(crate) why: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) alignment: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) source: Source,
    /// The km shown, as a station (m from the alignment's start).
    pub(crate) station: f64,
    /// The km as typed; the exaggeration as typed.
    pub(crate) km: String,
    pub(crate) exaggeration: String,
    pub(crate) read: Read,
}

#[derive(Debug, Clone)]
pub enum Event {
    Alignment(String),
    Profile(Option<String>),
    Source(Source),
    /// The km moved (a press or a drag in the plan or the profile, the slider): a station.
    Station(f64),
    Km(String),
    /// The typed km taken.
    KmGo,
    /// −2 the first section, −1 the one before, 1 the one after, 2 the last.
    Step(i8),
    Exaggeration(String),
    Close,
}

fn msg(event: Event) -> Message {
    Message::RoadViews(event)
}

fn at_station(s: f64) -> Message {
    msg(Event::Station(s))
}

/// A typed km: “1+150,5” (km and m) or “1150,5” (m), read as the alignment's km.
fn typed_km(text: &str) -> Result<f64, String> {
    let t = text.trim().replace(',', ".");
    let bad = || format!("“{}” bir km değil: 1+150 ya da 1150 yazın.", text.trim());
    match t.split_once('+') {
        Some((k, m)) => {
            let k: f64 = k.trim().parse().map_err(|_| bad())?;
            let m: f64 = m.trim().parse().map_err(|_| bad())?;
            Ok(k * 1000.0 + m)
        }
        None => t.parse().map_err(|_| bad()),
    }
}

impl Window {
    /// The section shown: the nearest to the km (OpenRoads' closest section), and its place among them.
    pub(crate) fn shown(&self) -> Option<(usize, &SectionRow)> {
        self.read
            .rows
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1.station - self.station).abs().total_cmp(&(b.1.station - self.station).abs()))
    }

    fn km_of(&self, s: f64) -> f64 {
        if self.read.regions.is_empty() { s } else { Stationing::km(&self.read.regions, s) }
    }

    fn set_station(&mut self, s: f64) {
        self.station = s.clamp(0.0, self.read.length.max(0.0));
        self.km = km_text(self.km_of(self.station), 3);
    }
}

impl App {
    /// Görünüşler: the window on the road project's first alignment, its first profile and its first corridor or
    /// section set.
    pub(crate) fn open_road_views(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Görünüşler yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_alignments().first() else {
            self.warn("Görünüşler bir aksın görünüşleridir: önce Güzergâh'ta bir aks oluşturun.".to_owned());
            return;
        };
        let alignment = first.id.clone();
        let (profile, source) = choices_on(&doc.model, &alignment);
        self.road_views_window = Some(Box::new(Window {
            alignment: Some(alignment),
            profile,
            source,
            station: 0.0,
            km: String::new(),
            exaggeration: "10".into(),
            read: Read::default(),
        }));
        self.dialog = Some(Dialog::RoadViews);
        self.refresh_road_views();
        // The first section's km, where there is one.
        if let Some(w) = self.road_views_window.as_mut() {
            let s = w.read.rows.first().map_or(0.0, |r| r.station);
            w.set_station(s);
        }
    }

    /// The window's drawing read again.
    fn refresh_road_views(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road_views_window.as_mut() else { return };
        let model = &doc.model;
        let mut read = Read::default();
        let built = w
            .alignment
            .as_ref()
            .and_then(|id| road_alignments::made_alignments(model).into_iter().find(|(d, _)| &d.id == id))
            .map(|(_, b)| b);
        match built {
            Some(Ok(b)) => {
                read.length = b.length();
                let n = 1000;
                read.line = (0..=n)
                    .filter_map(|i| b.place(read.length * i as f64 / n as f64))
                    .map(|p| (p.station, [p.x, p.y]))
                    .collect();
                read.regions = b.regions.clone();
            }
            Some(Err(why)) => read.why.push(format!("Aks kurulamıyor: {why}")),
            None => read.why.push("Bir aks seçin.".into()),
        }
        read.profile = w.profile.as_ref().and_then(|p| model.road_profiles().iter().find(|q| &q.id == p)).cloned();
        if let Some(a) = &w.alignment {
            read.ground = profile_window::ground_of(model, a, read.profile.as_ref().and_then(|p| p.surface.as_deref()));
        }
        match &w.source {
            Source::None => {}
            Source::Set(id) => match model.road_sections().iter().find(|s| &s.id == id) {
                Some(def) => match road_sections::preview(model, def, None) {
                    Ok((rows, _)) => read.rows = rows,
                    Err(why) => read.why.push(format!("“{}” kesit takımı kurulamıyor: {why}", def.name)),
                },
                None => read.why.push("Kesit takımı projede yok.".into()),
            },
            Source::Corridor(id) => match CorridorSections::of(model, id) {
                Ok(c) => read.rows = c.stations().into_iter().filter_map(|s| c.row_at(s)).collect(),
                Err(why) => read.why.push(why),
            },
        }
        read.rows.sort_by(|a, b| a.station.total_cmp(&b.station));
        w.read = read;
        let s = w.station;
        w.set_station(s);
    }

    pub(crate) fn road_views_event(&mut self, event: Event) -> Task<Message> {
        let Some(doc) = &self.document else { return Task::none() };
        let Some(w) = self.road_views_window.as_mut() else { return Task::none() };
        match event {
            Event::Alignment(id) => {
                let (profile, source) = choices_on(&doc.model, &id);
                w.alignment = Some(id);
                w.profile = profile;
                w.source = source;
                w.station = 0.0;
            }
            Event::Profile(p) => w.profile = p,
            Event::Source(s) => w.source = s,
            Event::Station(s) => {
                w.set_station(s);
                return Task::none();
            }
            Event::Km(t) => {
                w.km = t;
                return Task::none();
            }
            Event::KmGo => {
                let found = typed_km(&w.km).map(|k| {
                    if w.read.regions.is_empty() {
                        Some(k)
                    } else {
                        Stationing::stations(&w.read.regions, k).first().map(|(_, s)| *s)
                    }
                });
                match found {
                    Ok(Some(s)) if (-1e-9..=w.read.length + 1e-9).contains(&s) => w.set_station(s),
                    Ok(_) => {
                        let words = format!("km {} bu aksın üstünde değil.", w.km.trim());
                        self.warn(words);
                    }
                    Err(why) => self.warn(why),
                }
                return Task::none();
            }
            Event::Step(d) => {
                let stations: Vec<f64> = w.read.rows.iter().map(|r| r.station).collect();
                let here = w.station;
                let to = match d {
                    -2 => stations.first().copied(),
                    2 => stations.last().copied(),
                    -1 => stations.iter().rev().find(|s| **s < here - 1e-6).copied(),
                    _ => stations.iter().find(|s| **s > here + 1e-6).copied(),
                };
                if let Some(s) = to {
                    w.set_station(s);
                }
                return Task::none();
            }
            Event::Exaggeration(t) => {
                w.exaggeration = t;
                return Task::none();
            }
            Event::Close => {
                self.road_views_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_road_views();
        Task::none()
    }

    pub(crate) fn road_views_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road_views_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        // The choices.
        let alignments: Vec<(String, String)> =
            model.map_or_else(Vec::new, |m| m.road_alignments().iter().map(|a| (a.id.clone(), a.name.clone())).collect());
        let ids: Vec<String> = alignments.iter().map(|a| a.0.clone()).collect();
        let on_alignment = Select::new(
            alignments.iter().map(|a| Choice::new(a.1.clone())),
            w.alignment.as_ref().and_then(|c| ids.iter().position(|i| i == c)),
            move |i| msg(Event::Alignment(ids[i].clone())),
        )
        .placeholder("Aks");
        let on = |id: &String| Some(id) == w.alignment.as_ref();
        let profiles: Vec<(Option<String>, String)> = std::iter::once((None, "Yok".to_owned()))
            .chain(model.map_or_else(Vec::new, |m| {
                m.road_profiles().iter().filter(|p| on(&p.alignment)).map(|p| (Some(p.id.clone()), p.name.clone())).collect()
            }))
            .collect();
        let pids: Vec<Option<String>> = profiles.iter().map(|p| p.0.clone()).collect();
        let on_profile = Select::new(
            profiles.iter().map(|p| Choice::new(p.1.clone())),
            pids.iter().position(|p| *p == w.profile),
            move |i| msg(Event::Profile(pids[i].clone())),
        );
        let sources: Vec<(Source, String)> = std::iter::once((Source::None, "Yok".to_owned()))
            .chain(model.map_or_else(Vec::new, |m| {
                m.road_corridors()
                    .iter()
                    .filter(|c| on(&c.alignment))
                    .map(|c| (Source::Corridor(c.id.clone()), format!("{} (koridor)", c.name)))
                    .chain(
                        m.road_sections()
                            .iter()
                            .filter(|s| on(&s.alignment))
                            .map(|s| (Source::Set(s.id.clone()), format!("{} (kesit takımı)", s.name))),
                    )
                    .collect()
            }))
            .collect();
        let sids: Vec<Source> = sources.iter().map(|s| s.0.clone()).collect();
        let on_source = Select::new(
            sources.iter().map(|s| Choice::new(s.1.clone())),
            sids.iter().position(|s| *s == w.source),
            move |i| msg(Event::Source(sids[i].clone())),
        );
        let exaggeration = kentos_ui::widget::text_field("10", &w.exaggeration)
            .on_input(|t| msg(Event::Exaggeration(t)))
            .width(70);
        let top = row![
            words::field("Aks", container(on_alignment).width(170), None),
            words::field("Boykesit", container(on_profile).width(170), None),
            words::field("Enkesit", container(on_source).width(230), None),
            words::field("Düşey abartma", exaggeration, None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(10).push(top);
        for why in &w.read.why {
            body = body.push(label::caption(why.clone()).style(danger));
        }
        // The three views share what the window leaves: the plan and the profile 55 %, the section the rest.
        let room = (self.window_size.height - typography::scaled(330.0)).clamp(260.0, 620.0);
        let (upper, lower) = (room * 0.55, room * 0.45);
        let shown = w.shown();
        let ex = w.exaggeration.trim().replace(',', ".").parse::<f64>().ok().filter(|v| *v > 0.0).unwrap_or(10.0);
        let km = km_text(w.km_of(w.station), 3);
        let plan = iced::widget::canvas(Plan { line: &w.read.line, station: w.station, km, row: shown.map(|s| s.1) })
            .width(typography::scaled(380.0))
            .height(upper);
        let profile = iced::widget::canvas(profile_window::Chart::of(
            &w.read.ground,
            w.read.profile.as_ref(),
            ex,
            Some(w.station),
            at_station,
        )
        .compact(upper < typography::scaled(300.0)))
        .width(Fill)
        .height(upper);
        let section: Element<'_, Message> = match shown {
            Some((_, r)) => iced::widget::canvas(crate::section_window::Chart { row: Some(r), sample: None, exaggeration: None, layers: None })
                .width(Fill)
                .height(lower)
                .into(),
            None => container(label::caption("Enkesit seçilmedi ya da kesiti yok.").style(muted))
                .center(Fill)
                .height(lower)
                .into(),
        };
        body = body.push(
            row![
                container(plan).style(style::container::field_box),
                container(profile).style(style::container::field_box).width(Fill),
            ]
            .spacing(10),
        );
        body = body.push(container(section).style(style::container::field_box).width(Fill));
        // The km: the steps, the slider, the typed km, the section shown.
        let n = w.read.rows.len();
        let k = shown.map(|s| s.0);
        let length = w.read.length.max(1e-6);
        let slide = slider(0.0..=length, w.station.clamp(0.0, length), at_station).step(0.01).width(Fill);
        let km_field = kentos_ui::widget::text_field("1+150", &w.km)
            .on_input(|t| msg(Event::Km(t)))
            .on_submit(msg(Event::KmGo))
            .width(110);
        let said = match shown {
            Some((i, r)) => {
                let areas = r.cut.zip(r.fill).map_or_else(String::new, |(c, f)| {
                    format!(" · Yarma {} m² · Dolgu {} m²", comma(c, 2), comma(f, 2))
                });
                format!("Kesit km {} · {} / {n}{areas}", r.km_text, i + 1)
            }
            None => "Kesit yok".to_owned(),
        };
        let step = |words: &'static str, d: i8, enabled: bool| {
            words::secondary(words, enabled.then(|| msg(Event::Step(d))))
        };
        body = body.push(
            row![
                step(FIRST, -2, n > 0 && k != Some(0)),
                step(PREVIOUS, -1, w.read.rows.first().is_some_and(|r| r.station < w.station - 1e-6)),
                slide,
                step(NEXT, 1, w.read.rows.last().is_some_and(|r| r.station > w.station + 1e-6)),
                step(LAST, 2, n > 0 && k != Some(n - 1)),
                words::field("Km", km_field, None),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        body = body.push(
            row![
                label::caption(said),
                space::horizontal().width(Fill),
                label::caption("Plana ya da boykesite basın, sürükleyin: km oraya gider.").style(muted),
                words::secondary(CLOSE, Some(msg(Event::Close))),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
        );
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Close))
    }

    pub(crate) fn road_views_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road_views_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(FIRST) => Some(msg(Event::Step(-2))),
            Control::Press(PREVIOUS) => Some(msg(Event::Step(-1))),
            Control::Press(NEXT) => Some(msg(Event::Step(1))),
            Control::Press(LAST) => Some(msg(Event::Step(2))),
            Control::Fill("Km", t) => Some(msg(Event::Km(t.to_owned()))),
            Control::Fill("Düşey abartma", t) => Some(msg(Event::Exaggeration(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// An alignment's first profile and its first corridor, else its first sample line set.
fn choices_on(doc: &kentos_domain::Document, alignment: &str) -> (Option<String>, Source) {
    let profile = doc.road_profiles().iter().find(|p| p.alignment == alignment).map(|p| p.id.clone());
    let source = doc
        .road_corridors()
        .iter()
        .find(|c| c.alignment == alignment)
        .map(|c| Source::Corridor(c.id.clone()))
        .or_else(|| doc.road_sections().iter().find(|s| s.alignment == alignment).map(|s| Source::Set(s.id.clone())))
        .unwrap_or(Source::None);
    (profile, source)
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

/// The alignment in plan, north up: its line, the km's sample line (the shown section's, else a short tick across the
/// axis) and its km; a press or a drag moves the km to the nearest point of the axis.
struct Plan<'a> {
    line: &'a [(f64, [f64; 2])],
    station: f64,
    km: String,
    row: Option<&'a SectionRow>,
}

impl Plan<'_> {
    /// The fit of the line in `size`: the scale and the centre.
    fn fit(&self, size: Size) -> Option<(f64, [f64; 2])> {
        let mut pts: Vec<[f64; 2]> = self.line.iter().map(|p| p.1).collect();
        if let Some(r) = self.row {
            pts.extend(r.ends);
        }
        if pts.len() < 2 {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &pts {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        let pad = 18.0;
        let scale = ((size.width - 2.0 * pad) as f64 / (x1 - x0).max(1e-6))
            .min((size.height - 2.0 * pad) as f64 / (y1 - y0).max(1e-6));
        Some((scale, [0.5 * (x0 + x1), 0.5 * (y0 + y1)]))
    }

    fn at(size: Size, scale: f64, c: [f64; 2], p: [f64; 2]) -> Point {
        Point::new(
            (size.width as f64 * 0.5 + (p[0] - c[0]) * scale) as f32,
            (size.height as f64 * 0.5 - (p[1] - c[1]) * scale) as f32,
        )
    }

    /// The station of the axis's point nearest to `at` on the screen.
    fn station_at(&self, size: Size, at: Point) -> Option<f64> {
        let (scale, c) = self.fit(size)?;
        self.line
            .iter()
            .map(|(s, p)| {
                let q = Self::at(size, scale, c, *p);
                (*s, (q.x - at.x).hypot(q.y - at.y))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(s, _)| s)
    }

    /// The axis's point at a station, linear between its samples.
    fn point(&self, s: f64) -> Option<[f64; 2]> {
        let i = self.line.partition_point(|p| p.0 <= s);
        let (a, b) = (self.line.get(i.checked_sub(1)?)?, self.line.get(i).or(self.line.last())?);
        let t = if b.0 > a.0 { (s - a.0) / (b.0 - a.0) } else { 0.0 };
        Some([a.1[0] + (b.1[0] - a.1[0]) * t, a.1[1] + (b.1[1] - a.1[1]) * t])
    }
}

impl canvas::Program<Message> for Plan<'_> {
    /// Whether the km is being dragged.
    type State = bool;

    fn update(
        &self,
        dragging: &mut bool,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let s = self.station_at(bounds.size(), cursor.position_in(bounds)?)?;
                *dragging = true;
                Some(canvas::Action::publish(at_station(s)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if *dragging => {
                let p = cursor.position()?;
                let s = self.station_at(bounds.size(), Point::new(p.x - bounds.x, p.y - bounds.y))?;
                Some(canvas::Action::publish(at_station(s)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *dragging => {
                *dragging = false;
                Some(canvas::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(&self, _: &bool, renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let size = bounds.size();
        let Some((scale, c)) = self.fit(size) else {
            return vec![frame.into_geometry()];
        };
        let at = |p: [f64; 2]| Self::at(size, scale, c, p);
        let axis = Path::new(|b| {
            for (i, (_, p)) in self.line.iter().enumerate() {
                if i == 0 { b.move_to(at(*p)) } else { b.line_to(at(*p)) }
            }
        });
        frame.stroke(&axis, Stroke::default().with_width(1.8).with_color(tk.danger));
        // The shown section's sample line, faint and dashed when it is not at the km itself.
        if let Some(r) = self.row {
            let near = (r.station - self.station).abs() < 1e-3;
            let line = Stroke { line_dash: LineDash { segments: if near { &[] } else { &[4.0, 3.0] }, offset: 0 }, ..Stroke::default() };
            frame.stroke(
                &Path::line(at(r.ends[0]), at(r.ends[1])),
                Stroke { width: 1.2, ..line }.with_color(iced::Color { a: if near { 0.9 } else { 0.5 }, ..tk.text }),
            );
        }
        // The km: a dot on the axis and its words.
        if let Some(p) = self.point(self.station) {
            let q = at(p);
            frame.fill(&Path::circle(q, 4.5), tk.accent);
            frame.stroke(&Path::circle(q, 4.5), Stroke::default().with_width(1.5).with_color(tk.field));
            frame.fill_text(Text {
                content: format!("km {}", self.km),
                position: Point::new(q.x + 8.0, q.y - 16.0),
                color: tk.accent,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}
