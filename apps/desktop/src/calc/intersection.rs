//! Önden ve geriden kestirme (the web's `ui/calc/IntersectionDialog.ts`,
//! docs/adr/0070). Önden: angles measured at two known points A and B
//! towards the new point (α at A clockwise from B to P, β at B clockwise
//! from P to A: P lies right of A → B). Geriden: at the new point, angles
//! towards three known points seen left to right (α from A to B, β from B
//! to C). A sketch shows which angle is which. The computation is the
//! shared core's (`survey::intersection`).

use std::fmt;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Canvas, column, container, row};
use iced::{Element, Fill, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};
use kentos_domain::Document as Model;
use kentos_interaction::survey::Unit;
use kentos_interaction::survey::intersection::{forward_intersection, resection};
use kentos_interaction::{Format, Vec2, js_trim};
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog;
use kentos_ui::widget::segmented::Segmented;

use super::read::{Known, read_number, resolve_point};
use super::{
    Event, Field, event, footer_button, known_field, layer_select, number_field, result_table,
    summary,
};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

/// Which kestirme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Forward,
    Resection,
}

impl Kind {
    /// The step's and the report's title (the window's own is “Kestirme”).
    pub fn title(self) -> &'static str {
        match self {
            Kind::Forward => "Önden kestirme",
            Kind::Resection => "Geriden kestirme",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Kind::Forward => "İki bilinen noktada yeni noktaya açı ölçüldü.",
            Kind::Resection => "Yeni noktada üç bilinen noktaya açı ölçüldü.",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Forward => "Önden",
            Kind::Resection => "Geriden",
        })
    }
}

/// What is typed, kept while the app runs.
#[derive(Clone, Debug, Default)]
pub struct Form {
    pub kind: Kind,
    pub a: String,
    pub b: String,
    pub c: String,
    pub alpha: String,
    pub beta: String,
    pub name: String,
    pub layer: Option<String>,
}

/// The new point, found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub p: Vec2,
    pub name: String,
    /// Geriden's strength: near 0 the point is near the danger circle.
    pub strength: Option<f64>,
}

/// What the fields give: the point, or why not.
#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    pub errors: Vec<String>,
    pub result: Option<Found>,
}

/// The project's angle unit as the core takes it.
pub(crate) fn unit(model: &Model) -> Unit {
    match model.settings().angle_unit {
        kentos_contracts::AngleUnit::Deg => Unit::DEG,
        kentos_contracts::AngleUnit::Grad => Unit::GRAD,
    }
}

impl Form {
    /// The fields read and computed (the web's `recompute`).
    pub fn compute(&self, model: &Model) -> Computed {
        let mut errors = Vec::new();
        let mut known = |text: &str, label: &str| match resolve_point(model, text) {
            Known::Empty => {
                errors.push(format!("{label} verilmedi."));
                None
            }
            Known::Error(e) => {
                errors.push(format!("{label}: {e}"));
                None
            }
            Known::Point { p, .. } => Some(p),
        };
        let forward = self.kind == Kind::Forward;
        let a = known(&self.a, "A noktası");
        let b = known(&self.b, "B noktası");
        let c = if forward {
            None
        } else {
            known(&self.c, "C noktası")
        };
        let alpha = read_number(&self.alpha).filter(|v| !v.is_nan());
        let beta = read_number(&self.beta).filter(|v| !v.is_nan());
        if alpha.is_none() {
            errors.push("α açısını yazın.".to_owned());
        }
        if beta.is_none() {
            errors.push("β açısını yazın.".to_owned());
        }
        let name = match js_trim(&self.name) {
            "" => "P".to_owned(),
            name => name.to_owned(),
        };
        let mut result = None;
        if let (true, Some(a), Some(b), Some(alpha), Some(beta)) =
            (errors.is_empty(), a, b, alpha, beta)
        {
            let unit = unit(model);
            let found = if forward {
                forward_intersection(unit, a, b, alpha, beta).map(|p| (p, None))
            } else if let Some(c) = c {
                resection(unit, a, b, c, alpha, beta).map(|r| (r.p, Some(r.strength)))
            } else {
                Err(String::new())
            };
            match found {
                Ok((p, strength)) => {
                    result = Some(Found { p, name, strength });
                }
                Err(e) if !e.is_empty() => errors.push(e),
                Err(_) => {}
            }
        }
        Computed { errors, result }
    }

    /// The layer new points go to: the one chosen, else `poligon`, else the active one.
    pub fn layer_or_default(&self, model: &Model) -> String {
        let layers = model.layers();
        match &self.layer {
            Some(id) if layers.get(id).is_some() => id.clone(),
            _ if layers.get("poligon").is_some() => "poligon".to_owned(),
            _ => layers.active().to_owned(),
        }
    }

    /// The report's lines (the web's `copyReport`): none without a result.
    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        let found = self.compute(model).result?;
        let title = self.kind.title().to_owned();
        Some(vec![
            vec![title],
            vec!["Nokta".into(), "Y".into(), "X".into()],
            vec![found.name, format.coord(found.p.x), format.coord(found.p.y)],
            vec![
                "α".into(),
                self.alpha.clone(),
                "β".into(),
                self.beta.clone(),
            ],
        ])
    }

    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let computed = self.compute(model);
        let forward = self.kind == Kind::Forward;
        let unit = format.angle_unit_label();
        let kind = Segmented::new([Kind::Forward, Kind::Resection], self.kind, |k| {
            event(Event::Kind(k))
        });
        let mut knowns = column![
            known_field(model, format, "A noktası", Field::A, &self.a, None),
            known_field(model, format, "B noktası", Field::B, &self.b, None),
        ]
        .spacing(12);
        if !forward {
            knowns = knowns.push(known_field(
                model,
                format,
                "C noktası",
                Field::C,
                &self.c,
                None,
            ));
        }
        let (alpha, beta) = if forward {
            (
                format!("α: A'da B'den P'ye ({unit})"),
                format!("β: B'de P'den A'ya ({unit})"),
            )
        } else {
            (
                format!("α: P'de A'dan B'ye ({unit})"),
                format!("β: P'de B'den C'ye ({unit})"),
            )
        };
        knowns = knowns.push(
            row![
                angle_field(alpha, &self.alpha, |t| event(Event::Alpha(t))),
                angle_field(beta, &self.beta, |t| event(Event::Beta(t))),
                number_field("Yeni noktanın adı", &self.name, "P", |t| event(
                    Event::Name(t)
                )),
            ]
            .spacing(12),
        );
        let sketch = column![
            Canvas::new(Sketch { kind: self.kind })
                .width(Length::Fixed(SKETCH_W))
                .height(Length::Fixed(SKETCH_H)),
            label::caption(if forward {
                "Açılar saat yönünde ölçülür; P, A'dan B'ye bakınca sağdadır."
            } else {
                "Bilinen noktalar P'den bakınca soldan sağa A, B, C sırasındadır; açılar saat yönündedir."
            })
            .width(Length::Fixed(SKETCH_W)),
        ]
        .spacing(6);
        let mut dialog = Dialog::new("Kestirme")
            .push(words::field(
                "Kestirme türü",
                kind,
                Some(self.kind.hint().to_owned()),
            ))
            .push(row![knowns.width(Fill), container(sketch).padding([4, 0])].spacing(20));
        let lines = match &computed.result {
            None => computed
                .errors
                .iter()
                .take(6)
                .map(|e| (Line::Warn, e.clone()))
                .collect(),
            Some(found) => {
                let mut lines = vec![(Line::Ok, format!("{} noktası hesaplandı.", found.name))];
                if found.strength.is_some_and(|s| s < 0.05) {
                    lines.push((
                        Line::Warn,
                        "Nokta tehlike dairesine yakın (A, B, C ve durulan nokta neredeyse aynı çember üzerinde): küçük açı hataları konumu çok değiştirir. Başka bir bilinen noktayla denetleyin.".to_owned(),
                    ));
                }
                lines
            }
        };
        if let Some(summary) = summary(lines) {
            dialog = dialog.push(summary);
        }
        if let Some(found) = &computed.result {
            dialog = dialog.push(result_table(
                &["Nokta", "Y (sağa)", "X (yukarı)"],
                vec![vec![
                    found.name.clone(),
                    format.coord(found.p.x),
                    format.coord(found.p.y),
                ]],
                &[false, true, true],
            ));
        }
        let done = computed.result.is_some();
        dialog
            .action(footer_button(
                "Raporu kopyala",
                done.then(|| event(Event::CopyReport)),
                false,
            ))
            .action(
                row![
                    label::body("Katman"),
                    layer_select(model, &self.layer_or_default(model))
                ]
                .spacing(8)
                .align_y(iced::Center),
            )
            .action(footer_button(
                "Çizime ekle",
                done.then(|| event(Event::AddPoints)),
                true,
            ))
            .action(footer_button("Kapat", Some(event(Event::Close)), false))
            .width(820.0)
            .into()
    }
}

/// An angle field: its label and the value typed.
fn angle_field<'a>(
    title: String,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        label::caption(title),
        iced::widget::text_input("", value)
            .on_input(on_input)
            .padding([5, 8])
            .width(Fill)
            .font(typography::mono())
            .size(typography::body())
            .style(kentos_ui::style::field::input),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

/// The sketch's size: the web's 200 × 118 drawing at 1.2.
const SKETCH_W: f32 = 240.0;
const SKETCH_H: f32 = 141.6;
const SCALE: f32 = 1.2;

/// The web's sketch (an SVG there): which point is where and which angle is
/// which, not to scale.
struct Sketch {
    kind: Kind,
}

impl<Message> canvas::Program<Message> for Sketch {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let (ink, accent) = (t.text, t.accent);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let at = |x: f32, y: f32| Point::new(x * SCALE, y * SCALE);
        let line = |frame: &mut canvas::Frame, a: (f32, f32), b: (f32, f32)| {
            frame.stroke(
                &Path::line(at(a.0, a.1), at(b.0, b.1)),
                Stroke::default().with_color(ink).with_width(1.3),
            );
        };
        // An arc about `c` from `from` to `to`, the short way round, as the SVG's `A`.
        let arc =
            |frame: &mut canvas::Frame, c: (f32, f32), r: f32, from: (f32, f32), to: (f32, f32)| {
                let a0 = (from.1 - c.1).atan2(from.0 - c.0);
                let mut a1 = (to.1 - c.1).atan2(to.0 - c.0);
                while a1 < a0 {
                    a1 += std::f32::consts::TAU;
                }
                let (start, end) = if a1 - a0 > std::f32::consts::PI {
                    (a1, a0 + std::f32::consts::TAU)
                } else {
                    (a0, a1)
                };
                let path = Path::new(|b| {
                    b.arc(canvas::path::Arc {
                        center: at(c.0, c.1),
                        radius: r * SCALE,
                        start_angle: iced::Radians(start),
                        end_angle: iced::Radians(end),
                    });
                });
                frame.stroke(&path, Stroke::default().with_color(accent).with_width(1.6));
            };
        let mark = |frame: &mut canvas::Frame, x: f32, y: f32| {
            frame.fill_rectangle(at(x, y), Size::new(6.0 * SCALE, 6.0 * SCALE), ink);
        };
        let word = |frame: &mut canvas::Frame, s: &str, x: f32, y: f32, color| {
            frame.fill_text(Text {
                content: s.to_owned(),
                position: at(x, y - 11.0),
                color,
                size: Pixels(13.0 * SCALE),
                font: typography::ui(),
                ..Text::default()
            });
        };
        match self.kind {
            Kind::Forward => {
                line(&mut frame, (30.0, 24.0), (170.0, 24.0));
                line(&mut frame, (30.0, 24.0), (104.0, 96.0));
                line(&mut frame, (170.0, 24.0), (104.0, 96.0));
                arc(&mut frame, (30.0, 24.0), 24.0, (54.0, 24.0), (47.2, 40.7));
                arc(
                    &mut frame,
                    (170.0, 24.0),
                    24.0,
                    (153.6, 41.4),
                    (146.0, 24.0),
                );
                mark(&mut frame, 27.0, 21.0);
                mark(&mut frame, 167.0, 21.0);
                word(&mut frame, "A", 16.0, 18.0, ink);
                word(&mut frame, "B", 178.0, 18.0, ink);
                word(&mut frame, "P", 112.0, 112.0, ink);
                word(&mut frame, "α", 58.0, 44.0, accent);
                word(&mut frame, "β", 132.0, 46.0, accent);
                circle(&mut frame, at(104.0, 96.0), ink);
            }
            Kind::Resection => {
                line(&mut frame, (100.0, 100.0), (30.0, 22.0));
                line(&mut frame, (100.0, 100.0), (100.0, 14.0));
                line(&mut frame, (100.0, 100.0), (170.0, 22.0));
                arc(
                    &mut frame,
                    (100.0, 100.0),
                    20.0,
                    (86.6, 85.1),
                    (100.0, 80.0),
                );
                arc(
                    &mut frame,
                    (100.0, 100.0),
                    24.0,
                    (100.0, 76.0),
                    (116.0, 82.1),
                );
                mark(&mut frame, 27.0, 19.0);
                mark(&mut frame, 97.0, 11.0);
                mark(&mut frame, 167.0, 19.0);
                word(&mut frame, "A", 16.0, 16.0, ink);
                word(&mut frame, "B", 106.0, 12.0, ink);
                word(&mut frame, "C", 178.0, 16.0, ink);
                word(&mut frame, "P", 108.0, 114.0, ink);
                word(&mut frame, "α", 80.0, 76.0, accent);
                word(&mut frame, "β", 112.0, 74.0, accent);
                circle(&mut frame, at(100.0, 100.0), ink);
            }
        }
        vec![frame.into_geometry()]
    }
}

/// The new point: a small ring.
fn circle(frame: &mut canvas::Frame, at: Point, ink: iced::Color) {
    frame.stroke(
        &Path::circle(at, 4.0 * SCALE),
        Stroke::default().with_color(ink).with_width(1.4),
    );
}
