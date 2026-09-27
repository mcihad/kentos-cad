//! Harf ipucu (şeridin KeyTip'i, web'in `.keytips__tip`'i): bir öğenin
//! üstünde, klavyeyle ona ulaşmak için yazılacak harfleri gösteren küçük
//! kutu. Kutu öğenin katmanında değil, onun üstündeki katmanda çizilir; öğenin
//! kenarından taşabilir (sekmede altına, büyük düğmede alt kenarına, küçük
//! düğmede ikonunun üstüne; [`Place`]). Yazılanla başlamayan ipucu
//! soluklaşır ([`KeyTip::dim`]).

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::font::Weight;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{
    Background, Border, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Shadow,
    Size, Theme, Vector, mouse,
};

use crate::theme::{Tokens, typography};

/// Bir öğenin harf ipucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyTip {
    /// Yazılacak harfler (`DO`, `3`).
    pub tip: String,
    /// Yazılanla başlamıyor: soluk çizilir.
    pub dim: bool,
}

impl KeyTip {
    pub fn new(tip: impl Into<String>, dim: bool) -> Self {
        Self {
            tip: tip.into(),
            dim,
        }
    }
}

/// Kutunun öğeye göre yeri (web'in `KeyTips.badge`'i).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Place {
    /// Sekmenin altının ortası, 6 piksel yukarıda.
    Under,
    /// Büyük düğmenin altının ortası, 7 piksel yukarıda.
    Bottom,
    /// Küçük düğmede ikonun ortası (öğenin solundan uzaklığı), dikey ortanın
    /// 2 piksel üstü: yanındaki yazı okunur kalır.
    Icon(f32),
}

/// `content`, harf ipucuyla; ipucu yoksa olduğu gibi.
pub fn key_tip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tip: Option<KeyTip>,
    place: Place,
) -> Element<'a, Message> {
    match tip {
        Some(tip) => Element::new(Tipped {
            content: content.into(),
            tip,
            place,
        }),
        None => content.into(),
    }
}

struct Tipped<'a, Message> {
    content: Element<'a, Message>,
    tip: KeyTip,
    place: Place,
}

impl<Message> Widget<Message, Theme, Renderer> for Tipped<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
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
        let bounds = layout.bounds() + translation;
        let at = match self.place {
            Place::Under => Point::new(bounds.center_x(), bounds.y + bounds.height - 6.0),
            Place::Bottom => Point::new(bounds.center_x(), bounds.y + bounds.height - 7.0),
            Place::Icon(x) => Point::new(bounds.x + x, bounds.center_y() - 2.0),
        };
        let badge = overlay::Element::new(Box::new(Badge {
            at,
            tip: self.tip.clone(),
        }));
        match self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        ) {
            Some(content) => Some(overlay::Group::with_children(vec![content, badge]).overlay()),
            None => Some(badge),
        }
    }
}

/// The box: its top middle at `at` (the window's point).
struct Badge {
    at: Point,
    tip: KeyTip,
}

impl Badge {
    fn size(&self) -> (Size, Pixels) {
        let text = Pixels(typography::caption().round());
        let height = typography::scaled(16.0).round();
        let width = (typography::mono_width(&self.tip.tip, text.0) + typography::scaled(8.0))
            .max(typography::scaled(16.0))
            .round();
        (Size::new(width, height), text)
    }
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Badge {
    fn layout(&mut self, _renderer: &Renderer, _bounds: Size) -> layout::Node {
        let (size, _) = self.size();
        layout::Node::new(size).move_to(Point::new(
            (self.at.x - size.width / 2.0).round(),
            self.at.y.round(),
        ))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
        let t = Tokens::of(theme);
        let (_, size) = self.size();
        let bounds = layout.bounds();
        let alpha = if self.tip.dim { 0.25 } else { 1.0 };
        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: t.border.scale_alpha(alpha),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                shadow: Shadow {
                    color: t.shadow().scale_alpha(alpha),
                    offset: Vector::new(0.0, 1.0),
                    blur_radius: 4.0,
                },
                ..Quad::default()
            },
            Background::Color(t.popover.scale_alpha(alpha)),
        );
        renderer.fill_text(
            Text {
                content: self.tip.tip.clone(),
                bounds: bounds.size(),
                size,
                line_height: LineHeight::Absolute(Pixels(bounds.height)),
                font: Font {
                    weight: Weight::Semibold,
                    ..typography::mono()
                },
                align_x: text::Alignment::Center,
                align_y: Vertical::Center,
                shaping: Shaping::Basic,
                wrapping: Wrapping::None,
            },
            bounds.center(),
            t.text.scale_alpha(alpha),
            bounds,
        );
    }

    fn operate(
        &mut self,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        _operation: &mut dyn Operation,
    ) {
    }
}
