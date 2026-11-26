//! The drawing's overlays on the live view (plan/render.md RM-05, docs/adr/9205): what the app built for its camera,
//! drawn where the area's live camera puts it while a gesture runs.
//!
//! An overlay (the labels, the marks and grips, a data layer's labels, a terrain's or an alignment's marks) is built in
//! the app's view, for the app's camera. While the wheel turns or the middle button drags, the area draws with its
//! live camera and the app builds nothing (`live.rs`). The overlay is then moved and scaled from the camera it was
//! built for to the live one: under one turn and one map system the two put a point's screen place a scale and a
//! shift apart, so the overlay's every point lands where the scene draws the same world point. Under another turn,
//! or with a card that cannot be moved (it says what lies under the pointer), it is not drawn until the app builds it
//! again, once the view settles. Before, a frame drawn without the app's view would have left them where the view
//! started: the stuck layers of the owner's complaint.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay};
use iced::{Element, Event, Length, Rectangle, Size, Transformation, Vector, mouse};
use kentos_render_wgpu::Camera;

use super::live::LiveView;

/// How an overlay goes with the live view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Follows {
    /// Moved and scaled with it: the drawing's marks and labels.
    Moved,
    /// Drawn only while the view is the one it was built for: a card naming what lies under the pointer, a text
    /// being edited.
    Still,
    /// Moved, never scaled, with this world point (docs/adr/9251): a running tool's measurement and its value field
    /// beside the pointer stay beside the place they were laid out at, at their size on the screen.
    Pinned(kentos_render_wgpu::Vec2),
}

/// The overlay `content`, built for `built`, on the live view `live`.
pub struct Follow<'a, Message, Theme, Renderer> {
    built: Camera,
    live: LiveView,
    follows: Follows,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer> Follow<'a, Message, Theme, Renderer> {
    pub fn new(
        built: Camera,
        live: LiveView,
        follows: Follows,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self { built, live, follows, content: content.into() }
    }
}

/// From the screen of `built` to the screen of `now`, in the area's own pixels: the scale and the shift that put a
/// point's place under `built` at its place under `now`. None when no such pair does it (another turn or map system).
pub fn moved(built: &Camera, now: &Camera) -> Option<(f32, Vector)> {
    if built == now {
        return Some((1.0, Vector::ZERO));
    }
    let same_system = match (built.map, now.map) {
        (None, None) => true,
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        _ => false,
    };
    if built.twist != now.twist || !same_system || built.map_px() <= 0.0 {
        return None;
    }
    let k = now.map_px() / built.map_px();
    // The area's corner under `built`, a map point, is where `now` puts it: the shift.
    let corner = now.map_to_screen(built.screen_to_map(0.0, 0.0));
    let shift = Vector::new(corner[0] as f32, corner[1] as f32);
    (k.is_finite() && shift.x.is_finite() && shift.y.is_finite()).then_some((k as f32, shift))
}

/// How far the screen of `now` puts the world point `p` from where `built` put it, in whole pixels as a card's layout
/// places it: what moves a card pinned to `p` (any turn or map system). None where either camera does not reach it.
pub fn pinned(built: &Camera, now: &Camera, p: kentos_render_wgpu::Vec2) -> Option<Vector> {
    let ([ax, ay], [bx, by]) = (built.world_to_screen(p), now.world_to_screen(p));
    let shift = Vector::new((bx - ax).round() as f32, (by - ay).round() as f32);
    (shift.x.is_finite() && shift.y.is_finite()).then_some(shift)
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Follow<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
        let now = self.live.camera();
        let draw = |renderer: &mut Renderer| {
            self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
        };
        if now == self.built {
            draw(renderer);
            return;
        }
        let bounds = layout.bounds();
        let to = match self.follows {
            Follows::Still => return,
            Follows::Pinned(p) => {
                let Some(by) = pinned(&self.built, &now, p) else {
                    return;
                };
                Transformation::translate(by.x, by.y)
            }
            Follows::Moved => {
                let Some((k, shift)) = moved(&self.built, &now) else {
                    return;
                };
                let origin = Vector::new(bounds.x, bounds.y);
                Transformation::translate(origin.x + shift.x, origin.y + shift.y)
                    * Transformation::scale(k)
                    * Transformation::translate(-origin.x, -origin.y)
            }
        };
        renderer.with_layer(bounds, |renderer| {
            renderer.with_transformation(to, |renderer| draw(renderer));
        });
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
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
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Follow<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(follow: Follow<'a, Message, Theme, Renderer>) -> Self {
        Element::new(follow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        let mut c = Camera::default();
        c.set_size(800.0, 600.0);
        c.scale = 2.0;
        c
    }

    /// Every point of an overlay built for one camera lands, moved and scaled, where the other camera puts its world
    /// point: a pan, a zoom at the pointer, both.
    #[test]
    fn an_overlay_point_lands_where_the_live_camera_puts_its_world_point() {
        let built = camera();
        let mut now = built;
        now.zoom_at(1.15, 120.0, 340.0);
        now.pan_by(-37.0, 12.5);
        let (k, shift) = moved(&built, &now).expect("one turn, one system");
        for (x, y) in [(0.0, 0.0), (120.0, 340.0), (799.0, 1.0), (400.0, 599.0)] {
            let world = built.screen_to_world(x, y);
            let [ex, ey] = now.world_to_screen(world);
            let (gx, gy) = (k * x as f32 + shift.x, k * y as f32 + shift.y);
            assert!((f64::from(gx) - ex).abs() < 1e-3 && (f64::from(gy) - ey).abs() < 1e-3, "({x}, {y})");
        }
        assert_eq!(moved(&built, &built), Some((1.0, Vector::ZERO)));
    }

    /// A card pinned to a world point (docs/adr/9251) goes where the live camera puts the point, by whole pixels as its
    /// layout placed it, never scaled; a turn of the view moves it too, the card itself does not turn.
    #[test]
    fn a_pinned_card_goes_with_its_world_point_by_whole_pixels() {
        let built = camera();
        let p = built.screen_to_world(500.0, 200.0);
        let mut now = built;
        now.zoom_at(1.15, 120.0, 340.0);
        now.pan_by(-37.0, 12.5);
        let by = pinned(&built, &now, p).expect("a shift");
        let [x, y] = now.world_to_screen(p);
        assert!((500.0 + f64::from(by.x) - x).abs() <= 0.5 && (200.0 + f64::from(by.y) - y).abs() <= 0.5, "{by:?}");
        assert!(by.x.fract() == 0.0 && by.y.fract() == 0.0, "whole pixels: {by:?}");
        assert_eq!(pinned(&built, &built, p), Some(Vector::ZERO));
        let mut turned = built;
        turned.twist = 0.3;
        assert!(pinned(&built, &turned, p).is_some(), "a turn moves the card's place");
    }

    #[test]
    fn another_turn_is_not_a_move() {
        let built = camera();
        let mut turned = built;
        turned.twist = 0.3;
        assert!(moved(&built, &turned).is_none());
    }
}
