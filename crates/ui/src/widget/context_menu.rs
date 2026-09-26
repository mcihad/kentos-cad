//! Bağlam menüsü: bir öğeye sağ tıklanınca imlecin olduğu yerde açılan
//! komut listesi.
//!
//! ```text
//!   ┌──────────────────────────────────┐
//!   │ ◎  Katmana yakınlaştır         Z │
//!   │ ▦  Öznitelik tablosunu aç        │
//!   │ ──────────────────────────────── │
//!   │ ◐  Opaklık                     › │──┐ ✓ %100
//!   │ ──────────────────────────────── │  │   %75
//!   │ ✕  Çizimleri sil             Del │  │   %50
//!   └──────────────────────────────────┘  └───────
//! ```
//!
//! [`ContextMenu`] herhangi bir öğeyi sarar. Menü, pencerenin kenarına
//! taşacaksa sola ya da yukarı açılır. Kendi açık/kapalı durumunu tutar:
//! komut seçilince, dışarı tıklanınca ya da Esc'e basılınca kapanır. Başka
//! bir yere sağ tıklamak menüyü orada yeniden açar. Klavyeyle de kullanılır:
//! oklar komutlar arasında gezinir, sağ ok alt menüyü açar, Enter seçer.
//!
//! Menü içeriği ([`Menu`]) menü açıkken, sağ tıklanan noktanın öğe
//! içindeki konumuyla kurulur. Böylece model alanında tıklanan koordinata
//! göre komut sunulabilir:
//!
//! ```ignore
//! ContextMenu::new(row, move |_position| {
//!     Menu::new()
//!         .item("Katmana yakınlaştır", Message::ZoomToLayer(index))
//!         .icon(Icon::Target)
//!         .shortcut("Z")
//!         .separator()
//!         .submenu(
//!             "Opaklık",
//!             Menu::new()
//!                 .check("%100", opacity == 1.0, Message::Opacity(index, 1.0))
//!                 .check("%50", opacity == 0.5, Message::Opacity(index, 0.5)),
//!         )
//!         .separator()
//!         .item("Kaldır", Message::Remove(index))
//!         .danger()
//! })
//! ```
//!
//! Sarılan öğe sağ tıklamayı kendisi kullanırsa (ör. çizim aracının sağ
//! tıkla bitirmesi) menü açılmaz.
//!
//! macOS'ta Control + tık da sağ tık sayılır: izleyici dizüstülerde ikincil
//! tık çoğunlukla böyle yapılır. Sarılan öğeye sağ tık olarak iletilir;
//! böylece iç içe menülerde en içteki açılır, düğmeler de basılmış sayılmaz.
//!
//! [`MenuButton`] aynı menüyü sol tıkla, öğenin altına (sığmazsa üstüne)
//! hizalı açar; durum çubuğundaki göstergeler böyle çalışır:
//!
//! ```ignore
//! MenuButton::new(label::mono(scale), || {
//!     Menu::new()
//!         .check("1:25.000", current == 25_000, Message::Scale(25_000.0))
//!         .check("1:50.000", current == 50_000, Message::Scale(50_000.0))
//! })
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::widget::{Column, container, row, rule, space};
use iced::{
    Background, Center, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size, Theme,
    Vector, border, keyboard, mouse, window,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};

// Metni taşıyan ölçüler 12 piksellik gövde metninde tasarlandı ve yazı
// boyutuyla büyür.

/// Komut satırının yüksekliği.
const ITEM_HEIGHT: f32 = 26.0;
/// Bölücü satırın yüksekliği.
const SEPARATOR_HEIGHT: f32 = 9.0;
/// Bölüm başlığının yüksekliği.
const HEADER_HEIGHT: f32 = 24.0;
/// Menü kutusunun iç boşluğu.
const PADDING: f32 = 4.0;
/// İkon ya da işaret sütununun genişliği.
const ICON_SLOT: f32 = 16.0;
/// Menünün en dar ve en geniş hâli.
const MIN_WIDTH: f32 = 184.0;
const MAX_WIDTH: f32 = 360.0;
/// Alt menünün ana menünün üstüne binen kısmı.
const SUBMENU_OVERLAP: f32 = 2.0;
/// Menü imlecin bu kadar sağında ve altında açılır.
const CURSOR_GAP: f32 = 2.0;
/// Menü düğmesinin menüsüyle düğme arasındaki boşluk.
const BUTTON_GAP: f32 = 3.0;

fn item_height() -> f32 {
    typography::scaled(ITEM_HEIGHT)
}

fn header_height() -> f32 {
    typography::scaled(HEADER_HEIGHT)
}

/// Menünün komutları.
#[derive(Debug, Clone)]
pub struct Menu<Message> {
    items: Vec<Item<Message>>,
}

#[derive(Debug, Clone)]
enum Item<Message> {
    Command(Command<Message>),
    Submenu {
        icon: Option<Icon>,
        label: String,
        menu: Menu<Message>,
    },
    Separator,
    Header(String),
}

#[derive(Debug, Clone)]
struct Command<Message> {
    icon: Option<Icon>,
    label: String,
    shortcut: Option<String>,
    on_press: Option<Message>,
    checked: Option<bool>,
    danger: bool,
}

impl<Message> Item<Message> {
    fn height(&self) -> f32 {
        match self {
            Item::Command(_) | Item::Submenu { .. } => item_height(),
            Item::Separator => SEPARATOR_HEIGHT,
            Item::Header(_) => header_height(),
        }
    }

    /// Vurgulanabilir mi: etkin komut ya da alt menü.
    fn is_selectable(&self) -> bool {
        match self {
            Item::Command(command) => command.on_press.is_some(),
            Item::Submenu { menu, .. } => !menu.is_empty(),
            Item::Separator | Item::Header(_) => false,
        }
    }
}

impl<Message> Menu<Message> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Komut; `on_press` yoksa devre dışıdır.
    pub fn item(mut self, label: impl Into<String>, on_press: impl Into<Option<Message>>) -> Self {
        self.items.push(Item::Command(Command {
            icon: None,
            label: label.into(),
            shortcut: None,
            on_press: on_press.into(),
            checked: None,
            danger: false,
        }));
        self
    }

    /// İşaretlenebilir komut; işaretliyse ikon sütununda ✓ gösterilir.
    pub fn check(
        mut self,
        label: impl Into<String>,
        checked: bool,
        on_press: impl Into<Option<Message>>,
    ) -> Self {
        self.items.push(Item::Command(Command {
            icon: None,
            label: label.into(),
            shortcut: None,
            on_press: on_press.into(),
            checked: Some(checked),
            danger: false,
        }));
        self
    }

    /// Üzerine gelinince ya da sağ okla açılan alt menü. Boş alt menü devre
    /// dışı gösterilir.
    pub fn submenu(mut self, label: impl Into<String>, menu: Menu<Message>) -> Self {
        self.items.push(Item::Submenu {
            icon: None,
            label: label.into(),
            menu,
        });
        self
    }

    /// Komut grupları arasına çizgi. Menünün başında ya da art arda
    /// eklenen çizgiler yok sayılır.
    pub fn separator(mut self) -> Self {
        if self
            .items
            .last()
            .is_some_and(|item| !matches!(item, Item::Separator))
        {
            self.items.push(Item::Separator);
        }

        self
    }

    /// Sonraki komutları gruplayan, seçilemeyen başlık.
    pub fn header(mut self, title: impl Into<String>) -> Self {
        self.items.push(Item::Header(title.into()));
        self
    }

    /// Son eklenen komutun ya da alt menünün ikonu.
    pub fn icon(mut self, glyph: Icon) -> Self {
        match self.items.last_mut() {
            Some(Item::Command(command)) => command.icon = Some(glyph),
            Some(Item::Submenu { icon, .. }) => *icon = Some(glyph),
            _ => {}
        }

        self
    }

    /// Son eklenen komutun kısayolu; yalnızca gösterilir.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Some(Item::Command(command)) = self.items.last_mut() {
            command.shortcut = Some(shortcut.into());
        }

        self
    }

    /// Son eklenen komut geri alınamaz bir iş yapar (ör. silme); kırmızı
    /// yazılır.
    pub fn danger(mut self) -> Self {
        if let Some(Item::Command(command)) = self.items.last_mut() {
            command.danger = true;
        }

        self
    }

    /// Menüde gösterilecek bir şey var mı. Sondaki bölücü sayılmaz.
    pub fn is_empty(&self) -> bool {
        self.items
            .iter()
            .all(|item| matches!(item, Item::Separator))
    }

    /// Sondaki bölücü çizgiyi atar.
    fn trimmed(mut self) -> Self {
        while matches!(self.items.last(), Some(Item::Separator)) {
            self.items.pop();
        }

        for item in &mut self.items {
            if let Item::Submenu { menu, .. } = item {
                *menu = std::mem::take(menu).trimmed();
            }
        }

        self
    }

    /// Komutun menü kutusunun üstünden uzaklığı.
    fn offset(&self, index: usize) -> f32 {
        PADDING + self.items.iter().take(index).map(Item::height).sum::<f32>()
    }

    /// Kutunun üstünden `y` uzaklıktaki komut.
    fn item_at(&self, y: f32) -> Option<usize> {
        let mut top = PADDING;

        for (index, item) in self.items.iter().enumerate() {
            let bottom = top + item.height();

            if (top..bottom).contains(&y) {
                return Some(index);
            }

            top = bottom;
        }

        None
    }

    fn submenu_of(&self, index: usize) -> Option<&Menu<Message>> {
        match self.items.get(index) {
            Some(Item::Submenu { menu, .. }) if !menu.is_empty() => Some(menu),
            _ => None,
        }
    }

    /// `from` komutundan sonraki (ya da önceki) vurgulanabilir komut; uçlarda
    /// başa döner.
    fn step(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        let count = self.items.len();

        if count == 0 {
            return None;
        }

        let start = match (from, forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => index + count - 1,
            (None, true) => 0,
            (None, false) => count - 1,
        };

        (0..count)
            .map(|offset| {
                if forward {
                    (start + offset) % count
                } else {
                    (start + count - offset) % count
                }
            })
            .find(|&index| self.items[index].is_selectable())
    }

    /// Kutunun genişliği: en uzun satıra göre, sınırlar içinde. Metnin
    /// genişliği yazı ailesinin ortalama harf genişliğinden tahmin edilir.
    fn width(&self) -> f32 {
        let body = typography::body();
        let caption = typography::caption();

        let widest = self
            .items
            .iter()
            .map(|item| match item {
                Item::Command(command) => {
                    typography::text_width(&command.label, body)
                        + command.shortcut.as_deref().map_or(0.0, |shortcut| {
                            typography::mono_width(shortcut, caption) + 24.0
                        })
                }
                Item::Submenu { label, .. } => typography::text_width(label, body) + 24.0,
                Item::Header(title) => typography::text_width(title, caption),
                Item::Separator => 0.0,
            })
            .fold(0.0, f32::max);

        (widest + ICON_SLOT + 8.0 + 16.0 + PADDING * 2.0 + 8.0)
            .clamp(typography::scaled(MIN_WIDTH), typography::scaled(MAX_WIDTH))
    }
}

impl<Message> Default for Menu<Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// Sağ tıklanınca bağlam menüsü açan kap.
pub struct ContextMenu<'a, Message> {
    content: Element<'a, Message>,
    menu: Box<dyn Fn(Point) -> Menu<Message> + 'a>,
    trigger: Trigger,
    open: Option<Open<'a, Message>>,
    /// Uygulamanın açıp kapattığı menü: nerede (içerikteki nokta; `None`
    /// kapalı) ve kapanınca gönderilen mesaj.
    controlled: Option<(Option<Point>, Message)>,
}

/// Menüyü açan tıklama ve menünün yeri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    /// Sağ tık; menü imlecin yanında açılır.
    Secondary,
    /// Sol tık; menü öğenin altında ya da üstünde, sol kenarına hizalı açılır.
    Primary,
}

/// Açık menünün bu kareki içeriği.
struct Open<'a, Message> {
    menu: Menu<Message>,
    main: Element<'a, Message>,
    sub: Option<Element<'a, Message>>,
}

impl<'a, Message: Clone + 'a> ContextMenu<'a, Message> {
    /// `menu`, menü açıkken sağ tıklanan noktanın `content` içindeki
    /// konumuyla çağrılır. Boş menü döndürülürse menü açılmaz.
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        menu: impl Fn(Point) -> Menu<Message> + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            menu: Box::new(menu),
            trigger: Trigger::Secondary,
            open: None,
            controlled: None,
        }
    }

    /// Uygulamanın açtığı menü: `at` içerikteki noktadır, `None` iken
    /// kapalıdır. Sağ tık kendiliğinden açmaz (ör. çizim alanı, sağ tuşu
    /// basılı tutunca ya da bırakınca açar). Komut seçilince, dışarı
    /// tıklanınca ya da Esc'e basılınca kapanır ve `on_close` gönderilir;
    /// uygulama `at`'i `None` yapar.
    pub fn controlled(
        content: impl Into<Element<'a, Message>>,
        at: Option<Point>,
        menu: impl Fn(Point) -> Menu<Message> + 'a,
        on_close: Message,
    ) -> Self {
        Self {
            controlled: Some((at, on_close)),
            ..Self::new(content, menu)
        }
    }

    /// Uygulamanın açtığı menünün yeri durumla eşleşir: yeni bir yer menüyü
    /// orada yeniden açar; kapanmış menü, uygulama yerini değiştirene dek
    /// yeniden açılmaz.
    fn sync(&self, state: &mut State) -> bool {
        let Some((at, _)) = &self.controlled else {
            return false;
        };
        let wanted = at.map(|p| p - Point::ORIGIN);
        if wanted == state.synced {
            return false;
        }
        *state = State {
            anchor: wanted,
            synced: wanted,
            over: state.over,
            modifiers: state.modifiers,
            ..State::default()
        };
        true
    }
}

/// Tıklanınca altında (sığmazsa üstünde) menü açan öğe; menü açıkken ya da
/// üzerine gelinince hafif bir zeminle belirginleşir.
pub struct MenuButton<'a, Message>(ContextMenu<'a, Message>);

impl<'a, Message: Clone + 'a> MenuButton<'a, Message> {
    /// `menu` her açılışta yeniden kurulur. Boş menü döndürülürse menü
    /// açılmaz.
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        menu: impl Fn() -> Menu<Message> + 'a,
    ) -> Self {
        Self(ContextMenu {
            trigger: Trigger::Primary,
            ..ContextMenu::new(content, move |_| menu())
        })
    }
}

impl<'a, Message: Clone + 'a> From<MenuButton<'a, Message>> for Element<'a, Message> {
    fn from(button: MenuButton<'a, Message>) -> Self {
        Element::new(button.0)
    }
}

/// Vurgulanan komut: ana menüde ya da açık alt menüde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Main(usize),
    Sub(usize),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct State {
    /// Açıksa sağ tıklanan noktanın içerik içindeki konumu.
    anchor: Option<Vector>,
    /// Uygulamanın açtığı menüde son eşlenen yer ([`ContextMenu::controlled`]).
    synced: Option<Vector>,
    hovered: Option<Target>,
    /// Açık alt menünün ana menüdeki sırası.
    submenu: Option<usize>,
    /// İmleç menü düğmesinin üzerinde mi; değişince zemin yeniden çizilir.
    over: bool,
    /// Basılı değiştirici tuşlar; macOS'ta Control + tık sağ tıktır.
    modifiers: keyboard::Modifiers,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for ContextMenu<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::empty(), Tree::empty()]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 3 {
            tree.children[0].diff(&self.content);
        } else {
            tree.children = self.children();
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        let state = tree.state.downcast_mut::<State>();

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.modifiers = *modifiers;
        }

        if self.sync(state) {
            shell.invalidate_layout();
            shell.request_redraw();
        }

        let secondary;
        let event =
            if self.trigger == Trigger::Secondary && is_control_click(event, state.modifiers) {
                secondary = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
                &secondary
            } else {
                event
            };

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // Menü düğmesinin üzerine gelme zemini: yalnızca imleç girip
        // çıkınca yeniden çizilir.
        if self.trigger == Trigger::Primary
            && let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event
        {
            let over = cursor.is_over(layout.bounds());
            let state = tree.state.downcast_mut::<State>();

            if state.over != over {
                state.over = over;
                shell.request_redraw();
            }
        }

        // The app opens its own menu.
        if shell.is_event_captured() || self.controlled.is_some() {
            return;
        }

        let button = match self.trigger {
            Trigger::Secondary => mouse::Button::Right,
            Trigger::Primary => mouse::Button::Left,
        };

        if let Event::Mouse(mouse::Event::ButtonPressed(pressed)) = event
            && *pressed == button
            && let Some(position) = cursor.position_over(layout.bounds())
        {
            let anchor = position - layout.position();

            if (self.menu)(Point::ORIGIN + anchor).is_empty() {
                return;
            }

            let state = tree.state.downcast_mut::<State>();

            *state = State {
                anchor: Some(anchor),
                over: state.over,
                modifiers: state.modifiers,
                ..State::default()
            };

            shell.capture_event();
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
        let interaction = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );

        if self.trigger == Trigger::Primary
            && interaction == mouse::Interaction::None
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            interaction
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
        if self.trigger == Trigger::Primary {
            let open = tree.state.downcast_ref::<State>().anchor.is_some();
            let t = Tokens::of(theme);

            let alpha = if open {
                Some(0.1)
            } else if cursor.is_over(layout.bounds()) {
                Some(0.06)
            } else {
                None
            };

            if let Some(alpha) = alpha {
                renderer::Renderer::fill_quad(
                    renderer,
                    renderer::Quad {
                        bounds: layout.bounds(),
                        border: border::rounded(3.0),
                        ..renderer::Quad::default()
                    },
                    Background::Color(t.layer(alpha)),
                );
            }
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        self.sync(state);
        let [content_tree, main_tree, sub_tree] = children.as_mut_slice() else {
            return None;
        };

        let content = self.content.as_widget_mut().overlay(
            content_tree,
            layout,
            renderer,
            viewport,
            translation,
        );

        let menu = match state.anchor {
            Some(anchor) => {
                let menu = (self.menu)(Point::ORIGIN + anchor).trimmed();

                let main_highlight = match state.hovered {
                    Some(Target::Main(index)) => Some(index),
                    // Alt menüde gezinirken üst komut vurgulu kalır.
                    _ => state.submenu,
                };
                let sub_highlight = match state.hovered {
                    Some(Target::Sub(index)) => Some(index),
                    _ => None,
                };

                let main = panel(&menu, main_highlight);
                let sub = state
                    .submenu
                    .and_then(|index| menu.submenu_of(index))
                    .map(|submenu| panel(submenu, sub_highlight));

                let open = self.open.insert(Open { menu, main, sub });

                main_tree.diff(&open.main);

                if let Some(sub) = &open.sub {
                    sub_tree.diff(sub);
                }

                let (anchor, gap) = match self.trigger {
                    Trigger::Secondary => (
                        Rectangle::new(layout.position() + translation + anchor, Size::ZERO),
                        Vector::new(CURSOR_GAP, CURSOR_GAP),
                    ),
                    Trigger::Primary => {
                        (layout.bounds() + translation, Vector::new(0.0, BUTTON_GAP))
                    }
                };

                Some(overlay::Element::new(Box::new(Overlay {
                    anchor,
                    gap,
                    menu: &open.menu,
                    main: &mut open.main,
                    sub: open.sub.as_mut(),
                    main_tree,
                    sub_tree,
                    state,
                    on_close: self.controlled.as_ref().map(|(_, message)| message.clone()),
                })))
            }
            None => {
                self.open = None;
                None
            }
        };

        match (content, menu) {
            (Some(content), Some(menu)) => {
                Some(overlay::Group::with_children(vec![content, menu]).overlay())
            }
            (content, menu) => content.or(menu),
        }
    }
}

impl<'a, Message: Clone + 'a> From<ContextMenu<'a, Message>> for Element<'a, Message> {
    fn from(menu: ContextMenu<'a, Message>) -> Self {
        Element::new(menu)
    }
}

/// Açık menü: ana kutu ve varsa alt menü kutusu.
struct Overlay<'a, 'b, Message> {
    /// Menünün yanında açıldığı alan: sağ tıklanan nokta ya da menü
    /// düğmesinin sınırları (penceredeki konumuyla).
    anchor: Rectangle,
    /// Menüyle alan arasındaki yatay ve dikey boşluk.
    gap: Vector,
    menu: &'b Menu<Message>,
    main: &'b mut Element<'a, Message>,
    sub: Option<&'b mut Element<'a, Message>>,
    main_tree: &'b mut Tree,
    sub_tree: &'b mut Tree,
    state: &'b mut State,
    /// The app's menu tells it when it closes ([`ContextMenu::controlled`]).
    on_close: Option<Message>,
}

impl<'b, Message: Clone> Overlay<'_, 'b, Message> {
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        *self.state = State {
            over: self.state.over,
            modifiers: self.state.modifiers,
            // Closed: not opened again until the app gives another place.
            synced: self.state.synced,
            ..State::default()
        };
        if let Some(message) = &self.on_close {
            shell.publish(message.clone());
        }
        shell.invalidate_layout();
        shell.request_redraw();
    }

    fn set_hovered(&mut self, hovered: Option<Target>, shell: &mut Shell<'_, Message>) {
        if self.state.hovered != hovered {
            self.state.hovered = hovered;
            shell.request_redraw();
        }
    }

    fn set_submenu(&mut self, submenu: Option<usize>, shell: &mut Shell<'_, Message>) {
        if self.state.submenu != submenu {
            self.state.submenu = submenu;
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn open_submenu(&self) -> Option<&'b Menu<Message>> {
        let menu: &'b Menu<Message> = self.menu;

        self.state.submenu.and_then(|index| menu.submenu_of(index))
    }

    /// İmlecin altındaki komut.
    fn target_at(&self, layout: Layout<'_>, position: Point) -> Option<Target> {
        let mut layouts = layout.children();
        let main = layouts.next()?.bounds();
        let sub = layouts.next().map(|layout| layout.bounds());

        if let (Some(bounds), Some(menu)) = (sub, self.open_submenu())
            && bounds.contains(position)
        {
            return menu.item_at(position.y - bounds.y).map(Target::Sub);
        }

        main.contains(position)
            .then(|| self.menu.item_at(position.y - main.y))
            .flatten()
            .map(Target::Main)
    }

    fn is_over(&self, layout: Layout<'_>, cursor: mouse::Cursor) -> bool {
        cursor.position().is_some_and(|position| {
            layout
                .children()
                .any(|layout| layout.bounds().contains(position))
        })
    }

    /// Komutu çalıştırır: alt menüyü açar ya da mesajı yayınlayıp kapatır.
    fn activate(&mut self, target: Target, shell: &mut Shell<'_, Message>) {
        let menu: &'b Menu<Message> = self.menu;

        let item = match target {
            Target::Main(index) => menu.items.get(index),
            Target::Sub(index) => self.open_submenu().and_then(|menu| menu.items.get(index)),
        };

        match item {
            Some(Item::Command(Command {
                on_press: Some(message),
                ..
            })) => {
                shell.publish(message.clone());
                self.close(shell);
            }
            Some(Item::Submenu { menu, .. }) if !menu.is_empty() => {
                if let Target::Main(index) = target {
                    let first = menu.step(None, true);

                    self.set_submenu(Some(index), shell);
                    self.set_hovered(first.map(Target::Sub), shell);
                }
            }
            _ => {}
        }
    }

    fn key(&mut self, key: keyboard::Key<&str>, shell: &mut Shell<'_, Message>) -> bool {
        use keyboard::Key;
        use keyboard::key::Named;

        match key {
            Key::Named(Named::Escape) => {
                if let (Some(Target::Sub(_)), Some(parent)) =
                    (self.state.hovered, self.state.submenu)
                {
                    self.set_submenu(None, shell);
                    self.set_hovered(Some(Target::Main(parent)), shell);
                } else {
                    self.close(shell);
                }
            }
            Key::Named(Named::ArrowDown | Named::ArrowUp) => {
                let forward = key == Key::Named(Named::ArrowDown);

                let hovered = match self.state.hovered {
                    Some(Target::Sub(index)) => self
                        .open_submenu()
                        .and_then(|menu| menu.step(Some(index), forward))
                        .map(Target::Sub),
                    Some(Target::Main(index)) => {
                        self.set_submenu(None, shell);
                        self.menu.step(Some(index), forward).map(Target::Main)
                    }
                    None => self.menu.step(None, forward).map(Target::Main),
                };

                self.set_hovered(hovered, shell);
            }
            Key::Named(Named::ArrowRight) => {
                if let Some(Target::Main(index)) = self.state.hovered {
                    self.activate(Target::Main(index), shell);
                }
            }
            Key::Named(Named::ArrowLeft) => {
                if let (Some(Target::Sub(_)), Some(parent)) =
                    (self.state.hovered, self.state.submenu)
                {
                    self.set_submenu(None, shell);
                    self.set_hovered(Some(Target::Main(parent)), shell);
                }
            }
            Key::Named(Named::Enter | Named::Space) => {
                if let Some(target) = self.state.hovered {
                    self.activate(target, shell);
                }
            }
            _ => return false,
        }

        true
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Overlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);

        let main = self
            .main
            .as_widget_mut()
            .layout(self.main_tree, renderer, &limits);
        let size = main.size();

        // Sağa ya da aşağı sığmazsa alanın soluna ya da üstüne açılır.
        let anchor = self.anchor;
        let x = if anchor.x + self.gap.x + size.width <= bounds.width {
            anchor.x + self.gap.x
        } else {
            anchor.x + anchor.width - size.width
        };
        let below = anchor.y + anchor.height + self.gap.y;
        let y = if below + size.height <= bounds.height {
            below
        } else {
            anchor.y - self.gap.y - size.height
        };
        let origin = Point::new(
            x.clamp(0.0, (bounds.width - size.width).max(0.0)),
            y.clamp(0.0, (bounds.height - size.height).max(0.0)),
        );

        let mut children = vec![main.move_to(origin)];

        if let (Some(sub), Some(index)) = (self.sub.as_mut(), self.state.submenu) {
            let node = sub.as_widget_mut().layout(self.sub_tree, renderer, &limits);
            let sub_size = node.size();

            // Alt menünün ilk komutu üst komutla aynı hizada; sağa sığmazsa
            // sola açılır.
            let right = origin.x + size.width - SUBMENU_OVERLAP;
            let x = if right + sub_size.width <= bounds.width {
                right
            } else {
                (origin.x - sub_size.width + SUBMENU_OVERLAP).max(0.0)
            };
            let y = (origin.y + self.menu.offset(index) - PADDING)
                .min(bounds.height - sub_size.height)
                .max(0.0);

            children.push(node.move_to(Point::new(x, y)));
        }

        layout::Node::with_children(bounds, children)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let viewport = layout.bounds();
        let mut layouts = layout.children();

        if let Some(main) = layouts.next() {
            self.main.as_widget().draw(
                self.main_tree,
                renderer,
                theme,
                style,
                main,
                cursor,
                &viewport,
            );
        }

        if let (Some(sub), Some(layout)) = (self.sub.as_ref(), layouts.next()) {
            sub.as_widget().draw(
                self.sub_tree,
                renderer,
                theme,
                style,
                layout,
                cursor,
                &viewport,
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
        let over = self.is_over(layout, cursor);
        let target = cursor
            .position()
            .and_then(|position| self.target_at(layout, position));

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(Target::Main(index)) = target {
                    let submenu = self.menu.submenu_of(index).map(|_| index);
                    self.set_submenu(submenu, shell);
                }

                let selectable = target.filter(|target| match *target {
                    Target::Main(index) => self.menu.items[index].is_selectable(),
                    Target::Sub(index) => self
                        .open_submenu()
                        .is_some_and(|menu| menu.items[index].is_selectable()),
                });

                // Menünün dışına çıkınca vurgu kalkar; alt menü açık kalır.
                if over || self.state.hovered.is_some() {
                    self.set_hovered(selectable, shell);
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                if over {
                    shell.capture_event();
                } else {
                    let secondary = *button == mouse::Button::Right
                        || is_control_click(event, self.state.modifiers);

                    self.close(shell);

                    // Sağ tık (macOS'ta Control + tık) alttaki öğeye geçer ve
                    // menüyü orada açar; diğer tıklamalar yalnızca menüyü
                    // kapatır.
                    if !secondary {
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if over {
                    shell.capture_event();

                    if let Some(target) = target {
                        self.activate(target, shell);
                    }
                }
            }
            // Menü açıkken tekerlek menüyü kapatmaz ve alttaki öğelere geçmez
            // (harita yakınlaşmaz): macOS'ta iki parmakla tıklamanın ardından
            // izleme yüzeyi hemen küçük kaydırma olayları üretir; bunlar
            // menüyü açılır açılmaz kapatıyordu. Yerel menüler de böyledir.
            Event::Mouse(mouse::Event::WheelScrolled { .. }) => shell.capture_event(),
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if self.key(key.as_ref(), shell) {
                    shell.capture_event();
                }
            }
            Event::Window(window::Event::Unfocused) => self.close(shell),
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(position) = cursor.position() else {
            return mouse::Interaction::None;
        };

        let selectable = match self.target_at(layout, position) {
            Some(Target::Main(index)) => self.menu.items[index].is_selectable(),
            Some(Target::Sub(index)) => self
                .open_submenu()
                .is_some_and(|menu| menu.items[index].is_selectable()),
            None => false,
        };

        if selectable {
            mouse::Interaction::Pointer
        } else if self.is_over(layout, cursor) {
            mouse::Interaction::Idle
        } else {
            mouse::Interaction::None
        }
    }
}

/// macOS'un ikincil tıkı: Control basılıyken sol tık. Diğer sistemlerde
/// Control + tık seçime ekleme gibi işlerde kullanıldığı için sağ tık
/// sayılmaz.
fn is_control_click(event: &Event, modifiers: keyboard::Modifiers) -> bool {
    control_click(event, modifiers, cfg!(target_os = "macos"))
}

fn control_click(event: &Event, modifiers: keyboard::Modifiers, macos: bool) -> bool {
    macos
        && modifiers.control()
        && matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        )
}

/// Menü kutusu; `highlighted` komut vurgulanır.
fn panel<'a, Message: 'a>(
    menu: &Menu<Message>,
    highlighted: Option<usize>,
) -> Element<'a, Message> {
    let rows = menu
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| item_row(item, highlighted == Some(index)));

    container(Column::with_children(rows))
        .padding(PADDING)
        .width(menu.width())
        .style(style::container::popover)
        .into()
}

fn item_row<'a, Message: 'a>(item: &Item<Message>, highlighted: bool) -> Element<'a, Message> {
    let (glyph, text, shortcut, submenu, enabled, danger) = match item {
        Item::Command(command) => (
            // A check shows its ✓ when on, else its own icon (e.g. a snap kind's marker).
            match command.checked {
                Some(true) => Some(Icon::Check),
                Some(false) | None => command.icon,
            },
            command.label.clone(),
            command.shortcut.clone(),
            false,
            command.on_press.is_some(),
            command.danger,
        ),
        Item::Submenu { icon, label, menu } => {
            (*icon, label.clone(), None, true, !menu.is_empty(), false)
        }
        Item::Separator => {
            return container(rule::horizontal(1).style(style::field::hairline))
                .padding([0.0, PADDING])
                .height(SEPARATOR_HEIGHT)
                .align_y(Center)
                .into();
        }
        Item::Header(title) => {
            return container(label::caption(title.clone()))
                .padding([0, 8])
                .height(header_height())
                .align_y(Center)
                .into();
        }
    };

    let slot: Element<'a, Message> = match glyph {
        Some(glyph) => icon(glyph).size(14.0).into(),
        None => space::horizontal().width(14).into(),
    };

    let mut content = row![
        container(slot).width(ICON_SLOT).center_x(ICON_SLOT),
        label::body(text).width(Fill),
    ]
    .spacing(8)
    .align_y(Center);

    if let Some(shortcut) = shortcut {
        content = content.push(label::mono_caption(shortcut).style(move |theme: &Theme| {
            let t = Tokens::of(theme);

            iced::widget::text::Style {
                color: Some(if highlighted && enabled {
                    t.on_accent.scale_alpha(0.75)
                } else {
                    t.muted
                }),
            }
        }));
    }

    if submenu {
        content = content.push(icon(Icon::ChevronRight).size(12.0));
    }

    container(content)
        .padding([0, 8])
        .height(item_height())
        .width(Fill)
        .align_y(Center)
        .style(style::container::menu_item(highlighted, enabled, danger))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Menu<u8> {
        Menu::new()
            .separator()
            .header("Görünüm")
            .item("Yakınlaştır", 1_u8)
            .shortcut("Z")
            .item("Devre dışı", None)
            .separator()
            .separator()
            .submenu("Opaklık", Menu::new().check("%100", true, 2_u8))
            .item("Sil", 3_u8)
            .danger()
            .separator()
    }

    #[test]
    fn separators_are_not_doubled_or_leading() {
        let menu = sample().trimmed();

        assert!(!matches!(menu.items.first(), Some(Item::Separator)));
        assert!(!matches!(menu.items.last(), Some(Item::Separator)));
        assert_eq!(
            menu.items
                .iter()
                .filter(|item| matches!(item, Item::Separator))
                .count(),
            1
        );
    }

    #[test]
    fn rows_are_found_by_height() {
        let menu = sample().trimmed();

        // Başlık, iki komut, bölücü, alt menü, komut.
        assert_eq!(menu.item_at(PADDING + 1.0), Some(0));
        assert_eq!(menu.item_at(PADDING + header_height() + 1.0), Some(1));
        assert_eq!(
            menu.item_at(PADDING + header_height() + item_height() * 2.0 + 1.0),
            Some(3)
        );
        assert_eq!(
            menu.offset(4),
            PADDING + header_height() + item_height() * 2.0 + SEPARATOR_HEIGHT
        );
        assert_eq!(menu.item_at(-1.0), None);
    }

    #[test]
    fn keyboard_skips_headers_separators_and_disabled_commands() {
        let menu = sample().trimmed();

        assert_eq!(menu.step(None, true), Some(1));
        assert_eq!(menu.step(Some(1), true), Some(4));
        assert_eq!(menu.step(Some(5), true), Some(1));
        assert_eq!(menu.step(Some(1), false), Some(5));
        assert!(menu.submenu_of(4).is_some());
        assert!(menu.submenu_of(1).is_none());
    }

    #[test]
    fn control_click_is_a_secondary_click_on_macos_only() {
        let left = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let right = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        let control = keyboard::Modifiers::CTRL;

        assert!(control_click(&left, control, true));
        // Diğer sistemlerde Control + tık seçime ekler; menü açmaz.
        assert!(!control_click(&left, control, false));
        assert!(!control_click(&left, keyboard::Modifiers::empty(), true));
        // Komut (⌘) + tık macOS'ta seçime ekler.
        assert!(!control_click(&left, keyboard::Modifiers::LOGO, true));
        // Sağ tık zaten sağ tıktır; çevrilmez.
        assert!(!control_click(&right, control, true));
    }

    #[test]
    fn empty_menus_are_detected() {
        assert!(Menu::<u8>::new().separator().is_empty());
        assert!(!Menu::<u8>::new().header("Başlık").is_empty());
        assert!(
            (typography::scaled(MIN_WIDTH)..=typography::scaled(MAX_WIDTH))
                .contains(&Menu::new().item("Uzun bir komut adı", 1_u8).width())
        );
    }
}
