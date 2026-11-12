//! Çizimden boykesit (plan/road.md Y10.6b, docs/adr/0908 §13, `road.profiles.fit`): a profile read off the selected
//! line or open polyline drawn in a profile view (a project's red line drawn in DWG or DGN). The view is typed (where
//! its datum and first station stand in the drawing, its exaggeration); the profile is read as it is typed
//! (`road.profiles.fit`'s plan): drawn over the line with its PVIs, its grades, curves and how far it lies from the line;
//! Boykesiti oluştur writes it as one undo step.
//!
//! ```text
//! ┌ Çizimden boykesit ──────────────────────────────────────────────────────────────────────────────┐
//! │ Aks [▾] Yer (Y; X) [ ] Km [ ] Kıyas kotu [ ] Abartma [ ] Kurp boyu [ ] Ad [ ] Tolerans [ ]       │
//! │ n PVI · g eğim · c kurp · km a–b · çizgiden en çok d m                                          │
//! │ ┌ the line and the profile ┐  PVI  Km  Kot  Kurp                                                 │
//! │ └──────────────────────────┘  …                                                                  │
//! │                                                               [Kapat] [Boykesiti oluştur]        │
//! └────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::road;
use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{container, row, space};
use iced::{Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CommandWarning, ProfileFitted, ProfilesFit};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_profiles_fit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Çizimden boykesit";
const CLOSE: &str = "Kapat";
const MAKE: &str = "Boykesiti oluştur";
/// The typed fields, in order.
const FIELDS: [&str; 8] =
    ["Yer (Y; X)", "Km (m)", "Kıyas kotu (m)", "Abartma", "Kurp boyu (m)", "Ad", "Tolerans (m)", ""];

/// The profile as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) fitted: Option<ProfileFitted>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The profile's points (station, elevation).
    pub(crate) line: Vec<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The line's uid and its points in the drawing (arcs in pieces).
    pub(crate) uid: String,
    pub(crate) source: Vec<[f64; 2]>,
    pub(crate) alignment: Option<String>,
    /// The place, the station, the datum, the exaggeration, the curve length, the name and the tolerance, as typed.
    pub(crate) fields: [String; 7],
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Alignment(String),
    /// An index into [`FIELDS`].
    Field(usize, String),
    Make,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Road(road::Event::RoadProfileFit(event))
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

impl Window {
    /// The command's input as typed, or why it is not one.
    pub(crate) fn input(&self) -> Result<ProfilesFit, String> {
        let alignment = self.alignment.clone().ok_or("Bir aks seçin.")?;
        let parts: Vec<Option<f64>> =
            self.fields[0].split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s, "Yer").ok().flatten()).collect();
        let at = match parts[..] {
            [Some(x), Some(y)] => [x, y],
            _ => return Err("Görünümün kıyas kotunun ve ilk km'sinin çizimdeki yerini “Y; X” olarak yazın.".into()),
        };
        let datum = number(&self.fields[2], "Kıyas kotu")?.ok_or("Görünümün kıyas kotunu yazın.")?;
        let name = self.fields[5].trim();
        Ok(ProfilesFit {
            uid: self.uid.clone(),
            alignment,
            at,
            station: number(&self.fields[1], "Km")?,
            datum,
            exaggeration: number(&self.fields[3], "Abartma")?,
            length: number(&self.fields[4], "Kurp boyu")?,
            name: (!name.is_empty()).then(|| name.to_owned()),
            surface: None,
            tolerance: number(&self.fields[6], "Tolerans")?,
            expected_revision: None,
        })
    }

    /// A profile point (station, elevation) in the drawing, by the view as typed; none while it is not one.
    fn drawn(&self) -> Option<impl Fn([f64; 2]) -> [f64; 2] + '_> {
        let q = self.input().ok()?;
        let (s0, ex) = (q.station.unwrap_or(0.0), q.exaggeration.unwrap_or(1.0));
        Some(move |p: [f64; 2]| [q.at[0] + (p[0] - s0), q.at[1] + (p[1] - q.datum) * ex])
    }
}

impl App {
    /// Çizimden boykesit: the window on the one selected line or open polyline, the view's place at its lower left.
    pub(crate) fn open_road_profile_fit(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Çizimden boykesit yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_alignments().iter().find(|a| a.offset_of.is_none()) else {
            self.warn("Boykesit bir aksın üstündedir: önce Güzergâh'ta bir aks oluşturun.".to_owned());
            return;
        };
        let alignment = first.id.clone();
        let picked = match self.selection.ids() {
            [slot] => doc.model.get(*slot).and_then(|e| {
                let pts = crate::road_alignment_fit_window::drawn_points(e)?;
                doc.model.uid(*slot).map(|u| (u.to_string(), pts))
            }),
            _ => None,
        };
        let Some((uid, source)) = picked else {
            self.warn("Çizimden boykesit için önce boykesit görünümünde çizilmiş bir çizgi ya da açık çoklu çizgi seçin.".to_owned());
            return;
        };
        let x = source.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let y = source.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        self.road.road_profile_fit_window = Some(Box::new(Window {
            uid,
            source,
            alignment: Some(alignment),
            fields: [format!("{}; {}", round(x), round(y)), "0".into(), String::new(), "1".into(), String::new(), String::new(), String::new()],
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::RoadProfileFit));
        self.refresh_road_profile_fit();
    }

    fn refresh_road_profile_fit(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road.road_profile_fit_window.as_mut() else {
            return;
        };
        w.preview = match w
            .input()
            .and_then(|q| road_profiles_fit::preview(&doc.model, &q))
        {
            Err(why) => Preview {
                why: Some(why),
                ..Preview::default()
            },
            Ok((fitted, warnings, line)) => Preview {
                fitted: Some(fitted),
                warnings,
                why: None,
                line,
            },
        };
    }

    pub(crate) fn road_profile_fit_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.road.road_profile_fit_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Alignment(a) => w.alignment = Some(a),
            Event::Field(k, t) => w.fields[k.min(6)] = t,
            Event::Make => return self.make_road_profile_fit(),
            Event::Close => {
                self.road.road_profile_fit_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_road_profile_fit();
        Task::none()
    }

    /// Boykesiti oluştur: `road.profiles.fit`, one undo step.
    fn make_road_profile_fit(&mut self) -> Task<Message> {
        let Some(w) = self.road.road_profile_fit_window.as_ref() else {
            return Task::none();
        };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Boykesit oluşturulamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_profiles_fit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        self.say(
            Level::Success,
            format!(
                "“{}”: {} PVI, {} düşey kurp; çizgiden en çok {:.3} m ayrılıyor.",
                out.name,
                out.pvis.len(),
                out.curves,
                out.deviation
            ),
        );
        self.road.road_profile_fit_window = None;
        self.dialog = None;
        Task::none()
    }

    pub(crate) fn road_profile_fit_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road.road_profile_fit_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let alignments: Vec<(String, String)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.name.clone())).collect()
        });
        let ids: Vec<String> = alignments.iter().map(|a| a.0.clone()).collect();
        let on_alignment = Select::new(
            alignments.iter().map(|a| Choice::new(a.1.clone())),
            w.alignment.as_ref().and_then(|c| ids.iter().position(|i| i == c)),
            move |i| msg(Event::Alignment(ids[i].clone())),
        )
        .placeholder("Aks");
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let top = row![
            words::field("Aks", container(on_alignment).width(150), None),
            field("Y; X", 0, 150.0),
            field("0", 1, 70.0),
            field("örn. 90", 2, 80.0),
            field("1", 3, 60.0),
            field("kurpsuz", 4, 80.0),
            field("1. boykesit", 5, 120.0),
            field("yok", 6, 70.0),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top);
        match (&w.preview.why, &w.preview.fitted) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(f)) => {
                body = body.push(label::caption(format!(
                    "{} PVI · {} eğim · {} düşey kurp · {:.3}–{:.3} m · çizgiden en çok {:.3} m",
                    f.pvis.len(),
                    f.grades,
                    f.curves,
                    f.stations[0],
                    f.stations[1],
                    f.deviation
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let (line, pvis): (Vec<[f64; 2]>, Vec<[f64; 2]>) = match w.drawn() {
                    Some(to) => (
                        w.preview.line.iter().map(|p| to(*p)).collect(),
                        f.pvis.iter().map(|p| to([p.station, p.elevation])).collect(),
                    ),
                    None => (Vec::new(), Vec::new()),
                };
                let picture = crate::picture::picture(iced::widget::canvas(Plan { source: &w.source, line, pvis }).width(Fill).height(Fill), TITLE);
                use crate::road_dialog::{col, grouped, num};
                let list = crate::road_dialog::sheet_fill(
                    vec![col("PVI", 64.0), num("Km (m)", 100.0), num("Kot (m)", 100.0), num("Kurp (m)", 150.0)],
                    f.pvis
                        .iter()
                        .map(|p| {
                            let curve = if p.length > 0.0 {
                                grouped(p.length, 3)
                            } else if p.length_in > 0.0 {
                                format!("{} + {}", grouped(p.length_in, 3), grouped(p.length_out, 3))
                            } else {
                                "—".into()
                            };
                            vec![p.id.clone(), grouped(p.station, 3), grouped(p.elevation, 3), curve]
                        })
                        .collect(),
                    "PVI yok",
                );
                body = body.push_grow(
                    row![picture, list]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(280.0),
                );
            }
            _ => {}
        }
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                label::caption("Çizgi kalır; boykesit tek geri alma adımı.").style(muted),
                space::horizontal().width(Fill),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(MAKE, w.preview.fitted.as_ref().map(|_| msg(Event::Make))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1060.0).modal(msg(Event::Close))
    }

    pub(crate) fn road_profile_fit_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road.road_profile_fit_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(MAKE) => Some(msg(Event::Make)),
            Control::Fill(name, t) if FIELDS[..7].contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The line (muted), the profile over it and its PVIs, in the drawing.
struct Plan<'a> {
    source: &'a [[f64; 2]],
    line: Vec<[f64; 2]>,
    pvis: Vec<[f64; 2]>,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let pts: Vec<&[f64; 2]> = self.source.iter().chain(&self.line).chain(&self.pvis).collect();
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
        let pad = 18.0;
        let scale = ((bounds.width - 2.0 * pad) as f64 / (x1 - x0).max(1e-6)).min((bounds.height - 2.0 * pad) as f64 / (y1 - y0).max(1e-6));
        let (cx, cy) = (0.5 * (x0 + x1), 0.5 * (y0 + y1));
        let at = |p: [f64; 2]| {
            Point::new(
                (bounds.width as f64 * 0.5 + (p[0] - cx) * scale) as f32,
                (bounds.height as f64 * 0.5 - (p[1] - cy) * scale) as f32,
            )
        };
        let polyline = |line: &[[f64; 2]]| {
            Path::new(|b| {
                if let Some(first) = line.first() {
                    b.move_to(at(*first));
                    for p in &line[1..] {
                        b.line_to(at(*p));
                    }
                }
            })
        };
        frame.stroke(
            &polyline(&self.pvis),
            Stroke { line_dash: canvas::LineDash { segments: &[4.0, 3.0], offset: 0 }, ..Stroke::default() }
                .with_width(0.8)
                .with_color(tk.muted),
        );
        frame.stroke(&polyline(self.source), Stroke::default().with_width(4.0).with_color(iced::Color { a: 0.35, ..tk.text }));
        frame.stroke(&polyline(&self.line), Stroke::default().with_width(1.6).with_color(tk.danger));
        for p in &self.pvis {
            frame.stroke(&Path::circle(at(*p), 3.5), Stroke::default().with_width(1.2).with_color(tk.accent));
        }
        vec![frame.into_geometry()]
    }
}
