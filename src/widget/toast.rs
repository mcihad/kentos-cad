//! Bildirimler: alanın sağ alt köşesinde üst üste dizilen, kendiliğinden
//! kapanan kısa iletiler.
//!
//! ```text
//!                       ┌ 2 bildirim daha   Tümünü kapat ┐
//!   ┌───────────────────────────────────────────────────┐
//!   │ ⓘ Koordinat kopyalandı                          × │
//!   │   39.92000, 32.85000                              │
//!   │▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔                                │ ← kalan süre
//!   └───────────────────────────────────────────────────┘
//!   ┌───────────────────────────────────────────────────┐
//!   │ ✓ 3 çizim silindi  ×2              Geri al      × │
//!   └───────────────────────────────────────────────────┘
//! ```
//!
//! - Önem düzeyi ([`Severity`]) ikonun ve kalan süre çizgisinin rengidir.
//! - Eylem düğmesi (ör. "Geri al") eylemi yapar ve bildirimi kapatır.
//! - En yeni bildirim köşeye en yakın olandır. En fazla üç bildirim görünür;
//!   daha eskiler sayılır ve hepsi birden kapatılabilir. Aynı bildirim
//!   yinelenirse yenisi eklenmez: sayısı artar, süresi baştan başlar.
//! - Bilgi ve başarı 5, uyarı 8 saniyede kapanır; eylemli bildirim en az 8
//!   saniye durur. Hata kendiliğinden kapanmaz. İmleç bildirimlerin
//!   üzerindeyken süre durur.
//!
//! Bildirim kuyruğu ([`Toasts`]) uygulamanındır:
//!
//! ```ignore
//! self.toasts.push(Toast::success("3 çizim silindi").action("Geri al", Message::Undo));
//!
//! Toaster::new(map, &self.toasts, Message::ToastClosed)
//!
//! // update:
//! Message::ToastClosed(id) => self.toasts.dismiss(id),
//! ```

use std::iter;
use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::time::Instant;
use iced::widget::text::Wrapping;
use iced::widget::{Column, column, row, space};
use iced::{
    Background, Border, Element, Event, Length, Padding, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, border, mouse, window,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Tokens, typography};
use crate::widget::Severity;

/// Aynı anda görünen en fazla bildirim.
const VISIBLE: usize = 3;

/// Bildirimin genişliği, 12 piksellik gövde metnine göre.
const WIDTH: f32 = 340.0;

/// Alanın kenarına ve bildirimler arasındaki boşluk.
const GAP: f32 = 8.0;
const SPACING: f32 = 6.0;

/// Bildirimin iç boşlukları ve kapatma düğmesi.
const PAD_Y: f32 = 10.0;
const PAD_LEFT: f32 = 12.0;
const PAD_RIGHT: f32 = 6.0;
const CONTROL: f32 = 20.0;
const GLYPH: f32 = 12.0;

/// Eylem düğmesinin metin çevresindeki boşluğu.
const ACTION_PAD: Vector = Vector::new(8.0, 3.0);

/// Kendiliğinden kapanma süreleri.
const SHORT: Duration = Duration::from_secs(5);
const LONG: Duration = Duration::from_secs(8);

/// Kalan süre çizgisi yenilenirken iki kare arası.
const FRAME: Duration = Duration::from_millis(33);

/// Bildirimin kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(u64);

/// Bildirim: önem düzeyi, başlık, isteğe bağlı açıklama ve eylem.
#[derive(Debug, Clone)]
pub struct Toast<Message> {
    severity: Severity,
    title: String,
    body: Option<String>,
    action: Option<(String, Message)>,
    duration: Option<Duration>,
}

impl<Message> Toast<Message> {
    /// Bilgi ve başarı 5, uyarı 8 saniyede kapanır; hata kapanmaz.
    pub fn new(severity: Severity, title: impl Into<String>) -> Self {
        Self {
            severity,
            title: title.into(),
            body: None,
            action: None,
            duration: match severity {
                Severity::Info | Severity::Success => Some(SHORT),
                Severity::Warning => Some(LONG),
                Severity::Error => None,
            },
        }
    }

    pub fn info(title: impl Into<String>) -> Self {
        Self::new(Severity::Info, title)
    }

    pub fn success(title: impl Into<String>) -> Self {
        Self::new(Severity::Success, title)
    }

    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(Severity::Warning, title)
    }

    pub fn error(title: impl Into<String>) -> Self {
        Self::new(Severity::Error, title)
    }

    /// Başlığın altındaki açıklama; uzunsa satırlara bölünür.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Eylem düğmesi (ör. "Geri al"): basılınca `message` gönderilir ve
    /// bildirim kapanır. Eylemli bildirim en az 8 saniye durur.
    pub fn action(mut self, label: impl Into<String>, message: Message) -> Self {
        self.action = Some((label.into(), message));
        self.duration = self.duration.map(|duration| duration.max(LONG));
        self
    }

    /// Kapanma süresi.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Kendiliğinden kapanmaz.
    pub fn sticky(mut self) -> Self {
        self.duration = None;
        self
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Eylemi dışında aynı bildirim mi.
    fn same(&self, other: &Self) -> bool {
        self.severity == other.severity
            && self.title == other.title
            && self.body == other.body
            && self.action.as_ref().map(|(label, _)| label)
                == other.action.as_ref().map(|(label, _)| label)
    }
}

/// Bildirim kuyruğu, eskiden yeniye.
#[derive(Debug, Clone)]
pub struct Toasts<Message> {
    items: Vec<Item<Message>>,
    next: u64,
}

#[derive(Debug, Clone)]
struct Item<Message> {
    id: Id,
    toast: Toast<Message>,
    /// Aynı bildirimin kaç kez geldiği.
    count: u32,
    /// Her yinelemede artar; süre baştan başlar.
    revision: u32,
}

impl<Message> Default for Toasts<Message> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next: 0,
        }
    }
}

impl<Message> Toasts<Message> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bildirimi ekler. Aynısı zaten varsa yenisi eklenmez: sayısı artar,
    /// en yeni olur ve süresi baştan başlar.
    pub fn push(&mut self, toast: Toast<Message>) -> Id {
        if let Some(index) = self.items.iter().position(|item| item.toast.same(&toast)) {
            let mut item = self.items.remove(index);

            item.toast = toast;
            item.count += 1;
            item.revision += 1;

            let id = item.id;
            self.items.push(item);
            return id;
        }

        self.next += 1;
        let id = Id(self.next);

        self.items.push(Item {
            id,
            toast,
            count: 1,
            revision: 0,
        });

        id
    }

    /// Bildirimi kapatır; yoksa `false`.
    pub fn dismiss(&mut self, id: Id) -> bool {
        let count = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != count
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn contains(&self, id: Id) -> bool {
        self.items.iter().any(|item| item.id == id)
    }

    /// Bildirim ve kaç kez geldiği, eskiden yeniye.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Id, &Toast<Message>, u32)> {
        self.items
            .iter()
            .map(|item| (item.id, &item.toast, item.count))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// İçeriğin sağ alt köşesinde bildirimleri gösteren kap.
pub struct Toaster<'a, Message> {
    content: Element<'a, Message>,
    entries: Vec<Entry<'a, Message>>,
    /// Görünmeyen eski bildirimler; "Tümünü kapat" onları da kapatır.
    hidden: Vec<Id>,
    /// "n bildirim daha" ve "Tümünü kapat".
    more: [Element<'a, Message>; 2],
    on_dismiss: Box<dyn Fn(Id) -> Message + 'a>,
    /// Bildirimlerin alanın kenarlarına uzaklığı.
    padding: Padding,
}

struct Entry<'a, Message> {
    id: Id,
    severity: Severity,
    duration: Option<Duration>,
    revision: u32,
    action: Option<Message>,
    /// İleti (ikon, başlık, açıklama), eylem metni ve kapatma ikonu.
    parts: [Element<'a, Message>; 3],
}

const MESSAGE: usize = 0;
const ACTION: usize = 1;
const CLOSE: usize = 2;

impl<'a, Message: Clone + 'a> Toaster<'a, Message> {
    /// `content`'in köşesinde `toasts`'taki bildirimler; kapatılan
    /// bildirim `on_dismiss` ile bildirilir.
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        toasts: &Toasts<Message>,
        on_dismiss: impl Fn(Id) -> Message + 'a,
    ) -> Self {
        let skip = toasts.items.len().saturating_sub(VISIBLE);

        let entries = toasts.items[skip..]
            .iter()
            .map(|item| Entry::new(item))
            .collect();

        let hidden: Vec<Id> = toasts.items[..skip].iter().map(|item| item.id).collect();

        let more = [
            label::caption(format!("{} bildirim daha", hidden.len()))
                .wrapping(Wrapping::None)
                .into(),
            label::caption("Tümünü kapat")
                .style(style::text::accent)
                .wrapping(Wrapping::None)
                .into(),
        ];

        Self {
            content: content.into(),
            entries,
            hidden,
            more,
            on_dismiss: Box::new(on_dismiss),
            padding: Padding::new(GAP),
        }
    }

    /// Bildirimlerin alanın sağ ve alt kenarına uzaklığı (varsayılan 8
    /// piksel); ör. haritanın sağındaki gezinme çubuğunu açık bırakmak için.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message: Clone + 'a> Entry<'a, Message> {
    fn new(item: &Item<Message>) -> Self {
        let toast = &item.toast;

        let mut heading = row![label::strong(toast.title.clone())].spacing(6);

        if item.count > 1 {
            heading = heading.push(label::mono_caption(format!("×{}", item.count)));
        }

        let mut text = Column::new().push(heading).spacing(2);

        if let Some(body) = &toast.body {
            text = text.push(label::muted(body.clone()));
        }

        let message = row![
            icon(toast.severity.icon())
                .size(16.0)
                .tone(toast.severity.tone()),
            text,
        ]
        .spacing(10);

        let action: Element<'a, Message> = match &toast.action {
            Some((text, _)) => label::strong(text.clone())
                .style(style::text::accent)
                .wrapping(Wrapping::None)
                .into(),
            None => space::horizontal().width(0).into(),
        };

        Self {
            id: item.id,
            severity: toast.severity,
            duration: toast.duration,
            revision: item.revision,
            action: toast.action.as_ref().map(|(_, message)| message.clone()),
            parts: [
                column![message].into(),
                action,
                icon(Icon::Close).size(GLYPH).into(),
            ],
        }
    }

    fn tree(&self) -> Tree {
        Tree {
            tag: tree::Tag::stateless(),
            state: tree::State::None,
            children: self.parts.iter().map(Tree::new).collect(),
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, width: f32) -> Node {
        let close_x = width - PAD_RIGHT - CONTROL;

        let action = if self.action.is_some() {
            self.parts[ACTION].as_widget_mut().layout(
                &mut tree.children[ACTION],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(width / 2.0, f32::INFINITY)),
            )
        } else {
            Node::new(Size::ZERO)
        };

        let action_x = close_x - 2.0 - ACTION_PAD.x - action.size().width;
        let text_right = if self.action.is_some() {
            action_x - ACTION_PAD.x - 6.0
        } else {
            close_x - 6.0
        };
        let text_width = (text_right - PAD_LEFT).max(0.0);

        let message = self.parts[MESSAGE]
            .as_widget_mut()
            .layout(
                &mut tree.children[MESSAGE],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(text_width, f32::INFINITY)),
            )
            .move_to(Point::new(PAD_LEFT, PAD_Y));

        let action = action.move_to(Point::new(action_x, PAD_Y));

        let inset = (CONTROL - GLYPH) / 2.0;
        let close = self.parts[CLOSE]
            .as_widget_mut()
            .layout(
                &mut tree.children[CLOSE],
                renderer,
                &layout::Limits::new(Size::ZERO, Size::new(GLYPH, GLYPH)),
            )
            .move_to(Point::new(close_x + inset, PAD_Y - 2.0 + inset));

        let height = message.size().height.max(CONTROL - 4.0) + 2.0 * PAD_Y;

        Node::with_children(
            Size::new(width, height.round()),
            vec![message, action, close],
        )
    }
}

/// Bildirimin süresi: görünmeye başladığı an ve imleç üzerindeyken geçen
/// (sayılmayan) süre.
#[derive(Debug, Clone, Copy)]
struct Timer {
    id: Id,
    revision: u32,
    started: Instant,
    paused: Duration,
    paused_since: Option<Instant>,
    /// Kapanması bildirildi; bir daha bildirilmez.
    expired: bool,
}

impl Timer {
    fn new(id: Id, revision: u32, now: Instant, hovered: bool) -> Self {
        Self {
            id,
            revision,
            started: now,
            paused: Duration::ZERO,
            paused_since: hovered.then_some(now),
            expired: false,
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        let paused = self.paused
            + self
                .paused_since
                .map_or(Duration::ZERO, |since| now.saturating_duration_since(since));

        now.saturating_duration_since(self.started)
            .saturating_sub(paused)
    }

    fn pause(&mut self, now: Instant) {
        self.paused_since.get_or_insert(now);
    }

    fn resume(&mut self, now: Instant) {
        if let Some(since) = self.paused_since.take() {
            self.paused += now.saturating_duration_since(since);
        }
    }
}

/// Bildirimin tıklanabilir bölgeleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Action(Id),
    Close(Id),
    ClearAll,
}

#[derive(Default)]
struct State {
    timers: Vec<Timer>,
    hovering: bool,
    hot: Option<Target>,
    pressed: Option<Target>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Toaster<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let now = Instant::now();

        tree::State::new(State {
            timers: self
                .entries
                .iter()
                .map(|entry| Timer::new(entry.id, entry.revision, now, false))
                .collect(),
            ..State::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        iter::once(Tree::new(&self.content))
            .chain(self.more.iter().map(Tree::new))
            .chain(self.entries.iter().map(Entry::tree))
            .collect()
    }

    /// Bildirim ağaçları ve süreleri kimliklerine göre eşlenir; kapanan
    /// bildirimin yerine geçen bildirimin süresi karışmaz.
    fn diff(&self, tree: &mut Tree) {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let now = Instant::now();

        if children.len() < 3 {
            *children = self.children();
            state.timers = self
                .entries
                .iter()
                .map(|entry| Timer::new(entry.id, entry.revision, now, state.hovering))
                .collect();
            return;
        }

        children[0].diff(&self.content);
        children[1].diff(&self.more[0]);
        children[2].diff(&self.more[1]);

        let mut previous: Vec<(Timer, Tree)> =
            state.timers.drain(..).zip(children.drain(3..)).collect();

        for entry in &self.entries {
            let (timer, tree) = match previous.iter().position(|(timer, _)| timer.id == entry.id) {
                Some(index) => {
                    let (mut timer, mut tree) = previous.swap_remove(index);
                    tree.diff_children(&entry.parts);

                    // Yinelenen bildirimin süresi baştan başlar.
                    if timer.revision != entry.revision {
                        timer = Timer::new(entry.id, entry.revision, now, state.hovering);
                    }

                    (timer, tree)
                }
                None => (
                    Timer::new(entry.id, entry.revision, now, state.hovering),
                    entry.tree(),
                ),
            };

            state.timers.push(timer);
            children.push(tree);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let area = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let children = &mut tree.children;

        let content = self.content.as_widget_mut().layout(
            &mut children[0],
            renderer,
            &layout::Limits::new(area, area),
        );

        let padding = self.padding;
        let width = typography::scaled(WIDTH)
            .min(area.width - padding.left - padding.right)
            .max(0.0);
        let left = area.width - padding.right - width;

        // En yeni bildirim en altta; eskiler üstüne dizilir.
        let mut bottom = area.height - padding.bottom;
        let mut toasts: Vec<Node> = self
            .entries
            .iter_mut()
            .zip(children.iter_mut().skip(3))
            .rev()
            .map(|(entry, tree)| {
                let node = entry.layout(tree, renderer, width);
                let top = bottom - node.size().height;
                bottom = top - SPACING;

                node.move_to(Point::new(left, top))
            })
            .collect();
        toasts.reverse();

        let more = if self.hidden.is_empty() {
            Node::with_children(
                Size::ZERO,
                vec![Node::new(Size::ZERO), Node::new(Size::ZERO)],
            )
        } else {
            let limits = layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
            let count = self.more[0]
                .as_widget_mut()
                .layout(&mut children[1], renderer, &limits);
            let clear = self.more[1]
                .as_widget_mut()
                .layout(&mut children[2], renderer, &limits);

            let height = count.size().height.max(clear.size().height) + 8.0;
            let pill = Size::new(count.size().width + clear.size().width + 3.0 * 10.0, height);

            let count = count.move_to(Point::new(10.0, 4.0));
            let clear_x = pill.width - 10.0 - clear.size().width;
            let clear = clear.move_to(Point::new(clear_x, 4.0));

            Node::with_children(pill, vec![count, clear]).move_to(Point::new(
                area.width - padding.right - pill.width,
                bottom - height,
            ))
        };

        Node::with_children(
            area,
            iter::once(content)
                .chain(iter::once(more))
                .chain(toasts)
                .collect(),
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };

        let mut layouts = layout.children();
        let (Some(content), Some(more)) = (layouts.next(), layouts.next()) else {
            return;
        };
        let toasts: Vec<Layout<'_>> = layouts.collect();

        let over = cursor.position().is_some_and(|point| {
            (!self.hidden.is_empty() && more.bounds().contains(point))
                || toasts.iter().any(|toast| toast.bounds().contains(point))
        });

        // İmleç bildirimlerin üzerindeyken süreler durur.
        if over != state.hovering {
            state.hovering = over;

            for timer in &mut state.timers {
                if over {
                    timer.pause(now);
                } else {
                    timer.resume(now);
                }
            }
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            let mut running = false;

            for (timer, entry) in state.timers.iter_mut().zip(&self.entries) {
                let Some(duration) = entry.duration else {
                    continue;
                };

                if timer.expired {
                    continue;
                }

                if timer.elapsed(now) >= duration {
                    timer.expired = true;
                    shell.publish((self.on_dismiss)(entry.id));
                } else if timer.paused_since.is_none() {
                    running = true;
                }
            }

            if running {
                shell.request_redraw_at(now + FRAME);
            }
        }

        let target = cursor
            .position()
            .and_then(|point| target(&self.entries, &toasts, more, !self.hidden.is_empty(), point));

        if let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event
            && target != state.hot
        {
            state.hot = target;
            shell.request_redraw();
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if target.is_some() => {
                state.pressed = target;
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.pressed.is_some() =>
            {
                let pressed = state.pressed.take();

                if pressed == target {
                    match pressed {
                        Some(Target::Action(id)) => {
                            if let Some(message) = self
                                .entries
                                .iter()
                                .find(|entry| entry.id == id)
                                .and_then(|entry| entry.action.clone())
                            {
                                shell.publish(message);
                            }

                            shell.publish((self.on_dismiss)(id));
                        }
                        Some(Target::Close(id)) => shell.publish((self.on_dismiss)(id)),
                        Some(Target::ClearAll) => {
                            for id in self
                                .hidden
                                .iter()
                                .copied()
                                .chain(self.entries.iter().map(|entry| entry.id))
                            {
                                shell.publish((self.on_dismiss)(id));
                            }
                        }
                        None => {}
                    }
                }

                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }

        // Bildirimlerin üzerindeki basış ve tekerlek arkadaki içeriğe geçmez.
        if over
            && matches!(
                event,
                Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::WheelScrolled { .. })
            )
        {
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            &mut children[0],
            event,
            content,
            if over { cursor.levitate() } else { cursor },
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        let (Some(content), Some(more)) = (layouts.next(), layouts.next()) else {
            return mouse::Interaction::None;
        };
        let toasts: Vec<Layout<'_>> = layouts.collect();

        if let Some(point) = cursor.position() {
            if target(&self.entries, &toasts, more, !self.hidden.is_empty(), point).is_some() {
                return mouse::Interaction::Pointer;
            }

            if (!self.hidden.is_empty() && more.bounds().contains(point))
                || toasts.iter().any(|toast| toast.bounds().contains(point))
            {
                return mouse::Interaction::Idle;
            }
        }

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            content,
            cursor,
            viewport,
            renderer,
        )
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
        let bounds = layout.bounds();
        let mut layouts = layout.children();
        let (Some(content), Some(more)) = (layouts.next(), layouts.next()) else {
            return;
        };
        let toasts: Vec<Layout<'_>> = layouts.collect();

        let over = cursor.position().is_some_and(|point| {
            (!self.hidden.is_empty() && more.bounds().contains(point))
                || toasts.iter().any(|toast| toast.bounds().contains(point))
        });

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            content,
            if over { cursor.levitate() } else { cursor },
            viewport,
        );

        if self.entries.is_empty() {
            return;
        }

        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        let t = Tokens::of(theme);
        let now = Instant::now();
        let text = renderer::Style { text_color: t.text };

        renderer.with_layer(clip, |renderer| {
            if !self.hidden.is_empty() {
                frame(renderer, &t, more.bounds(), false);

                for ((part, tree), layout) in self
                    .more
                    .iter()
                    .zip(&tree.children[1..3])
                    .zip(more.children())
                {
                    part.as_widget()
                        .draw(tree, renderer, theme, &text, layout, cursor, &clip);
                }

                if matches!(state.hot, Some(Target::ClearAll)) {
                    underline(renderer, more.children().nth(1), t.accent);
                }
            }

            for ((entry, tree), layout) in self
                .entries
                .iter()
                .zip(tree.children.iter().skip(3))
                .zip(&toasts)
            {
                let toast = layout.bounds();

                frame(renderer, &t, toast, true);

                let mut parts = layout.children();
                let (Some(message), Some(action), Some(close)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };

                entry.parts[MESSAGE].as_widget().draw(
                    &tree.children[MESSAGE],
                    renderer,
                    theme,
                    &text,
                    message,
                    mouse::Cursor::Unavailable,
                    &clip,
                );

                if entry.action.is_some() {
                    let area = action_area(action.bounds());
                    let pressed = state.pressed == Some(Target::Action(entry.id));

                    if pressed || state.hot == Some(Target::Action(entry.id)) {
                        highlight(renderer, &t, area, pressed);
                    }

                    entry.parts[ACTION].as_widget().draw(
                        &tree.children[ACTION],
                        renderer,
                        theme,
                        &text,
                        action,
                        mouse::Cursor::Unavailable,
                        &clip,
                    );
                }

                let pressed = state.pressed == Some(Target::Close(entry.id));
                let hot = pressed || state.hot == Some(Target::Close(entry.id));

                if hot {
                    highlight(renderer, &t, close_area(close.bounds()), pressed);
                }

                entry.parts[CLOSE].as_widget().draw(
                    &tree.children[CLOSE],
                    renderer,
                    theme,
                    &renderer::Style {
                        text_color: if hot { t.text } else { t.muted },
                    },
                    close,
                    mouse::Cursor::Unavailable,
                    &clip,
                );

                // Kalan süre: önem renginde, soldan sağa kısalan çizgi.
                if let (Some(duration), Some(timer)) = (
                    entry.duration,
                    state.timers.iter().find(|timer| timer.id == entry.id),
                ) {
                    let left = 1.0 - timer.elapsed(now).as_secs_f32() / duration.as_secs_f32();
                    let width = ((toast.width - 2.0) * left.clamp(0.0, 1.0)).round();

                    if width > 0.0 {
                        renderer.fill_quad(
                            Quad {
                                bounds: Rectangle {
                                    x: toast.x + 1.0,
                                    y: toast.y + toast.height - 3.0,
                                    width,
                                    height: 2.0,
                                },
                                ..Quad::default()
                            },
                            Background::Color(entry.severity.color(&t)),
                        );
                    }
                }
            }
        });
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let content = layout.children().next()?;

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Toaster<'a, Message>> for Element<'a, Message> {
    fn from(toaster: Toaster<'a, Message>) -> Self {
        Element::new(toaster)
    }
}

/// Noktanın üzerindeki tıklanabilir bölge.
fn target<Message>(
    entries: &[Entry<'_, Message>],
    toasts: &[Layout<'_>],
    more: Layout<'_>,
    has_more: bool,
    point: Point,
) -> Option<Target> {
    for (entry, layout) in entries.iter().zip(toasts) {
        let mut parts = layout.children().skip(ACTION);

        if let Some(action) = parts.next()
            && entry.action.is_some()
            && action_area(action.bounds()).contains(point)
        {
            return Some(Target::Action(entry.id));
        }

        if let Some(close) = parts.next()
            && close_area(close.bounds()).contains(point)
        {
            return Some(Target::Close(entry.id));
        }
    }

    if has_more
        && let Some(clear) = more.children().nth(1)
        && clear.bounds().expand(4.0).contains(point)
    {
        return Some(Target::ClearAll);
    }

    None
}

fn action_area(text: Rectangle) -> Rectangle {
    text.expand([ACTION_PAD.y, ACTION_PAD.x])
}

fn close_area(glyph: Rectangle) -> Rectangle {
    glyph.expand((CONTROL - GLYPH) / 2.0)
}

/// Bildirimin zemini, kenarı ve gölgesi.
fn frame(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle, shadow: bool) {
    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                color: t.border,
                width: 1.0,
                radius: RADIUS.into(),
            },
            shadow: if shadow {
                Shadow {
                    color: t.shadow(),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 14.0,
                }
            } else {
                Shadow::default()
            },
            ..Quad::default()
        },
        Background::Color(t.popover),
    );
}

/// Üzerine gelinen ya da basılan düğmenin zemini.
fn highlight(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle, pressed: bool) {
    renderer.fill_quad(
        Quad {
            bounds,
            border: border::rounded(3.0),
            ..Quad::default()
        },
        Background::Color(t.layer(if pressed { 0.12 } else { 0.07 })),
    );
}

/// Bağlantı gibi davranan metnin altı çizgisi.
fn underline(renderer: &mut Renderer, text: Option<Layout<'_>>, color: iced::Color) {
    let Some(text) = text else {
        return;
    };

    let bounds = text.bounds();

    renderer.fill_quad(
        Quad {
            bounds: Rectangle {
                x: bounds.x,
                y: bounds.y + bounds.height - 1.0,
                width: bounds.width,
                height: 1.0,
            },
            ..Quad::default()
        },
        Background::Color(color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_toasts_are_counted_not_stacked() {
        let mut toasts = Toasts::new();

        let first = toasts.push(Toast::<()>::info("Koordinat kopyalandı"));
        toasts.push(Toast::warning("Yalnızca çizimler silinebilir"));
        let again = toasts.push(Toast::info("Koordinat kopyalandı"));

        assert_eq!(first, again);
        assert_eq!(toasts.len(), 2);

        // Yinelenen bildirim en yeni olur.
        let (id, toast, count) = toasts.iter().last().expect("bildirim yok");
        assert_eq!(
            (id, toast.title(), count),
            (first, "Koordinat kopyalandı", 2)
        );

        assert!(toasts.dismiss(first));
        assert!(!toasts.contains(first));
        assert!(!toasts.dismiss(first));
    }

    #[test]
    fn durations_follow_the_severity() {
        assert_eq!(Toast::<()>::info("a").duration, Some(SHORT));
        assert_eq!(Toast::<()>::warning("a").duration, Some(LONG));
        assert_eq!(Toast::<()>::error("a").duration, None);

        // Eylemli bildirim en az 8 saniye durur; hata yine kapanmaz.
        assert_eq!(
            Toast::success("a").action("Geri al", ()).duration,
            Some(LONG)
        );
        assert_eq!(Toast::error("a").action("Yeniden dene", ()).duration, None);
        assert_eq!(Toast::<()>::info("a").sticky().duration, None);
    }

    #[test]
    fn timers_stop_while_hovered() {
        let start = Instant::now();
        let mut timer = Timer::new(Id(1), 0, start, false);

        timer.pause(start + Duration::from_secs(2));
        assert_eq!(
            timer.elapsed(start + Duration::from_secs(10)),
            Duration::from_secs(2)
        );

        timer.resume(start + Duration::from_secs(10));
        assert_eq!(
            timer.elapsed(start + Duration::from_secs(11)),
            Duration::from_secs(3)
        );
    }
}
