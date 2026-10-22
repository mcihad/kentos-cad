//! A viewport boundary editor: percentages in, the shared setViewport operation out.
use crate::{
    Message,
    designer::{Designer, Dialog as Window},
};
use iced::widget::{button, canvas, column, text_editor};
use iced::{Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_sheet::{clip::Clip, ops::Op, viewport::SetViewport};
use kentos_ui::{
    label, style,
    theme::{Tokens, typography},
    widget::{Dialog, Segmented},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Frame,
    Rectangle,
    Polygon,
}
impl core::fmt::Display for Mode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Frame => "Tam çerçeve",
            Self::Rectangle => "Dikdörtgen",
            Self::Polygon => "Çokgen",
        })
    }
}
#[derive(Clone, Debug)]
pub enum ClipMessage {
    Open(String),
    Mode(Mode),
    Edit(text_editor::Action),
    Apply,
}
#[derive(Debug)]
pub(crate) struct ClipDialog {
    pub id: String,
    pub mode: Mode,
    pub content: text_editor::Content,
    pub error: Option<String>,
    rectangle_draft: Option<String>,
    polygon_draft: Option<String>,
    aspect: f32,
}
impl ClipDialog {
    fn of(id: String, clip: &Clip, aspect: f32) -> Self {
        let mode = match clip {
            Clip::Frame => Mode::Frame,
            Clip::Rectangle { .. } => Mode::Rectangle,
            Clip::Polygon { .. } => Mode::Polygon,
        };
        let points = match clip {
            Clip::Rectangle {
                bounds: [l, t, r, b],
            } => vec![[*l, *t], [*r, *b]],
            _ => clip.points(),
        };
        let text = points
            .iter()
            .map(|p| format!("{} {}", f64::from(p[0]) / 10000., f64::from(p[1]) / 10000.))
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            id,
            mode,
            aspect,
            content: text_editor::Content::with_text(&text),
            rectangle_draft: (mode == Mode::Rectangle).then(|| text.clone()),
            polygon_draft: (mode == Mode::Polygon).then(|| text.clone()),
            error: None,
        }
    }
    fn boundary(&self) -> Result<Clip, String> {
        if self.mode == Mode::Frame {
            return Ok(Clip::Frame);
        }
        let mut points = Vec::new();
        for (line, text) in self
            .content
            .text()
            .lines()
            .filter(|v| !v.trim().is_empty())
            .enumerate()
        {
            let values = text
                .split_whitespace()
                .map(|v| v.replace(',', ".").parse::<f64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| format!("{}. satırda iki yüzde yazın; örnek: 10 20.", line + 1))?;
            if values.len() != 2
                || values
                    .iter()
                    .any(|v| !v.is_finite() || !(0. ..=100.).contains(v))
            {
                return Err(format!(
                    "{}. satırda 0–100 arasında iki yüzde olmalı.",
                    line + 1
                ));
            }
            points.push([
                (values[0] * 10000.).round() as i32,
                (values[1] * 10000.).round() as i32,
            ]);
        }
        let clip = match self.mode {
            Mode::Frame => Clip::Frame,
            Mode::Rectangle => {
                if points.len() != 2 {
                    return Err(
                        "Dikdörtgen için sol üst ve sağ alt köşeyi iki satıra yazın.".into(),
                    );
                }
                Clip::Rectangle {
                    bounds: [points[0][0], points[0][1], points[1][0], points[1][1]],
                }
            }
            Mode::Polygon => Clip::Polygon { points },
        };
        clip.validate().map_err(|e| e.message)?;
        Ok(clip)
    }
}
impl Designer {
    pub(crate) fn viewport_clip_update(&mut self, m: ClipMessage) {
        match m {
            ClipMessage::Open(id) => {
                if let Some(i) = self.book.item(&id)
                    && !i.locked
                    && let kentos_sheet::ItemKind::Map(map) = &i.kind
                {
                    self.dialog = Some(Window::ViewportClip(ClipDialog::of(id, &map.clip, {
                        let r = kentos_sheet::display::map_content_rect(i, map);
                        r.width as f32 / r.height.max(1) as f32
                    })));
                    self.error = None;
                }
            }
            ClipMessage::Apply => {
                let Some(Window::ViewportClip(d)) = &self.dialog else {
                    return;
                };
                let id = d.id.clone();
                let clip = d.boundary();
                match clip {
                    Ok(clip) => {
                        self.commit(vec![Op::SetViewport(SetViewport {
                            id,
                            clip: Some(clip),
                            ..Default::default()
                        })]);
                        if self.error.is_none() {
                            self.dialog = None;
                        } else if let Some(Window::ViewportClip(d)) = &mut self.dialog {
                            d.error = self.error.clone();
                        }
                    }
                    Err(error) => {
                        if let Some(Window::ViewportClip(d)) = &mut self.dialog {
                            d.error = Some(error);
                        }
                    }
                }
            }
            other => {
                if let Some(Window::ViewportClip(d)) = &mut self.dialog {
                    match other {
                        ClipMessage::Edit(action) => {
                            d.content.perform(action);
                            d.error = None;
                        }
                        ClipMessage::Mode(mode) if mode != d.mode => {
                            match d.mode {
                                Mode::Rectangle => d.rectangle_draft = Some(d.content.text()),
                                Mode::Polygon => d.polygon_draft = Some(d.content.text()),
                                Mode::Frame => {}
                            }
                            d.mode = mode;
                            let text = match mode {
                                Mode::Frame => "0 0
100 0
100 100
0 100"
                                    .to_owned(),
                                Mode::Rectangle => d.rectangle_draft.clone().unwrap_or_else(|| {
                                    "0 0
100 100"
                                        .into()
                                }),
                                Mode::Polygon => d.polygon_draft.clone().unwrap_or_else(|| {
                                    "0 0
100 0
100 100
0 100"
                                        .into()
                                }),
                            };
                            d.content = text_editor::Content::with_text(&text);
                            d.error = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    pub(crate) fn viewport_clip_view<'a>(&'a self, d: &'a ClipDialog) -> Element<'a, Message> {
        let on = Message::ViewportClip;
        let mut body = column![
            label::muted("Sınır, içerik çerçevesinin yüzdesidir. Ölçek ve model değişmez."),
            Segmented::new(
                [Mode::Frame, Mode::Rectangle, Mode::Polygon],
                d.mode,
                move |m| on(ClipMessage::Mode(m))
            )
            .width(Fill)
        ]
        .spacing(10);
        if d.mode != Mode::Frame {
            body = body.push(label::body(if d.mode == Mode::Rectangle {
                "Her satıra yatay ve düşey yüzde: önce sol üst, sonra sağ alt köşe."
            } else {
                "Her satıra yatay ve düşey yüzde yazın. Sıra sınırı izler; son köşe ilkine kapanır."
            }));
            body = body.push(
                text_editor(&d.content)
                    .on_action(move |a| on(ClipMessage::Edit(a)))
                    .font(typography::ui())
                    .size(typography::body())
                    .height(typography::scaled(125.))
                    .style(style::field::text_area),
            );
        }
        let parsed = d.boundary();
        if let Some(error) = d.error.clone().or_else(|| parsed.as_ref().err().cloned()) {
            body = body.push(label::body(error).style(style::text::danger));
        }
        body = body.push(
            canvas::Canvas::new(Preview(parsed.ok(), d.aspect))
                .width(Fill)
                .height(110.),
        );
        Dialog::new("Görünüm penceresi kırpması")
            .width(520.)
            .scroll(body)
            .action(
                button(label::body("Vazgeç"))
                    .on_press(Message::CloseDialog)
                    .style(style::button::secondary),
            )
            .action(
                button(label::strong("Uygula"))
                    .on_press_maybe(d.boundary().is_ok().then_some(on(ClipMessage::Apply)))
                    .style(style::button::primary),
            )
            .into()
    }
}
struct Preview(Option<Clip>, f32);
impl<M> canvas::Program<M> for Preview {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let t = Tokens::of(theme);
        let height = (bounds.height - 16.).max(1.);
        let width = (height * self.1).min((bounds.width - 24.).max(1.));
        let height = (width / self.1).min(height);
        let rect = Rectangle::new(
            Point::new((bounds.width - width) / 2., (bounds.height - height) / 2.),
            iced::Size::new(width, height),
        );
        frame.fill_rectangle(rect.position(), rect.size(), t.surface_alt);
        frame.stroke(
            &canvas::Path::rectangle(rect.position(), rect.size()),
            canvas::Stroke::default().with_color(t.border),
        );
        if let Some(clip) = &self.0 {
            let points = clip.points();
            let path = canvas::Path::new(|p| {
                for (i, point) in points.iter().enumerate() {
                    let at = Point::new(
                        rect.x + point[0] as f32 / 1e6 * rect.width,
                        rect.y + point[1] as f32 / 1e6 * rect.height,
                    );
                    if i == 0 {
                        p.move_to(at);
                    } else {
                        p.line_to(at);
                    }
                }
                p.close();
            });
            frame.fill(&path, t.accent.scale_alpha(0.15));
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(t.accent)
                    .with_width(1.5),
            );
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentage_editor_matches_all_independent_valid_boundaries() {
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/sheet/v1/viewport-clips.json"
        ))
        .unwrap();
        for case in data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["expected"].get("clip").is_some())
        {
            let clip: Clip = serde_json::from_value(case["expected"]["clip"].clone()).unwrap();
            let d = ClipDialog::of("m".into(), &clip, 1.);
            assert_eq!(d.boundary().unwrap(), clip);
        }
        for text in ["NaN 20\n100 100", "-1 0\n100 100", "0 0 0\n100 100", "0 0"] {
            let mut d = ClipDialog::of("m".into(), &Clip::Frame, 1.);
            d.mode = Mode::Rectangle;
            d.content = text_editor::Content::with_text(text);
            assert!(d.boundary().is_err());
        }
    }
}
