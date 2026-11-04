//! Çizgiden aks (plan/road.md Y10.6, docs/adr/0908 §12, `road.alignments.fit`): an alignment made of the selected line
//! or open polyline as Civil 3D's Create Alignment from Objects and Create Best Fit Alignment do. The alignment is read
//! as it is typed (`road.alignments.fit`'s plan): drawn over the line with its PIs, its tangents, curves, length and how
//! far it lies from the line; Aksı oluştur writes it as one undo step and opens Güzergâh on it.
//!
//! ```text
//! ┌ Çizgiden aks ──────────────────────────────────────────────────────────────────────────┐
//! │ Yöntem [Eleman eleman | En iyi uyum] Ad [ ] Kurp yarıçapı [ ] En büyük yarıçap [ ] …     │
//! │ n PI · t aliyman · c kurp · boy L m · en çok sapma d m                                  │
//! │ ┌ the line and the alignment ┐  PI  Y  X  R                                              │
//! │ └────────────────────────────┘  …                                                        │
//! │                                                         [Kapat] [Aksı oluştur]           │
//! └────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{AlignmentFitMethod, AlignmentFitted, AlignmentsFit, CommandWarning, Entity};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_alignments_fit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Çizgiden aks";
const CLOSE: &str = "Kapat";
const MAKE: &str = "Aksı oluştur";
const METHODS: [(&str, AlignmentFitMethod); 2] =
    [("Eleman eleman", AlignmentFitMethod::Elements), ("En iyi uyum", AlignmentFitMethod::BestFit)];

/// The alignment as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) fitted: Option<AlignmentFitted>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
    /// The alignment's points in plan, for the picture.
    pub(crate) line: Vec<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The line's uid and its points (with the arcs' bulges, drawn as chords).
    pub(crate) uid: String,
    pub(crate) source: Vec<[f64; 2]>,
    pub(crate) method: AlignmentFitMethod,
    pub(crate) reverse: bool,
    /// The name, the curve radius, the maximum radius, the tolerance and the start km, as typed.
    pub(crate) fields: [String; 5],
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Method(AlignmentFitMethod),
    Reverse(bool),
    /// 0 the name, 1 the radius, 2 the maximum radius, 3 the tolerance, 4 the start km.
    Field(usize, String),
    Make,
    Close,
}

fn msg(event: Event) -> Message {
    Message::RoadAlignmentFit(event)
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
    pub(crate) fn input(&self) -> Result<AlignmentsFit, String> {
        let name = self.fields[0].trim();
        Ok(AlignmentsFit {
            uid: self.uid.clone(),
            method: self.method,
            name: (!name.is_empty()).then(|| name.to_owned()),
            radius: match self.method {
                AlignmentFitMethod::Elements => number(&self.fields[1], "Kurp yarıçapı")?,
                AlignmentFitMethod::BestFit => None,
            },
            max_radius: match self.method {
                AlignmentFitMethod::BestFit => number(&self.fields[2], "En büyük yarıçap")?,
                AlignmentFitMethod::Elements => None,
            },
            tolerance: number(&self.fields[3], "Tolerans")?,
            reverse: self.reverse,
            start: number(&self.fields[4], "Başlangıç km'si")?,
            expected_revision: None,
        })
    }
}

impl App {
    /// Çizgiden aks: the window on the one selected line or open polyline.
    pub(crate) fn open_road_alignment_fit(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Çizgiden aks yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let picked = match self.selection.ids() {
            [slot] => doc.model.get(*slot).and_then(|e| {
                let pts = drawn_points(e)?;
                doc.model.uid(*slot).map(|u| (u.to_string(), pts))
            }),
            _ => None,
        };
        let Some((uid, source)) = picked else {
            self.warn("Çizgiden aks için önce bir çizgi ya da açık çoklu çizgi seçin.".to_owned());
            return;
        };
        self.road_alignment_fit_window = Some(Box::new(Window {
            uid,
            source,
            method: AlignmentFitMethod::Elements,
            reverse: false,
            fields: [String::new(), String::new(), String::new(), "0.05".into(), String::new()],
            preview: Preview::default(),
        }));
        self.dialog = Some(Dialog::RoadAlignmentFit);
        self.refresh_road_alignment_fit();
    }

    fn refresh_road_alignment_fit(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.road_alignment_fit_window.as_mut() else { return };
        w.preview = match w.input().and_then(|q| road_alignments_fit::preview(&doc.model, &q, 400)) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok((fitted, warnings, line)) => Preview { fitted: Some(fitted), warnings, why: None, line },
        };
    }

    pub(crate) fn road_alignment_fit_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.road_alignment_fit_window.as_mut() else { return Task::none() };
        match event {
            Event::Method(m) => w.method = m,
            Event::Reverse(r) => w.reverse = r,
            Event::Field(k, t) => w.fields[k.min(4)] = t,
            Event::Make => return self.make_road_alignment_fit(),
            Event::Close => {
                self.road_alignment_fit_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_road_alignment_fit();
        Task::none()
    }

    /// Aksı oluştur: `road.alignments.fit`, one undo step.
    fn make_road_alignment_fit(&mut self) -> Task<Message> {
        let Some(w) = self.road_alignment_fit_window.as_ref() else { return Task::none() };
        let input = match w.input() {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Aks oluşturulamaz: {why}"));
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_alignments_fit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        self.say(
            Level::Success,
            format!(
                "“{}”: {} PI, {} kurp, {:.3} m; çizgiden en çok {:.3} m ayrılıyor.",
                out.name,
                out.pis.len(),
                out.curves,
                out.length,
                out.deviation
            ),
        );
        self.road_alignment_fit_window = None;
        self.dialog = None;
        Task::none()
    }

    pub(crate) fn road_alignment_fit_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road_alignment_fit_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let on_method = Select::new(
            METHODS.iter().map(|(n, _)| Choice::new(*n)),
            METHODS.iter().position(|(_, m)| *m == w.method),
            |i| msg(Event::Method(METHODS[i].1)),
        );
        let field = |hint: &'static str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width)
        };
        let mut top = row![
            words::field("Yöntem", container(on_method).width(150), None),
            words::field("Ad", field("1. aks", 0, 150.0), None),
        ]
        .spacing(10);
        top = match w.method {
            AlignmentFitMethod::Elements => top.push(words::field("Kurp yarıçapı (m)", field("kurpsuz", 1, 110.0), None)),
            AlignmentFitMethod::BestFit => top.push(words::field("En büyük yarıçap (m)", field("örn. 500", 2, 120.0), None)),
        };
        top = top
            .push(words::field("Tolerans (m)", field("yok", 3, 90.0), None))
            .push(words::field("Baş km (m)", field("0", 4, 90.0), None))
            .push(kentos_ui::widget::Switch::new(w.reverse, |r| msg(Event::Reverse(r))).label("Ters yön"));
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0).push(top.align_y(iced::alignment::Vertical::Bottom));
        match (&w.preview.why, &w.preview.fitted) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(f)) => {
                body = body.push(label::caption(format!(
                    "{} PI · {} aliyman · {} kurp · boy {:.3} m · çizgiden en çok {:.3} m",
                    f.pis.len(),
                    f.tangents,
                    f.curves,
                    f.length,
                    f.deviation
                )));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let picture = crate::picture::picture(iced::widget::canvas(Plan { source: &w.source, line: &w.preview.line, fitted: f }).width(Fill).height(Fill), TITLE);
                let mut list = Column::new().spacing(2).push(
                    row![
                        label::caption("PI").style(muted).width(50),
                        label::caption("Y").style(muted).width(110),
                        label::caption("X").style(muted).width(110),
                        label::caption("R").style(muted).width(80),
                    ]
                    .spacing(6),
                );
                for p in &f.pis {
                    list = list.push(
                        row![
                            label::caption(p.id.clone()).width(50),
                            label::caption(format!("{:.3}", p.x)).width(110),
                            label::caption(format!("{:.3}", p.y)).width(110),
                            label::caption(if p.radius > 0.0 { format!("{:.3}", p.radius) } else { "—".into() }).width(80),
                        ]
                        .spacing(6),
                    );
                }
                body = body.push_grow(
                    row![picture, container(scrollable(list).height(Fill)).width(typography::scaled(440.0)).padding([6, 8])]
                        .spacing(12)
                        .height(Fill),
                    typography::scaled(300.0),
                );
            }
            _ => {}
        }
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            row![
                label::caption("Çizgi kalır; aks tek geri alma adımı.").style(muted),
                space::horizontal().width(Fill),
                words::secondary(CLOSE, Some(msg(Event::Close))),
                words::primary(MAKE, w.preview.fitted.as_ref().map(|_| msg(Event::Make))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(960.0).modal(msg(Event::Close))
    }

    pub(crate) fn road_alignment_fit_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.road_alignment_fit_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(MAKE) => Some(msg(Event::Make)),
            Control::Press("Eleman eleman") => Some(msg(Event::Method(AlignmentFitMethod::Elements))),
            Control::Press("En iyi uyum") => Some(msg(Event::Method(AlignmentFitMethod::BestFit))),
            Control::Fill("Ad", t) => Some(msg(Event::Field(0, t.to_owned()))),
            Control::Fill("Kurp yarıçapı (m)", t) => Some(msg(Event::Field(1, t.to_owned()))),
            Control::Fill("En büyük yarıçap (m)", t) => Some(msg(Event::Field(2, t.to_owned()))),
            Control::Fill("Tolerans (m)", t) => Some(msg(Event::Field(3, t.to_owned()))),
            Control::Fill("Baş km (m)", t) => Some(msg(Event::Field(4, t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A line's or a polyline's points, its arcs in pieces; none for another object.
pub(crate) fn drawn_points(e: &Entity) -> Option<Vec<[f64; 2]>> {
    Some(match e {
        Entity::Line(l) => vec![[l.a.x, l.a.y], [l.b.x, l.b.y]],
        Entity::Polyline(p) => {
            let bulges = p.bulges.clone().unwrap_or_default();
            let mut out = Vec::new();
            for (i, w) in p.pts.windows(2).enumerate() {
                out.extend(arc_points([w[0].x, w[0].y], [w[1].x, w[1].y], bulges.get(i).copied().unwrap_or(0.0)));
            }
            if let Some(last) = p.pts.last() {
                out.push([last.x, last.y]);
            }
            out
        }
        _ => return None,
    })
}

/// A segment from `a` towards `b` as points, `b` left out: an arc (bulge `g`, the tangent of a quarter of its turn)
/// in 16 pieces, a straight segment its start.
fn arc_points(a: [f64; 2], b: [f64; 2], g: f64) -> Vec<[f64; 2]> {
    if g == 0.0 {
        return vec![a];
    }
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = dx.hypot(dy);
    let turn = 4.0 * g.atan();
    let r = chord / (2.0 * (0.5 * turn).sin()).abs();
    let h = r * (0.5 * turn).cos() * turn.signum();
    let c = [0.5 * (a[0] + b[0]) - dy / chord * h, 0.5 * (a[1] + b[1]) + dx / chord * h];
    let a0 = (a[1] - c[1]).atan2(a[0] - c[0]);
    (0..16).map(|k| {
        let t = a0 + turn * k as f64 / 16.0;
        [c[0] + r * t.cos(), c[1] + r * t.sin()]
    })
    .collect()
}

/// The line (muted), the alignment over it and its PIs, north up.
struct Plan<'a> {
    source: &'a [[f64; 2]],
    line: &'a [[f64; 2]],
    fitted: &'a AlignmentFitted,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let pis: Vec<[f64; 2]> = self.fitted.pis.iter().map(|p| [p.x, p.y]).collect();
        let pts: Vec<&[f64; 2]> = self.source.iter().chain(self.line).chain(&pis).collect();
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
        // The tangents through the PIs, thin and dashed.
        frame.stroke(
            &polyline(&pis),
            Stroke { line_dash: canvas::LineDash { segments: &[4.0, 3.0], offset: 0 }, ..Stroke::default() }
                .with_width(0.8)
                .with_color(tk.muted),
        );
        frame.stroke(&polyline(self.source), Stroke::default().with_width(4.0).with_color(iced::Color { a: 0.35, ..tk.text }));
        frame.stroke(&polyline(self.line), Stroke::default().with_width(1.6).with_color(tk.danger));
        for p in &pis {
            frame.stroke(&Path::circle(at(*p), 3.5), Stroke::default().with_width(1.2).with_color(tk.accent));
        }
        vec![frame.into_geometry()]
    }
}
