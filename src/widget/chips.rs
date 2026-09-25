//! Etiket girişi: yazılan değerler kaldırılabilir etiketlere dönüşür.
//!
//! ```text
//! ┌───────────────────────────────────────────────┐
//! │ [park ×] [yeşil alan ×] [okul ×] kent▏  ⇥ kentsel dönüşüm │
//! └───────────────────────────────────────────────┘
//! ```
//!
//! - Enter ya da virgül yazılanı etikete çevirir; aynı etiket (büyük/küçük
//!   harf ve Türkçe harf ayırmadan) ikinci kez eklenmez.
//! - Boş alanda Backspace son etiketi siler; etiketin × düğmesi onu siler.
//! - Öneri verilmişse yazılanla başlayan ilki alanın sağında görünür; Tab
//!   onu tamamlar.
//! - Etiketler sığmayınca alt satıra geçer.
//!
//! ```ignore
//! ChipInput::new(&self.tags, Message::TagsChanged)
//!     .suggestions(["park", "okul", "kentsel dönüşüm"])
//!     .placeholder("Etiket ekleyin")
//! ```

use std::rc::Rc;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::widget::text::Wrapping;
use iced::widget::{TextInput, button, container, row, text, text_input};
use iced::{
    Background, Border, Center, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    mouse,
};

use crate::attribute::text as search;
use crate::icon::{Icon, icon};
use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Tokens, typography};
use crate::widget::dropdown::propagate;

/// Etiket satırının yüksekliği, iç boşluk ve aralık.
const LINE: f32 = 22.0;
const PAD: f32 = 4.0;
const GAP: f32 = 4.0;
/// Yazı alanının en az genişliği.
const FIELD: f32 = 90.0;

/// İçteki metin girişinin mesajları.
#[derive(Debug, Clone)]
enum Edit {
    Input(String),
    Submit,
}

type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Etiket girişi.
pub struct ChipInput<'a, Message> {
    chips: Vec<String>,
    on_change: Rc<dyn Fn(Vec<String>) -> Message + 'a>,
    suggestions: Vec<String>,
    placeholder: String,
    width: Length,
    /// Etiketler (× düğmeleriyle) ve yazı alanı.
    tags: Vec<Element<'a, Message>>,
    field: Element<'a, Edit>,
    hint: Element<'a, Edit>,
}

impl<'a, Message: Clone + 'a> ChipInput<'a, Message> {
    pub fn new(chips: &[String], on_change: impl Fn(Vec<String>) -> Message + 'a) -> Self {
        let on_change: Rc<dyn Fn(Vec<String>) -> Message + 'a> = Rc::new(on_change);

        // Her etiketin × düğmesi onsuz listeyi bildirir.
        let tags = chips
            .iter()
            .enumerate()
            .map(|(index, chip)| {
                let mut rest = chips.to_vec();
                rest.remove(index);

                container(
                    row![
                        text(chip.clone())
                            .font(typography::ui())
                            .size(typography::caption())
                            .wrapping(Wrapping::None),
                        button(icon(Icon::Close).size(8.0))
                            .on_press((on_change)(rest))
                            .padding(3)
                            .style(style::button::subtle),
                    ]
                    .spacing(2)
                    .align_y(Center),
                )
                .padding([0, 2])
                .height(typography::scaled(LINE) - 2.0)
                .align_y(Center)
                .style(chip_style)
                .into()
            })
            .collect();

        Self {
            chips: chips.to_vec(),
            on_change,
            suggestions: Vec::new(),
            placeholder: String::new(),
            width: Length::Fill,
            tags,
            field: field("", "").into(),
            hint: text("").into(),
        }
    }

    /// Tamamlanabilecek değerler.
    pub fn suggestions<S: Into<String>>(
        mut self,
        suggestions: impl IntoIterator<Item = S>,
    ) -> Self {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    /// Etiket yokken yazı alanında gösterilen metin.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Yazılanla başlayan, henüz eklenmemiş ilk öneri.
    fn suggestion(&self, typed: &str) -> Option<&str> {
        let typed = search::fold(typed.trim());

        if typed.is_empty() {
            return None;
        }

        self.suggestions
            .iter()
            .find(|suggestion| {
                let folded = search::fold(suggestion);
                folded.starts_with(&typed) && folded != typed && !self.contains(suggestion)
            })
            .map(String::as_str)
    }

    fn contains(&self, chip: &str) -> bool {
        let folded = search::fold(chip.trim());
        self.chips
            .iter()
            .any(|existing| search::fold(existing) == folded)
    }

    /// Yazılanı etiket olarak ekler; boşsa ya da varsa eklenmez.
    fn commit(&self, typed: &str) -> Option<Message> {
        let typed = typed.trim();

        (!typed.is_empty() && !self.contains(typed)).then(|| {
            let mut chips = self.chips.clone();
            chips.push(typed.to_owned());
            (self.on_change)(chips)
        })
    }

    /// Yazı alanını ve öneriyi duruma göre kurar.
    fn build(&mut self, state: &State) {
        let placeholder = if self.chips.is_empty() {
            self.placeholder.as_str()
        } else {
            ""
        };

        self.field = field(placeholder, &state.buffer).into();
        self.hint = match self.suggestion(&state.buffer) {
            Some(suggestion) => text(format!("⇥ {suggestion}"))
                .font(typography::ui())
                .size(typography::caption())
                .wrapping(Wrapping::None)
                .style(style::text::muted)
                .into(),
            None => text("").into(),
        };
    }
}

fn field<'a>(placeholder: &str, content: &str) -> TextInput<'a, Edit, Theme, Renderer> {
    text_input(placeholder, content)
        .on_input(Edit::Input)
        .on_submit(Edit::Submit)
        .font(typography::ui())
        .size(typography::body())
        .padding([2, 4])
        .style(style::field::bare_input)
}

/// Etiketin zemini.
fn chip_style(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);

    container::Style {
        background: Some(Background::Color(t.surface_alt)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: 3.0.into(),
        },
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

#[derive(Debug, Default)]
struct State {
    buffer: String,
    hovered: bool,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for ChipInput<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        [Tree::new(&self.field), Tree::new(&self.hint)]
            .into_iter()
            .chain(self.tags.iter().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children.resize_with(2 + self.tags.len(), Tree::empty);
        tree.children[0].diff(&self.field);
        tree.children[1].diff(&self.hint);

        for (child, tag) in tree.children[2..].iter_mut().zip(&self.tags) {
            child.diff(tag);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.build(tree.state.downcast_ref::<State>());

        let line = typography::scaled(LINE);
        let width = limits.resolve(self.width, Length::Shrink, Size::ZERO).width;
        let inner = (width - 2.0 * PAD).max(0.0);
        let (fields, tags) = tree.children.split_at_mut(2);

        let mut x = PAD;
        let mut y = PAD;
        let mut nodes = Vec::with_capacity(self.tags.len());

        // Etiketler soldan sağa, sığmayan alt satıra.
        for (tag, tree) in self.tags.iter_mut().zip(tags.iter_mut()) {
            let node = tag.as_widget_mut().layout(
                tree,
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(inner, line)),
            );
            let size = node.size();

            if x > PAD && x + size.width > PAD + inner {
                x = PAD;
                y += line + GAP;
            }

            nodes.push(node.move_to(Point::new(x, y + ((line - size.height) / 2.0).round())));
            x += size.width + GAP;
        }

        // Yazı alanı satırın kalanını alır; çok darsa alt satıra geçer.
        let minimum = typography::scaled(FIELD);

        if x > PAD && PAD + inner - x < minimum {
            x = PAD;
            y += line + GAP;
        }

        let hint = self.hint.as_widget_mut().layout(
            &mut fields[1],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(inner / 2.0, line)),
        );
        let hint_width = hint.size().width;
        let field_width = (PAD + inner - x - hint_width - if hint_width > 0.0 { GAP } else { 0.0 })
            .max(minimum / 2.0);

        let field = self.field.as_widget_mut().layout(
            &mut fields[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(field_width, line)),
        );
        let field_height = field.size().height;
        let field = field.move_to(Point::new(x, y + ((line - field_height) / 2.0).round()));
        let hint_height = hint.size().height;
        let hint = hint.move_to(Point::new(
            PAD + inner - hint_width,
            y + ((line - hint_height) / 2.0).round(),
        ));

        let height = y + line + PAD;

        Node::with_children(
            Size::new(width, height),
            [field, hint].into_iter().chain(nodes).collect(),
        )
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
        let bounds = layout.bounds();
        let mut layouts = layout.children();
        let (Some(field_layout), Some(_)) = (layouts.next(), layouts.next()) else {
            return;
        };
        let tag_layouts: Vec<Layout<'_>> = layouts.collect();

        // Etiketlerin × düğmeleri.
        for ((tag, tree), tag_layout) in self
            .tags
            .iter_mut()
            .zip(tree.children[2..].iter_mut())
            .zip(&tag_layouts)
        {
            tag.as_widget_mut().update(
                tree,
                event,
                *tag_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if shell.is_event_captured() {
                return;
            }
        }

        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let focused = input_state(&children[0]).is_focused();

        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            let hovered = cursor.is_over(bounds);

            if hovered != state.hovered {
                state.hovered = hovered;
                shell.request_redraw();
            }
        }

        if focused && let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event {
            match key {
                // Boş alanda Backspace son etiketi siler.
                keyboard::Key::Named(key::Named::Backspace)
                    if state.buffer.is_empty() && !self.chips.is_empty() =>
                {
                    let mut chips = self.chips.clone();
                    chips.pop();
                    shell.publish((self.on_change)(chips));
                    shell.capture_event();
                    return;
                }
                // Tab öneriyi tamamlar.
                keyboard::Key::Named(key::Named::Tab) => {
                    if let Some(suggestion) = self.suggestion(&state.buffer).map(str::to_owned) {
                        state.buffer = suggestion;
                        input_state_mut(&mut children[0]).move_cursor_to_end();
                        self.build(state);
                        shell.invalidate_layout();
                        shell.capture_event();
                        return;
                    }
                }
                _ => {}
            }
        }

        let mut edits = Vec::new();
        let mut local = Shell::new(&mut edits);

        self.field.as_widget_mut().update(
            &mut children[0],
            event,
            field_layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );

        propagate(&local, shell);
        drop(local);

        for edit in edits {
            match edit {
                // Virgül yazılanı etikete çevirir; ardından gelen yazı kalır.
                Edit::Input(input) => match input.rsplit_once(',') {
                    Some((done, rest)) => {
                        let mut chips = self.chips.clone();

                        for part in done.split(',') {
                            let part = part.trim();

                            if !part.is_empty()
                                && !chips
                                    .iter()
                                    .any(|chip| search::fold(chip) == search::fold(part))
                            {
                                chips.push(part.to_owned());
                            }
                        }

                        if chips != self.chips {
                            shell.publish((self.on_change)(chips));
                        }

                        state.buffer = rest.trim_start().to_owned();
                    }
                    None => state.buffer = input,
                },
                Edit::Submit => {
                    if let Some(message) = self.commit(&state.buffer) {
                        shell.publish(message);
                    }

                    state.buffer.clear();
                }
            }

            self.build(state);
            shell.invalidate_layout();
        }

        // Alanın boş yerine tıklamak yazı alanına odaklanır.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && !shell.is_event_captured()
            && cursor.is_over(bounds)
        {
            let input = input_state_mut(&mut children[0]);

            if !input.is_focused() {
                input.focus();
                shell.request_redraw();
            }

            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut layouts = layout.children();
        let _field = layouts.next();
        let _hint = layouts.next();

        for ((tag, tree), tag_layout) in self.tags.iter().zip(&tree.children[2..]).zip(layouts) {
            let interaction = tag
                .as_widget()
                .mouse_interaction(tree, tag_layout, cursor, viewport, renderer);

            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }

        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let focused = input_state(&tree.children[0]).is_focused();

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: if focused {
                        t.accent
                    } else if state.hovered {
                        t.muted
                    } else {
                        t.border
                    },
                    width: 1.0,
                    radius: RADIUS.into(),
                },
                ..Quad::default()
            },
            Background::Color(t.field),
        );

        let mut layouts = layout.children();
        let (Some(field), Some(hint)) = (layouts.next(), layouts.next()) else {
            return;
        };

        for ((tag, tree), tag_layout) in self.tags.iter().zip(&tree.children[2..]).zip(layouts) {
            tag.as_widget().draw(
                tree,
                renderer,
                theme,
                &renderer::Style { text_color: t.text },
                tag_layout,
                cursor,
                viewport,
            );
        }

        self.field.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style { text_color: t.text },
            field,
            cursor,
            &bounds,
        );
        self.hint.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            &renderer::Style {
                text_color: t.muted,
            },
            hint,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            if let Some(field) = layout.children().next() {
                self.field.as_widget_mut().operate(
                    &mut tree.children[0],
                    field,
                    renderer,
                    operation,
                );
            }
        });
    }
}

impl<'a, Message: Clone + 'a> From<ChipInput<'a, Message>> for Element<'a, Message> {
    fn from(input: ChipInput<'a, Message>) -> Self {
        Element::new(input)
    }
}

fn input_state(tree: &Tree) -> &text_input::State<Paragraph> {
    tree.state.downcast_ref::<text_input::State<Paragraph>>()
}

fn input_state_mut(tree: &mut Tree) -> &mut text_input::State<Paragraph> {
    tree.state.downcast_mut::<text_input::State<Paragraph>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(chips: &[&str]) -> ChipInput<'static, Vec<String>> {
        let chips: Vec<String> = chips.iter().map(|chip| (*chip).to_owned()).collect();

        ChipInput::new(&chips, |chips| chips).suggestions(["park", "Kentsel dönüşüm", "okul"])
    }

    #[test]
    fn chips_are_added_once_and_completed() {
        let chips = input(&["Park"]);

        assert_eq!(
            chips.commit("  okul "),
            Some(vec!["Park".to_owned(), "okul".to_owned()])
        );
        assert_eq!(chips.commit("PARK"), None);
        assert_eq!(chips.commit("   "), None);

        // Öneri: yazılanla başlayan, eklenmemiş ilk değer; Türkçe harf ayırmaz.
        assert_eq!(chips.suggestion("KENT"), Some("Kentsel dönüşüm"));
        assert_eq!(chips.suggestion("pa"), None);
        assert_eq!(chips.suggestion(""), None);
    }
}

/// Gerçek olaylarla: Enter, virgül, Tab ile tamamlama ve Backspace.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::widget::container;
    use iced::{Element, Point, Size};

    use super::ChipInput;
    use crate::snapshot::{Input, Snapshot};

    // Görüntü alma durumu doğrudan listedir; imza durumun türünü izler.
    #[allow(clippy::ptr_arg)]
    fn view(chips: &Vec<String>) -> Element<'_, Vec<String>> {
        container(ChipInput::new(chips, |chips| chips).suggestions(["kentsel dönüşüm"]))
            .padding(10)
            .into()
    }

    #[test]
    fn chips_are_typed_completed_and_removed() {
        let mut snapshot = Snapshot::new(Size::new(500.0, 80.0)).expect("çizici kurulamadı");
        let mut chips: Vec<String> = vec!["park".to_owned()];
        let mut update = |chips: &mut Vec<String>, next: Vec<String>| *chips = next;
        let mut input =
            |chips: &mut Vec<String>, input| snapshot.input(chips, view, &mut update, input);

        // Alanın boş yerine tıklamak yazı alanına odaklanır.
        input(&mut chips, Input::Click(Point::new(400.0, 20.0)));
        input(&mut chips, Input::Type("okul".to_owned()));
        input(&mut chips, Input::Key(Named::Enter));
        assert_eq!(chips, ["park", "okul"]);

        // Virgül ekler; aynısı ikinci kez eklenmez.
        input(&mut chips, Input::Type("PARK, çarşı,".to_owned()));
        assert_eq!(chips, ["park", "okul", "çarşı"]);

        // Tab öneriyi tamamlar, Enter ekler.
        input(&mut chips, Input::Type("kent".to_owned()));
        input(&mut chips, Input::Key(Named::Tab));
        input(&mut chips, Input::Key(Named::Enter));
        assert_eq!(chips, ["park", "okul", "çarşı", "kentsel dönüşüm"]);

        // Boş alanda Backspace son etiketi siler.
        input(&mut chips, Input::Key(Named::Backspace));
        assert_eq!(chips, ["park", "okul", "çarşı"]);
    }
}
