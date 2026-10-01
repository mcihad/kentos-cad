use std::rc::Rc;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self as advanced_text, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, input_method::InputMethod, overlay};
use iced::keyboard::{Key, key::Named};
use iced::time::Instant;
use iced::widget::text_editor::{self, Action, Binding, Edit, KeyPress, Status};
use iced::widget::{TextEditor, button, column, container, row, space, text};
use iced::{
    Border, Element, Event, Fill, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse, window,
};

use crate::icon::{Icon, icon};
use crate::style;
use crate::theme::{Tokens, motion, typography};

use super::surface::{Surface, focused, status_color};
use super::{CompletionEvent, CompletionState, completion_menu};
use super::{RunStatus, font, strong_font, syntax};

type KeyBinding<'a, Message> = Box<dyn Fn(KeyPress) -> Option<Binding<Message>> + 'a>;

/// A Python source editor with a synchronized gutter, smoothly moving active
/// line, optional header and footer, and focus/execution lighting.
pub struct PythonEditor<'a, Message> {
    content: &'a text_editor::Content,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
    title: Option<&'a str>,
    modified: bool,
    status: RunStatus,
    revision: u64,
    height: Length,
    size: f32,
    footer: bool,
    numbers: bool,
    prompt: bool,
    placeholder: &'a str,
    run: Option<Message>,
    save: Option<Message>,
    undo: Option<Message>,
    redo: Option<Message>,
    binding: Option<KeyBinding<'a, Message>>,
    completions: Option<(&'a CompletionState, completion_menu::Events<'a, Message>)>,
}

impl<'a, Message: Clone + 'a> PythonEditor<'a, Message> {
    pub fn new(
        content: &'a text_editor::Content,
        on_action: impl Fn(Action) -> Message + 'a,
    ) -> Self {
        Self {
            content,
            on_action: Box::new(on_action),
            title: Some("Adsız.py"),
            modified: false,
            status: RunStatus::Ready,
            revision: 0,
            height: Fill,
            size: typography::body() + 1.0,
            footer: true,
            numbers: true,
            prompt: false,
            placeholder: "# Python kodunuzu yazın",
            run: None,
            save: None,
            undo: None,
            redo: None,
            binding: None,
            completions: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
    pub fn header(mut self, visible: bool) -> Self {
        if !visible {
            self.title = None;
        }
        self
    }
    pub fn footer(mut self, visible: bool) -> Self {
        self.footer = visible;
        self
    }
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }
    pub fn status(mut self, status: RunStatus) -> Self {
        self.status = status;
        self
    }
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(8.0);
        self
    }
    pub fn line_numbers(mut self, visible: bool) -> Self {
        self.numbers = visible;
        self
    }
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
    pub fn on_run(mut self, message: Message) -> Self {
        self.run = Some(message);
        self
    }
    pub fn on_save(mut self, message: Message) -> Self {
        self.save = Some(message);
        self
    }
    pub fn on_undo(mut self, message: Message) -> Self {
        self.undo = Some(message);
        self
    }
    pub fn on_redo(mut self, message: Message) -> Self {
        self.redo = Some(message);
        self
    }
    /// Replaces the default key map; default Iced bindings can be obtained
    /// with `Binding::from_key_press`. This preserves host completion APIs.
    pub fn key_binding(
        mut self,
        binding: impl Fn(KeyPress) -> Option<Binding<Message>> + 'a,
    ) -> Self {
        self.binding = Some(Box::new(binding));
        self
    }
    /// Render the controller's suggestions. The host may enrich them from a
    /// real interpreter, language server or another completion provider.
    pub fn completions(
        mut self,
        state: &'a CompletionState,
        on_event: impl Fn(CompletionEvent) -> Message + 'a,
    ) -> Self {
        self.completions = Some((state, Rc::new(on_event)));
        self
    }
    pub(super) fn prompt(mut self) -> Self {
        self.prompt = true;
        self.numbers = false;
        self
    }

    fn view(self) -> Element<'a, Message> {
        let run_key = self.run.clone();
        let save_key = self.save.clone();
        let undo_key = self.undo.clone();
        let redo_key = self.redo.clone();
        let custom = self.binding;
        let completion_keys = self.completions.clone();
        let child = TextEditor::new(self.content)
            .font(font())
            .size(self.size)
            .line_height(advanced_text::LineHeight::Relative(1.65))
            .wrapping(advanced_text::Wrapping::None)
            .padding(Padding::from([12, 12]))
            .height(Fill)
            .placeholder(self.placeholder)
            .on_action(Raw::Edit)
            .highlight_with::<syntax::Python>((), syntax::format)
            .key_binding(move |kp| {
                if matches!(kp.status, Status::Focused { .. })
                    && let Some((state, events)) = &completion_keys
                {
                    let key = kp.key.as_ref();
                    let event = if kp.modifiers.control() && key == Key::Named(Named::Space) {
                        Some(CompletionEvent::Request)
                    } else if state.open && !kp.modifiers.command() && !kp.modifiers.alt() {
                        match key {
                            Key::Named(Named::Escape) => Some(CompletionEvent::Dismiss),
                            Key::Named(Named::ArrowDown) => Some(CompletionEvent::Move(1)),
                            Key::Named(Named::ArrowUp) => Some(CompletionEvent::Move(-1)),
                            Key::Named(Named::Tab | Named::Enter)
                                if !state.items.is_empty() && !kp.modifiers.shift() =>
                            {
                                Some(CompletionEvent::Accept)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some(event) = event {
                        return Some(Binding::Custom(Raw::Custom(events(event))));
                    }
                }
                if let Some(binding) = &custom {
                    return binding(kp).map(map_binding);
                }
                if !matches!(kp.status, Status::Focused { .. }) {
                    return None;
                }
                let key = kp.key.as_ref();
                let command = kp.modifiers.command();
                if (key == Key::Named(Named::F5) || (command && key == Key::Named(Named::Enter)))
                    && let Some(message) = &run_key
                {
                    return Some(Binding::Custom(Raw::Custom(message.clone())));
                }
                if command && let Key::Character(ch) = key {
                    let message = if ch.eq_ignore_ascii_case("s") {
                        save_key.as_ref()
                    } else if ch.eq_ignore_ascii_case("z") {
                        if kp.modifiers.shift() {
                            redo_key.as_ref()
                        } else {
                            undo_key.as_ref()
                        }
                    } else if ch.eq_ignore_ascii_case("y") {
                        redo_key.as_ref()
                    } else {
                        None
                    };
                    if let Some(message) = message {
                        return Some(Binding::Custom(Raw::Custom(message.clone())));
                    }
                }
                if key == Key::Named(Named::Tab) && !command && !kp.modifiers.alt() {
                    return Some(Binding::Custom(Raw::Edit(Action::Edit(
                        if kp.modifiers.shift() {
                            Edit::Unindent
                        } else {
                            Edit::Indent
                        },
                    ))));
                }
                Binding::from_key_press(kp)
            })
            .style(|theme: &Theme, _| {
                let t = Tokens::of(theme);
                text_editor::Style {
                    background: iced::Color::TRANSPARENT.into(),
                    border: Border::default(),
                    placeholder: t.muted,
                    value: t.text,
                    selection: t.accent.scale_alpha(0.26),
                }
            });
        let body = Body {
            child,
            content: self.content,
            on_action: self.on_action,
            size: self.size,
            numbers: self.numbers,
            prompt: self.prompt,
            completions: self.completions,
            menu: None,
        };
        let mut stack = column![];
        if let Some(title) = self.title {
            let mut header = row![
                icon(Icon::Document).size(16.0),
                text(title).font(strong_font()).size(typography::body()),
            ]
            .spacing(9)
            .align_y(iced::Center);
            if self.modified {
                header = header.push(text("●").size(9).style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).warning),
                }));
            }
            header = header.push(space::horizontal());
            if let Some(save) = self.save {
                header = header.push(
                    button(text("Kaydet").font(font()).size(typography::caption()))
                        .on_press(save)
                        .padding([5, 9])
                        .style(style::button::flat),
                );
            }
            if let Some(run) = self.run {
                header = header.push(
                    button(
                        row![
                            icon(Icon::Play).size(12.0),
                            text("Çalıştır").font(font()).size(typography::caption())
                        ]
                        .spacing(6)
                        .align_y(iced::Center),
                    )
                    .on_press(run)
                    .padding([5, 10])
                    .style(style::button::secondary),
                );
            }
            stack = stack.push(container(header).width(Fill).padding([9, 14]));
            stack = stack.push(super::super::horizontal_divider());
        }
        stack = stack.push(Element::new(body));
        if self.footer {
            let position = self.content.cursor().position;
            let character_column = self.content.line(position.line).map_or(0, |line| {
                line.text
                    .get(..position.column)
                    .unwrap_or(&line.text)
                    .chars()
                    .count()
            });
            let status = self.status;
            let footer = row![
                text(format!("●  {}", status.label()))
                    .font(font())
                    .size(typography::caption())
                    .style(move |theme: &Theme| text::Style {
                        color: Some(status_color(status, &Tokens::of(theme)))
                    }),
                space::horizontal(),
                text(format!(
                    "Satır {}, Sütun {}",
                    position.line + 1,
                    character_column + 1
                ))
                .font(font())
                .size(typography::caption()),
                text("UTF-8").font(font()).size(typography::caption()),
                text("Python")
                    .font(strong_font())
                    .size(typography::caption()),
            ]
            .spacing(16)
            .align_y(iced::Center);
            stack = stack
                .push(super::super::horizontal_divider())
                .push(container(footer).padding([7, 14]).width(Fill));
        }
        Surface::new(
            container(stack).height(self.height).width(Fill).padding(1),
            self.status,
            self.revision,
        )
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<PythonEditor<'a, Message>> for Element<'a, Message> {
    fn from(editor: PythonEditor<'a, Message>) -> Self {
        editor.view()
    }
}

enum Raw<Message> {
    Edit(Action),
    Custom(Message),
}

fn map_binding<Message>(binding: Binding<Message>) -> Binding<Raw<Message>> {
    match binding {
        Binding::Unfocus => Binding::Unfocus,
        Binding::Copy => Binding::Copy,
        Binding::Cut => Binding::Cut,
        Binding::Paste => Binding::Paste,
        Binding::Move(m) => Binding::Move(m),
        Binding::Select(m) => Binding::Select(m),
        Binding::SelectWord => Binding::SelectWord,
        Binding::SelectLine => Binding::SelectLine,
        Binding::SelectAll => Binding::SelectAll,
        Binding::Insert(c) => Binding::Insert(c),
        Binding::Enter => Binding::Enter,
        Binding::Backspace => Binding::Backspace,
        Binding::Delete => Binding::Delete,
        Binding::Sequence(bindings) => {
            Binding::Sequence(bindings.into_iter().map(map_binding).collect())
        }
        Binding::Custom(message) => Binding::Custom(Raw::Custom(message)),
    }
}

struct Body<'a, Message> {
    child: TextEditor<'a, syntax::Python, Raw<Message>>,
    content: &'a text_editor::Content,
    on_action: Box<dyn Fn(Action) -> Message + 'a>,
    size: f32,
    numbers: bool,
    prompt: bool,
    completions: Option<(&'a CompletionState, completion_menu::Events<'a, Message>)>,
    menu: Option<Element<'a, Message>>,
}

struct BodyState {
    top: f32,
    horizontal: f32,
    caret: text_editor::Cursor,
    reveal: bool,
    shift: bool,
    dragging: bool,
    active_from: f32,
    active: usize,
    changed: Instant,
    now: Instant,
    anchor: Rectangle,
}

impl BodyState {
    fn line(&self) -> f32 {
        let p = motion::progress(self.changed, self.now, Duration::from_millis(150));
        self.active_from + (self.active as f32 - self.active_from) * p
    }
}

impl<Message> Body<'_, Message> {
    fn gutter(&self) -> f32 {
        if self.prompt {
            self.size * 3.0 + 18.0
        } else if self.numbers {
            self.size
                * 0.6
                * self
                    .content
                    .line_count()
                    .max(1)
                    .ilog10()
                    .saturating_add(1)
                    .max(2) as f32
                + 28.0
        } else {
            6.0
        }
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Body<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BodyState>()
    }
    fn state(&self) -> tree::State {
        let active = self.content.cursor().position.line;
        tree::State::new(BodyState {
            top: 0.0,
            horizontal: 0.0,
            caret: self.content.cursor(),
            reveal: true,
            shift: false,
            dragging: false,
            active_from: active as f32,
            active,
            changed: Instant::now(),
            now: Instant::now(),
            anchor: Rectangle::default(),
        })
    }
    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.child as &dyn Widget<Raw<Message>, Theme, Renderer>),
            Tree::empty(),
        ]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.child as &dyn Widget<Raw<Message>, Theme, Renderer>);
        let active = self.content.cursor().position.line;
        let state = tree.state.downcast_mut::<BodyState>();
        if state.caret != self.content.cursor() {
            state.caret = self.content.cursor();
            state.reveal = true;
        }
        if state.active != active {
            state.active_from = state.line();
            state.active = active;
            state.changed = Instant::now();
        }
    }
    fn size(&self) -> Size<Length> {
        Size::new(Fill, Fill)
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let bounds = limits.resolve(Fill, Fill, Size::ZERO);
        let gutter = self.gutter().min(bounds.width);
        let code_width = (bounds.width - gutter).max(0.0);
        let longest = self
            .content
            .lines()
            .map(|line| {
                line.text
                    .chars()
                    .map(|ch| if ch == '\t' { 4 } else { 1 })
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0) as f32
            * self.size
            * 0.6
            + 24.0;
        let width = code_width.max(longest);
        let state = tree.state.downcast_mut::<BodyState>();
        state.horizontal = state.horizontal.clamp(0.0, (width - code_width).max(0.0));
        let height = (bounds.height - if width > code_width { 8.0 } else { 0.0 }).max(0.0);
        let child = self
            .child
            .layout(
                &mut tree.children[0],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(width, height)),
            )
            .move_to(Point::new(gutter - state.horizontal, 0.0));
        layout::Node::with_children(bounds, vec![child])
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        let code = Rectangle {
            x: layout.bounds().x + self.gutter(),
            width: (layout.bounds().width - self.gutter()).max(0.0),
            height: child.bounds().height,
            ..layout.bounds()
        };
        let max_horizontal = (child.bounds().width - code.width).max(0.0);
        let state = tree.state.downcast_mut::<BodyState>();
        if let Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.shift = modifiers.shift();
        }
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(layout.bounds())
        {
            let (x, y, unit) = match delta {
                mouse::ScrollDelta::Lines { x, y } => (*x, *y, self.size * 3.0),
                mouse::ScrollDelta::Pixels { x, y } => (*x, *y, 1.0),
            };
            if (x != 0.0 || state.shift) && max_horizontal > 0.0 {
                state.horizontal = (state.horizontal - if state.shift { y } else { x } * unit)
                    .clamp(0.0, max_horizontal);
                state.reveal = false;
                shell.invalidate_layout();
                shell.request_redraw();
                shell.capture_event();
                return;
            }
        }
        let track = Rectangle {
            x: code.x,
            y: layout.bounds().y + layout.bounds().height - 8.0,
            width: code.width,
            height: 8.0,
        };
        if max_horizontal > 0.0 {
            if matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            ) && cursor.is_over(track)
            {
                state.dragging = true;
            }
            if matches!(
                event,
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            ) && state.dragging
            {
                state.dragging = false;
                shell.capture_event();
                return;
            }
            if state.dragging
                && let Some(position) = cursor.position()
            {
                let thumb = (code.width * code.width / child.bounds().width)
                    .max(28.0)
                    .min(code.width);
                let ratio = (position.x - track.x - thumb / 2.0) / (track.width - thumb).max(1.0);
                state.horizontal = (ratio * max_horizontal).clamp(0.0, max_horizontal);
                state.reveal = false;
                shell.invalidate_layout();
                shell.request_redraw();
                shell.capture_event();
                return;
            }
        }
        let mut messages = Vec::new();
        let mut child_shell = Shell::new(&mut messages);
        let child_cursor = if cursor.position().is_some_and(|point| {
            point.x < code.x && point.y >= code.y && point.y <= code.y + code.height
        }) {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };
        self.child.update(
            &mut tree.children[0],
            event,
            child,
            child_cursor,
            renderer,
            clipboard,
            &mut child_shell,
            &code.intersection(viewport).unwrap_or_default(),
        );
        let state = tree.state.downcast_mut::<BodyState>();
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = *now;
        }
        // The input method cursor is the editor's actual laid-out coordinate,
        // including keyboard scrolling and automatic reveal. This keeps the
        // gutter aligned with the source without access to private Iced fields.
        if let InputMethod::Enabled { cursor: caret, .. } = child_shell.input_method() {
            state.anchor = *caret;
            let cursor = self.content.cursor();
            let first = cursor
                .selection
                .map_or(cursor.position.line, |s| s.line.min(cursor.position.line));
            state.top =
                (first as f32 - (caret.y - child.bounds().y - 12.0) / (self.size * 1.65)).max(0.0);
            if state.reveal && self.content.cursor().selection.is_none() {
                let shift = if caret.x > code.x + code.width - 16.0 {
                    caret.x - (code.x + code.width - 16.0)
                } else if caret.x < code.x + 12.0 {
                    caret.x - (code.x + 12.0)
                } else {
                    0.0
                };
                let horizontal = (state.horizontal + shift).clamp(0.0, max_horizontal);
                if horizontal != state.horizontal {
                    state.horizontal = horizontal;
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                state.reveal = false;
            }
        }
        let max_top = (self.content.line_count() as f32
            - (child.bounds().height - 24.0) / (self.size * 1.65))
            .max(0.0)
            .ceil();
        let top = std::cell::Cell::new(state.top);
        let reveal = std::cell::Cell::new(state.reveal);
        shell.merge(child_shell, |raw| match raw {
            Raw::Edit(action) => {
                if action.is_edit() {
                    reveal.set(true);
                }
                if let Action::Scroll { lines } = &action {
                    top.set((top.get() + *lines as f32).clamp(0.0, max_top));
                }
                (self.on_action)(action)
            }
            Raw::Custom(message) => message,
        });
        state.top = top.get();
        state.reveal = reveal.get();
        if motion::running(state.changed, state.now, Duration::from_millis(150))
            && layout.bounds().intersection(viewport).is_some()
        {
            shell.request_redraw_at(state.now + Duration::from_millis(8));
        }
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };
        let Some(child) = layout.children().next() else {
            return;
        };
        let state = tree.state.downcast_ref::<BodyState>();
        let bounds = layout.bounds();
        let t = Tokens::of(theme);
        let line_height = self.size * 1.65;
        let gutter = self.gutter();
        let is_focused = focused(&tree.children[0]);
        renderer.with_layer(clip, |renderer| {
            if self.numbers || self.prompt {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            width: gutter,
                            ..bounds
                        },
                        ..Quad::default()
                    },
                    t.surface.scale_alpha(0.35),
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            x: bounds.x + gutter - 1.0,
                            width: 1.0,
                            ..bounds
                        },
                        ..Quad::default()
                    },
                    t.border.scale_alpha(0.6),
                );
            }
            let active_y = bounds.y + 12.0 + (state.line() - state.top) * line_height;
            if is_focused {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            y: active_y,
                            height: line_height,
                            ..bounds
                        },
                        ..Quad::default()
                    },
                    t.accent.scale_alpha(if t.is_dark { 0.07 } else { 0.045 }),
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            x: bounds.x + 1.0,
                            y: active_y + 2.0,
                            width: 2.0,
                            height: line_height - 4.0,
                        },
                        border: Border {
                            radius: 1.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    t.accent,
                );
            }
            let code = Rectangle {
                x: bounds.x + gutter,
                width: (bounds.width - gutter).max(0.0),
                height: child.bounds().height,
                ..bounds
            };
            if let Some(code_clip) = code.intersection(&clip) {
                renderer.with_layer(code_clip, |renderer| {
                    self.child.draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        child,
                        cursor,
                        &code_clip,
                    );
                    if is_focused
                        && let Some((completion, _)) = &self.completions
                        && completion.open
                        && let Some(item) = completion.selected()
                        && let Some(suffix) = item.insert.strip_prefix(&completion.prefix)
                        && self
                            .content
                            .line(self.content.cursor().position.line)
                            .is_some_and(|line| {
                                line.text
                                    .get(self.content.cursor().position.column..)
                                    .is_some_and(|after| after.trim().is_empty())
                            })
                    {
                        renderer.fill_text(
                            advanced_text::Text {
                                content: suffix.to_owned(),
                                bounds: Size::new(code.width, line_height),
                                size: self.size.into(),
                                line_height: advanced_text::LineHeight::Relative(1.65),
                                font: font(),
                                align_x: advanced_text::Alignment::Default,
                                align_y: iced::alignment::Vertical::Top,
                                shaping: advanced_text::Shaping::Advanced,
                                wrapping: advanced_text::Wrapping::None,
                            },
                            Point::new(state.anchor.x + 1.0, state.anchor.y),
                            t.muted.scale_alpha(0.45),
                            code_clip,
                        );
                    }
                });
            }
            if child.bounds().width > code.width && code.width > 0.0 {
                let width = (code.width * code.width / child.bounds().width)
                    .max(28.0)
                    .min(code.width);
                let offset =
                    state.horizontal / (child.bounds().width - code.width) * (code.width - width);
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            x: code.x + offset,
                            y: bounds.y + bounds.height - 6.0,
                            width,
                            height: 3.0,
                        },
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    t.muted.scale_alpha(if state.dragging { 0.7 } else { 0.35 }),
                );
            }
            let first = state.top.floor() as usize;
            let count = ((bounds.height / line_height).ceil() as usize + 2)
                .min(self.content.line_count().saturating_sub(first));
            if self.numbers || self.prompt {
                for line in first..first + count {
                    let words = if self.prompt {
                        if line == 0 {
                            ">>>".to_owned()
                        } else {
                            "...".to_owned()
                        }
                    } else {
                        (line + 1).to_string()
                    };
                    renderer.fill_text(
                        advanced_text::Text {
                            content: words,
                            bounds: Size::new(gutter - 12.0, line_height),
                            size: self.size.into(),
                            line_height: advanced_text::LineHeight::Relative(1.65),
                            font: if line == state.active {
                                strong_font()
                            } else {
                                font()
                            },
                            align_x: advanced_text::Alignment::Right,
                            align_y: iced::alignment::Vertical::Top,
                            shaping: advanced_text::Shaping::Advanced,
                            wrapping: advanced_text::Wrapping::None,
                        },
                        Point::new(
                            bounds.x + gutter - 12.0,
                            bounds.y + 12.0 + (line as f32 - state.top) * line_height,
                        ),
                        if self.prompt || (line == state.active && is_focused) {
                            t.accent_hover
                        } else {
                            t.muted.scale_alpha(0.65)
                        },
                        clip,
                    );
                }
            }
        });
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::default(), |child| {
                self.child
                    .mouse_interaction(&tree.children[0], child, cursor, viewport, renderer)
            })
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.child
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let (state, events) = self.completions.as_ref()?;
        if !state.open || !focused(&tree.children[0]) {
            self.menu = None;
            return None;
        }
        let menu = self
            .menu
            .insert(completion_menu::view(state, events.clone()));
        tree.children[1].diff(&*menu);
        Some(overlay::Element::new(Box::new(completion_menu::Menu {
            content: menu,
            tree: &mut tree.children[1],
            anchor: tree.state.downcast_ref::<BodyState>().anchor + translation,
            events: events.clone(),
        })))
    }
}
