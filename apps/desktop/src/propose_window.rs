//! Kırmızı kot önerisi (plan/road.md Y25, docs/adr/0926 §3, `profile.propose`): new elevations for a profile's PVIs
//! that follow the terrain profile as closely as the user's rules allow: the ends held or not, a most grade, the area
//! cut equal to the area filled. The rules are the user's; KentOS fills in none. Önizle runs `road.profiles.propose`
//! (nothing written): the ground, the profile as it is and the proposal; beside, the PVIs (was → proposed), the grades
//! and those at the limit, the root mean square difference, the areas cut and filled. Yeni boykesit olarak yaz writes
//! the proposal as a new profile with `road.profiles.create` (one undo step).
//!
//! ```text
//! ┌ Kırmızı kot önerisi ────────────────────────────────────────────────────────────────────────────┐
//! │ Boykesit [Uzun kot ▾]  En büyük eğim (%) [ ]  ☐ Uçları kilitle  ☐ Dengeli                        │
//! │ ┌ ground, profile, proposal ┐  PVI'lar · Eğimler · Fark · Kazı ve dolgu alanı                     │
//! │ └───────────────────────────┘                                                                    │
//! │                                       [Yeni boykesit olarak yaz] [Önizle] [Kapat]                │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{GradeLimit, ProfileProposed, ProfilePvi, ProfilesCreate, ProfilesPropose};
use kentos_native_application::{ExecutionContext, road_profiles, road_profiles_create, road_profiles_propose};
use kentos_road::profile::vertical;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kırmızı kot önerisi";
const PREVIEW: &str = "Önizle";
const CLOSE: &str = "Kapat";
const WRITE: &str = "Yeni boykesit olarak yaz";
pub(crate) const MOST: &str = "En büyük eğim (%)";
pub(crate) const ENDS: &str = "Uçları kilitle";
pub(crate) const BALANCED: &str = "Dengeli";

/// A read: the proposal, the profile as it is and the ground, each as (station, elevation) to draw.
#[derive(Debug, Clone)]
pub struct Read {
    pub(crate) answer: ProfileProposed,
    pub(crate) was: Vec<[f64; 2]>,
    pub(crate) proposed: Vec<[f64; 2]>,
    pub(crate) ground: Vec<(f64, Option<f64>)>,
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) profile: Option<String>,
    pub(crate) most: String,
    pub(crate) ends: bool,
    pub(crate) balanced: bool,
    pub(crate) preview: Option<Result<Read, String>>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Profile(String),
    Most(String),
    Ends(bool),
    Balanced(bool),
    Preview,
    Write,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Propose(event)
}

/// The profile through `pvis` sampled every two metres (and at its pieces' ends); the PVIs' polygon where its curves
/// do not build.
fn sampled(pvis: &[vertical::Pvi]) -> Vec<[f64; 2]> {
    match vertical::build(pvis) {
        Ok(v) => {
            let mut out = Vec::new();
            let steps = ((v.end - v.start) / 2.0).ceil().max(1.0) as usize;
            for i in 0..=steps {
                let s = v.start + (v.end - v.start) * i as f64 / steps as f64;
                if let Some((z, _, _)) = v.at(s) {
                    out.push([s, z]);
                }
            }
            out
        }
        Err(_) => pvis.iter().map(|p| [p.station, p.elevation]).collect(),
    }
}

impl Window {
    pub(crate) fn input(&self, pvis: &[ProfilePvi]) -> Result<ProfilesPropose, String> {
        let Some(profile) = self.profile.clone() else { return Err("Bir boykesit seçin.".into()) };
        let t = self.most.trim();
        let most_grade = if t.is_empty() {
            None
        } else {
            Some(t.replace(',', ".").parse::<f64>().map_err(|_| format!("{MOST} bir sayı olmalı: “{t}”."))? / 100.0)
        };
        let hold = if self.ends {
            [pvis.first(), pvis.last()].into_iter().flatten().map(|p| p.id.clone()).collect()
        } else {
            Vec::new()
        };
        Ok(ProfilesPropose { profile, hold, most_grade, balanced: self.balanced })
    }
}

impl App {
    /// Kırmızı kot önerisi: the window on the road project's first profile with a surface.
    pub(crate) fn open_propose(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Kırmızı kot önerisi yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let list = doc.model.road_profiles();
        let Some(first) = list.iter().find(|p| p.surface.is_some()).or(list.first()).map(|p| p.id.clone()) else {
            self.warn("Kırmızı kot önerisi bir boykesitte kurulur: önce Boykesit'te bir boykesit oluşturun.".to_owned());
            return;
        };
        self.propose_window = Some(Box::new(Window { profile: Some(first), ..Window::default() }));
        self.dialog = Some(Dialog::Propose);
    }

    pub(crate) fn propose_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.propose_window.as_mut() else { return Task::none() };
        match event {
            Event::Profile(p) => w.profile = Some(p),
            Event::Most(t) => w.most = t,
            Event::Ends(on) => w.ends = on,
            Event::Balanced(on) => w.balanced = on,
            Event::Preview => {
                let Some(doc) = &self.document else { return Task::none() };
                let def = w.profile.as_ref().and_then(|id| doc.model.road_profiles().iter().find(|p| &p.id == id)).cloned();
                let read = match def {
                    None => Err("Bir boykesit seçin.".to_owned()),
                    Some(def) => w.input(&def.pvis).and_then(|q| {
                        let (answer, warnings, ground) = road_profiles_propose::preview(&doc.model, &q)?;
                        let was_pvis = road_profiles::pvis_of(&def);
                        let mut new_pvis = was_pvis.clone();
                        for (p, n) in new_pvis.iter_mut().zip(&answer.pvis) {
                            p.elevation = n.elevation;
                        }
                        Ok((Read { was: sampled(&was_pvis), proposed: sampled(&new_pvis), answer, ground }, warnings))
                    }),
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
            Event::Write => {
                let Some(Ok(r)) = &w.preview else { return Task::none() };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let Some(def) = doc.model.road_profiles().iter().find(|p| p.id == r.answer.profile).cloned() else {
                    return Task::none();
                };
                let names: Vec<String> = doc.model.road_profiles().iter().map(|p| p.name.to_lowercase()).collect();
                let mut name = format!("{} önerisi", def.name);
                let mut k = 2;
                while names.contains(&name.to_lowercase()) {
                    name = format!("{} önerisi {k}", def.name);
                    k += 1;
                }
                let pvis = def
                    .pvis
                    .iter()
                    .zip(&r.answer.pvis)
                    .map(|(p, n)| ProfilePvi { elevation: n.elevation, ..p.clone() })
                    .collect();
                let create = ProfilesCreate {
                    name: name.clone(),
                    alignment: def.alignment.clone(),
                    pvis,
                    id: None,
                    surface: def.surface.clone(),
                    expected_revision: None,
                };
                let result = road_profiles_create::execute(&mut ExecutionContext::new(&mut doc.model), create);
                if self.answered(result).is_some() {
                    self.say(kentos_interaction::Level::Success, format!("“{name}” boykesiti yazıldı."));
                    self.propose_window = None;
                    self.dialog = None;
                }
                return Task::none();
            }
            Event::Close => {
                self.propose_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.preview = None;
        w.warnings.clear();
        Task::none()
    }

    pub(crate) fn propose_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.propose_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.profile.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = container(Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Profile(ids[i].clone())))).width(200);
        let most = kentos_ui::widget::text_field("", &w.most).on_input(|t| msg(Event::Most(t))).width(110);
        let top = row![
            words::field("Boykesit", pick, None),
            words::field(MOST, most, None),
            words::check(w.ends, ENDS, Some(msg(Event::Ends(!w.ends)))),
            words::check(w.balanced, BALANCED, Some(msg(Event::Balanced(!w.balanced)))),
        ]
        .spacing(14)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = Column::new().spacing(12).push(top);
        let mut can_write = false;
        match &w.preview {
            None => {
                body = body.push(
                    label::caption(
                        "Öneri PVI'ların km'sini ve parabolik kurplarını korur, kotlarını araziye en yakın yapar. Sınırlar tasarım standardınındır; KentOS değer vermez.",
                    )
                    .style(muted),
                );
            }
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(r)) => {
                can_write = true;
                let picture = container(iced::widget::canvas(Draw { read: r }).width(Fill).height(typography::scaled(360.0))).style(style::container::field_box);
                let line = |k: String, v: String| row![label::caption(k).style(muted).width(130), label::body(v)].spacing(8);
                let a = &r.answer;
                let mut side = Column::new().spacing(5).width(400).padding(iced::Padding { right: 14.0, ..iced::Padding::ZERO });
                side = side.push(line("Karesel ortalama fark".into(), format!("{:.3} m", a.rms)));
                side = side.push(line("Kazı alanı".into(), format!("{:.1} m²", a.cut)));
                side = side.push(line("Dolgu alanı".into(), format!("{:.1} m²", a.fill)));
                side = side.push(line("Yüzeydeki boy".into(), format!("{:.1} m", a.covered)));
                side = side.push(label::caption("PVI'lar: önceki → önerilen kot").style(muted));
                for p in &a.pvis {
                    let lock = if p.held { " · kilitli" } else { "" };
                    side = side.push(line(format!("{} · {:.1} m", p.id, p.station), format!("{:.3} → {:.3}{lock}", p.was, p.elevation)));
                }
                side = side.push(label::caption("Eğimler").style(muted));
                for g in &a.grades {
                    let limit = match g.limit {
                        Some(GradeLimit::Rise) => " · en büyük çıkış",
                        Some(GradeLimit::Fall) => " · en büyük iniş",
                        None => "",
                    };
                    side = side.push(line(format!("{} – {}", g.between[0], g.between[1]), format!("%{:.3}{limit}", 100.0 * g.grade)));
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, scrollable(side).height(typography::scaled(360.0))].spacing(12));
            }
        }
        body = body.push(
            row![
                space::horizontal().width(Fill),
                words::secondary(WRITE, can_write.then(|| msg(Event::Write))),
                words::secondary(PREVIEW, Some(msg(Event::Preview))),
                words::secondary(CLOSE, Some(msg(Event::Close)))
            ]
            .spacing(8),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn propose_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.propose_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(ENDS) => Some(msg(Event::Ends(!w.ends))),
            Control::Press(BALANCED) => Some(msg(Event::Balanced(!w.balanced))),
            Control::Fill(MOST, t) => Some(msg(Event::Most(t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

struct Draw<'a> {
    read: &'a Read,
}

impl canvas::Program<Message> for Draw<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let r = self.read;
        let all = r.was.iter().chain(&r.proposed).copied().chain(r.ground.iter().filter_map(|(s, z)| z.map(|z| [*s, z])));
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for p in all {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        if !x0.is_finite() {
            return vec![frame.into_geometry()];
        }
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let (left, right, top, bottom) = (52.0, 14.0, 26.0, 26.0);
        let x = |s: f64| (left + (s - x0) / (x1 - x0).max(1e-9) * (w - left - right)) as f32;
        let y = |z: f64| (top + (y1 - z) / (y1 - y0).max(1e-9) * (h - top - bottom)) as f32;
        let polyline = |pts: &[[f64; 2]]| {
            Path::new(|p| {
                for (i, q) in pts.iter().enumerate() {
                    let at = Point::new(x(q[0]), y(q[1]));
                    if i == 0 { p.move_to(at) } else { p.line_to(at) }
                }
            })
        };
        // The ground, broken where it is off the surface.
        let mut run: Vec<[f64; 2]> = Vec::new();
        let ground = Stroke::default().with_width(1.4).with_color(tk.text);
        for &(s, z) in &r.ground {
            match z {
                Some(z) if s >= x0 && s <= x1 => run.push([s, z]),
                _ => {
                    if run.len() > 1 {
                        frame.stroke(&polyline(&run), ground);
                    }
                    run.clear();
                }
            }
        }
        if run.len() > 1 {
            frame.stroke(&polyline(&run), ground);
        }
        let dashed = Stroke { line_dash: LineDash { segments: &[6.0, 4.0], offset: 0 }, ..Stroke::default() };
        frame.stroke(&polyline(&r.was), Stroke { width: 1.2, ..dashed.with_color(Color { a: 0.8, ..tk.muted }) });
        frame.stroke(&polyline(&r.proposed), Stroke::default().with_width(2.2).with_color(tk.danger));
        for p in &r.answer.pvis {
            let at = Point::new(x(p.station), y(p.elevation));
            if p.held {
                frame.fill(&Path::rectangle(Point::new(at.x - 4.0, at.y - 4.0), iced::Size::new(8.0, 8.0)), tk.danger);
            } else {
                frame.stroke(&Path::circle(at, 4.0), Stroke::default().with_width(1.6).with_color(tk.danger));
            }
        }
        let caption = |frame: &mut canvas::Frame, content: String, at: Point| {
            frame.fill_text(Text { content, position: at, color: tk.muted, size: Pixels(11.0), ..Text::default() });
        };
        caption(&mut frame, format!("{y1:.1} m"), Point::new(4.0, y(y1) - 6.0));
        caption(&mut frame, format!("{y0:.1} m"), Point::new(4.0, y(y0) - 6.0));
        caption(&mut frame, format!("{x0:.0} m"), Point::new(x(x0) - 6.0, (h - bottom + 8.0) as f32));
        caption(&mut frame, format!("{x1:.0} m"), Point::new(x(x1) - 30.0, (h - bottom + 8.0) as f32));
        caption(
            &mut frame,
            "Siyah arazi, kesikli şimdiki kırmızı kot, kırmızı öneri; dolu kare kilitli PVI".into(),
            Point::new(10.0, 6.0),
        );
        vec![frame.into_geometry()]
    }
}
