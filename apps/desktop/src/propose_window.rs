//! Kırmızı kot önerisi (plan/road.md Y25, docs/adr/0926 §3, `profile.propose`): new elevations for a profile's PVIs
//! that follow the terrain profile as closely as the user's rules allow: the ends held or not, a most grade, the area
//! cut equal to the area filled. The rules are the user's; KentOS fills in none. The window reads
//! `road.profiles.propose` (nothing written) whenever a value changes (docs/adr/0930 §4: no Önizle): the ground, the
//! profile as it is and the proposal; under it the root mean square difference, the areas cut and filled and the
//! length on the surface; the PVIs (was → proposed) or the grades (those at the limit) one table at a time. Yeni
//! boykesit olarak yaz, the window's one primary action, writes the proposal as a new profile with
//! `road.profiles.create` (one undo step). The layout is the road windows' one (road_dialog.rs).
//!
//! ```text
//! ┌ Kırmızı kot önerisi ───────────────────────────────────────────────────────────── × ┐
//! │ Girdi ───────────────   ┌ ground, profile, proposal ───────────────────────────────┐ │
//! │  Boykesit [Uzun kot ▾]  └──────────────────────────────────────────────────────────┘ │
//! │ Kurallar ────────────   [fark] [kazı alanı] [dolgu alanı] [yüzeydeki boy]           │
//! │  En büyük eğim [ ] %    (PVI'lar | Eğimler)                                         │
//! │  ☐ Uçları kilitle       table                                                       │
//! │  ☐ Dengeli                                                                          │
//! │                                               [Kapat] [■ Yeni boykesit olarak yaz] │
//! └─────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::{Color, Element, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{GradeLimit, ProfileProposed, ProfilePvi, ProfilesCreate, ProfilesPropose};
use kentos_native_application::{ExecutionContext, road_profiles, road_profiles_create, road_profiles_propose};
use kentos_road::profile::vertical;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog::{self, RoadDialog, num};
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
    pub(crate) detail: Detail,
}

/// Which list the window shows under its figures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Detail {
    #[default]
    Pvis,
    Grades,
}

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Detail::Pvis => "PVI'lar",
            Detail::Grades => "Eğimler",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Profile(String),
    Most(String),
    Ends(bool),
    Balanced(bool),
    Detail(Detail),
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
        let _ = self.propose_event(Event::Preview);
    }

    pub(crate) fn propose_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.propose_window.as_mut() else { return Task::none() };
        match event {
            Event::Profile(p) => w.profile = Some(p),
            Event::Most(t) => w.most = t,
            Event::Ends(on) => w.ends = on,
            Event::Balanced(on) => w.balanced = on,
            Event::Detail(d) => {
                w.detail = d;
                return Task::none();
            }
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
        // The window only reads: it reads again with the new value (docs/adr/0930 §4).
        self.propose_event(Event::Preview)
    }

    pub(crate) fn propose_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.propose_window else {
            return iced::widget::text("").into();
        };
        let list: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone())).collect());
        let ids: Vec<String> = list.iter().map(|c| c.0.clone()).collect();
        let at = w.profile.as_ref().and_then(|c| ids.iter().position(|i| i == c));
        let pick = Select::new(list.iter().map(|c| Choice::new(c.1.clone())), at, move |i| msg(Event::Profile(ids[i].clone())));
        let most = kentos_ui::widget::text_field("sınırsız", &w.most).on_input(|t| msg(Event::Most(t)));
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let form = Form::new()
            .label_width(130.0)
            .section("Girdi")
            .field("Boykesit", pick)
            .section("Kurallar")
            .field(MOST, most)
            .help("Boş: eğim sınırı yok.")
            .row(words::check(w.ends, ENDS, Some(msg(Event::Ends(!w.ends)))))
            .row(words::check(w.balanced, BALANCED, Some(msg(Event::Balanced(!w.balanced)))))
            .row(kentos_ui::label::caption("Dengeli: kazı alanı dolgu alanına eşit olur.").style(muted));
        let close = msg(Event::Close);
        let mut d = RoadDialog::new(TITLE, form, close).legend(
            "Siyah arazi · kesikli şimdiki kırmızı kot · kırmızı öneri · dolu kare kilitli PVI",
        );
        match &w.preview {
            None => {
                d = d.empty("Öneri PVI'ların km'sini ve kurplarını korur, kotlarını araziye en yakın yapar.");
            }
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(r)) => {
                let a = &r.answer;
                d = d
                    .picture(iced::widget::canvas(Draw { read: r }).width(iced::Fill).height(iced::Fill), 300.0)
                    .figure("Karesel ortalama fark", format!("{:.3} m", a.rms))
                    .figure("Kazı alanı", format!("{:.1} m²", a.cut))
                    .figure("Dolgu alanı", format!("{:.1} m²", a.fill))
                    .figure("Yüzeydeki boy", format!("{:.1} m", a.covered));
                for wn in &w.warnings {
                    d = d.note(Banner::warning(wn.clone()));
                }
                let table = match w.detail {
                    Detail::Pvis => road_dialog::table(
                        vec![road_dialog::col("PVI", 90.0), num("Km", 90.0), num("Önceki kot", 110.0), num("Önerilen kot", 110.0), road_dialog::col("", 80.0)],
                        a.pvis
                            .iter()
                            .map(|p| {
                                vec![
                                    p.id.clone(),
                                    format!("{:.1}", p.station),
                                    format!("{:.3}", p.was),
                                    format!("{:.3}", p.elevation),
                                    if p.held { "kilitli".into() } else { String::new() },
                                ]
                            })
                            .collect(),
                        "PVI yok",
                    ),
                    Detail::Grades => road_dialog::table(
                        vec![road_dialog::col("Aralık", 120.0), num("Eğim (%)", 100.0), road_dialog::col("Sınır", 140.0)],
                        a.grades
                            .iter()
                            .map(|g| {
                                vec![
                                    format!("{} – {}", g.between[0], g.between[1]),
                                    format!("{:.3}", 100.0 * g.grade),
                                    match g.limit {
                                        Some(GradeLimit::Rise) => "en büyük çıkış".into(),
                                        Some(GradeLimit::Fall) => "en büyük iniş".into(),
                                        None => String::new(),
                                    },
                                ]
                            })
                            .collect(),
                        "Eğim yok",
                    ),
                };
                d = d.detail(road_dialog::detail_choice([Detail::Pvis, Detail::Grades], w.detail, |x| msg(Event::Detail(x)), table));
            }
        }
        let can_write = matches!(w.preview, Some(Ok(_)));
        d.primary(WRITE, can_write.then(|| msg(Event::Write))).view()
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
        vec![frame.into_geometry()]
    }
}
