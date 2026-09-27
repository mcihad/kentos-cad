//! Öneri alanı: yazdıkça altında uygulamanın bulduğu önerilerin açıldığı
//! form alanı (ör. Paylaş penceresinin “Kişi ekle”si; web'in
//! `ui/cloud/shareFind.ts`'i).
//!
//! ```text
//! ┌──────────────────────────────────┐
//! │ ay▏                              │
//! └──────────────────────────────────┘
//! ┌──────────────────────────────────┐
//! │ Ayşe Yılmaz          şu an Sahip │
//! │ ayse@kurum.gov.tr                │
//! │ Ayten Kara                       │
//! │ ayten@kurum.gov.tr               │
//! └──────────────────────────────────┘
//! ```
//!
//! Önerileri uygulama verir ([`Suggestion`]), ör. sunucunun bulduğu
//! kişiler; bileşen listeyi, vurguyu ve tuşları yönetir. Liste alanın
//! altında, alanın genişliğinde açılır (altta yer yoksa üstünde); en çok
//! altı öneri görünür, gerisine tekerlekle ya da oklarla gelinir ve sağdaki
//! ince çubuk listenin neresinde olunduğunu gösterir. Önerilerin üstünde
//! seçilemeyen bir not durabilir ([`Suggest::note`], ör. “kimse
//! bulunamadı”).
//!
//! | Tuş | Davranış |
//! |-----|----------|
//! | ↑ ↓ | Öneriler arasında gezinir; uçlardan öbür uca döner. |
//! | Enter | Vurgulanan öneriyi seçer ([`Suggest::on_pick`]); liste kapalıyken [`Suggest::on_submit`]. |
//! | Esc | Açık listeyi kapatır, yazı kalır; liste kapalıyken alan odağı bırakır. |
//!
//! Satırın üstüne gelmek onu vurgular, tıklamak seçer; odak alanda kalır,
//! yazmaya devam edilebilir. Liste alan odakta değilken, dışarı tıklanınca
//! ya da Esc'le kapanır; yazı değişince yeniden açılır.
//!
//! ```ignore
//! Suggest::new(&self.query, "Ad ya da e-posta yazın")
//!     .on_input(Message::Query)
//!     .suggestions(found.iter().map(|p| Suggestion::new(&p.name).detail(&p.email)))
//!     .on_pick(Message::Picked)
//!     .on_submit(Message::Share)
//! ```

use std::borrow::Cow;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{Focusable, Operation};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, container, text_input};
use iced::{
    Background, Center, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size, Theme,
    Vector, border, keyboard, mouse,
};

use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

/// En çok bu kadar öneri görünür (web: 240 px'lik liste).
const SHOWN: usize = 6;
/// Listeyle alan arası (web: 4 px) ve pencere kenarında kalan boşluk.
const GAP: f32 = 4.0;
const EDGE: f32 = 8.0;
/// Listenin en dar hâli: dar alanda da e-postalar okunur.
const MIN_WIDTH: f32 = 240.0;
/// Konum çubuğunun genişliği ve listenin kenarına uzaklığı.
const THUMB: f32 = 3.0;
const THUMB_INSET: f32 = 3.0;

/// Bir öneri: adı, altındaki açıklaması (ör. e-posta), sağındaki notu
/// (ör. kişinin projedeki rolü). [`Suggestion::offer`] başka bir işi
/// öneren satırdır (ör. “… adresine davet gönder”): vurgu renginde yazılır.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion<'a> {
    pub title: Cow<'a, str>,
    pub detail: Option<Cow<'a, str>>,
    pub note: Option<Cow<'a, str>>,
    pub offer: bool,
}

impl<'a> Suggestion<'a> {
    pub fn new(title: impl Into<Cow<'a, str>>) -> Self {
        Self {
            title: title.into(),
            detail: None,
            note: None,
            offer: false,
        }
    }

    /// Başka bir iş öneren satır.
    pub fn offer(title: impl Into<Cow<'a, str>>) -> Self {
        Self {
            offer: true,
            ..Self::new(title)
        }
    }

    pub fn detail(mut self, detail: impl Into<Cow<'a, str>>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn note(mut self, note: impl Into<Cow<'a, str>>) -> Self {
        self.note = Some(note.into());
        self
    }
}

type OnInput<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;

/// Öneri alanı. Bkz. modül belgesi.
pub struct Suggest<'a, Message> {
    value: String,
    placeholder: String,
    on_input: Option<OnInput<'a, Message>>,
    suggestions: Vec<Suggestion<'a>>,
    note: Option<String>,
    on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_submit: Option<Message>,
    id: widget::Id,
    width: Length,
}

impl<'a, Message: Clone + 'a> Suggest<'a, Message> {
    /// Yazısı ve boşken görünen ipucuyla; [`Suggest::on_input`] verilmezse
    /// alan kapalıdır.
    pub fn new(value: impl Into<String>, placeholder: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: placeholder.into(),
            on_input: None,
            suggestions: Vec::new(),
            note: None,
            on_pick: None,
            on_submit: None,
            id: widget::Id::unique(),
            width: Length::Fill,
        }
    }

    /// Yazılan her değişiklik.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    /// Yazılan her değişiklik; `None` alanı kapatır.
    pub fn on_input_maybe(self, on_input: Option<impl Fn(String) -> Message + 'a>) -> Self {
        match on_input {
            Some(on_input) => self.on_input(on_input),
            None => self,
        }
    }

    /// Yazılana göre öneriler.
    pub fn suggestions(mut self, suggestions: impl IntoIterator<Item = Suggestion<'a>>) -> Self {
        self.suggestions = suggestions.into_iter().collect();
        self
    }

    /// Önerilerin üstünde seçilemeyen not (ör. kimse bulunamadı).
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Enter ya da tıklama: önerinin sırası.
    pub fn on_pick(mut self, on_pick: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_pick = Some(Box::new(on_pick));
        self
    }

    /// Liste kapalıyken Enter (ör. seçilen kişiyle paylaş).
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Yazı alanının kimliği: uygulama onunla odaklar.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = id.into();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

impl<'a, Message: Clone + 'a> From<Suggest<'a, Message>> for Element<'a, Message> {
    fn from(s: Suggest<'a, Message>) -> Self {
        let on_input: Option<Rc<dyn Fn(String) -> Message + 'a>> = s.on_input.map(Rc::from);
        let enabled = on_input.is_some();
        let input = text_input(&s.placeholder, &s.value)
            .id(s.id.clone())
            .on_input_maybe(on_input.map(|f| move |text| f(text)))
            // A form field's height: the choices and dates beside it line up.
            .padding([3, 8])
            .size(typography::body())
            .font(typography::ui())
            .width(Fill)
            .style(style::field::input);
        Element::new(Field {
            input: input.into(),
            value: s.value,
            enabled,
            suggestions: s.suggestions,
            note: s.note,
            on_pick: s.on_pick,
            on_submit: s.on_submit,
            id: s.id,
            width: s.width,
            list: None,
        })
    }
}

struct Field<'a, Message> {
    input: Element<'a, Message>,
    value: String,
    enabled: bool,
    suggestions: Vec<Suggestion<'a>>,
    note: Option<String>,
    on_pick: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_submit: Option<Message>,
    id: widget::Id,
    width: Length,
    /// Açık liste; `overlay`da kurulur.
    list: Option<Element<'a, Message>>,
}

#[derive(Debug, Default)]
struct State {
    focused: bool,
    /// Vurgulanan öneri.
    active: usize,
    /// Görünen ilk öneri.
    first: usize,
    /// Liste kapatıldı (Esc ya da dışarı tıklama); yazı değişince açılır.
    dismissed: bool,
    /// Son görülen yazı: değişince vurgu başa döner.
    seen: String,
}

impl State {
    /// Vurgulanan öneriyi görünür tutar; kısalan listede başa kayar.
    fn reveal(&mut self, count: usize) {
        if self.active < self.first {
            self.first = self.active;
        } else if self.active >= self.first + SHOWN {
            self.first = self.active + 1 - SHOWN;
        }
        self.first = self.first.min(count.saturating_sub(SHOWN));
    }
}

impl<Message: Clone> Field<'_, Message> {
    fn is_open(&self, state: &State) -> bool {
        self.enabled
            && state.focused
            && !state.dismissed
            && (!self.suggestions.is_empty() || self.note.is_some())
    }

    fn active(&self, state: &State) -> usize {
        state.active.min(self.suggestions.len().saturating_sub(1))
    }

    /// Yazı alanının odağını okur.
    fn probe(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) {
        let mut probe = Probe {
            target: self.id.clone(),
            focused: false,
        };
        if let Some(input) = layout.children().next() {
            self.input
                .as_widget_mut()
                .operate(&mut tree.children[0], input, renderer, &mut probe);
        }
        let state = tree.state.downcast_mut::<State>();
        if probe.focused && !state.focused {
            state.dismissed = false;
        }
        state.focused = probe.focused;
    }

    /// Alan odaktayken basılan tuş; ele alındıysa `true`.
    fn key(&self, named: Named, state: &mut State, shell: &mut Shell<'_, Message>) -> bool {
        let open = self.is_open(state);
        let count = self.suggestions.len();
        match named {
            Named::ArrowDown | Named::ArrowUp if open && count > 0 => {
                let current = self.active(state);
                state.active = if named == Named::ArrowDown {
                    (current + 1) % count
                } else {
                    (current + count - 1) % count
                };
                state.reveal(count);
                true
            }
            Named::Enter if open && count > 0 => match &self.on_pick {
                Some(on_pick) => {
                    shell.publish(on_pick(self.active(state)));
                    true
                }
                None => false,
            },
            Named::Enter => match &self.on_submit {
                Some(message) => {
                    shell.publish(message.clone());
                    true
                }
                None => false,
            },
            // Esc closes the list first; with the list closed the field lets go of the focus.
            Named::Escape if open => {
                state.dismissed = true;
                true
            }
            _ => false,
        }
    }
}

/// Yazı alanının odağını okuyan işlem.
struct Probe {
    target: widget::Id,
    focused: bool,
}

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if id == Some(&self.target) {
            self.focused = state.is_focused();
        }
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Field<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.input), Tree::empty()]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 2 {
            tree.children[0].diff(&self.input);
        } else {
            tree.children = self.children();
        }
        let state = tree.state.downcast_mut::<State>();
        if state.seen != self.value {
            state.seen.clone_from(&self.value);
            state.active = 0;
            state.first = 0;
            state.dismissed = false;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width);
        let input = self
            .input
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let size = limits.resolve(self.width, Length::Shrink, input.size());
        layout::Node::with_children(Size::new(size.width, input.size().height), vec![input])
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
        let Some(input_layout) = layout.children().next() else {
            return;
        };
        // The list's keys come before the text field's.
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modifiers,
            ..
        }) = event
        {
            self.probe(tree, layout, renderer);
            let state = tree.state.downcast_mut::<State>();
            if state.focused && !modifiers.command() && self.key(*named, state, shell) {
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }
        self.input.as_widget_mut().update(
            &mut tree.children[0],
            event,
            input_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            return;
        }
        let before = tree.state.downcast_ref::<State>().focused;
        self.probe(tree, layout, renderer);
        if tree.state.downcast_ref::<State>().focused != before {
            shell.request_redraw();
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
        layout
            .children()
            .next()
            .map(|input| {
                self.input.as_widget().mouse_interaction(
                    &tree.children[0],
                    input,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .unwrap_or_default()
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
        if let Some(input) = layout.children().next() {
            self.input.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                input,
                cursor,
                viewport,
            );
        }
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
            if let Some(input) = layout.children().next() {
                self.input.as_widget_mut().operate(
                    &mut tree.children[0],
                    input,
                    renderer,
                    operation,
                );
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        if !self.is_open(state) {
            self.list = None;
            return None;
        }
        state.active = self.active(state);
        state.reveal(self.suggestions.len());
        let list = self.list.insert(list(
            &self.suggestions,
            self.note.as_deref(),
            state.first,
            state.active,
        ));
        let list_tree = &mut children[1];
        list_tree.diff(&*list);
        Some(overlay::Element::new(Box::new(Open {
            field: layout.bounds() + translation,
            list,
            tree: list_tree,
            state,
            count: self.suggestions.len(),
            noted: self.note.is_some(),
            on_pick: self.on_pick.as_deref(),
        })))
    }
}

/// The note and the suggestions shown (the web's `share-suggest`), the active one marked.
fn list<'a, Message: 'a>(
    suggestions: &[Suggestion<'a>],
    note: Option<&str>,
    first: usize,
    active: usize,
) -> Element<'a, Message> {
    let mut rows = Column::new();
    if let Some(note) = note {
        rows = rows.push(
            container(
                label::caption(note.to_owned())
                    .wrapping(Wrapping::WordOrGlyph)
                    .style(style::text::muted),
            )
            .padding([8, 10])
            .width(Fill),
        );
    }
    for (i, s) in suggestions.iter().enumerate().skip(first).take(SHOWN) {
        rows = rows.push(suggestion_row(s, i == active));
    }
    container(rows)
        .padding(4)
        .width(Fill)
        .style(style::container::popover)
        .into()
}

fn suggestion_row<'a, Message: 'a>(s: &Suggestion<'a>, active: bool) -> Element<'a, Message> {
    // An offer is a sentence: it wraps rather than being cut.
    let title = label::body(s.title.clone().into_owned());
    let title = if s.offer {
        title
            .wrapping(Wrapping::WordOrGlyph)
            .style(style::text::accent)
    } else {
        title.wrapping(Wrapping::None)
    };
    let mut who = column![title].spacing(1);
    if let Some(detail) = &s.detail {
        who = who.push(
            label::caption(detail.clone().into_owned())
                .wrapping(Wrapping::None)
                .style(style::text::muted),
        );
    }
    let mut content = Row::new()
        .push(container(who).width(Fill).clip(true))
        .spacing(8)
        .align_y(Center);
    if let Some(note) = &s.note {
        content = content.push(
            label::caption(note.clone().into_owned())
                .wrapping(Wrapping::None)
                .style(style::text::muted),
        );
    }
    container(content)
        .padding([6, 10])
        .width(Fill)
        .style(style::container::suggestion(active))
        .into()
}

/// The open list over everything, under the field (over it when there is no room below).
struct Open<'a, 'b, Message> {
    /// The field on the window.
    field: Rectangle,
    list: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    state: &'b mut State,
    count: usize,
    noted: bool,
    on_pick: Option<&'b (dyn Fn(usize) -> Message + 'a)>,
}

impl<Message> Open<'_, '_, Message> {
    /// The suggestion under the pointer.
    fn row_at(&self, layout: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let at = cursor.position()?;
        let rows = layout.children().next()?.children().next()?.children();
        rows.skip(usize::from(self.noted))
            .position(|row| row.bounds().contains(at))
            .map(|i| self.state.first + i)
    }

    /// The list's box on the window.
    fn bounds(layout: Layout<'_>) -> Option<Rectangle> {
        layout.children().next().map(|l| l.bounds())
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Open<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let width = self
            .field
            .width
            .max(typography::scaled(MIN_WIDTH))
            .min(bounds.width - 2.0 * EDGE);
        let node = self.list.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, bounds.height)),
        );
        let size = node.size();
        let x = self.field.x.min(bounds.width - size.width - EDGE).max(EDGE);
        let below = self.field.y + self.field.height + GAP;
        let above = self.field.y - GAP - size.height;
        let y = if below + size.height > bounds.height - EDGE && above >= EDGE {
            above
        } else {
            below
        };
        layout::Node::with_children(bounds, vec![node.move_to(Point::new(x, y))])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(list) = layout.children().next() else {
            return;
        };
        self.list.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            list,
            cursor,
            &layout.bounds(),
        );
        // Where the rows shown are in the whole list, when some are out of sight.
        if self.count <= SHOWN {
            return;
        }
        let Some(rows) = list.children().next() else {
            return;
        };
        let mut shown = rows.children().skip(usize::from(self.noted));
        let (Some(top), Some(bottom)) = (shown.next(), shown.last()) else {
            return;
        };
        let (top, bottom) = (top.bounds().y, bottom.bounds().y + bottom.bounds().height);
        let track = bottom - top;
        let count = self.count as f32;
        let thumb = Rectangle {
            x: list.bounds().x + list.bounds().width - THUMB - THUMB_INSET,
            y: top + track * self.state.first as f32 / count,
            width: THUMB,
            height: (track * SHOWN as f32 / count).max(THUMB * 4.0),
        };
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: thumb,
                border: border::rounded(THUMB / 2.0),
                ..renderer::Quad::default()
            },
            Background::Color(Tokens::of(theme).muted.scale_alpha(0.5)),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(list) = Self::bounds(layout) else {
            return;
        };
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(index) = self.row_at(layout, cursor)
                    && self.state.active != index
                {
                    self.state.active = index;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(list) => {
                let down = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y < 0.0,
                    mouse::ScrollDelta::Pixels { y, .. } => *y < 0.0,
                };
                let last = self.count.saturating_sub(SHOWN);
                let first = self.state.first;
                self.state.first = if down {
                    (first + 1).min(last)
                } else {
                    first.saturating_sub(1)
                };
                // The highlight stays on a row in sight.
                self.state.active = self
                    .state
                    .active
                    .clamp(self.state.first, self.state.first + SHOWN - 1);
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(index) = self.row_at(layout, cursor) {
                    if let Some(on_pick) = self.on_pick {
                        shell.publish(on_pick(index));
                    }
                    // The field keeps the focus: typing goes on.
                    shell.capture_event();
                    shell.request_redraw();
                } else if cursor.is_over(list) {
                    // The note or the list's padding: nothing.
                    shell.capture_event();
                } else if !cursor.is_over(self.field) {
                    // A press elsewhere closes the list; the text stays.
                    self.state.dismissed = true;
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.row_at(layout, cursor).is_some() {
            mouse::Interaction::Pointer
        } else if Self::bounds(layout).is_some_and(|list| cursor.is_over(list)) {
            mouse::Interaction::Idle
        } else {
            // Outside the list the widgets below keep the pointer.
            mouse::Interaction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highlight_stays_in_sight() {
        let mut state = State::default();
        for active in 0..10 {
            state.active = active;
            state.reveal(10);
            assert!(
                (state.first..state.first + SHOWN).contains(&active),
                "{active} in sight from {}",
                state.first
            );
        }
        assert_eq!(state.first, 4, "the last six");
        state.active = 0;
        state.reveal(10);
        assert_eq!(state.first, 0, "back at the top");
        state.active = 2;
        state.first = 9;
        state.reveal(3);
        assert_eq!(state.first, 0, "a shorter list starts at its top");
    }

    #[test]
    fn a_suggestion_says_what_it_is() {
        let s = Suggestion::new("Ayşe Yılmaz")
            .detail("ayse@kurum.gov.tr")
            .note("şu an Sahip");
        assert_eq!(s.detail.as_deref(), Some("ayse@kurum.gov.tr"));
        assert_eq!(s.note.as_deref(), Some("şu an Sahip"));
        assert!(!s.offer);
        assert!(Suggestion::offer("davet gönder…").offer);
    }
}
