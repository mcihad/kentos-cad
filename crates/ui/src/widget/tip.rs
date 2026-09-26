//! Zengin ipuçları (DESIGN.md §7.8).
//!
//! İpucu imleç öğenin üzerinde 450 ms durunca açılır; bir ipucu kapandıktan
//! sonraki 600 ms içinde komşu öğelerinki hemen açılır (araç çubuğunda
//! gezinirken). Tıklamada kapanır ve imleç öğeden çıkana dek yeniden
//! açılmaz; öğenin kendi açılır menüsü açıkken hiç görünmez (menünün
//! üstüne binmesin).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::widget::tooltip::Position;
use iced::widget::{column, container, tooltip};
use iced::{Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced::{mouse, window};

use crate::label;
use crate::style;
use crate::theme::typography;

/// İpucu içeriği: başlık, isteğe bağlı açıklama ve eş aralıklı bir ayrıntı
/// satırı (ör. komut satırı karşılığı).
///
/// Yalnızca başlığı olan ipucu tek satırlık sade bir etiket olarak gösterilir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    title: String,
    body: Option<String>,
    detail: Option<String>,
}

impl Tip {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: None,
            detail: None,
        }
    }

    /// Ne yaptığını anlatan açıklama.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Eş aralıklı ayrıntı satırı (ör. "Komut: CIZGI").
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn view<'a, Message: 'a>(self) -> Element<'a, Message> {
        if self.body.is_none() && self.detail.is_none() {
            return container(label::caption(self.title).style(style::text::default))
                .padding([4, 8])
                .style(style::container::popover)
                .into();
        }

        let mut content = column![label::strong(self.title)]
            .spacing(3)
            .max_width(typography::scaled(260.0));

        if let Some(body) = self.body {
            content = content.push(label::caption(body));
        }

        if let Some(detail) = self.detail {
            content = content.push(label::mono_caption(detail));
        }

        container(content)
            .padding([7, 10])
            .style(style::container::popover)
            .into()
    }
}

impl From<String> for Tip {
    fn from(title: String) -> Self {
        Self::new(title)
    }
}

impl From<&str> for Tip {
    fn from(title: &str) -> Self {
        Self::new(title)
    }
}

/// `content`'e ipucu ekler.
pub fn tip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tip: Tip,
    position: tooltip::Position,
) -> Element<'a, Message> {
    Element::new(Hint {
        content: content.into(),
        tip: tip.view(),
        position,
    })
}

/// İpucunun açılmadan önceki beklemesi.
const DELAY: Duration = Duration::from_millis(450);
/// Bir ipucu kapandıktan sonra komşunun ipucunun hemen açıldığı süre.
const WARM: Duration = Duration::from_millis(600);
/// Öğe ile ipucu arasındaki boşluk ve ipucunun pencere kenarından payı.
const GAP: f32 = 5.0;
const PADDING: f32 = 5.0;

/// Son ipucunun kapandığı an (sıcak süre için).
static CLOSED: Mutex<Option<Instant>> = Mutex::new(None);

fn warm(now: Instant) -> bool {
    CLOSED
        .lock()
        .ok()
        .and_then(|closed| *closed)
        .is_some_and(|at| now.saturating_duration_since(at) < WARM)
}

fn closed(now: Instant) {
    if let Ok(mut closed) = CLOSED.lock() {
        *closed = Some(now);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    /// İmleç `at`'te geldi; ipucu beklemeden sonra açılır.
    Waiting {
        at: Instant,
    },
    Open,
    /// Tıklandı: imleç çıkana dek ipucu yok.
    Pressed,
}

/// Bir olaydan sonraki durum: imleç öğenin üzerinde mi (`over`), fareye
/// basıldı mı (`pressed`), sıcak süre içinde mi (`warm`). Yanında, beklemenin
/// sonunda yeniden bakılacak an ve açık ipucunun bu olayla kapanıp
/// kapanmadığı (sıcak süreyi başlatır; tıklama başlatmaz).
fn step(
    state: State,
    over: bool,
    pressed: bool,
    now: Instant,
    warm: bool,
) -> (State, Option<Instant>, bool) {
    match (state, over) {
        (State::Open, false) => (State::Idle, None, true),
        (_, false) => (State::Idle, None, false),
        (_, true) if pressed => (State::Pressed, None, false),
        (State::Pressed, true) => (State::Pressed, None, false),
        (State::Open, true) => (State::Open, None, false),
        (State::Idle, true) if warm => (State::Open, None, false),
        (State::Idle, true) => (State::Waiting { at: now }, Some(now + DELAY), false),
        (State::Waiting { at }, true) if now.saturating_duration_since(at) >= DELAY => {
            (State::Open, None, false)
        }
        (State::Waiting { at }, true) => (State::Waiting { at }, Some(at + DELAY), false),
    }
}

/// Öğe ve ipucu: iced'in `Tooltip`'i gibi, §7.8'in kurallarıyla.
struct Hint<'a, Message> {
    content: Element<'a, Message>,
    tip: Element<'a, Message>,
    position: Position,
}

impl<Message> Widget<Message, Theme, Renderer> for Hint<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.tip)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget(), self.tip.as_widget()]);
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
        if let Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_)) = event {
            let state = tree.state.downcast_mut::<State>();
            let now = Instant::now();
            let over = cursor.position_over(layout.bounds()).is_some();
            let pressed = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)));
            let (next, recheck, was_closed) = step(*state, over, pressed, now, warm(now));
            if was_closed {
                closed(now);
            }
            if let Some(at) = recheck {
                shell.request_redraw_at(at);
            }
            if next != *state {
                let shown = |s: State| s == State::Open;
                if shown(next) != shown(*state) {
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                *state = next;
            }
        }
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
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
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let open = *tree.state.downcast_ref::<State>() == State::Open;
        let (content_tree, tip_tree) = match tree.children.as_mut_slice() {
            [content, tip] => (content, tip),
            _ => return None,
        };
        // The element's own menu, when open, hides the tip: it would lie over the menu.
        if let Some(menu) = self.content.as_widget_mut().overlay(
            content_tree,
            layout,
            renderer,
            viewport,
            translation,
        ) {
            return Some(menu);
        }
        open.then(|| {
            overlay::Element::new(Box::new(Bubble {
                tip: &mut self.tip,
                tree: tip_tree,
                anchor: Rectangle::new(layout.position() + translation, layout.bounds().size()),
                position: self.position,
            }))
        })
    }
}

/// The open tip, laid out beside its element and kept inside the window.
struct Bubble<'a, 'b, Message> {
    tip: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    anchor: Rectangle,
    position: Position,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Bubble<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let window = Rectangle::with_size(bounds);
        let node = self.tip.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds).shrink(Padding::new(PADDING)),
        );
        let size = node.size();
        let a = self.anchor;
        let centre_x = a.x + (a.width - size.width) / 2.0;
        let centre_y = a.y + (a.height - size.height) / 2.0;
        let at = match self.position {
            Position::Top => Point::new(centre_x, a.y - size.height - GAP - PADDING),
            Position::Bottom => Point::new(centre_x, a.y + a.height + GAP + PADDING),
            Position::Left => Point::new(a.x - size.width - GAP - PADDING, centre_y),
            Position::Right => Point::new(a.x + a.width + GAP + PADDING, centre_y),
            Position::FollowCursor => Point::new(a.x, a.y - size.height),
        };
        let x = at.x.clamp(
            window.x + PADDING,
            (window.width - size.width - PADDING).max(PADDING),
        );
        let y = at.y.clamp(
            window.y + PADDING,
            (window.height - size.height - PADDING).max(PADDING),
        );
        node.move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.tip.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &Rectangle::with_size(Size::INFINITE),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{DELAY, State, step};

    #[test]
    fn a_tip_waits_then_opens_and_closes_when_the_cursor_leaves() {
        let t0 = Instant::now();
        let (s, recheck, _) = step(State::Idle, true, false, t0, false);
        assert_eq!(s, State::Waiting { at: t0 });
        assert_eq!(recheck, Some(t0 + DELAY));
        // Moving about within the delay keeps waiting from the first moment.
        let (s, _, _) = step(s, true, false, t0 + Duration::from_millis(200), false);
        assert_eq!(s, State::Waiting { at: t0 });
        let (s, _, _) = step(s, true, false, t0 + DELAY, false);
        assert_eq!(s, State::Open);
        // Leaving closes it and starts the warm period.
        let (s, _, closed) = step(s, false, false, t0 + Duration::from_secs(1), false);
        assert_eq!((s, closed), (State::Idle, true));
        // Leaving before it opened is not a close.
        let (s, _, _) = step(State::Idle, true, false, t0, false);
        let (_, _, closed) = step(s, false, false, t0, false);
        assert!(!closed);
    }

    #[test]
    fn a_neighbour_opens_at_once_while_warm() {
        let t0 = Instant::now();
        let (s, recheck, _) = step(State::Idle, true, false, t0, true);
        assert_eq!((s, recheck), (State::Open, None));
    }

    #[test]
    fn a_click_closes_the_tip_until_the_cursor_leaves() {
        let t0 = Instant::now();
        let (s, _, closed) = step(State::Open, true, true, t0, false);
        assert_eq!(s, State::Pressed);
        assert!(!closed, "a click does not warm the neighbours");
        // Staying over the element, even after the delay, brings no tip.
        let (s, _, _) = step(s, true, false, t0 + Duration::from_secs(2), true);
        assert_eq!(s, State::Pressed);
        // Out and in again: it waits and opens as usual.
        let (s, _, _) = step(s, false, false, t0 + Duration::from_secs(3), false);
        assert_eq!(s, State::Idle);
        let (s, _, _) = step(s, true, false, t0 + Duration::from_secs(4), false);
        assert!(matches!(s, State::Waiting { .. }));
    }
}
