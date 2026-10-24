//! The drawing's text over the scene under true sizes, as a widget of its own (docs/adr/0626 §7). A text's place
//! on the screen is its world's under the camera, so a picture of the texts made for one view is, at the same scale,
//! the picture for another moved: while the view pans, the last picture is laid over the view so, and nothing is
//! laid out again. A zoom draws the texts afresh, every frame: whether a text reads as letters or as a band, and a
//! band's least width, are the scale's, and a scaled picture would show them as the old scale had them. The picture
//! is made again once the view settles or leaves the area it was made for (a quarter of the view round it is drawn
//! too). Grid north, the scale bar and the axes follow the screen: drawn afresh every frame, as they were.

use std::cell::Cell;

use iced::advanced::Renderer as _;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::widget::canvas::{self, Frame};
use iced::{Length, Rectangle, Renderer, Size, Theme, Vector, mouse};
use kentos_render_wgpu::Camera;

use super::Labels;
use crate::app::Message;

/// What is drawn round the view, of its larger side: a pan of this much finds its texts already laid out.
const MARGIN: f32 = 0.25;

/// The texts' layer: the program that lays them out, what its picture depends on with the view (`key`) and
/// without it (`content`), and whether the view is moving.
pub(super) struct Layer<'a> {
    pub(super) labels: Labels<'a>,
    pub(super) key: u64,
    pub(super) content: u64,
    pub(super) moving: bool,
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    key: Cell<u64>,
    content: Cell<u64>,
    /// The view the picture was made for, and its margin in pixels.
    made: Cell<Option<(Camera, f32)>>,
}

impl Layer<'_> {
    /// Whether the picture made for `made` (drawn `margin` pixels round it) still serves the view: the same size,
    /// turn and scale (a pan), the view inside the area drawn.
    fn serves(&self, made: &Camera, margin: f32) -> bool {
        let now = &self.labels.view;
        if made.width != now.width
            || made.height != now.height
            || made.twist != now.twist
            || made.map_px() != now.map_px()
        {
            return false;
        }
        let drawn = made.visible_bounds();
        let m = f64::from(margin) / made.scale;
        let seen = now.visible_bounds();
        drawn.min_x - m <= seen.min_x
            && drawn.min_y - m <= seen.min_y
            && drawn.max_x + m >= seen.max_x
            && drawn.max_y + m >= seen.max_y
    }
}

/// From the picture made for `made` to the view `now` at its scale: the screen's points move with the view's middle
/// (the camera maps a map point `m` to `(m − centre) · px` flipped in y, plus half the area; `Camera::map_to_screen`).
fn moved(made: &Camera, now: &Camera) -> Vector {
    let px = now.map_px();
    Vector::new(
        ((made.center.x - now.center.x) * px) as f32,
        (-(made.center.y - now.center.y) * px) as f32,
    )
}

impl Widget<Message, Theme, Renderer> for Layer<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }
        let state = tree.state.downcast_ref::<State>();
        let margin = (bounds.width.max(bounds.height) * MARGIN).round();
        let serves = self.moving
            && state.content.get() == self.content
            && state
                .made
                .get()
                .is_some_and(|(made, m)| self.serves(&made, m));
        if !serves && state.key.get() != self.key {
            state.cache.clear();
            state.key.set(self.key);
            state.content.set(self.content);
            state.made.set(Some((self.labels.view, margin)));
        }
        let (made, margin) = state.made.get().unwrap_or((self.labels.view, margin));
        let size = Size::new(bounds.width + 2.0 * margin, bounds.height + 2.0 * margin);
        let geometry = state.cache.draw(renderer, size, |frame| {
            frame.translate(Vector::new(margin, margin));
            self.labels.paint_texts(frame, margin);
        });
        let to_view = moved(&made, &self.labels.view) - Vector::new(margin, margin);
        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                renderer.with_translation(to_view, |renderer| {
                    renderer.draw_geometry(geometry);
                });
                // The marks follow the screen, not the drawing.
                if self.labels.map_marks.is_some() {
                    let mut frame = Frame::new(renderer, bounds.size());
                    self.labels.paint_marks(&mut frame);
                    renderer.draw_geometry(frame.into_geometry());
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use kentos_render_wgpu::{Camera, Vec2};

    use super::moved;

    /// A world point where the picture made for one view puts it, moved, is where a panned view puts it.
    #[test]
    fn the_picture_moves_with_the_view() {
        let mut made = Camera::default();
        made.set_size(1440.0, 900.0);
        made.center = Vec2::new(486_512.0, 4_420_118.0);
        made.scale = 3.5;
        let mut panned = made;
        panned.pan_by(37.0, -12.0);
        let mut far = made;
        far.pan_by(-300.5, 180.25);
        for now in [made, panned, far] {
            let by = moved(&made, &now);
            for p in [
                Vec2::new(486_512.0, 4_420_118.0),
                Vec2::new(486_400.5, 4_420_200.25),
                Vec2::new(486_700.0, 4_420_000.0),
            ] {
                let [ax, ay] = made.world_to_screen(p);
                let [bx, by_] = now.world_to_screen(p);
                let (gx, gy) = (ax as f32 + by.x, ay as f32 + by.y);
                assert!(
                    (gx - bx as f32).abs() < 0.01 && (gy - by_ as f32).abs() < 0.01,
                    "({gx}, {gy}) against ({bx}, {by_})"
                );
            }
        }
    }
}
