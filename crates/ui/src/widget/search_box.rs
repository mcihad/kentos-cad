//! Arama kutusu: yazdıkça altında sonuç listesi açılan alan (ör. şeridin
//! "Komut ara"sı; web'in `ui/ribbon/search.ts`'i).
//!
//! ```text
//!  ┌──────────────────────────── Alt+Q ┐
//!  │ ⌕  çiz                              │
//!  └─────────────────────────────────────┘
//!      ┌─────────────────────────────────────────┐
//!      │ ⟋  Çizgi                        L    ▭  │
//!      │    Giriş › Çizim                         │
//!      │ ⌒  Çoklu çizgi                 PL   ▭  │
//!      └─────────────────────────────────────────┘
//! ```
//!
//! Sonuçları uygulama verir ([`Found`]); bileşen listeyi, vurguyu ve
//! tuşları yönetir:
//!
//! | Tuş | Davranış |
//! |-----|----------|
//! | ↑ ↓ | Sonuçlar arasında gezinir; uçlardan öbür uca döner. |
//! | Enter | Vurgulanan sonucu çalıştırır ([`SearchBox::on_run`]); Alt ile yerini gösterir ([`SearchBox::on_reveal`]). |
//! | Esc | Yazı varsa siler; yoksa kutu odağı bırakır. |
//!
//! Satırın üstüne gelmek onu vurgular, tıklamak çalıştırır. Satırın
//! sağındaki iğne yerini gösterir. Çalışmayan sonuç soluk görünür ve
//! çalıştırılmaz. Liste kutunun sağ kenarına hizalı, altında açılır. Kutu
//! odağı yitirince ya da dışarı tıklanınca kapanır; yazı kalır ve kutuya
//! dönünce liste yeniden açılır. Dar yerde [`SearchBox::compact`] kutuyu,
//! odakta değilken, yalnız büyütece indirir; büyütece basmak kutuyu açar.

use std::borrow::Cow;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{self, Focusable, Operation};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{Column, column, container, row, space, text_input};
use iced::{
    Background, Center, Element, Event, Fill, Length, Padding, Point, Rectangle, Renderer, Size,
    Theme, Vector, border, keyboard, mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

/// Kutunun genişliği ve yüksekliği (web: 206 × 24 px).
const WIDTH: f32 = 206.0;
const HEIGHT: f32 = 24.0;
/// Büyütecin sütunu ve dar yerdeki kutu.
const GLASS: f32 = 24.0;
const COMPACT: f32 = 26.0;
/// Listenin genişliği ve kutuyla arası (web: 390 px, 4 px).
const LIST_WIDTH: f32 = 390.0;
const LIST_GAP: f32 = 4.0;
/// Pencere kenarıyla liste arasında kalan en az boşluk (web: 8 px).
const EDGE: f32 = 8.0;
/// Sonuç satırı (web: 38 px) ve listenin iç boşluğu.
const ROW: f32 = 38.0;
const PADDING: f32 = 4.0;
/// Satırın sağındaki iğnenin tıklama alanı.
const PIN: f32 = 28.0;

/// Bir sonuç: ikonu, adı, altındaki açıklaması (ör. şeritteki yeri),
/// kısayolu, çalışıp çalışmadığı ve yerinin gösterilip gösterilemediği.
#[derive(Clone, Debug)]
pub struct Found<'a> {
    pub icon: Icon,
    pub title: Cow<'a, str>,
    pub detail: Cow<'a, str>,
    pub shortcut: Option<Cow<'a, str>>,
    pub enabled: bool,
    pub revealable: bool,
}

/// Arama kutusu. Bkz. modül belgesi. Yazıları kendisi tutar: şeridin
/// genişliğe göre kurulan satırında da kurulabilir.
pub struct SearchBox<'a, Message> {
    value: String,
    placeholder: String,
    hint: Option<String>,
    results: Vec<Found<'a>>,
    empty: Option<String>,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_run: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_reveal: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_down: Option<Message>,
    id: widget::Id,
    compact: bool,
    fill: bool,
    height: f32,
    room: Option<f32>,
}

impl<'a, Message: Clone + 'a> SearchBox<'a, Message> {
    pub fn new(
        value: impl Into<String>,
        placeholder: impl Into<String>,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self {
        Self {
            value: value.into(),
            placeholder: placeholder.into(),
            hint: None,
            results: Vec::new(),
            empty: None,
            on_input: Box::new(on_input),
            on_run: None,
            on_reveal: None,
            on_down: None,
            id: widget::Id::unique(),
            compact: false,
            fill: false,
            height: HEIGHT,
            room: None,
        }
    }

    /// Kutunun sağındaki kısayol ipucu (ör. "Alt+Q"); odakta gizlenir.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Yazılana göre sonuçlar.
    pub fn results(mut self, results: Vec<Found<'a>>) -> Self {
        self.results = results;
        self
    }

    /// Yazı var ama sonuç yokken listede görünen yazı.
    pub fn empty(mut self, text: impl Into<String>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// Enter ya da tıklama: sonucun sırası.
    pub fn on_run(mut self, on_run: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_run = Some(Box::new(on_run));
        self
    }

    /// Alt+Enter ya da iğne: sonucun sırası.
    pub fn on_reveal(mut self, on_reveal: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_reveal = Some(Box::new(on_reveal));
        self
    }

    /// Yazı alanının kimliği: uygulama onunla odaklar.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = id.into();
        self
    }

    /// Dar yer: odakta değilken yalnız büyüteç.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Listesi yokken ↓: yazılanı gösteren öğeye geçmek için (ör. katman
    /// ağacının ilk satırı).
    pub fn on_down(mut self, message: Message) -> Self {
        self.on_down = Some(message);
        self
    }

    /// Bulunduğu yerin bütün genişliği (ör. panelin arama kutusu).
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Kutunun yüksekliği (ölçeklenmeden önce; varsayılanı 24).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Kutunun alabileceği en çok genişlik (piksel): dar satırda odaklanan
    /// kutu yanındakileri dışarı itmesin.
    pub fn room(mut self, room: f32) -> Self {
        self.room = Some(room);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SearchBox<'a, Message>> for Element<'a, Message> {
    fn from(b: SearchBox<'a, Message>) -> Self {
        // Shared: the field types with it, Esc clears with it.
        let on_input: Rc<dyn Fn(String) -> Message + 'a> = Rc::from(b.on_input);
        let typed = on_input.clone();
        let input = text_input(&b.placeholder, &b.value)
            .id(b.id.clone())
            .on_input(move |text| typed(text))
            .padding(Padding::ZERO)
            .size(typography::body())
            .font(typography::ui())
            .width(Fill)
            .style(style::field::bare_input);
        let glass = container(icon(Icon::Search).size(14.0).tone(Tone::Muted))
            .center_x(Fill)
            .center_y(Fill);
        let hint = b.hint.map(|hint| {
            container(label::mono_caption(hint))
                .padding([0, 5])
                .style(style::container::keycap)
                .into()
        });
        // A halo round the box while its field has the keyboard.
        crate::widget::focus_ring(Element::new(Search {
            glass: glass.into(),
            input: input.into(),
            hint,
            value: b.value,
            results: b.results,
            empty: b.empty,
            on_input,
            on_run: b.on_run,
            on_reveal: b.on_reveal,
            on_down: b.on_down,
            id: b.id,
            compact: b.compact,
            fill: b.fill,
            height: b.height,
            room: b.room,
            list: None,
        }))
        .into()
    }
}

struct Search<'a, Message> {
    glass: Element<'a, Message>,
    input: Element<'a, Message>,
    hint: Option<Element<'a, Message>>,
    value: String,
    results: Vec<Found<'a>>,
    empty: Option<String>,
    on_input: Rc<dyn Fn(String) -> Message + 'a>,
    on_run: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_reveal: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_down: Option<Message>,
    id: widget::Id,
    compact: bool,
    fill: bool,
    height: f32,
    room: Option<f32>,
    /// Açık liste; `overlay`da kurulur.
    list: Option<Element<'a, Message>>,
}

#[derive(Debug, Default)]
struct State {
    focused: bool,
    /// Vurgulanan sonuç.
    active: usize,
    /// Liste kapatıldı (dışarı tıklandı); yazı değişince ya da odak
    /// dönünce açılır.
    dismissed: bool,
    /// Son görülen yazı: değişince vurgu başa döner.
    seen: String,
    hovered: bool,
}

impl<Message: Clone> Search<'_, Message> {
    fn is_open(&self, state: &State) -> bool {
        state.focused
            && !state.dismissed
            && !self.value.trim().is_empty()
            && (!self.results.is_empty() || self.empty.is_some())
    }

    fn active(&self, state: &State) -> usize {
        state.active.min(self.results.len().saturating_sub(1))
    }

    fn narrow(&self, state: &State) -> bool {
        self.compact && !state.focused
    }

    /// Yazı alanının odağını okur.
    fn probe(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) {
        let mut probe = Probe {
            target: self.id.clone(),
            focused: false,
        };
        if let Some(input) = layout.children().nth(1) {
            self.input
                .as_widget_mut()
                .operate(&mut tree.children[1], input, renderer, &mut probe);
        }
        let state = tree.state.downcast_mut::<State>();
        if probe.focused && !state.focused {
            state.dismissed = false;
        }
        state.focused = probe.focused;
    }

    /// Kutu odaktayken basılan tuş; ele alındıysa `true`.
    fn key(
        &self,
        named: Named,
        alt: bool,
        state: &mut State,
        shell: &mut Shell<'_, Message>,
    ) -> Option<Clear> {
        let open = self.is_open(state);
        let count = self.results.len();
        match named {
            Named::ArrowDown | Named::ArrowUp if open && count > 0 => {
                let current = self.active(state);
                state.active = if named == Named::ArrowDown {
                    (current + 1) % count
                } else {
                    (current + count - 1) % count
                };
                Some(Clear::No)
            }
            Named::ArrowDown if !open => {
                let message = self.on_down.clone()?;
                shell.publish(message);
                Some(Clear::No)
            }
            Named::Enter if open && count > 0 => {
                let index = self.active(state);
                if alt {
                    self.reveal(index, shell);
                } else {
                    self.run(index, shell);
                }
                Some(Clear::No)
            }
            // Yazı varken Esc onu siler; yoksa yazı alanı odağı bırakır (kendi Esc'i).
            Named::Escape if !self.value.is_empty() => Some(Clear::Yes),
            _ => None,
        }
    }

    fn run(&self, index: usize, shell: &mut Shell<'_, Message>) {
        if let (Some(found), Some(on_run)) = (self.results.get(index), &self.on_run)
            && found.enabled
        {
            shell.publish(on_run(index));
        }
    }

    fn reveal(&self, index: usize, shell: &mut Shell<'_, Message>) {
        if let (Some(found), Some(on_reveal)) = (self.results.get(index), &self.on_reveal)
            && found.revealable
        {
            shell.publish(on_reveal(index));
        }
    }
}

/// Esc'in yazıyı silip silmediği.
enum Clear {
    Yes,
    No,
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

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Search<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.glass),
            Tree::new(&self.input),
            self.hint.as_ref().map_or_else(Tree::empty, Tree::new),
            Tree::empty(),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 4 {
            tree.children[0].diff(&self.glass);
            tree.children[1].diff(&self.input);
            match &self.hint {
                Some(hint) => tree.children[2].diff(hint),
                None => tree.children[2] = Tree::empty(),
            }
        } else {
            tree.children = self.children();
        }
        let state = tree.state.downcast_mut::<State>();
        if state.seen != self.value {
            state.seen.clone_from(&self.value);
            state.active = 0;
            state.dismissed = false;
        }
    }

    fn size(&self) -> Size<Length> {
        let width = if self.fill {
            Length::Fill
        } else {
            Length::Shrink
        };
        Size::new(width, Length::Fixed(typography::scaled(self.height)))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let narrow = self.narrow(tree.state.downcast_ref::<State>());
        let height = typography::scaled(self.height);
        let width = if narrow {
            typography::scaled(COMPACT)
        } else if self.fill {
            limits.max().width
        } else {
            let full = typography::scaled(WIDTH);
            self.room
                .map_or(full, |room| full.min(room.max(typography::scaled(COMPACT))))
        };
        let glass_width = typography::scaled(if narrow { COMPACT } else { GLASS });
        let glass = self
            .glass
            .as_widget_mut()
            .layout(
                &mut tree.children[0],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(glass_width, height)),
            )
            .move_to(Point::ORIGIN);
        let hint = match &mut self.hint {
            Some(hint) if !narrow => {
                let node = hint.as_widget_mut().layout(
                    &mut tree.children[2],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(width, height)),
                );
                let size = node.size();
                node.move_to(Point::new(
                    width - size.width - 4.0,
                    (height - size.height) / 2.0,
                ))
            }
            _ => layout::Node::new(Size::ZERO),
        };
        let room = if narrow {
            0.0
        } else {
            (width - glass_width - 4.0).max(0.0)
        };
        let input = self.input.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(room, height)),
        );
        let input_height = input.size().height;
        let input = input.move_to(Point::new(glass_width, (height - input_height) / 2.0));
        layout::Node::with_children(Size::new(width, height), vec![glass, input, hint])
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
        let (Some(glass_layout), Some(input_layout)) = (layouts.next(), layouts.next()) else {
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
            if state.focused
                && !modifiers.command()
                && let Some(clear) = self.key(*named, modifiers.alt(), state, shell)
            {
                if let Clear::Yes = clear {
                    shell.publish((self.on_input)(String::new()));
                }
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        // A narrow box opens on a press on its magnifier.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && self.narrow(tree.state.downcast_ref::<State>())
            && cursor.is_over(glass_layout.bounds())
        {
            self.input.as_widget_mut().operate(
                &mut tree.children[1],
                input_layout,
                renderer,
                &mut operation::focusable::focus(self.id.clone()),
            );
            shell.invalidate_layout();
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        self.input.as_widget_mut().update(
            &mut tree.children[1],
            event,
            input_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            let state = tree.state.downcast_mut::<State>();
            let over = cursor.is_over(bounds);
            if over != state.hovered {
                state.hovered = over;
                shell.request_redraw();
            }
            return;
        }
        let before = tree.state.downcast_ref::<State>().focused;
        self.probe(tree, layout, renderer);
        if tree.state.downcast_ref::<State>().focused != before {
            // The box widens or narrows, and the hint shows or goes.
            shell.invalidate_layout();
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
        let state = tree.state.downcast_ref::<State>();
        if self.narrow(state) && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }
        layout
            .children()
            .nth(1)
            .map(|input| {
                self.input.as_widget().mouse_interaction(
                    &tree.children[1],
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
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let narrow = self.narrow(state);
        // The web's `.rsearch`: the field's ground and edge; the accent edge
        // while typed in; a narrow box is the bare magnifier.
        if !narrow {
            let edge = if state.focused {
                t.accent
            } else if state.hovered {
                t.muted
            } else {
                t.border
            };
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds,
                    border: border::rounded(style::button::radius())
                        .color(edge)
                        .width(1.0),
                    ..renderer::Quad::default()
                },
                Background::Color(t.field),
            );
        } else if state.hovered {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds,
                    border: border::rounded(style::button::radius()),
                    ..renderer::Quad::default()
                },
                Background::Color(t.surface_hover),
            );
        }
        let mut layouts = layout.children();
        if let Some(glass) = layouts.next() {
            self.glass.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                glass,
                cursor,
                viewport,
            );
        }
        if let Some(input) = layouts.next()
            && !narrow
        {
            self.input.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                input,
                cursor,
                viewport,
            );
        }
        if let (Some(hint), Some(hint_layout)) = (&self.hint, layouts.next())
            && !narrow
            && !state.focused
        {
            hint.as_widget().draw(
                &tree.children[2],
                renderer,
                theme,
                style,
                hint_layout,
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
        operation.container(Some(&self.id), layout.bounds());
        operation.traverse(&mut |operation| {
            if let Some(input) = layout.children().nth(1) {
                self.input.as_widget_mut().operate(
                    &mut tree.children[1],
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
        viewport: &Rectangle,
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
        let active = self.active(state);
        let list = self
            .list
            .insert(list(&self.results, self.empty.as_deref(), active));
        let list_tree = &mut children[3];
        list_tree.diff(&*list);
        Some(overlay::Element::new(Box::new(Results {
            field: layout.bounds() + translation,
            window: *viewport,
            list,
            tree: list_tree,
            state,
            results: &self.results,
            on_run: self.on_run.as_deref(),
            on_reveal: self.on_reveal.as_deref(),
        })))
    }
}

/// The results as rows (the web's `rsearch__row`s), the active one marked.
fn list<'a, Message: 'a>(
    results: &[Found<'a>],
    empty: Option<&str>,
    active: usize,
) -> Element<'a, Message> {
    let rows: Element<'a, Message> = if results.is_empty() {
        container(
            label::caption(empty.unwrap_or_default().to_owned())
                .wrapping(Wrapping::WordOrGlyph)
                .style(style::text::muted),
        )
        .padding([8.0, 8.0])
        .width(Fill)
        .into()
    } else {
        Column::with_children(
            results
                .iter()
                .enumerate()
                .map(|(i, found)| result_row(found, i == active)),
        )
        .into()
    };
    container(rows)
        .padding(PADDING)
        .width(typography::scaled(LIST_WIDTH))
        .style(style::container::popover)
        .into()
}

fn result_row<'a, Message: 'a>(found: &Found<'a>, active: bool) -> Element<'a, Message> {
    let tone = if found.enabled {
        Tone::Inherit
    } else {
        Tone::Muted
    };
    let title = label::body(found.title.clone().into_owned()).wrapping(Wrapping::None);
    let title = if found.enabled {
        title
    } else {
        title.style(style::text::muted)
    };
    let detail = label::caption(found.detail.clone().into_owned())
        .wrapping(Wrapping::None)
        .style(style::text::muted);
    let mut content = row![
        container(icon(found.icon).size(16.0).tone(tone))
            .width(20)
            .center_y(Fill),
        container(column![title, detail].spacing(1))
            .width(Fill)
            .center_y(Fill)
            .clip(true),
    ]
    .spacing(10)
    .align_y(Center);
    if let Some(shortcut) = &found.shortcut {
        content = content
            .push(label::mono_caption(shortcut.clone().into_owned()).style(style::text::muted));
    }
    content = content.push(if found.revealable {
        Element::from(
            container(icon(Icon::Tabs).size(14.0).tone(Tone::Muted))
                .width(typography::scaled(PIN))
                .center_x(typography::scaled(PIN)),
        )
    } else {
        space::horizontal().width(typography::scaled(PIN)).into()
    });
    let enabled = found.enabled;
    container(content)
        .padding(Padding::from([3.0, 6.0]).left(8.0))
        .height(typography::scaled(ROW))
        .width(Fill)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: active.then(|| {
                    if enabled {
                        t.accent.scale_alpha(0.14).into()
                    } else {
                        t.surface_hover.into()
                    }
                }),
                border: border::rounded(style::button::radius()),
                ..container::Style::default()
            }
        })
        .into()
}

/// The open list over everything, under the box.
struct Results<'a, 'b, Message> {
    /// The box on the window.
    field: Rectangle,
    window: Rectangle,
    list: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    state: &'b mut State,
    results: &'b [Found<'a>],
    on_run: Option<&'b (dyn Fn(usize) -> Message + 'a)>,
    on_reveal: Option<&'b (dyn Fn(usize) -> Message + 'a)>,
}

impl<Message> Results<'_, '_, Message> {
    /// The row under the pointer and whether it is on the row's pin.
    fn row_at(&self, list: Rectangle, cursor: mouse::Cursor) -> Option<(usize, bool)> {
        let at = cursor.position_over(list)?;
        let offset = at.y - list.y - PADDING;
        let row = typography::scaled(ROW);
        let index = (offset >= 0.0).then(|| (offset / row) as usize)?;
        (index < self.results.len()).then(|| {
            let pin = at.x >= list.x + list.width - PADDING - typography::scaled(PIN) - 6.0;
            (index, pin)
        })
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Results<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.list.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let size = node.size();
        // Right-aligned to the box, kept on the window (the web's `place`).
        let x = (self.field.x + self.field.width - size.width)
            .min(bounds.width - size.width - EDGE)
            .max(EDGE);
        let y = self.field.y + self.field.height + LIST_GAP;
        let _ = self.window;
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
        if let Some(list) = layout.children().next() {
            self.list.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                list,
                cursor,
                &layout.bounds(),
            );
        }
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
        let Some(list) = layout.children().next().map(|l| l.bounds()) else {
            return;
        };
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some((index, _)) = self.row_at(list, cursor)
                    && self.state.active != index
                {
                    self.state.active = index;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some((index, pin)) = self.row_at(list, cursor) {
                    let found = &self.results[index];
                    let message = if pin && found.revealable {
                        self.on_reveal.map(|reveal| reveal(index))
                    } else if found.enabled {
                        self.on_run.map(|run| run(index))
                    } else {
                        None
                    };
                    if let Some(message) = message {
                        shell.publish(message);
                    }
                    shell.capture_event();
                    shell.request_redraw();
                } else if cursor.is_over(list) {
                    // The list's own padding or the empty text: nothing.
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
        let over = layout
            .children()
            .next()
            .is_some_and(|list| self.row_at(list.bounds(), cursor).is_some());
        if over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
