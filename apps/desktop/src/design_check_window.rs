//! Tasarım denetimi (plan/road.md Y21, docs/adr/0922 §3, `alignment.check`): an alignment or a profile checked against
//! the user's design rules, as Civil 3D's design check sets and OpenRoads' design standards do. The limits are the
//! user's (the design standard's); KentOS fills in none, a rule left empty is not checked. Önizle runs
//! `road.alignments.check` or `road.profiles.check` (nothing written): along the stations, the alignment's elements
//! (tangent, clothoid, curve) or the profile, the stretches breaking a rule red; beside, the findings in words and,
//! for an alignment, its curve pairs.
//!
//! ```text
//! ┌ Tasarım denetimi ───────────────────────────────────────────────────────────────────────────────┐
//! │ Denetlenen [Aks ▾] Aks [Ters ▾]  En küçük yarıçap [ ] … Kırıklar [Var ▾]                           │
//! │ ┌ elements along the stations, red stretches ┐  Bulgular: kural · PI · km · değer / sınır          │
//! │ └────────────────────────────────────────────┘  Kurp çiftleri: ters · aynı yön · bitişik          │
//! │                                                                         [Önizle] [Kapat]         │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Size, Task, Theme, mouse};
use kentos_commands::{AlignmentsCheck, AlignmentsChecked, DesignFinding, RoadProfilesCheck, RoadProfilesChecked};
use kentos_native_application::{road_alignments, road_alignments_check, road_profiles, road_profiles_check};
use kentos_road::alignment::Kind;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Tasarım denetimi";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
/// The alignment's rules, in order.
pub(crate) const ALIGNMENT_FIELDS: [&str; 7] = [
    "En küçük yarıçap (m)",
    "En kısa kurp (m)",
    "En kısa klotoid (m)",
    "En kısa aliyman (m)",
    "Ters kurp arası (m)",
    "Aynı yön arası (m)",
    "En çok yarıçap oranı",
];
/// The profile's rules, in order; grades in percent.
pub(crate) const PROFILE_FIELDS: [&str; 6] = [
    "En dik eğim (%)",
    "En yatık eğim (%)",
    "Tepe K en az",
    "Çukur K en az",
    "En kısa düşey kurp (m)",
    "Kurplar arası eğim (m)",
];

/// What was read last.
#[derive(Debug, Clone)]
pub(crate) enum Read {
    Alignment(AlignmentsChecked),
    Profile(RoadProfilesChecked),
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    /// Checking a profile, else an alignment.
    pub(crate) profile_mode: bool,
    pub(crate) alignment: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) kinks: bool,
    pub(crate) alignment_fields: [String; 7],
    pub(crate) profile_fields: [String; 6],
    pub(crate) preview: Option<Result<Read, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Mode(bool),
    Alignment(String),
    Profile(String),
    Kinks(bool),
    AlignmentField(usize, String),
    ProfileField(usize, String),
    Preview,
    Close,
}

fn msg(event: Event) -> Message {
    Message::DesignCheck(event)
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what} bir sayı olmalı: “{t}”."))
}

impl Window {
    pub(crate) fn alignment_input(&self) -> Result<AlignmentsCheck, String> {
        let Some(alignment) = self.alignment.clone() else { return Err("Bir aks seçin.".into()) };
        let f = |k: usize| number(&self.alignment_fields[k], ALIGNMENT_FIELDS[k]);
        Ok(AlignmentsCheck {
            alignment,
            least_radius: f(0)?,
            least_arc: f(1)?,
            least_spiral: f(2)?,
            least_tangent: f(3)?,
            least_reverse: f(4)?,
            least_broken_back: f(5)?,
            most_radius_ratio: f(6)?,
            kinks: self.kinks,
        })
    }

    pub(crate) fn profile_input(&self) -> Result<RoadProfilesCheck, String> {
        let Some(profile) = self.profile.clone() else { return Err("Bir boykesit seçin.".into()) };
        let f = |k: usize| number(&self.profile_fields[k], PROFILE_FIELDS[k]);
        let percent = |k: usize| f(k).map(|v| v.map(|p| p / 100.0));
        Ok(RoadProfilesCheck {
            profile,
            most_grade: percent(0)?,
            least_grade: percent(1)?,
            least_crest_k: f(2)?,
            least_sag_k: f(3)?,
            least_curve: f(4)?,
            least_grade_length: f(5)?,
        })
    }
}

impl App {
    /// Tasarım denetimi: the window on the road project's first alignment and profile.
    pub(crate) fn open_design_check(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Tasarım denetimi yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let alignment = doc.model.road_alignments().iter().find(|a| a.offset_of.is_none()).map(|a| a.id.clone());
        let Some(alignment) = alignment else {
            self.warn("Tasarım denetimi bir aksta okunur: önce Güzergâh'ta bir aks oluşturun.".to_owned());
            return;
        };
        let profile = doc.model.road_profiles().first().map(|p| p.id.clone());
        self.design_check_window = Some(Box::new(Window { alignment: Some(alignment), profile, ..Window::default() }));
        self.dialog = Some(Dialog::DesignCheck);
    }

    pub(crate) fn design_check_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.design_check_window.as_mut() else { return Task::none() };
        match event {
            Event::Mode(p) => w.profile_mode = p,
            Event::Alignment(a) => w.alignment = Some(a),
            Event::Profile(p) => w.profile = Some(p),
            Event::Kinks(k) => w.kinks = k,
            Event::AlignmentField(k, t) => w.alignment_fields[k.min(ALIGNMENT_FIELDS.len() - 1)] = t,
            Event::ProfileField(k, t) => w.profile_fields[k.min(PROFILE_FIELDS.len() - 1)] = t,
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let read = if w.profile_mode {
                    w.profile_input()
                        .and_then(|q| road_profiles_check::preview(&doc.model, &q))
                        .map(|(r, x)| (Read::Profile(r), x))
                } else {
                    w.alignment_input()
                        .and_then(|q| road_alignments_check::preview(&doc.model, &q))
                        .map(|(r, x)| (Read::Alignment(r), x))
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
                self.design_check_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.preview = None;
        w.warnings.clear();
        Task::none()
    }

    pub(crate) fn design_check_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.design_check_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let mode = container(Select::new(["Aks", "Boykesit"].into_iter().map(Choice::new), Some(usize::from(w.profile_mode)), |i| {
            msg(Event::Mode(i == 1))
        }))
        .width(110);
        let mut top = row![words::field("Denetlenen", mode, None)].spacing(10).align_y(iced::alignment::Vertical::Bottom);
        let mut fields = row![].spacing(8).align_y(iced::alignment::Vertical::Bottom);
        if w.profile_mode {
            let list: Vec<(String, String)> =
                model.map_or_else(Vec::new, |m| m.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone())).collect());
            let ids: Vec<String> = list.iter().map(|p| p.0.clone()).collect();
            let at = w.profile.as_ref().and_then(|p| ids.iter().position(|i| i == p));
            let pick = container(Select::new(list.iter().map(|p| Choice::new(p.1.clone())), at, move |i| msg(Event::Profile(ids[i].clone())))).width(170);
            top = top.push(words::field("Boykesit", pick, None));
            for (k, name) in PROFILE_FIELDS.iter().enumerate() {
                let input = kentos_ui::widget::text_field("yok", &w.profile_fields[k]).on_input(move |t| msg(Event::ProfileField(k, t))).width(120);
                fields = fields.push(words::field(name, input, None));
            }
        } else {
            let list: Vec<(String, String)> = model.map_or_else(Vec::new, |m| {
                m.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| (a.id.clone(), a.name.clone())).collect()
            });
            let ids: Vec<String> = list.iter().map(|p| p.0.clone()).collect();
            let at = w.alignment.as_ref().and_then(|p| ids.iter().position(|i| i == p));
            let pick = container(Select::new(list.iter().map(|p| Choice::new(p.1.clone())), at, move |i| msg(Event::Alignment(ids[i].clone())))).width(170);
            let kinks = container(Select::new(["Yok", "Var"].into_iter().map(Choice::new), Some(usize::from(w.kinks)), |i| msg(Event::Kinks(i == 1)))).width(80);
            top = top.push(words::field("Aks", pick, None)).push(words::field("Kurpsuz kırık", kinks, None));
            for (k, name) in ALIGNMENT_FIELDS.iter().enumerate() {
                let input =
                    kentos_ui::widget::text_field("yok", &w.alignment_fields[k]).on_input(move |t| msg(Event::AlignmentField(k, t))).width(118);
                fields = fields.push(words::field(name, input, None));
            }
        }
        let mut body = Column::new().spacing(12).push(top).push(scrollable(fields).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())));
        match &w.preview {
            None => {
                body = body.push(
                    label::caption("Sınırlar tasarım standardınındır; KentOS değer vermez. Boş bırakılan kural denetlenmez.").style(muted),
                )
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(read)) => {
                let findings = match read {
                    Read::Alignment(a) => &a.findings,
                    Read::Profile(p) => &p.findings,
                };
                let shape = match (read, model) {
                    (Read::Alignment(a), Some(m)) => road_alignments::made_alignments(m)
                        .into_iter()
                        .find(|(d, _)| d.id == a.alignment)
                        .and_then(|(_, b)| b.ok())
                        .map(|b| Shape::Elements(b.core.elements.iter().map(|e| (e.kind, e.station, e.station + e.length)).collect())),
                    (Read::Profile(p), Some(m)) => road_profiles::made_profiles(m)
                        .into_iter()
                        .find(|(d, _)| d.id == p.profile)
                        .and_then(|(_, b)| b.ok())
                        .map(|b| {
                            let (s0, s1) = (b.vertical.start, b.vertical.end);
                            let n = 400;
                            Shape::Profile(
                                (0..=n)
                                    .filter_map(|i| {
                                        let s = s0 + (s1 - s0) * f64::from(i) / f64::from(n);
                                        b.vertical.at(s).map(|z| [s, z.0])
                                    })
                                    .collect(),
                            )
                        }),
                    _ => None,
                };
                let picture = container(
                    iced::widget::canvas(Strip { shape: shape.unwrap_or(Shape::Elements(Vec::new())), findings })
                        .width(Fill)
                        .height(typography::scaled(300.0)),
                )
                .style(style::container::field_box);
                let mut side = Column::new().spacing(6).width(470);
                if findings.is_empty() {
                    side = side.push(label::body("Verilen kuralların hepsi tutuyor."));
                } else {
                    side = side.push(label::caption(format!("{} bulgu", findings.len())).style(muted));
                    for f in findings {
                        side = side.push(finding_row(f));
                    }
                }
                if let Read::Alignment(a) = read
                    && !a.pairs.is_empty()
                {
                    side = side.push(label::caption("Kurp çiftleri").style(muted));
                    for p in &a.pairs {
                        let kind = match (p.reverse, p.joined) {
                            (true, true) => "ters, bitişik",
                            (true, false) => "ters",
                            (false, true) => "aynı yön, bitişik (birleşik kurp)",
                            (false, false) => "aynı yön",
                        };
                        side = side.push(label::caption(format!("{} – {}: {kind} · aradaki aliyman {:.2} m", p.between[0], p.between[1], p.tangent)));
                    }
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, scrollable(side).height(typography::scaled(300.0))].spacing(12));
            }
        }
        body = body.push(
            row![space::horizontal().width(Fill), words::secondary(PREVIEW, Some(msg(Event::Preview))), words::secondary(CLOSE, Some(msg(Event::Close)))]
                .spacing(8),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn design_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.design_check_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Fill(name, t) if !w.profile_mode && ALIGNMENT_FIELDS.contains(&name) => {
                let k = ALIGNMENT_FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::AlignmentField(k, t.to_owned())))
            }
            Control::Fill(name, t) if w.profile_mode && PROFILE_FIELDS.contains(&name) => {
                let k = PROFILE_FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::ProfileField(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn finding_row(f: &DesignFinding) -> Element<'_, Message> {
    let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let at = if (f.to - f.from).abs() < 1e-9 { format!("{:.2} m", f.from) } else { format!("{:.2} – {:.2} m", f.from, f.to) };
    Column::new().spacing(1).push(label::body(f.message.clone()).style(danger)).push(label::caption(at).style(muted)).into()
}

/// What the strip draws along the stations.
enum Shape {
    /// The alignment's elements: kind, start and end station.
    Elements(Vec<(Kind, f64, f64)>),
    /// The profile's (station, elevation).
    Profile(Vec<[f64; 2]>),
}

struct Strip<'a> {
    shape: Shape,
    findings: &'a [DesignFinding],
}

impl canvas::Program<Message> for Strip<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let text = |frame: &mut canvas::Frame, s: String, p: Point, color: Color| {
            frame.fill_text(Text { content: s, position: p, color, size: Pixels(11.0), ..Text::default() })
        };
        let (s0, s1) = match &self.shape {
            Shape::Elements(e) => (e.first().map_or(0.0, |x| x.1), e.last().map_or(1.0, |x| x.2)),
            Shape::Profile(p) => (p.first().map_or(0.0, |x| x[0]), p.last().map_or(1.0, |x| x[0])),
        };
        let (left, right) = (16.0_f32, bounds.width - 16.0);
        let x = |s: f64| left + ((s - s0) / (s1 - s0).max(1e-9)) as f32 * (right - left);
        let (top, bottom) = (28.0_f32, bounds.height - 34.0);
        for f in self.findings {
            let (a, b) = (x(f.from), x(f.to));
            frame.fill_rectangle(Point::new(a.min(b) - 1.0, top), Size::new((b - a).abs().max(3.0), bottom - top), Color { a: 0.18, ..tk.danger });
            frame.stroke(&Path::line(Point::new(a, top), Point::new(a, bottom)), Stroke::default().with_width(1.0).with_color(tk.danger));
        }
        match &self.shape {
            Shape::Elements(els) => {
                let mid = (top + bottom) / 2.0;
                for (kind, a, b) in els {
                    let (color, h) = match kind {
                        Kind::Tangent => (tk.muted, 6.0),
                        Kind::Spiral => (tk.success, 14.0),
                        Kind::Arc => (tk.accent, 22.0),
                    };
                    frame.fill_rectangle(Point::new(x(*a), mid - h / 2.0), Size::new((x(*b) - x(*a)).max(1.0), h), color);
                }
                text(&mut frame, "Gri aliyman, yeşil klotoid, mavi kurp; kırmızı kuralı bozan yer".into(), Point::new(10.0, 6.0), tk.muted);
            }
            Shape::Profile(pts) if pts.len() > 1 => {
                let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p[1]), b.max(p[1])));
                let y = |z: f64| bottom - ((z - lo) / (hi - lo).max(1e-6)) as f32 * (bottom - top);
                let path = Path::new(|p| {
                    p.move_to(Point::new(x(pts[0][0]), y(pts[0][1])));
                    for q in &pts[1..] {
                        p.line_to(Point::new(x(q[0]), y(q[1])));
                    }
                });
                frame.stroke(&path, Stroke::default().with_width(2.0).with_color(tk.danger));
                text(&mut frame, format!("{hi:.2}"), Point::new(left, top - 14.0), tk.muted);
                text(&mut frame, format!("{lo:.2}"), Point::new(left, bottom + 2.0), tk.muted);
                text(&mut frame, "Boykesit; kırmızı alan kuralı bozan yer".into(), Point::new(10.0, 6.0), tk.muted);
            }
            Shape::Profile(_) => {}
        }
        text(&mut frame, format!("{s0:.0}"), Point::new(left, bounds.height - 18.0), tk.muted);
        text(&mut frame, format!("{s1:.0} m"), Point::new(right - 50.0, bounds.height - 18.0), tk.muted);
        vec![frame.into_geometry()]
    }
}
