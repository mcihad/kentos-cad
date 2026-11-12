//! The builder's editor (DESIGN.md §7.16): Iced's text editor in the mono
//! face, coloured by the core's tokens ([`super::highlight`]), with what a
//! highlighter cannot draw on a canvas over it (the error's and the
//! warnings' wavy lines, a box round the parenthesis at the cursor and its
//! pair) and the completion list beside the word being written.
//!
//! The text does not wrap and the editor is as large as its text (at least
//! its box): it never scrolls inside, a scrollable around it does, so the
//! canvas over it and the list stand where the characters are. Every face
//! the builder writes in is monospaced (0.6 em a character, as KentOS UI's
//! typography says), so a column is a fixed width.

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::text_editor::{Binding, KeyPress, Status};
use iced::widget::{
    Column, Stack, button, canvas as canvas_widget, container, row, scrollable, text, text_editor,
};
use iced::{
    Center, Element, Fill, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};
use kentos_expression::editor::{Completion, units};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Mode, Tokens, typography};
use kentos_ui::widget::beside::beside;

use super::highlight::{self, Painter, Syntax};
use super::{Builder, EDITOR, Event, ev};
use crate::app::Message;

/// The scrollable round the editor: the builder scrolls it to keep the cursor in view.
pub(crate) const SCROLLER: &str = "expression-builder-scroller";

/// A character's width in the mono faces (em).
const ADVANCE: f32 = 0.6;
/// Space between the editor's edge and its text.
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 8.0;
/// The completion list's rows at most, before it scrolls.
const LIST_ROWS: usize = 8;
/// The completion list's width (at 12 px text).
const LIST_WIDTH: f32 = 340.0;

/// The editor's measures at the interface's text size.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Metrics {
    pub size: f32,
    pub line: f32,
    pub advance: f32,
}

impl Metrics {
    pub fn now() -> Metrics {
        let size = typography::body();
        Metrics {
            size,
            line: (size * 1.6).round(),
            advance: size * ADVANCE,
        }
    }

    /// Where a character's cell starts in the editor (its top left).
    pub fn cell(&self, line: usize, column: usize) -> Point {
        Point::new(
            PAD_X + column as f32 * self.advance,
            PAD_Y + line as f32 * self.line,
        )
    }

    /// The editor's size for a text of this many lines, the longest this long.
    pub fn extent(&self, lines: usize, longest: usize) -> Size {
        Size::new(
            2.0 * PAD_X + (longest + 2) as f32 * self.advance,
            2.0 * PAD_Y + (lines + 1) as f32 * self.line,
        )
    }
}

/// A UTF-16 position as a line and a column of characters.
pub(crate) fn line_column(src: &str, at: usize) -> (usize, usize) {
    let (line, byte) = units::to_line_byte(src, at);
    let start = src
        .split('\n')
        .take(line)
        .map(|l| l.len() + 1)
        .sum::<usize>();
    let column = src[start..start + byte].chars().count();
    (line, column)
}

/// The text's lines and its longest line, in characters.
fn size_of(src: &str) -> (usize, usize) {
    let lines = src.split('\n');
    let longest = lines.clone().map(|l| l.chars().count()).max().unwrap_or(0);
    (lines.count(), longest)
}

/// Keys of the editor: the list's while it is open, Ctrl+Space and
/// Ctrl+Enter, Esc; the rest are the editor's own. Keys pressed while the
/// editor does not have the keyboard are not its.
fn keys(kp: KeyPress, list: bool) -> Option<Binding<Message>> {
    if !matches!(kp.status, Status::Focused { .. }) {
        return None;
    }
    let named = match kp.key.as_ref() {
        Key::Named(n) => Some(n),
        _ => None,
    };
    let custom = |e: Event| Some(Binding::Custom(ev(e)));
    let control = kp.modifiers.control();
    match named {
        Some(Named::Space) if control => return custom(Event::Complete),
        Some(Named::Enter) if control => return custom(Event::Ok),
        _ => {}
    }
    if list {
        match named {
            Some(Named::ArrowDown) => return custom(Event::Step(1)),
            Some(Named::ArrowUp) => return custom(Event::Step(-1)),
            Some(Named::PageDown) => return custom(Event::Step(7)),
            Some(Named::PageUp) => return custom(Event::Step(-7)),
            Some(Named::Enter | Named::Tab) => return custom(Event::AcceptActive),
            Some(Named::Escape) => return custom(Event::CloseList),
            _ => {}
        }
    } else if named == Some(Named::Escape) {
        // Esc in the editor is the window's Vazgeç (the web's), not the editor's unfocus.
        return custom(Event::Cancel);
    }
    Binding::from_key_press(kp)
}

/// A wavy line or a box to draw over the text.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mark {
    Wave {
        line: usize,
        from: usize,
        to: usize,
        error: bool,
    },
    Box {
        line: usize,
        column: usize,
        lone: bool,
    },
}

/// The canvas over the editor.
struct Marks {
    marks: Vec<Mark>,
    metrics: Metrics,
}

impl<M> canvas::Program<M> for Marks {
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
        let m = self.metrics;
        let mut frame = Frame::new(renderer, bounds.size());
        for mark in &self.marks {
            match *mark {
                Mark::Wave {
                    line,
                    from,
                    to,
                    error,
                } => {
                    let start = m.cell(line, from);
                    // An error at the end of the text is marked on the place after it.
                    let width = (to.max(from + 1) - from) as f32 * m.advance;
                    let y = start.y + (m.line + m.size) / 2.0 + 1.5;
                    let path = Path::new(|p| {
                        let step = 2.0;
                        let mut x = start.x;
                        p.move_to(Point::new(x, y));
                        let mut up = true;
                        while x < start.x + width {
                            x = (x + step).min(start.x + width);
                            p.line_to(Point::new(x, if up { y - 1.5 } else { y }));
                            up = !up;
                        }
                    });
                    let color = if error { t.danger } else { t.warning };
                    frame.stroke(&path, Stroke::default().with_color(color).with_width(1.0));
                }
                Mark::Box { line, column, lone } => {
                    let at = m.cell(line, column);
                    let height = (m.size * 1.35).round();
                    let top = at.y + ((m.line - height) / 2.0).round();
                    let rect = Path::rectangle(
                        Point::new(at.x - 0.5, top + 0.5),
                        Size::new(m.advance + 1.0, height),
                    );
                    if lone {
                        frame.stroke(
                            &rect,
                            Stroke::default().with_color(t.danger).with_width(1.0),
                        );
                    } else {
                        frame.fill(&rect, t.muted.scale_alpha(0.18));
                        frame.stroke(&rect, Stroke::default().with_color(t.muted).with_width(1.0));
                    }
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

/// What the canvas draws for the builder's text.
fn marks(b: &Builder) -> Vec<Mark> {
    let src = &b.source;
    let mut out = Vec::new();
    let mut wave = |start: usize, end: usize, error: bool| {
        let (l0, c0) = line_column(src, start);
        let (l1, c1) = line_column(src, end);
        for line in l0..=l1 {
            let from = if line == l0 { c0 } else { 0 };
            let to = if line == l1 {
                c1
            } else {
                src.split('\n').nth(line).map_or(0, |l| l.chars().count())
            };
            if to > from || (start == end && line == l0) {
                out.push(Mark::Wave {
                    line,
                    from,
                    to,
                    error,
                });
            }
        }
    };
    for w in &b.check.warnings {
        wave(w.start, w.end, false);
    }
    if let Some(e) = &b.check.error {
        wave(e.start, e.end, true);
    }
    if let Some(br) = b.bracket {
        let mut boxed = |at: usize, lone: bool| {
            let (line, column) = line_column(src, at);
            out.push(Mark::Box { line, column, lone });
        };
        boxed(br.at, br.partner.is_none());
        if let Some(p) = br.partner {
            boxed(p, false);
        }
    }
    out
}

/// The completion list: the entries' names in their colours, the other
/// name each was found by, and what each is.
fn list<'a>(c: &'a Completion, active: usize, theme_mode: Mode) -> Element<'a, Message> {
    let syntax = Syntax::of(theme_mode);
    let rows = c
        .items
        .iter()
        .enumerate()
        .fold(Column::new().spacing(1), |rows, (i, item)| {
            let color = match item.kind {
                kentos_expression::editor::Kind::Field => syntax.field,
                kentos_expression::editor::Kind::Variable => syntax.variable,
                kentos_expression::editor::Kind::Function => syntax.function,
                kentos_expression::editor::Kind::Keyword => syntax.keyword,
                kentos_expression::editor::Kind::Operator => syntax.operator,
            };
            let mut line = row![
                text(item.label.clone())
                    .font(typography::mono())
                    .size(typography::body())
                    .color(color)
            ]
            .spacing(8)
            .align_y(Center);
            if let Some(alias) = &item.alias {
                line = line.push(
                    container(label::caption(alias.clone()).style(style::text::muted))
                        .padding([0, 5])
                        .style(style::container::field_box),
                );
            }
            line = line.push(
                container(
                    label::caption(item.detail.clone())
                        .style(style::text::muted)
                        .wrapping(Wrapping::None),
                )
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .clip(true),
            );
            rows.push(
                button(line)
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::list_item(i == active))
                    .on_press(ev(Event::Accept(i))),
            )
        });
    // As tall as its rows, at most LIST_ROWS of them; then it scrolls.
    let most = LIST_ROWS as f32 * typography::scaled(26.0);
    container(
        scrollable(rows)
            .id(super::LIST)
            .direction(style::field::body_scrollbar())
            .height(Length::Shrink),
    )
    .max_height(most + 8.0)
    .width(Length::Fixed(typography::scaled(LIST_WIDTH)))
    .padding(4)
    .style(style::container::popover)
    .into()
}

/// The editor with its marks and list, filling its place.
pub(super) fn view(b: &Builder, mode: Mode) -> Element<'_, Message> {
    let m = Metrics::now();
    let list_open = b.completion.is_some();
    let area = iced::widget::responsive(move |room| {
        let (lines, longest) = size_of(&b.source);
        let text_size = m.extent(lines, longest);
        let size = Size::new(
            text_size.width.max(room.width),
            text_size.height.max(room.height),
        );
        let area = text_editor(&b.content)
            .id(EDITOR)
            .on_action(super::edit)
            .font(typography::mono())
            .size(m.size)
            .line_height(LineHeight::Absolute(m.line.into()))
            .padding(Padding::from([PAD_Y, PAD_X]))
            .wrapping(Wrapping::None)
            .width(size.width)
            .height(Length::Fixed(size.height))
            .highlight_with::<Painter>(b.lines.clone(), highlight::format)
            .key_binding(move |kp| keys(kp, list_open))
            .style(style::field::text_area);
        let over = canvas_widget(Marks {
            marks: marks(b),
            metrics: m,
        })
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height));
        let scroller = scrollable(Stack::with_children([area.into(), over.into()]))
            .id(SCROLLER)
            .direction(Direction::Both {
                vertical: Scrollbar::new().width(8).scroller_width(6),
                horizontal: Scrollbar::new().width(8).scroller_width(6),
            })
            .on_scroll(|v| {
                ev(Event::Scrolled(
                    v.absolute_offset().x,
                    v.absolute_offset().y,
                ))
            })
            .width(Fill)
            .height(Fill);
        let mut layers: Vec<Element<'_, Message>> = vec![scroller.into()];
        if let Some(c) = &b.completion {
            // Under the word being written (its line's middle, less the scroll); slid left as
            // far as it must to stay in the box, not flipped to the other side of the word.
            let (line, column) = line_column(&b.source, c.start);
            let at = m.cell(line, column) - Vector::new(b.scroll.0, b.scroll.1);
            let width = typography::scaled(LIST_WIDTH);
            let x = at.x.min(room.width - width - 8.0).max(8.0);
            let middle = Point::new(x, at.y + m.line / 2.0);
            layers.push(beside(
                list(c, b.active, mode),
                middle,
                Vector::new(0.0, m.line / 2.0),
            ));
        }
        Stack::with_children(layers).width(Fill).height(Fill).into()
    });
    // The box's size, for keeping the cursor in view as the text grows past it.
    iced::widget::sensor(area)
        .on_show(|s| ev(Event::Room(s.width, s.height)))
        .on_resize(|s| ev(Event::Room(s.width, s.height)))
        .into()
}
