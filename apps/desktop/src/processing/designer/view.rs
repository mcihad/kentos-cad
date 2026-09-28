//! How Model tasarımcısı looks (the web's ModelDesigner and its `mdesign`
//! rules): the parts on the left, the diagram in the middle with its zoom
//! buttons, the selected box's settings on the right; Düzenle, the model's
//! state, Kapat, Kaydet ve çalıştır… and Kaydet at the foot. The window has
//! the web's size (1400 × 880 at most, 92 % of the window's height).
//! Closing with changes and Modeli sil ask over it.

use iced::widget::tooltip::Position;
use iced::widget::{button, canvas, column, container, responsive, row, space, stack};
use iced::{Border, Center, Element, Fill, Length, Theme};
use kentos_interaction::Format;
use kentos_processing::designer::{self as plan, StatusKind, texts};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Confirm, ContextMenu, Dialog, Menu, Tip, overlay, tip, vertical_divider};

use super::canvas::Diagram;
use super::inspector::{self, Context};
use super::{Asking, Designer, Event, palette};
use crate::app::{App, Message};

fn ev(e: Event) -> Message {
    Message::ModelDesigner(e)
}

/// The window's size, the web's pixels at the default text size.
const WIDTH: f32 = 1400.0;
const HEIGHT: f32 = 880.0;

impl App {
    /// Model tasarımcısı over the drawing, and what it asks over itself.
    pub(crate) fn model_designer_view(&self) -> Element<'_, Message> {
        let Some(d) = self.processing.designer.as_deref() else {
            return space().into();
        };
        let mut layers: Vec<Element<'_, Message>> = vec![
            responsive(move |room| {
                let width = typography::from_default(WIDTH).min(room.width - 60.0);
                let height = typography::from_default(HEIGHT).min(room.height * 0.92);
                overlay::modal(
                    self.model_designer_window(d, width, height),
                    ev(Event::Close),
                )
            })
            .into(),
        ];
        match d.asking {
            Some(Asking::Unsaved) => {
                layers.push(overlay::modal(unsaved(d), ev(Event::Answer(None))))
            }
            Some(Asking::Remove) => layers.push(overlay::modal(
                Confirm::new(
                    texts::remove::TITLE,
                    ev(Event::Answer(Some(true))),
                    ev(Event::Answer(None)),
                )
                .message(texts::remove::question(&d.draft.label))
                .confirm(texts::remove::ACTION)
                .destructive(),
                ev(Event::Answer(None)),
            )),
            None => {}
        }
        stack(layers).into()
    }

    fn model_designer_window<'a>(
        &'a self,
        d: &'a Designer,
        width: f32,
        height: f32,
    ) -> Element<'a, Message> {
        let registry = &self.processing.registry;
        // A narrow window (or large text) takes from the side columns before the diagram.
        let left = typography::scaled(palette::WIDTH).min(width * 0.26).floor();
        let right = typography::scaled(inspector::WIDTH)
            .min(width * 0.34)
            .floor();
        // What a fixed value's control reads, as the tool window's (processing/window.rs).
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |doc| Format::of(doc.settings()));
        let palette = crate::viewport::palette(self.canvas());
        let color = move |c: &str| {
            palette
                .resolve(c)
                .map_or_else(|| crate::view::hex_color(c), crate::view::rgba_color)
        };
        let cx = Context {
            registry,
            doc: self.document.as_ref().map(|doc| &doc.model),
            format: &format,
            color: &color,
        };
        let cols = row![
            container(palette::palette(d, registry))
                .width(Length::Fixed(left))
                .height(Fill),
            vertical_divider(),
            self.model_diagram(d),
            vertical_divider(),
            container(inspector::inspector(d, &cx))
                .width(Length::Fixed(right))
                .height(Fill)
                .style(style::container::header),
        ]
        .height(Fill);
        let frame = container(cols)
            .height(Fill)
            .clip(true)
            .style(|t: &Theme| container::Style {
                border: Border {
                    color: Tokens::of(t).border,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(4.0).into(),
                },
                ..container::Style::default()
            });
        Dialog::new(d.title())
            .push(frame)
            .push(footer(d))
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }

    /// The diagram with its zoom buttons, and the menu a wire let go over a step opens.
    fn model_diagram<'a>(&'a self, d: &'a Designer) -> Element<'a, Message> {
        let registry = &self.processing.registry;
        let carrying = d
            .carrying
            .as_deref()
            .and_then(|id| registry.tool(id))
            .map(|t| (t.label, t.icon));
        let problems = d
            .draft
            .steps
            .iter()
            .filter_map(|s| d.first_problem(&s.id).map(|p| (s.id.as_str(), p)))
            .collect();
        let diagram = canvas(Diagram {
            model: &d.draft,
            view: d.view,
            floor: d.floor,
            selected: d.selected.as_ref(),
            problems,
            registry,
            carrying,
            carried: d.carrying.as_deref(),
            known: d.canvas,
        })
        .width(Fill)
        .height(Fill);
        let zoom = |glyph: Icon, words: &'static str, e: Event| {
            tip(
                button(icon(glyph).size(15.0))
                    .padding(5)
                    .style(style::button::ghost)
                    .on_press(ev(e)),
                Tip::new(words),
                Position::Top,
            )
        };
        let tools = container(
            row![
                zoom(
                    Icon::ZoomOut,
                    texts::canvas::ZOOM_OUT,
                    Event::Zoom(1.0 / plan::canvas::ZOOM_STEP)
                ),
                zoom(
                    Icon::ZoomIn,
                    texts::canvas::ZOOM_IN,
                    Event::Zoom(plan::canvas::ZOOM_STEP)
                ),
                zoom(
                    crate::icons::from_web(Some("zoomExtents")),
                    texts::canvas::FIT,
                    Event::Fit
                ),
            ]
            .spacing(1),
        )
        .padding(2)
        .style(|t: &Theme| {
            let k = Tokens::of(t);
            container::Style {
                background: Some(k.surface.into()),
                border: Border {
                    color: k.border,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(4.0).into(),
                },
                ..container::Style::default()
            }
        });
        let body = stack![
            diagram,
            container(tools)
                .width(Fill)
                .height(Fill)
                .align_right(Fill)
                .align_bottom(Fill)
                .padding(10),
        ];
        let menu = d.wire_menu.as_ref().and_then(|m| {
            plan::connect_choices(&d.draft, &m.from, &m.to, &|id| registry.tool(id))
                .map(|c| (m.to.clone(), c))
        });
        let at = d.wire_menu.as_ref().map(|m| m.at);
        ContextMenu::controlled(
            body,
            at,
            move |_| {
                let Some((step, choices)) = &menu else {
                    return Menu::new();
                };
                let mut out = Menu::new().header(choices.header.clone());
                for c in &choices.items {
                    out = out.check(
                        c.label.clone(),
                        c.checked,
                        ev(Event::Connect {
                            step: step.clone(),
                            param: c.param.clone(),
                            src: c.src.clone(),
                        }),
                    );
                    if let Some(detail) = &c.detail {
                        out = out.detail(detail.clone());
                    }
                }
                if let Some(none) = &choices.none {
                    out = out.item(none.clone(), None::<Message>);
                }
                out
            },
            ev(Event::MenuClose),
        )
        .into()
    }
}

/// Düzenle, the model's state (its problems or that it is ready), Kapat,
/// Kaydet ve çalıştır… and Kaydet.
fn footer(d: &Designer) -> Element<'_, Message> {
    use texts::footer as words;
    let layout = tip(
        button(
            row![
                icon(crate::icons::from_web(Some("columns"))).size(14.0),
                label::body(words::LAYOUT)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([5, 10])
        .style(style::button::ghost)
        .on_press(ev(Event::Layout)),
        Tip::new(words::LAYOUT_TIP),
        Position::Top,
    );
    let (kind, text) =
        plan::designer_status(&d.problems, d.draft.steps.len(), d.draft.inputs.len());
    let warn = kind == StatusKind::Warn;
    let status = row![
        icon(if warn { Icon::Warning } else { Icon::Success })
            .size(16.0)
            .tone(if warn { Tone::Warning } else { Tone::Success }),
        label::body(text).style(style::text::muted).width(Fill),
    ]
    .spacing(8)
    .align_y(Center)
    .width(Fill);
    let close = button(label::body(words::CLOSE))
        .padding([5, 14])
        .style(style::button::secondary)
        .on_press(ev(Event::Close));
    let save_run = button(
        row![icon(Icon::Play).size(14.0), label::body(words::SAVE_RUN)]
            .spacing(6)
            .align_y(Center),
    )
    .padding([5, 14])
    .style(style::button::secondary)
    .on_press(ev(Event::SaveRun));
    let save = button(label::body(words::SAVE).style(style::text::on_accent))
        .padding([5, 14])
        .style(style::button::primary)
        .on_press(ev(Event::Save));
    row![layout, status, close, save_run, save]
        .spacing(10)
        .align_y(Center)
        .into()
}

/// Closing with changes (the web's `askUnsaved`, DESIGN.md §7.9.1): the
/// leaving answer aside on the left, Vazgeç, and saving first in weight.
fn unsaved(d: &Designer) -> Element<'_, Message> {
    let name = if kentos_processing::text::js_trim(&d.draft.label).is_empty() {
        texts::save::UNNAMED
    } else {
        d.draft.label.as_str()
    };
    let symbol = container(icon(Icon::Warning).size(18.0).tone(Tone::Warning))
        .center_x(36)
        .center_y(36)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(
                Tokens::of(t).warning.scale_alpha(0.14),
            )),
            border: iced::border::rounded(18.0),
            ..container::Style::default()
        });
    let text = column![
        label::title("Kaydedilmemiş değişiklikler"),
        label::body(format!(
            "“{name}” içinde kaydedilmemiş değişiklikler var. {}",
            texts::unsaved::AFTER
        )),
    ]
    .spacing(6)
    .width(Fill);
    let verb = texts::unsaved::VERB;
    let answer = |words: String, e: Event| {
        button(label::body(words))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(e))
    };
    let actions = row![
        answer(format!("Kaydetmeden {verb}"), Event::Answer(Some(false))),
        space::horizontal(),
        answer("Vazgeç".to_owned(), Event::Answer(None)),
        button(label::body(format!("Kaydet ve {verb}")).style(style::text::on_accent))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::Answer(Some(true)))),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![row![symbol, text].spacing(14), actions].spacing(18))
        .width(typography::scaled(480.0))
        .padding(18)
        .style(style::container::popover)
        .into()
}
