//! How the SVG editor looks (the web's `SvgEditor` and its `svge__*`
//! rules): the tools and the shape list on the left, the bar, the tracing
//! reference's bar, the canvas and the XML source in the middle, the
//! properties and tabs on the right; name, category, what the window said,
//! Vazgeç, Farklı kaydet… and Kaydet at the foot. The window has the web's
//! size (1320 × 860 at most, 92 % of the window's height) and stands over
//! the window that opened it; its own windows (import, export, document
//! properties, tracing, the library, save as) stand over it.

use iced::widget::{
    Column, button, canvas, column, container, pin, responsive, row, scrollable, shader, space,
    stack, text_input,
};
use iced::{Center, Element, Fill, Length, Rectangle, Renderer, Theme};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog, Sash, Tip, horizontal_divider, overlay, tip, vertical_divider};

use super::files::{self, FileDialog};
use super::list::list_column;
use super::menus::bar;
use super::paint::View;
use super::raster::{Quad, Raster};
use super::stage::{Palette, Stage};
use super::state::{Question, SvgEditor};
use super::{Event, TOOLS, change, ev};
use crate::app::{App, Dialog as Under, Message};

const WIDTH: f32 = 1320.0;
const HEIGHT: f32 = 860.0;
const LEFT: f32 = 210.0;
const RIGHT: f32 = 330.0;

/// The field and the paper under the drawing (the reference is drawn over them, the shapes over it).
struct Backdrop<'a> {
    ed: &'a SvgEditor,
    paper: String,
}

impl canvas::Program<Message> for Backdrop<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let pal = Palette::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), pal.field);
        let c = &self.ed.camera;
        let a = c.to_screen([0.0, 0.0]);
        let b = c.to_screen([self.ed.doc.width, self.ed.doc.height]);
        let (x, y, w, h) = (
            a[0] as f32,
            a[1] as f32,
            (b[0] - a[0]) as f32,
            (b[1] - a[1]) as f32,
        );
        // A soft shadow under the paper (the web's drop shadow).
        for (i, alpha) in [(3.0, 0.10), (2.0, 0.14), (1.0, 0.18)] {
            frame.fill_rectangle(
                iced::Point::new(x - i + 1.0, y - i + 2.0),
                iced::Size::new(w + 2.0 * i - 2.0, h + 2.0 * i - 2.0),
                iced::Color::from_rgba(0.0, 0.0, 0.0, alpha),
            );
        }
        let paper = kentos_ui::widget::color::parse_hex(&self.paper).unwrap_or(pal.panel);
        frame.fill_rectangle(iced::Point::new(x, y), iced::Size::new(w, h), paper);
        vec![frame.into_geometry()]
    }
}

/// The reference's picture at its place on the canvas.
fn reference_layer<'a>(ed: &'a SvgEditor) -> Option<Element<'a, Message>> {
    let r = ed.files.reference.as_ref()?;
    if !r.visible {
        return None;
    }
    let pixels = r.pixels.clone()?;
    let view = View {
        zoom: ed.camera.zoom,
        ox: ed.camera.ox,
        oy: ed.camera.oy,
    };
    let s = &r.spec;
    let p = |x: f64, y: f64| {
        let q = view.point([x, y]);
        [q.x, q.y]
    };
    Some(
        shader(Raster {
            pixels,
            quad: Quad([
                p(s.x, s.y),
                p(s.x + s.width, s.y),
                p(s.x, s.y + s.height),
                p(s.x + s.width, s.y + s.height),
            ]),
            opacity: s.opacity as f32,
        })
        .width(Fill)
        .height(Fill)
        .into(),
    )
}

/// A tool of the left column: its icon, name and key, lit when on.
fn tool_button<'a>(ed: &SvgEditor, t: &'static super::ToolDef) -> Element<'a, Message> {
    let on = ed.tool == t.id;
    let face = row![
        icon(crate::icons::from_web(Some(t.icon))).size(16.0),
        label::body(t.label).width(Fill),
        label::caption(t.key)
            .font(typography::mono())
            .style(style::text::muted),
    ]
    .spacing(8)
    .align_y(Center);
    tip(
        button(face)
            .padding([5, 8])
            .width(Fill)
            .style(style::button::tool(on))
            .on_press(ev(Event::Tool(t.id))),
        Tip::new(format!("{} ({}): {}", t.label, t.key, t.hint)),
        iced::widget::tooltip::Position::Right,
    )
}

fn left<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    let tools = Column::with_children(TOOLS.iter().map(|t| tool_button(ed, t))).spacing(2);
    column![
        tools,
        horizontal_divider(),
        scrollable(list_column(ed))
            .direction(style::field::thin_scrollbar())
            .height(Fill),
    ]
    .spacing(10)
    .into()
}

/// The guide's window, pinned where it was opened.
fn guide_popup<'a>(ed: &SvgEditor, size: iced::Size) -> Option<Element<'a, Message>> {
    let pop = ed.rulers.popup.as_ref()?;
    let field = |key: &'static str, words: &str, value: &str| {
        let key_s = key;
        container(
            row![
                container(label::caption(words.to_owned()).style(style::text::muted))
                    .width(typography::scaled(52.0)),
                text_input("", value)
                    .id(iced::widget::Id::from(format!("svge:guide-{key_s}")))
                    .on_input(move |t| {
                        change(move |ed| {
                            if let Some(p) = &mut ed.rulers.popup {
                                match key_s {
                                    "x" => p.x = t.clone(),
                                    "y" => p.y = t.clone(),
                                    _ => p.angle = t.clone(),
                                }
                            }
                        })
                    })
                    .on_submit(change(|ed| ed.guide_apply()))
                    .padding([3, 6])
                    .size(typography::body())
                    .style(style::field::validated(false))
                    .width(Fill),
            ]
            .spacing(6)
            .align_y(Center),
        )
    };
    let id = pop.id.clone();
    let body = column![
        label::caption("Kılavuz").font(typography::ui_strong()),
        field("x", "X", &pop.x),
        field("y", "Y", &pop.y),
        field("a", "Açı (°)", &pop.angle),
        row![
            button(label::caption("Sil").style(style::text::danger))
                .padding([3, 10])
                .style(style::button::secondary)
                .on_press(change(move |ed| ed.remove_guide(&id))),
            space::horizontal(),
            button(label::caption("Tamam").style(style::text::on_accent))
                .padding([3, 10])
                .style(style::button::primary)
                .on_press(change(|ed| ed.guide_apply())),
        ]
        .align_y(Center),
    ]
    .spacing(6);
    let w = typography::scaled(190.0);
    let h = typography::scaled(170.0);
    let x = (pop.at[0] as f32 + 8.0).min(size.width - w).max(0.0);
    let y = (pop.at[1] as f32 + 8.0).min(size.height - h).max(0.0);
    Some(
        pin(container(body)
            .padding(10)
            .width(w)
            .style(style::container::popover))
        .x(x)
        .y(y)
        .into(),
    )
}

fn center<'a>(ed: &'a SvgEditor, paper: String, ink: String) -> Element<'a, Message> {
    let mut layers: Vec<Element<'a, Message>> = vec![
        canvas(Backdrop {
            ed,
            paper: paper.clone(),
        })
        .width(Fill)
        .height(Fill)
        .into(),
    ];
    if let Some(r) = reference_layer(ed) {
        layers.push(r);
    }
    layers.push(
        canvas(Stage { ed, ink, paper })
            .width(Fill)
            .height(Fill)
            .into(),
    );
    let stage = responsive(move |size| {
        let mut l: Vec<Element<'a, Message>> = Vec::new();
        if let Some(p) = guide_popup(ed, size) {
            l.push(p);
        }
        stack(l).width(Fill).height(Fill).into()
    });
    layers.push(stage.into());
    let mut c = Column::new().push(bar(ed));
    if let Some(b) = files::reference::bar(ed) {
        c = c.push(b);
    }
    // The source shares the middle with the canvas, two parts to three, until
    // its edge is dragged; then it keeps its own height.
    let source = ed.files.source.as_ref();
    c = c.push(
        container(stack(layers))
            .height(match source {
                Some(s) if s.height.is_none() => Length::FillPortion(3),
                _ => Fill,
            })
            .width(Fill)
            .clip(true),
    );
    if let Some(s) = source {
        let (_, _, most) = source_room(ed, s);
        c = c.push(source_sash(ed, s));
        c = c.push(
            container(files::source::view(ed, s)).height(match s.height {
                Some(h) => Length::Fixed(h.min(most)),
                None => Length::FillPortion(2),
            }),
        );
    }
    c.height(Fill).width(Fill).into()
}

/// The source's height now (its own, or two thirds of the canvas's while they
/// share the middle), the least and the most it may take: the canvas and the
/// source keep 120 pixels each at the default text size.
fn source_room(ed: &SvgEditor, s: &files::source::SourcePanel) -> (f32, f32, f32) {
    let canvas = ed.camera.size.map_or(400.0, |(_, h)| h as f32);
    let now = s.height.unwrap_or(canvas * 2.0 / 3.0);
    let least = typography::from_default(120.0);
    (now, least, (canvas + now - least).max(least))
}

/// The source's top edge (the web's grip): dragged, the source takes that
/// height; a double click gives the first share back.
fn source_sash<'a>(ed: &SvgEditor, s: &files::source::SourcePanel) -> Element<'a, Message> {
    let (now, least, most) = source_room(ed, s);
    Sash::horizontal(now, |px| {
        change(move |ed| {
            if let Some(s) = ed.files.source.as_mut() {
                s.height = Some(px);
            }
        })
    })
    .reverse()
    .range(least..=most)
    .on_double_click(change(|ed| {
        if let Some(s) = ed.files.source.as_mut() {
            s.height = None;
        }
    }))
    .into()
}

/// The foot: name and category, what the window said, Vazgeç, Farklı kaydet… and Kaydet.
/// On a narrow window what it said takes its own line above, on both platforms (the web's
/// foot squeezed it to a word or two); the line stays while nothing is said, so the canvas
/// does not jump as messages come and go.
fn footer<'a>(ed: &'a SvgEditor, narrow: bool) -> Element<'a, Message> {
    let field = |id: &str, hint: &str, value: &str, width: f32, on: fn(String) -> Event| {
        text_input(hint, value)
            .id(iced::widget::Id::from(format!("svge:{id}")))
            .on_input(move |t| ev(on(t)))
            .padding([4, 7])
            .size(typography::body())
            .style(style::field::validated(false))
            .width(Length::Fixed(typography::from_default(width)))
    };
    let status: Element<'a, Message> = match &ed.said {
        Some((text, warn)) => {
            let warn = *warn;
            row![
                icon(if warn { Icon::Warning } else { Icon::Check })
                    .size(14.0)
                    .tone(if warn { Tone::Warning } else { Tone::Success }),
                label::body(text.clone())
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(move |t: &Theme| iced::widget::text::Style {
                        color: Some(if warn {
                            Tokens::of(t).warning
                        } else {
                            Tokens::of(t).muted
                        }),
                    }),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        }
        // A blank line's height.
        None if narrow => label::body(" ").into(),
        None => space().into(),
    };
    let (inline, above): (Element<'a, Message>, Option<Element<'a, Message>>) = if narrow {
        (space::horizontal().into(), Some(status))
    } else {
        (container(status).width(Fill).into(), None)
    };
    let foot = row![
        row![
            label::body("Ad").style(style::text::muted),
            field("name", "", &ed.name, 180.0, Event::Name)
        ]
        .spacing(6)
        .align_y(Center),
        row![
            label::body("Kategori").style(style::text::muted),
            field("path", "Ana / Alt", &ed.path_text, 200.0, Event::Path)
        ]
        .spacing(6)
        .align_y(Center),
        inline,
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(Event::Close)),
        tip(
            button(label::body("Farklı kaydet…"))
                .padding([5, 14])
                .style(style::button::secondary)
                .on_press(ev(Event::File(files::Event::Cmd(files::FileCmd::SaveAs)))),
            Tip::new("Yeni bir ad ve kategoriyle kitaplığa kaydeder (Ctrl+Shift+S)".to_owned()),
            iced::widget::tooltip::Position::Top,
        ),
        button(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                label::body("Kaydet").style(style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([5, 14])
        .style(style::button::primary)
        .on_press(ev(Event::Save)),
    ]
    .spacing(10)
    .align_y(Center);
    match above {
        Some(status) => column![status, foot].spacing(8).into(),
        None => foot.into(),
    }
}

/// Closing with changes, or another drawing replacing them (`askUnsaved`).
fn question<'a>(ed: &SvgEditor, q: &Question) -> Element<'a, Message> {
    let (after, verb) = super::update::pending_words(q);
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
            "“{}” içinde kaydedilmemiş değişiklikler var. {after}",
            ed.save_name()
        )),
    ]
    .spacing(6)
    .width(Fill);
    let answer = |words: String, e: Event| {
        button(label::body(words))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(e))
    };
    let actions = row![
        answer(format!("Kaydetmeden {verb}"), Event::Discard),
        space::horizontal(),
        answer("Vazgeç".to_owned(), Event::Stay),
        button(label::body(format!("Kaydet ve {verb}")).style(style::text::on_accent))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::SaveAndClose)),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![row![symbol, text].spacing(14), actions].spacing(18))
        .width(typography::scaled(480.0))
        .padding(18)
        .style(style::container::popover)
        .into()
}

/// Farklı kaydet (`saveAs`).
fn save_as<'a>(form: &files::SaveAs) -> Element<'a, Message> {
    let to = form.to;
    let field = |id: &'static str, words: &str, value: &str, on: fn(&mut files::SaveAs, String)| {
        crate::style::fields::labelled(
            words,
            text_input("", value)
                .id(iced::widget::Id::from(id))
                .on_input(move |t| files::save_as_change(move |f| on(f, t.clone())))
                .on_submit(ev(Event::File(files::Event::SaveAs)))
                .padding([4, 7])
                .size(typography::body())
                .style(style::field::validated(false))
                .width(Fill),
            None,
        )
    };
    let body = column![
        field("svge:saveas", "Ad", &form.name, |f, t| f.name = t),
        field("svge:saveaspath", "Kategori (A / B)", &form.path, |f, t| f.path = t),
        crate::style::fields::labelled(
            "Nereye",
            super::panels::seg(
                &[
                    super::panels::Opt("user", "Kitaplığım"),
                    super::panels::Opt("project", "Proje"),
                ],
                Some(if to == kentos_native_style::library::Source::Project {
                    "project"
                } else {
                    "user"
                }),
                &["Bütün projelerde", "Yalnızca bu projede"],
                |v| {
                    files::save_as_change(move |f| {
                        f.to = if v == "project" {
                            kentos_native_style::library::Source::Project
                        } else {
                            kentos_native_style::library::Source::User
                        }
                    })
                },
            ),
            None,
        ),
        crate::style::fields::hint("Kitaplığım bütün projelerde, Proje yalnızca bu projede görünür. Açık çizim artık yeni kayda bağlanır."),
    ]
    .spacing(12);
    Dialog::new("Farklı kaydet")
        .push(body)
        .push(
            row![
                space::horizontal(),
                button(label::body("Vazgeç"))
                    .padding([5, 14])
                    .style(style::button::secondary)
                    .on_press(change(|ed| ed.files.dialog = None)),
                button(
                    row![
                        icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                        label::body("Kaydet").style(style::text::on_accent)
                    ]
                    .spacing(6)
                    .align_y(Center)
                )
                .padding([5, 14])
                .style(style::button::primary)
                .on_press(ev(Event::File(files::Event::SaveAs))),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .width(typography::unscaled(typography::from_default(440.0)))
        .into()
}

/// A picture dropped on the window: a reference, or traced.
fn dropped<'a>(name: &str) -> Element<'a, Message> {
    let choice = |words: &str, detail: &str, e: Message| {
        button(
            column![
                label::body(words.to_owned()),
                label::caption(detail.to_owned()).style(style::text::muted)
            ]
            .spacing(2),
        )
        .padding([6, 10])
        .width(Fill)
        .style(style::button::secondary)
        .on_press(e)
    };
    Dialog::new(format!("“{name}”"))
        .push(choice(
            "İzleme altlığı yap",
            "Kilitli, yarı saydam arka plan; üzerinden çizin",
            change(|ed| {
                if let Some(FileDialog::Dropped(name, bytes)) = ed.files.dialog.take() {
                    let images = ed.images.clone();
                    files::reference::load(ed, &name, &bytes, &images);
                }
            }),
        ))
        .push(choice(
            "Bitmap izle…",
            "Görüntüyü delikli yollara çevirir",
            change(|ed| {
                if let Some(FileDialog::Dropped(name, bytes)) = ed.files.dialog.take() {
                    files::trace::open(ed, Some((name, bytes)));
                }
            }),
        ))
        .width(typography::unscaled(typography::from_default(360.0)))
        .into()
}

impl App {
    /// The SVG editor over the window that opened it.
    pub(crate) fn svgedit_view(&self) -> Element<'_, Message> {
        let Some(ed) = &self.styles.svg_editor else {
            return space().into();
        };
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        match ed.under {
            Some(Under::StyleManager) if self.styles.manager.is_some() => {
                layers.push(self.style_manager_view());
            }
            Some(Under::SymbolDesigner) if self.styles.designer.is_some() => {
                layers.push(self.designer_view());
            }
            _ => {}
        }
        let (ink, paper) = {
            let t = self.style_palette();
            let ink = if ed.options.ink_auto {
                t.ink
            } else {
                ed.options.ink.clone()
            };
            let paper = ed.doc.background.clone().unwrap_or(t.paper);
            (ink, paper)
        };
        *ed.theme.borrow_mut() = (ink.clone(), paper.clone());
        layers.push(
            responsive(move |room| {
                let width = typography::from_default(WIDTH).min(room.width - 60.0);
                let height = typography::from_default(HEIGHT).min(room.height * 0.92);
                overlay::modal(
                    self.svgedit_window(ed, width, height, paper.clone(), ink.clone()),
                    ev(Event::Close),
                )
            })
            .into(),
        );
        if let Some(d) = &ed.files.dialog {
            let over: Element<'_, Message> = match d {
                FileDialog::Import(d) => responsive(move |room| {
                    files::import::view(
                        ed,
                        d,
                        &ed.ink(),
                        typography::from_default(820.0).min(room.width - 60.0),
                        room.height * 0.92,
                    )
                })
                .into(),
                FileDialog::Export(st) => responsive(move |room| {
                    files::export::view(
                        ed,
                        st,
                        typography::from_default(760.0).min(room.width - 60.0),
                        room.height * 0.92,
                    )
                })
                .into(),
                FileDialog::DocProps(d) => files::docprops::view(ed, d),
                FileDialog::Trace(d) => responsive(move |room| {
                    files::trace::view(
                        ed,
                        d,
                        typography::from_default(1000.0).min(room.width - 60.0),
                        typography::from_default(640.0).min(room.height * 0.92),
                    )
                })
                .into(),
                FileDialog::SaveAs(form) => save_as(form),
                FileDialog::Library(p) => responsive(move |room| {
                    self.svgedit_library_view(
                        p,
                        typography::from_default(760.0).min(room.width - 60.0),
                    )
                })
                .into(),
                FileDialog::Place => files::reference::place_view(ed),
                FileDialog::Dropped(name, _) => dropped(name),
            };
            layers.push(overlay::modal(over, change(|ed| ed.files.dialog = None)));
        }
        if let Some(q) = &ed.question {
            layers.push(overlay::modal(question(ed, q), ev(Event::Stay)));
        }
        stack(layers).into()
    }

    fn svgedit_window<'a>(
        &'a self,
        ed: &'a SvgEditor,
        width: f32,
        height: f32,
        paper: String,
        ink: String,
    ) -> Element<'a, Message> {
        let left_w = typography::from_default(LEFT).min(width * 0.2).floor();
        let right_w = typography::from_default(RIGHT).min(width * 0.3).floor();
        let cols = row![
            container(left(ed))
                .width(Length::Fixed(left_w))
                .height(Fill)
                .padding(10)
                .style(style::container::header),
            vertical_divider(),
            container(center(ed, paper, ink)).width(Fill).height(Fill),
            vertical_divider(),
            container(
                // Inside: the sides' padding (2 × 14) and the scrollbar with its gap (6 + 8).
                scrollable(
                    container(super::panels::with_room(right_w - 42.0, || {
                        super::panels::panel(ed)
                    }))
                    .padding([10, 14])
                )
                .direction(style::field::body_scrollbar())
                .height(Fill)
            )
            .width(Length::Fixed(right_w))
            .height(Fill)
            .style(style::container::header),
        ]
        .height(Fill);
        let frame = container(cols)
            .height(Fill)
            .clip(true)
            .style(|t: &Theme| container::Style {
                border: iced::Border {
                    color: Tokens::of(t).border,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(4.0).into(),
                },
                ..container::Style::default()
            });
        Dialog::new(ed.title())
            .push(frame)
            .push(footer(ed, width < typography::from_default(1250.0)))
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }
}
